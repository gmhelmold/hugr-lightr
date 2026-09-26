use super::*;

#[test]
fn arg_conditioned_entry_selects_matching_rule() {
    let r = profile(
        r#"{ "defaultAction": "SCMP_ACT_ERRNO", "defaultErrnoRet": 1,
             "syscalls": [
               { "names": ["ioctl"], "action": "SCMP_ACT_ALLOW",
                 "args": [ { "index": 1, "value": 4294967298, "op": "SCMP_CMP_EQ" } ] },
               { "names": ["ioctl"], "action": "SCMP_ACT_ERRNO", "errnoRet": 13 }
             ] }"#,
    )
    .expect("arg-conditioned rules compile");
    let nr = syscall_nr("ioctl").unwrap() as u32;
    assert_eq!(
        run_bpf_args(
            &r.prog,
            AUDIT_ARCH_X86_64,
            nr,
            [0, 4_294_967_298, 0, 0, 0, 0]
        ),
        SECCOMP_RET_ALLOW,
        "matching 64-bit arg must select first rule"
    );
    assert_eq!(
        run_bpf_args(&r.prog, AUDIT_ARCH_X86_64, nr, [0, 7, 0, 0, 0, 0]),
        SECCOMP_RET_ERRNO | 13,
        "non-matching arg must continue to the next rule"
    );
}

#[test]
fn multiple_arg_conditions_are_anded() {
    let c = profile(
        r#"{ "defaultAction": "SCMP_ACT_ERRNO",
             "syscalls": [ { "names": ["ioctl"], "action": "SCMP_ACT_ALLOW",
               "args": [
                 { "index": 0, "value": 1, "op": "SCMP_CMP_EQ" },
                 { "index": 1, "value": 2, "op": "SCMP_CMP_EQ" }
               ] } ] }"#,
    )
    .expect("multi-arg profile compiles");
    let nr = syscall_nr("ioctl").unwrap() as u32;
    assert_eq!(
        run_bpf_args(&c.prog, AUDIT_ARCH_X86_64, nr, [1, 2, 0, 0, 0, 0]),
        SECCOMP_RET_ALLOW
    );
    assert_eq!(
        run_bpf_args(&c.prog, AUDIT_ARCH_X86_64, nr, [1, 3, 0, 0, 0, 0]),
        SECCOMP_RET_ERRNO | 1
    );
}

#[test]
fn all_arg_operators_have_selective_64_bit_semantics() {
    let cases = [
        (
            "SCMP_CMP_EQ",
            0x1_0000_000a_u64,
            None,
            0x1_0000_000a,
            0x1_0000_000b,
        ),
        (
            "SCMP_CMP_NE",
            0x1_0000_000a,
            None,
            0x1_0000_000b,
            0x1_0000_000a,
        ),
        (
            "SCMP_CMP_LT",
            0x1_0000_000a,
            None,
            0x0_ffff_ffff,
            0x2_0000_0000,
        ),
        (
            "SCMP_CMP_LE",
            0x1_0000_000a,
            None,
            0x1_0000_000a,
            0x2_0000_0000,
        ),
        (
            "SCMP_CMP_GT",
            0x1_0000_000a,
            None,
            0x2_0000_0000,
            0x0_ffff_ffff,
        ),
        (
            "SCMP_CMP_GE",
            0x1_0000_000a,
            None,
            0x1_0000_000a,
            0x0_ffff_ffff,
        ),
        ("SCMP_CMP_MASKED_EQ", 0x10, Some(0xf0), 0x1f, 0x2f),
    ];
    for (op, value, value_two, matching, nonmatching) in cases {
        let value_two_json = value_two
            .map(|v| format!(", \"valueTwo\": {v}"))
            .unwrap_or_default();
        let profile_json = format!(
            r#"{{ "defaultAction": "SCMP_ACT_ERRNO",
                 "syscalls": [ {{ "names": ["ioctl"], "action": "SCMP_ACT_ALLOW",
                   "args": [ {{ "index": 0, "value": {value}, "op": "{op}"{value_two_json} }} ] }} ] }}"#
        );
        let c = profile(&profile_json).expect("operator profile compiles");
        let nr = syscall_nr("ioctl").unwrap() as u32;
        assert_eq!(
            run_bpf_args(&c.prog, AUDIT_ARCH_X86_64, nr, [matching, 0, 0, 0, 0, 0]),
            SECCOMP_RET_ALLOW,
            "{op} must allow matching value"
        );
        assert_eq!(
            run_bpf_args(&c.prog, AUDIT_ARCH_X86_64, nr, [nonmatching, 0, 0, 0, 0, 0]),
            SECCOMP_RET_ERRNO | 1,
            "{op} must use default action for non-matching value"
        );
    }
}

#[test]
fn masked_eq_does_not_mask_expected_value_bits() {
    let c = profile(
        r#"{ "defaultAction": "SCMP_ACT_ERRNO", "syscalls": [
             { "names": ["ioctl"], "action": "SCMP_ACT_ALLOW",
               "args": [{ "index": 0, "value": 17, "valueTwo": 240,
                           "op": "SCMP_CMP_MASKED_EQ" }] } ] }"#,
    )
    .expect("masked-eq profile compiles");
    let nr = syscall_nr("ioctl").unwrap() as u32;
    assert_eq!(
        run_bpf_args(&c.prog, AUDIT_ARCH_X86_64, nr, [0x1f, 0, 0, 0, 0, 0]),
        SECCOMP_RET_ERRNO | 1,
        "unmasked expected bits make this predicate unsatisfiable"
    );
    assert_eq!(
        run_bpf_args(&c.prog, AUDIT_ARCH_X86_64, nr, [0x2f, 0, 0, 0, 0, 0]),
        SECCOMP_RET_ERRNO | 1,
        "masked argument mismatch must use default action"
    );
}

#[test]
fn masked_eq_compares_high_word_without_masking_expected_value() {
    let c = profile(
        r#"{ "defaultAction": "SCMP_ACT_ERRNO", "syscalls": [
             { "names": ["ioctl"], "action": "SCMP_ACT_ALLOW",
               "args": [{ "index": 0, "value": 4294967313, "valueTwo": 240,
                           "op": "SCMP_CMP_MASKED_EQ" }] } ] }"#,
    )
    .expect("high-word masked-eq profile compiles");
    let nr = syscall_nr("ioctl").unwrap() as u32;
    assert_eq!(
        run_bpf_args(
            &c.prog,
            AUDIT_ARCH_X86_64,
            nr,
            [0x2_0000_001f, 0, 0, 0, 0, 0]
        ),
        SECCOMP_RET_ERRNO | 1,
        "unmasked high-word expected bits make this predicate unsatisfiable"
    );
}

#[test]
fn invalid_arg_grammar_fails_closed() {
    for json in [
        r#"{ "defaultAction": "SCMP_ACT_ALLOW", "syscalls": [{ "names": ["ioctl"], "action": "SCMP_ACT_ALLOW", "args": [{ "index": 6, "value": 1, "op": "SCMP_CMP_EQ" }] }] }"#,
        r#"{ "defaultAction": "SCMP_ACT_ALLOW", "syscalls": [{ "names": ["ioctl"], "action": "SCMP_ACT_ALLOW", "args": [{ "index": 0, "value": 1, "op": "SCMP_CMP_UNKNOWN" }] }] }"#,
        r#"{ "defaultAction": "SCMP_ACT_ALLOW", "syscalls": [{ "names": ["ioctl"], "action": "SCMP_ACT_ALLOW", "args": [{ "index": 0, "value": 1, "op": "SCMP_CMP_EQ", "valueTwo": 1 }] }] }"#,
        r#"{ "defaultAction": "SCMP_ACT_ALLOW", "syscalls": [{ "names": ["ioctl"], "action": "SCMP_ACT_ALLOW", "args": [{ "index": 0, "value": 1, "op": "SCMP_CMP_MASKED_EQ" }] }] }"#,
        r#"{ "defaultAction": "SCMP_ACT_ALLOW", "defaultErrnoRet": 1, "syscalls": [] }"#,
        r#"{ "defaultAction": "SCMP_ACT_ALLOW", "syscalls": [{ "names": ["ioctl"], "action": "SCMP_ACT_ALLOW", "errnoRet": 1 }] }"#,
        r#"{ "defaultAction": "SCMP_ACT_ALLOW", "syscalls": [{ "names": ["ioctl"], "action": "SCMP_ACT_KILL", "errnoRet": 1 }] }"#,
        r#"{ "defaultAction": "SCMP_ACT_ALLOW", "archMap": [], "syscalls": [] }"#,
        r#"{ "defaultAction": "SCMP_ACT_ALLOW", "syscalls": [{ "names": ["ioctl"], "action": "SCMP_ACT_ALLOW", "includes": { "caps": ["CAP_SYS_ADMIN"] } }] }"#,
        r#"{ "defaultAction": "SCMP_ACT_ALLOW", "unknownField": true, "syscalls": [] }"#,
    ] {
        assert!(
            profile(json).is_err(),
            "invalid seccomp arg grammar must fail closed"
        );
    }
    assert_eq!(
        profile(
            r#"{ "defaultAction": "SCMP_ACT_ALLOW", "architectures": ["SCMP_ARCH_AARCH64"], "syscalls": [] }"#,
        )
        .is_err(),
        cfg!(target_arch = "x86_64"),
        "profile architecture must match the active seccomp ABI"
    );
}

#[test]
fn default_errno_ret_controls_default_action() {
    let c =
        profile(r#"{ "defaultAction": "SCMP_ACT_ERRNO", "defaultErrnoRet": 13, "syscalls": [] }"#)
            .expect("default errno profile compiles");
    assert_eq!(
        run_bpf(
            &c.prog,
            AUDIT_ARCH_X86_64,
            syscall_nr("read").unwrap() as u32
        ),
        SECCOMP_RET_ERRNO | 13
    );
}

#[cfg(target_os = "linux")]
#[test]
fn kernel_enforces_arg_condition_and_mixed_actions() {
    // Control: prove the host test environment permits both calls before the
    // child adds its filter. The child then allows only AF_UNIX.
    unsafe {
        let control_unix = libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0);
        assert!(control_unix >= 0, "control AF_UNIX socket failed");
        libc::close(control_unix);
        let control_inet = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        assert!(control_inet >= 0, "control AF_INET socket failed");
        libc::close(control_inet);
    }

    let c = profile(
        r#"{ "defaultAction": "SCMP_ACT_ERRNO", "defaultErrnoRet": 1,
             "syscalls": [
               { "names": ["socket"], "action": "SCMP_ACT_ALLOW",
                 "args": [ { "index": 0, "value": 1, "op": "SCMP_CMP_EQ" } ] },
               { "names": ["exit_group"], "action": "SCMP_ACT_ALLOW" }
             ] }"#,
    )
    .expect("runtime profile compiles");

    unsafe {
        let child = libc::fork();
        assert!(child >= 0, "fork failed");
        if child == 0 {
            if c.apply().is_err() {
                libc::_exit(101);
            }
            let allowed = libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0);
            if allowed < 0 {
                libc::_exit(102);
            }
            let denied = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
            let denied_errno = *libc::__errno_location();
            if denied != -1 || denied_errno != libc::EPERM {
                libc::_exit(103);
            }
            libc::_exit(0);
        }
        let mut status = 0;
        assert_eq!(libc::waitpid(child, &mut status, 0), child);
        assert!(libc::WIFEXITED(status));
        assert_eq!(libc::WEXITSTATUS(status), 0);
    }
}
