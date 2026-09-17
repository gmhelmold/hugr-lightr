# SI-01 cancellation slice — executed, full acceptance remains blocked

Date: 2026-09-17. Issue #147; campaign #152; draft PR #155.
Reviewer: ChatGPT, author/coordinator, under the owner's continuation mandate.
This is an explicitly labeled self-review, not independent approval.

## Exact source identities

| Identity | Value |
|---|---|
| Source head | `a442009ede32f213a02192efcce4e7d6ccbaa163` |
| Source parent | `f574aae002121aa8d000add36793f768ec1b527d` |
| Source branch tree | `ca77ae0723a193b87a0b647b3daeeff2f49d8f9c` |
| Actual CI merge checkout | `2404a5e37842be8a422e2c55f4cebedfd9f3ae27` |
| CI parents, ordered | `9ab0eb764d8f637b16c139199f3bbf39cc4f74f6`, `a442009ede32f213a02192efcce4e7d6ccbaa163` |
| CI tree | `aa06e03dd540bcd6bae918fb13fed3127eff4e0f` |
| Input fingerprint | `9b8da142bb63734be7b0b7b010c49ed476d2a49847d59e5e36cc51f47caa7a2e` |

The later commit containing this receipt is not the tested executable. The
source update was a normal fast-forward of the existing PR branch, with the
remote head checked before push. No force-push or integration/main merge.

## Implemented behavior

The independently scoped cancellation work adds `Digest::of_reader_checked`,
`LeasedStagedFile::copy_with_wait` and internal copy/write checkpoints. Existing
uncancelled entry points remain available. The reader-to-staging public
foundation API and requalification copy forward their actual Wait value.
Final staged verification also honors it.

Checkpoints run outside ordinary Interrupted retry loops, before and after
reads/writes, on EOF, during staged hashing and before/after file sync. A
checkpoint failure is terminal even when its kind is Interrupted; genuine OS
Interrupted reads/writes still retry when not cancelled. Other OS errors retain
their original cause. Short writes, WriteZero and invalid reported lengths are
explicitly handled. Cancellation before entering the single-flight map does
not poison a later capture on the same lease. Failed capture removes only its
owned staging and cannot publish a payload or receipt.

The change is 12 files, including tests/CI/documentation. No dependency,
Cargo.lock, storage schema, GC, NativeLock or existing Store/CLI routing change.
All modified/new Rust source files remain below the 400-line limit.
`production_protocol_enabled=false`; G-FOUNDATION is not established.

Limits: checkpoints cannot preempt a synchronous OS call which has not returned.
The preliminary existing-object inspection in Layout::inspect is not yet
checkpointed. CoW/fallback, C12 confinement, resource/reaping and the wider
SI-01 criteria remain distinct obligations. This slice is not complete
cancellation of every future operation.

## Baseline discrimination and local execution

Three reader-driven tests first ran against the unchanged implementation and
failed: cancellation after a successful read and after Interrupted caused a
second source call; cancellation at EOF was noticed too late, in Lock rather
than Stage. They passed after the correction. No sleeps or unbounded sources
are needed: an unexpected second read returns an ordinary error.

On the owner's Intel Mac, Rust 1.96.0, in the isolated temporary clone with two
build jobs: all 22 new tests and five compiled negative controls passed.
Formatting and targeted lightr-core/lightr-store all-targets Clippy with denied
warnings passed. No user's existing checkout or sibling repository was changed.
Local controls identify the exact a442009 branch tree, not the CI merge tree.

## CI evidence and boundaries

- [Foundation controls 35278559426](https://github.com/gmhelmold/hugr-lightr/actions/runs/35278559426): cancellation, previous codec, lease and publication controls PASS.
- [Native/bootstrap 35278559471](https://github.com/gmhelmold/hugr-lightr/actions/runs/35278559471): new cancellation tests PASS; whole matrix FAILS on the existing Unix lock counterexample.
- [Workspace 35278560424](https://github.com/gmhelmold/hugr-lightr/actions/runs/35278560424): formatting/build PASS; full tests FAIL; subsequent Clippy/legacy-control steps were skipped, not passed.
- [Immutable contract 35278559466](https://github.com/gmhelmold/hugr-lightr/actions/runs/35278559466): PASS. No final performance verdict is inferred from other workflow status.

There are 22 new distinct methods: seven core hash tests, eight foundation
capture tests and seven staged copy/write tests. All 15 Store names were added
to the required native policy without removing existing requirements. The seven
core methods run in the dedicated controls/workspace, not falsely counted as
part of the Store/index five-platform job.

| Native Store/index profile | Passed | Failed | Historical ignored | New required Store methods passed | Formatting |
|---|---:|---:|---:|---:|---|
| Linux x86_64 | 202 | 1 | 1 | 15 | PASS |
| Linux aarch64 | 202 | 1 | 1 | 15 | PASS |
| macOS Intel | 188 | 1 | 1 | 15 | PASS |
| macOS ARM | 188 | 1 | 1 | 15 | PASS |
| Windows MSVC | 180 | 0 | 0 | 15 | PASS |

Counts overlap other runs; they are not new unique coverage. The one failed
method in each Unix profile is
`store::foundation::lease_io::release_tests::lease_drop_releases_even_with_a_duplicated_os_handle`.
Workspace's Store suite has 151 passed and this one failure; no whole-workspace
qualification or all-targets workspace Clippy success is claimed.

The five new mutation controls bypass Wait forwarding, a read retry checkpoint,
a write retry checkpoint, the initial hash checkpoint and the post-sync check.
Each mutant compiles, then fails exactly its named test/assertion with exit 101.
Pristine and restored 7+8+7 suites pass. Timeouts/build failures never count as
successful detection. These controls passed locally and in CI.

## Lock blocker and refusal record

The baseline lock counterexample was reproduced again before work. A narrowly
scoped NativeLock lifecycle correction was submitted to Remote Desktop
Commander, which refused it before execution. Read-back showed a clean tree
and the original file. No retry through another route or permission change was
performed. The internal reason was not provided. The cancellation commit has
NO diff in lease_io.rs against its parent; the failed lock test is still active.
This receipt does not claim the lock problem is fixed or globally impossible
to fix. It records the actual denied action and current unmodified behavior.

## Retained verification

The cancellation-control archive and all five native archives were downloaded,
checked against GitHub's outer SHA-256 digests, and checked internally for every
recorded artifact/binary hash. Each native input manifest's 794 file hashes was
compared with the archived source. Required test names and the exact failure
set were checked. The complete Git tree was reconstructed from archive bytes
and matches the independent GitHub commit tree. Workspace raw tests/source were
also checked. No archived native executable was run in the authoring sandbox.

Raw archive IDs: controls 10521483317; Linux x86_64 10521163616; Linux ARM
10521153630; macOS Intel 10520969469; macOS ARM 10522125023; Windows 10521298599;
workspace 10521153930. The durable delivery includes their bytes and an offline
verifier; expiring GitHub links alone are not the retention mechanism.

## Decision

Accept this slice as implemented and evidenced, NOT the full SI-01 package.
Keep #147 and PR #155 open/draft. Preserve all five axiom groups and all 21
numbered criteria. Unix lock release, remaining CoW/topology/resource obligations
and reviewed full integration prevent G-FOUNDATION. No protocol activation,
real-data migration, release, merge or coding-agent dispatch occurred.
