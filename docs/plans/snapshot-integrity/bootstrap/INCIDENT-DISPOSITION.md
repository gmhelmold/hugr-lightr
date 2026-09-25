# SI-00 incident dispositions — actual CLI evidence, 2026-09-17

Executable source head: `98f5d3bc1146cbff964f8495a515e40c8e621c16`.
Actual native merge checkout: `3952a49031884535baa46ba01ed5c52a4f09e696`.
CLI SHA-256: `67ac37728cce96d35e437f0146578657be9e4c810dfe358d4f7b9599c71e9e64`.
CI run: `35244264875`; raw CLI/trace artifact: `10506463555`.
These observations do not certify the proposed runtime protocol or close a fix.

## Path spelling — reproduced through the unmodified production CLI

`diagnose_capture.py` uses separate synthetic source, Store/home and destination
roots. Ordinary-file and ordinary-dangling-link controls both snapshot/hydrate
successfully and independently match the originals. Source bytes/links remain
unchanged in all four cases.

A source containing only `literal\backslash` fails snapshot with ENOENT. The
file-syscall trace shows a successful stat/open of `source/literal\\backslash`,
followed later by an attempted open of `source/literal/backslash` and ENOENT.
This is the actual wrong-path open, not an inference from a generic errno.
`scan.rs` replaces backslashes in the recorded relative name; `snapshot.rs`
reconstructs the ingestion path from that changed name. Owner: SI-02/#148.

A separate source containing only `link -> absent\leaf` snapshots successfully.
Hydrate **with `--verify` also exits zero**, but readlink on the result is
`absent/leaf`. The trace records `symlink("absent/leaf", ...)`. The independent
source/restored-tree comparator reports TREE_MISMATCH. File-hash verification
cannot correct a link target already transformed in the manifest. This is a
reproduced fidelity bug, not merely an anticipated failure. Owner: SI-02/#148,
with public-reader regression coverage in SI-03/E12/E14.

These minimal cases explain the path defect present in the old F5 fixture. They
do not establish the historical macOS acceptance incident's cause. In the latest
untraced baseline, execution failed earlier at F3-08-new; F0-F2 each completed
20 measured samples, F3 retained five, and F4/F5 were not reached. Do not present
this run as a complete F0-F5 or no-regression result.

## Shared temporary ownership — controlled public-path reproduction

The archived, hash-verified Linux CLI was run in the authoring container
(Linux x86_64, glibc 2.41). `probe_temp_collision.py` builds a **test-only** C
interposer with cc 14.2.0 and supplies it only to an isolated subprocess.
It preserves monotonic time, freezes CLOCK_REALTIME (used for the legacy suffix),
and holds the first four successful CAS temporary opens until all four have
arrived. Real filesystem operations and source bytes are not stubbed. A barrier
or compilation failure is INVALID_DIAGNOSTIC, never a causal test success.

| Control / witness | Actual observation |
|---|---|
| Ordinary clock, four workers | Snapshot exit 0; current reference created |
| Fixed clock, one worker | Snapshot exit 0; one staged open; no watchdog timeout |
| Fixed clock, four workers | Four different threads opened the SAME pathname and inode; exit 1 with ENOENT; no current reference created; no watchdog timeout |

All sources stayed unchanged. This proves the baseline lacks exclusive ownership
when its temporary-name generator collides. The original error propagation did
prevent that failing operation from publishing its ref. It does **not** measure
natural collision probability or retrospectively attribute every F1/F3/macOS
ENOENT to this mechanism. SI-01/#147 must replace name-based ownership with an
exclusive reservation and a public-path causal regression. E01/E02 remain future
remediation obligations; this is not a claim they already pass the fixed code.

The separate traced 20-attempt, 200-identical-file observation in the CI run had
no failures. That neither erases the untraced F3 failure nor qualifies concurrency;
tracing changes scheduling and the fixture is smaller than F3.

## Reproduce the controlled witness

Use the recorded executable and a fresh output directory on Linux/glibc:

```sh
python3 scripts/si00/probe_temp_collision.py \
  --binary /path/to/verified/lightr-baseline \
  --expected-binary-sha256 67ac37728cce96d35e437f0146578657be9e4c810dfe358d4f7b9599c71e9e64 \
  --out fresh-collision-evidence
```

A zero exit from this diagnostic means SHARED_STAGING_REPRODUCED, **not** that
Lightr passed. It is intentionally not installed as a production acceptance gate.
After remediation, replace it with a witness of exclusive staging that exercises
the corrected allocator, rather than requiring the legacy bug to remain.

Windows checkout provenance was already repaired and remains verified. Formatting
still fails. Full SI-00 compatibility/acceptance and integration, all production
fixes, G-FOUNDATION/G-ACTIVATION and main merge remain separate decisions.
