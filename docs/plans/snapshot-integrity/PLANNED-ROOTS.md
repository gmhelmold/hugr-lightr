# SI-01: read-only inspection of planned protected roots

Part of #147 / #152 under Accepted ADR-0020 and C12. Base integration:
`408dd9436ef05a612dfec4cd8bea7948e6bf767d`. Public routing remains inactive.

## Contract and limits

`ProtectedRoot::ExistingDirectory` preserves the old strict directory contract.
`PlannedDirectory` explicitly permits absence; it does not initialize a Store.
Both inspection types expose `inspect_configured`; the existing `inspect`
methods still require every protected root to exist. No source is invented for
destination-only inspection. No new syscall, dependency, write or format is added.

Each absent root retains its nearest natively resolved existing ancestor and
its exact missing suffix. Native component resolution, symlink/parent ordering,
no-follow absence checks, local-volume qualification and budgets are reused.
An existing public ancestor of a planned root remains prohibited. Two absent
paths with the same native ancestor and an exact prefix overlap are rejected.
Different missing spellings under that same ancestor return Unsupported: their
case/Unicode alias behavior has not been established by an actual reservation.
This deliberately also rejects harmless absent siblings rather than guessing.
Distinct existing anchors can establish separation even when one anchor is
above the other: the first actually absent component cannot be that already
existing branch. Mount/profile checks remain mandatory.

Revalidation does not refresh a stale observation. Creation of the planned
root, replacement/movement of its retained ancestor, or changed source identity
fails explicitly. A later operation needs a new inspection. None of this freezes
future namespace changes, checks destination emptiness, proves whole-tree native
representation, or supplies an anchored writer/lease. Windows remains explicitly
Unsupported after cancellation/deadline checks. Root inventory completeness is
still the caller's obligation; there is no automatic inventory discovery.

## Five acceptance groups

### Success criteria

Inspect an explicitly planned root without creating it. Reject prohibited or
unprovable overlap and preserve native errors and the identity of opened sources.

### Completeness criteria

Existing/planned and mixed roots; source and destination-only/paired paths;
existing public ancestors; equal/nested/ambiguous missing paths; distinct native
anchors; native alias followed by parent; dangling links and wrong types;
creation/replacement during revalidation; bounds, cancellation and platform errors.

### Quality standards

Reuse the native walker, preserve existing tests and control selectors, and
keep the 32-root/128-depth/512-handle observation limits. New tests own their
scratch. Rust 1.96.0, denied warnings, formatting, native required-name policy
and compiling causal controls are mandatory. No retry-until-green or sleep-based
race assertions. Review all changes against the exact integration base.

### Invariants

No initialization, source/output write, lease claim, fabricated native alias
proof, relaxed ExistingDirectory contract, dependency/codec change, or activation.
The original package criteria and controls remain mandatory.

### Definition of Done

Seventeen new methods on each qualified Unix profile and three portable methods
on Windows must execute; ten topology causal controls (seven retained plus three
new) must compile and fail their named assertions, then restore to passing source.
All applicable exact-candidate Actions gates pass before reviewed expected-head
merge. Verify fresh parent CI and retire only the delivery branch. SI-01 and the
campaign remain open; main, G-FOUNDATION and G-ACTIVATION are unchanged.

## Reproduction and status

```sh
cargo +1.96.0 test --locked -p lightr-store --lib planned_roots_tests::
python3 scripts/si01/topology_controls.py --out planned-root-controls
python3 -m unittest discover -s scripts/si00 -p 'test_*.py' -v
```

The delivery PR records actual source/run identities and outcomes. These commands
and acceptance counts are obligations, not an assertion that Actions has run.

### Local candidate evidence — 2026-09-19

Owner's Intel Mac, macOS 15.3.2 (24D81), Rust 1.96.0, Python 3.14.5:
Store library264 passed, zero failed/ignored/filtered; eight existing doctests
passed; Clippy all Store targets and formatting passed. Support113 Python
methods and CI-policy30 passed. Counts overlap and are not new native test
counts. Seventeen Store methods are new (three portable and fourteen Unix).
The source/test policy comparison preserved every pre-existing requirement;
old topology/destination tests, native I/O helper, accepted plan and Cargo.lock
remained byte-identical to the base. These local results do not qualify Linux
or Windows. Native/causal Actions evidence is still required by the delivery PR.

Raw local log identities (SHA-256; logs retained outside the repository):

- `store-tests.log`: `b19c654c3231e1b8952fcbe3deb10c320dd04ba184ed94802cc29a1ba8973a27`
- `clippy.log`: `e3156e99e1524d655b2ed07b22b4d813722f014caf96e24820d311c340deb50f`
- `doctests.log`: `d7f1d80af4c2bb1f3962bc7037b52ef40ebc896d4327c3d82be4d73ec9fbee4d`
- `support.log`: `8b17b6afff3b16876bcc2ea2f8f9d5a2f8ea410c1ad6082e3431ca5c688d3869`
- `ci-policy.log`: `596202679ea3a7646f77cdb210bfa80a059d7cb4c8c76ba57ac42e32be87f51d`

### Control-selector review

The first candidate made the older `if !self` mutation ambiguous: it matched
both the new missing-suffix check and the old identity revalidation. Scope that
older selector to its following `.observed` access, preserving its behavioral
assertion and required disposition. A fourth policy method now requires exactly
one source seam for each of all ten controls. The revised support suite passed
114 methods locally; no Rust behavior, count, native assertion or gate was removed.

- `support-scoped-controls.log` SHA-256: `e9a2a42a308459f5cf191565761125347872403daa7ca9c792a317304a94a5bd`
