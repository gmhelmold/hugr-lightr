//! WP-#107/#108 seccomp compiler unit tests (cBPF `run_bpf` simulator + shape/
//! selectivity/overflow guards). Extracted for the <=400-LOC godfile invariant.

use super::*;

mod arg_rules;

fn profile(json: &str) -> std::io::Result<CompiledSeccomp> {
    let p: OciSeccomp = serde_json::from_str(json)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    compile(&p)
}

/// Minimal cBPF interpreter for the exact subset our compiler emits
/// (`LD|W|ABS`, `JMP|JEQ|K`, `RET|K`). Returns the `SECCOMP_RET_*` value the
/// program yields for a given `(arch, nr)`. This is what pins the CONTROL
/// FLOW — the #108 first cut compiled to a valid program with the right ret
/// VALUES but the wrong fall-through, which an index/value assertion missed
/// but this simulator catches.
fn run_bpf(prog: &[libc::sock_filter], arch: u32, nr: u32) -> u32 {
    run_bpf_args(prog, arch, nr, [0; 6])
}

fn run_bpf_args(prog: &[libc::sock_filter], arch: u32, nr: u32, args: [u64; 6]) -> u32 {
    let mut pc = 0usize;
    let mut acc: u32 = 0;
    loop {
        let insn = prog[pc];
        if insn.code == BPF_LD | BPF_W | BPF_ABS {
            acc = match insn.k {
                SECCOMP_DATA_NR_OFFSET => nr,
                SECCOMP_DATA_ARCH_OFFSET => arch,
                offset
                    if (SECCOMP_DATA_ARGS_OFFSET..SECCOMP_DATA_ARGS_OFFSET + 48)
                        .contains(&offset) =>
                {
                    let arg_offset = offset - SECCOMP_DATA_ARGS_OFFSET;
                    let index = (arg_offset / 8) as usize;
                    let word = (arg_offset % 8) as usize;
                    if word == 0 {
                        args[index] as u32
                    } else if word == 4 {
                        (args[index] >> 32) as u32
                    } else {
                        panic!("unexpected arg offset {offset}")
                    }
                }
                other => panic!("unexpected LD offset {other}"),
            };
            pc += 1;
        } else if insn.code == BPF_ALU | BPF_AND | BPF_K {
            acc &= insn.k;
            pc += 1;
        } else if insn.code == BPF_JMP | BPF_JEQ | BPF_K {
            let taken = if acc == insn.k { insn.jt } else { insn.jf } as usize;
            pc += 1 + taken;
        } else if insn.code == BPF_JMP | BPF_JGT | BPF_K {
            let taken = if acc > insn.k { insn.jt } else { insn.jf } as usize;
            pc += 1 + taken;
        } else if insn.code == BPF_JMP | BPF_JGE | BPF_K {
            let taken = if acc >= insn.k { insn.jt } else { insn.jf } as usize;
            pc += 1 + taken;
        } else if insn.code == BPF_JMP | BPF_JSET | BPF_K {
            let taken = if acc & insn.k != 0 { insn.jt } else { insn.jf } as usize;
            pc += 1 + taken;
        } else if insn.code == BPF_JMP | BPF_JA {
            pc += 1 + insn.k as usize;
        } else if insn.code == BPF_RET | BPF_K {
            return insn.k;
        } else {
            panic!("unexpected opcode {:#x}", insn.code);
        }
    }
}

const I386_ARCH: u32 = 0x4000_0003; // AUDIT_ARCH_I386 — a "foreign" arch here.

#[test]
fn deny_list_selectively_blocks_only_listed_syscalls() {
    // default ALLOW, mkdir/mkdirat → ERRNO(EPERM). The filter MUST block ONLY
    // mkdir/mkdirat and ALLOW everything else (the bug was: it blocked all).
    let c = profile(
            r#"{ "defaultAction": "SCMP_ACT_ALLOW",
                 "syscalls": [ { "names": ["mkdir","mkdirat"], "action": "SCMP_ACT_ERRNO", "errnoRet": 1 } ] }"#,
        )
        .expect("supported profile compiles");
    // ld arch, arch-kill, initial ld nr, x32-kill, 2×(ld nr,jeq,ja,ret), final ret = 15.
    assert_eq!(
        c.prog.len(),
        15,
        "expected rule-skip 13-insn program (offset-safe)"
    );
    let nr = |n| syscall_nr(n).unwrap() as u32;
    assert_eq!(
        run_bpf(&c.prog, AUDIT_ARCH_X86_64, nr("mkdir")),
        SECCOMP_RET_ERRNO | 1,
        "mkdir must be ERRNO(EPERM)"
    );
    assert_eq!(
        run_bpf(&c.prog, AUDIT_ARCH_X86_64, nr("mkdirat")),
        SECCOMP_RET_ERRNO | 1,
        "mkdirat must be ERRNO(EPERM)"
    );
    // THE REGRESSION GUARD: a non-listed syscall MUST fall through to default ALLOW.
    assert_eq!(
        run_bpf(&c.prog, AUDIT_ARCH_X86_64, nr("write")),
        SECCOMP_RET_ALLOW,
        "non-listed syscall (write) MUST fall through to default ALLOW, not the listed action"
    );
    assert_eq!(
        run_bpf(&c.prog, AUDIT_ARCH_X86_64, nr("execve")),
        SECCOMP_RET_ALLOW,
        "non-listed syscall (execve) MUST be ALLOW"
    );
    // Foreign and x32 execution must never inherit default ALLOW.
    assert_eq!(
        run_bpf(&c.prog, I386_ARCH, nr("mkdir")),
        SECCOMP_RET_KILL_PROCESS,
        "foreign arch must fail closed"
    );
    assert_eq!(
        run_bpf(&c.prog, AUDIT_ARCH_X86_64, nr("mkdir") | X32_SYSCALL_BIT),
        SECCOMP_RET_KILL_PROCESS,
        "x32 syscall ABI must fail closed"
    );
}

#[test]
fn allow_list_default_deny_allows_only_listed() {
    // default ERRNO (deny), allow only write — the inverse shape. Proves the
    // compiler is correct for allow-lists too (default-deny is the Docker shape).
    let c = profile(
        r#"{ "defaultAction": "SCMP_ACT_ERRNO",
                 "syscalls": [ { "names": ["write"], "action": "SCMP_ACT_ALLOW" } ] }"#,
    )
    .expect("allow-list profile compiles");
    let nr = |n| syscall_nr(n).unwrap() as u32;
    assert_eq!(
        run_bpf(&c.prog, AUDIT_ARCH_X86_64, nr("write")),
        SECCOMP_RET_ALLOW,
        "listed write must be ALLOW"
    );
    assert_eq!(
        run_bpf(&c.prog, AUDIT_ARCH_X86_64, nr("mkdir")),
        SECCOMP_RET_ERRNO | 1,
        "non-listed syscall must hit default ERRNO"
    );
}

#[test]
fn default_only_profile_compiles() {
    let c = profile(r#"{ "defaultAction": "SCMP_ACT_ERRNO", "syscalls": [] }"#)
        .expect("default-only compiles");
    // ld arch, arch-kill, ld nr, x32-kill, listed-ret, default-ret = 7.
    assert_eq!(c.prog.len(), 7);
}

#[test]
fn unknown_syscall_name_is_unsupported() {
    let r = profile(
        r#"{ "defaultAction": "SCMP_ACT_ALLOW",
                 "syscalls": [ { "names": ["totally_not_a_syscall"], "action": "SCMP_ACT_ERRNO" } ] }"#,
    );
    assert!(r.is_err(), "unknown syscall name must fail closed");
}

#[test]
fn mixed_actions_are_selected_per_rule() {
    let r = profile(
        r#"{ "defaultAction": "SCMP_ACT_ALLOW",
                 "syscalls": [ { "names": ["mkdir"], "action": "SCMP_ACT_ERRNO", "errnoRet": 13 },
                               { "names": ["rmdir"], "action": "SCMP_ACT_KILL" } ] }"#,
    )
    .expect("mixed per-syscall actions compile");
    let nr = |n| syscall_nr(n).unwrap() as u32;
    assert_eq!(
        run_bpf(&r.prog, AUDIT_ARCH_X86_64, nr("mkdir")),
        SECCOMP_RET_ERRNO | 13
    );
    assert_eq!(
        run_bpf(&r.prog, AUDIT_ARCH_X86_64, nr("rmdir")),
        SECCOMP_RET_KILL_THREAD
    );
    assert_eq!(
        run_bpf(&r.prog, AUDIT_ARCH_X86_64, nr("write")),
        SECCOMP_RET_ALLOW
    );
}

#[test]
fn builtin_default_profile_compiles_and_is_default_deny() {
    // The vendored `seccomp_default.json` parses + compiles (every listed name
    // must resolve in `syscall_nr`, or this fails closed) ...
    let c = compile_default().expect("built-in default profile compiles");
    let nr = |n| syscall_nr(n).unwrap() as u32;
    // ... a representative ALLOWED syscall falls through to the ALLOW block ...
    assert_eq!(
        run_bpf(&c.prog, AUDIT_ARCH_X86_64, nr("read")),
        SECCOMP_RET_ALLOW,
        "an allow-listed syscall (read) must be ALLOW"
    );
    assert_eq!(
        run_bpf(&c.prog, AUDIT_ARCH_X86_64, nr("arch_prctl")),
        SECCOMP_RET_ALLOW,
        "arch_prctl (C-runtime startup) must be ALLOW or every workload traps"
    );
    // ... and a syscall NOT in the allow-list hits the default ERRNO(EPERM),
    // proving this is a real default-deny allow-list (not allow-all).
    assert_eq!(
        run_bpf(&c.prog, AUDIT_ARCH_X86_64, nr("ptrace")),
        SECCOMP_RET_ERRNO | 1,
        "a non-allow-listed syscall (ptrace) must be ERRNO(EPERM)"
    );
    assert_eq!(
        run_bpf(&c.prog, AUDIT_ARCH_X86_64, nr("reboot")),
        SECCOMP_RET_ERRNO | 1,
        "a non-allow-listed syscall (reboot) must be ERRNO(EPERM)"
    );
    // OVERFLOW GUARD: the default profile has ~289 entries, so its cBPF program
    // is >256 instructions. With the old far-jump layout the early JEQs' u8 jt
    // overflowed → wrong mapping; these allow-listed syscalls span EARLY
    // (arch_prctl above), MID, and LATE positions — all must still be ALLOW,
    // proving the inline-RET layout has no positional truncation at any size.
    for name in [
        "openat",
        "futex",
        "getrandom",
        "writev",
        "exit_group",
        "wait4",
    ] {
        assert_eq!(
                run_bpf(&c.prog, AUDIT_ARCH_X86_64, nr(name)),
                SECCOMP_RET_ALLOW,
                "allow-listed syscall {name} must be ALLOW regardless of its position in a >256-entry profile"
            );
    }
    // Foreign and x32 execution fail closed even though native default is ERRNO.
    assert_eq!(
        run_bpf(&c.prog, I386_ARCH, nr("read")),
        SECCOMP_RET_KILL_PROCESS,
        "foreign arch must fail closed"
    );
    assert_eq!(
        run_bpf(&c.prog, AUDIT_ARCH_X86_64, nr("read") | X32_SYSCALL_BIT),
        SECCOMP_RET_KILL_PROCESS,
        "x32 syscall ABI must fail closed"
    );
}

#[test]
fn unsupported_default_action_is_rejected() {
    let r = profile(r#"{ "defaultAction": "SCMP_ACT_TRACE", "syscalls": [] }"#);
    assert!(r.is_err(), "unsupported defaultAction must fail closed");
}

#[test]
fn oversized_filter_fails_closed() {
    let entries = std::iter::repeat_n(
        r#"{ "names": ["read"], "action": "SCMP_ACT_ERRNO", "errnoRet": 1 }"#,
        1100,
    )
    .collect::<Vec<_>>()
    .join(",");
    let json = format!(r#"{{ "defaultAction": "SCMP_ACT_ALLOW", "syscalls": [{entries}] }}"#);
    assert!(profile(&json).is_err(), "oversized filter must fail closed");
}
