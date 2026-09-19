# PR #169: PTY descriptor ownership before exec

## Observed counterexample

The completed log of macOS14 job 105834928095 in CI 35419706288 identifies
`closing_the_master_releases_the_exiting_child` as the unfinished test.
Thirty other CRI methods completed; this one exceeded 60 seconds and the job
was cancelled after approximately 40 minutes. The exact blocked native stack
was not captured. Source 97e3f09 changed only diagnostic gates, so its green
runs alone could not establish that this counterexample had been removed.
The original log hash is 7c24aaec85ef5ff2ebd84c25898ae4e6ed420d6e206844b7f07eb3e78cc626fc.

## Demonstrable defect and correction

The original openpty and dup calls create inheritable descriptors. The PTY
fixture sets FD_CLOEXEC only afterwards, leaving an allocation-to-fcntl race
with concurrent child creation. Production open_exec_tty does not even perform
that later flag update. Inheriting a master keeps a reference alive after the
consumer closes its own handles; a last-master disconnect is no longer assured.
This is a concrete ownership defect, not a recovered stack of the old failure.

A bounded C observation on the owner's Intel macOS15.3.2 returned flags0 for
both original endpoints and dup, versus flags1 for both endpoints opened with
O_CLOEXEC. The replacement retained native terminal identity on both endpoints.
The Rust regressions separately verify endpoint flags, duplicate flags and
actual absence of the original endpoint identities after an unrelated exec.
Descriptor numbers reused for other files are not misclassified as leaks.

Darwin now opens /dev/ptmx and its TIOCPTYGNAME-selected slave with O_CLOEXEC
and O_NOCTTY atomically; grant/unlock errors propagate, and owned Files release
partial acquisitions. No shared ptsname buffer, fallback to inheritable fds,
parent-held slave, relay or output buffer is introduced. dup_file uses the
standard library's close-on-exec clone on Unix. Intentional stdio transfer
continues through Command's existing stdin/stdout/stderr configuration.
Linux PTY allocation, namespace setup and public StreamSession remain unchanged.

## Five acceptance groups

**Success criteria:** no unrequested PTY endpoint survives exec; native output,
EOF, disconnect and real exit status retain the original behavior assertions.

**Completeness criteria:** both newly allocated endpoints, both duplicate kinds,
post-exec descriptor identity, original echo and complete CRI composition across
all four configured macOS profiles. Child probe is a helper, not another case.

**Quality standards:** Rust1.96.0; deny warnings; bounded native CI; fallible
owned descriptors; existing PTY and Windows-link controls remain required.
New compiled controls must detect removal of atomic CLOEXEC and inheritable dup.

**Invariants:** no retry-until-green, swallowed I/O error, broad serialization,
public vocabulary/dependency change, borrowed-fd close, namespace-policy change,
main integration or storage protocol activation.

**Definition of Done:** record the actual before/after regression results;
review exact source and all native/full/causal gates; expected-head integration
only after success; verify fresh parent CI and retire only the delivery branch.
The previous cancelled log remains evidence, never reclassified as a pass.
Results and remaining uncertainty belong in the current PR review.

Primary sources: Rust1.96.0 std os/fd/owned.rs (atomic descriptor clone),
Apple XNU bsd/kern/tty_dev.c and kern_exit.c (master references and exit drain).
