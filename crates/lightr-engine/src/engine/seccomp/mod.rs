//! WP-#108 (seccomp): hand-rolled OCI seccomp profile → classic-BPF (cBPF)
//! compiler + apply, for the rootless `ns` engine. ZERO new crate dependencies
//! (no `seccompiler`, no `libseccomp`) — just `serde_json` (already a workspace
//! dep) for the profile parse and raw `libc` for the install.
//!
//! Scope (FROZEN, fail-closed): supported OCI rules compile without broadening
//! a rule on parse or code-generation failure. Anything outside the supported
//! set returns an `io::Error` so the caller (PID 1, pre-execv) `_exit`s rather
//! than exec under a WRONG/absent filter — the same fail-closed discipline as
//! #106 AppArmor.
//!
//! SUPPORTED:
//!   * `defaultAction` ∈ {ALLOW, ERRNO, KILL, KILL_PROCESS, KILL_THREAD}.
//!   * Per-syscall entries with zero or more OCI `args` conditions. Conditions
//!     are ANDed, and entries are evaluated in profile order.
//!   * Per-syscall entries may use different actions. The first matching entry
//!     returns its action; no match returns `defaultAction`.
//!   * Syscall NAMEs resolved → numbers via a `libc::SYS_*` table (so the numbers
//!     are target-correct). An unknown name FAILS CLOSED.
//!   * x86_64 architecture only. `archMap`, `includes`, and `excludes` are
//!     rejected rather than silently ignored; capability-aware profiles need a
//!     later runtime contract.
//!
//! The compiled filter is a cBPF decision tree:
//!   load arch → (foreign arch ⇒ KILL_PROCESS) → load nr → x32 guard → ordered rules;
//!   each rule tests syscall number, then 64-bit arguments, then returns its
//!   action. Rule skips use `JA` (32-bit offsets), never u8 conditional jumps.

#![cfg(target_os = "linux")]

use std::io::{Error, ErrorKind, Read};

// WP godfile-split: the syscall-name→number table (~320 LOC on its own) lives in
// `syscalls.rs`; the compiler core + apply stay here. `syscall_nr` is `pub(super)`.
mod bpf;
mod syscalls;
use bpf::{ArgCondition, ArgOp, Rule};
use syscalls::syscall_nr;

// ── seccomp_data field offsets (uapi/linux/seccomp.h `struct seccomp_data`) ──────
const SECCOMP_DATA_NR_OFFSET: u32 = 0; // u32 nr
const SECCOMP_DATA_ARCH_OFFSET: u32 = 4; // u32 arch
const SECCOMP_DATA_ARGS_OFFSET: u32 = 16; // u64 args[0]
const SECCOMP_MAX_ARGS: u32 = 6;

// ── audit arch (uapi/linux/audit.h). x86_64 only (this engine's validated arch). ─
const AUDIT_ARCH_X86_64: u32 = 0xC000_003E;
const X32_SYSCALL_BIT: u32 = 0x4000_0000;
const SECCOMP_MAX_FILTER_INSNS: usize = 4096;

// ── seccomp return actions (uapi/linux/seccomp.h) ───────────────────────────────
const SECCOMP_RET_KILL_PROCESS: u32 = 0x8000_0000;
const SECCOMP_RET_KILL_THREAD: u32 = 0x0000_0000; // a.k.a. SECCOMP_RET_KILL
const SECCOMP_RET_ERRNO: u32 = 0x0005_0000; // | (errno & 0xffff)
const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;

// ── classic-BPF opcodes (uapi/linux/bpf_common.h) ───────────────────────────────
const BPF_LD: u16 = 0x00;
const BPF_W: u16 = 0x00;
const BPF_ABS: u16 = 0x20;
const BPF_JMP: u16 = 0x05;
const BPF_JA: u16 = 0x00;
const BPF_JEQ: u16 = 0x10;
const BPF_JGT: u16 = 0x20;
const BPF_JGE: u16 = 0x30;
const BPF_JSET: u16 = 0x40;
const BPF_ALU: u16 = 0x04;
const BPF_AND: u16 = 0x50;
const BPF_K: u16 = 0x00;
const BPF_RET: u16 = 0x06;

// ── seccomp(2) / prctl seccomp constants (uapi/linux/seccomp.h, sys/prctl.h) ─────
const SECCOMP_SET_MODE_FILTER: libc::c_ulong = 1;
const SECCOMP_MODE_FILTER: libc::c_int = 2;

// ── OCI seccomp profile JSON (the runtime-spec subset we accept) ─────────────────

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
struct OciSeccomp {
    default_action: String,
    #[serde(default)]
    default_errno_ret: Option<u32>,
    #[serde(rename = "_comment", default)]
    #[allow(dead_code)]
    _comment: Option<String>,
    #[serde(default)]
    architectures: Vec<String>,
    #[serde(default)]
    arch_map: Option<serde_json::Value>,
    #[serde(default)]
    syscalls: Vec<OciSyscall>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
struct OciSyscall {
    names: Vec<String>,
    action: String,
    #[serde(default)]
    errno_ret: Option<u32>,
    #[serde(default)]
    args: Vec<OciArg>,
    #[serde(default)]
    includes: Option<serde_json::Value>,
    #[serde(default)]
    excludes: Option<serde_json::Value>,
    #[serde(default)]
    #[allow(dead_code)]
    comment: Option<String>,
}

#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
struct OciArg {
    index: u32,
    value: u64,
    #[serde(default)]
    value_two: Option<u64>,
    op: String,
}

/// Map an OCI `SCMP_ACT_*` string + optional errnoRet to a `SECCOMP_RET_*` value.
/// Unknown action ⇒ `None` (caller fails closed).
fn action_ret(action: &str, errno_ret: Option<u32>) -> Option<u32> {
    Some(match action {
        "SCMP_ACT_ALLOW" if errno_ret.is_none() => SECCOMP_RET_ALLOW,
        "SCMP_ACT_ERRNO" => {
            // errno = low 16 bits; default 1 (EPERM) when unspecified.
            let errno = errno_ret.unwrap_or(1);
            if errno > 0xffff {
                return None;
            }
            SECCOMP_RET_ERRNO | errno
        }
        "SCMP_ACT_KILL" | "SCMP_ACT_KILL_THREAD" if errno_ret.is_none() => SECCOMP_RET_KILL_THREAD,
        "SCMP_ACT_KILL_PROCESS" if errno_ret.is_none() => SECCOMP_RET_KILL_PROCESS,
        _ => return None,
    })
}

/// A compiled, ready-to-install classic-BPF seccomp filter.
pub struct CompiledSeccomp {
    prog: Vec<libc::sock_filter>,
}

/// Read + parse + compile an OCI seccomp JSON profile at `path` into a cBPF
/// program. Fails closed (`io::Error`) on any unsupported shape so the caller
/// never execs under a wrong/absent filter.
pub fn compile_from_path(path: &str) -> std::io::Result<CompiledSeccomp> {
    let mut buf = String::new();
    std::fs::File::open(path)?.read_to_string(&mut buf)?;
    let profile: OciSeccomp = serde_json::from_str(&buf).map_err(|e| {
        Error::new(
            ErrorKind::InvalidData,
            format!("seccomp profile parse: {e}"),
        )
    })?;
    compile(&profile)
}

/// Compile the BUILT-IN `--seccomp default` curated profile (vendored
/// `seccomp_default.json`): a default-deny (ERRNO/EPERM) allow-list derived from
/// the Docker/moby default profile, filtered to the x86_64 names `syscall_nr`
/// resolves. Same fail-closed `compile` path as `compile_from_path`, but the
/// profile is embedded at build time (`include_str!`) so it needs no host file.
pub fn compile_default() -> std::io::Result<CompiledSeccomp> {
    const DEFAULT_PROFILE: &str = include_str!("seccomp_default.json");
    let profile: OciSeccomp = serde_json::from_str(DEFAULT_PROFILE).map_err(|e| {
        Error::new(
            ErrorKind::InvalidData,
            format!("built-in seccomp profile parse: {e}"),
        )
    })?;
    compile(&profile)
}

fn err_unsupported(msg: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidData, msg.into())
}

fn compile(profile: &OciSeccomp) -> std::io::Result<CompiledSeccomp> {
    if profile.arch_map.is_some() {
        return Err(err_unsupported(
            "seccomp archMap selectors are unsupported on x86_64",
        ));
    }
    if profile
        .architectures
        .iter()
        .any(|arch| arch != "SCMP_ARCH_X86_64")
    {
        return Err(err_unsupported(
            "seccomp profile targets an unsupported architecture",
        ));
    }

    // Default action — must be a supported shape.
    let default_ret =
        action_ret(&profile.default_action, profile.default_errno_ret).ok_or_else(|| {
            err_unsupported(format!(
                "unsupported seccomp defaultAction: {}",
                profile.default_action
            ))
        })?;

    // Expand each OCI entry into one ordered rule per syscall name. Keeping
    // entries separate preserves mixed actions and duplicate syscall entries
    // whose argument predicates differ.
    let mut rules = Vec::new();
    for sc in &profile.syscalls {
        if sc.includes.is_some() || sc.excludes.is_some() {
            return Err(err_unsupported(
                "seccomp syscall capability or architecture selectors are unsupported",
            ));
        }
        let ret = action_ret(&sc.action, sc.errno_ret).ok_or_else(|| {
            err_unsupported(format!("unsupported seccomp syscall action: {}", sc.action))
        })?;
        if sc.names.is_empty() {
            return Err(err_unsupported("seccomp syscall entry has no names"));
        }
        let args = sc
            .args
            .iter()
            .map(parse_arg)
            .collect::<std::io::Result<Vec<_>>>()?;
        for name in &sc.names {
            let nr = syscall_nr(name).ok_or_else(|| {
                err_unsupported(format!("unsupported syscall in seccomp profile: {name}"))
            })?;
            rules.push(Rule {
                nr: u32::try_from(nr)
                    .map_err(|_| err_unsupported(format!("invalid syscall number for {name}")))?,
                action: ret,
                args: args.clone(),
            });
        }
    }

    let prog = bpf::compile(default_ret, &rules)?;
    if prog.len() > SECCOMP_MAX_FILTER_INSNS {
        return Err(err_unsupported(format!(
            "seccomp filter has {} instructions; kernel limit is {}",
            prog.len(),
            SECCOMP_MAX_FILTER_INSNS
        )));
    }
    Ok(CompiledSeccomp { prog })
}

fn parse_arg(arg: &OciArg) -> std::io::Result<ArgCondition> {
    if arg.index >= SECCOMP_MAX_ARGS {
        return Err(err_unsupported(format!(
            "seccomp argument index {} is outside 0..{}",
            arg.index,
            SECCOMP_MAX_ARGS - 1
        )));
    }
    let op = match arg.op.as_str() {
        "SCMP_CMP_EQ" => ArgOp::Eq,
        "SCMP_CMP_NE" => ArgOp::Ne,
        "SCMP_CMP_LT" => ArgOp::Lt,
        "SCMP_CMP_LE" => ArgOp::Le,
        "SCMP_CMP_GT" => ArgOp::Gt,
        "SCMP_CMP_GE" => ArgOp::Ge,
        "SCMP_CMP_MASKED_EQ" => ArgOp::MaskedEq,
        _ => {
            return Err(err_unsupported(format!(
                "unsupported seccomp argument op: {}",
                arg.op
            )))
        }
    };
    if matches!(op, ArgOp::MaskedEq) != arg.value_two.is_some() {
        return Err(err_unsupported(
            "seccomp valueTwo is required only for SCMP_CMP_MASKED_EQ",
        ));
    }
    let offset = SECCOMP_DATA_ARGS_OFFSET + arg.index * 8;
    let value_two = arg.value_two.unwrap_or(0);
    Ok(ArgCondition {
        low_offset: offset,
        high_offset: offset + 4,
        value_low: arg.value as u32,
        value_high: (arg.value >> 32) as u32,
        value_two_low: value_two as u32,
        value_two_high: (value_two >> 32) as u32,
        op,
    })
}

#[inline]
fn stmt(code: u16, k: u32) -> libc::sock_filter {
    libc::sock_filter {
        code,
        jt: 0,
        jf: 0,
        k,
    }
}

#[inline]
fn jump(code: u16, k: u32, jt: u8, jf: u8) -> libc::sock_filter {
    libc::sock_filter { code, jt, jf, k }
}

impl CompiledSeccomp {
    /// Install the compiled filter on the CURRENT thread/process. Sets
    /// `NO_NEW_PRIVS` first (required for an unprivileged seccomp filter), then
    /// installs via `seccomp(2)` (preferred), falling back to
    /// `prctl(PR_SET_SECCOMP)` on an `ENOSYS` kernel. Returns `Err` on any
    /// failure so the caller fails closed.
    pub fn apply(&self) -> std::io::Result<()> {
        // 1. NO_NEW_PRIVS — without it the kernel rejects an unprivileged filter.
        let r = unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
        if r != 0 {
            return Err(Error::last_os_error());
        }

        let len = u16::try_from(self.prog.len())
            .map_err(|_| Error::new(ErrorKind::InvalidData, "seccomp filter is too large"))?;
        let fprog = libc::sock_fprog {
            len,
            filter: self.prog.as_ptr() as *mut libc::sock_filter,
        };

        // 2. Install via seccomp(2); fall back to prctl on ENOSYS.
        let r = unsafe {
            libc::syscall(
                libc::SYS_seccomp,
                SECCOMP_SET_MODE_FILTER,
                0u64,
                &fprog as *const _,
            )
        };
        if r == 0 {
            return Ok(());
        }
        let e = Error::last_os_error();
        if e.raw_os_error() == Some(libc::ENOSYS) {
            let r = unsafe {
                libc::prctl(
                    libc::PR_SET_SECCOMP,
                    SECCOMP_MODE_FILTER,
                    &fprog as *const _,
                    0,
                    0,
                )
            };
            if r == 0 {
                return Ok(());
            }
            return Err(Error::last_os_error());
        }
        Err(e)
    }
}

#[cfg(test)]
mod tests;
