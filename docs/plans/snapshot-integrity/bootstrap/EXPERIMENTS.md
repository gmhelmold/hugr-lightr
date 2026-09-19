# SI-00 experiment registration and performance method

Plan v2.2 remains authoritative. The following names are **reserved future test
identifiers**, not existing passing tests. No placeholder test bodies are added.
Each test belongs in the named owner's delivery and must use the production path.

| Family | Reserved test | Owner | Discriminating observation |
|---|---|---|---|
| E01 | `si_e01_exclusive_staging_collision` | SI-01 | forced collision preserves first allocation |
| E02 | `si_e02_same_digest_waiters_terminate` | SI-01 | threads and processes finish with valid bytes or original failure |
| E03 | `si_e03_capture_verification_cannot_be_bypassed` | SI-01/02 | wrong staged identity cannot become readiness |
| E04 | `si_e04_post_install_failure_requires_requalification` | SI-01 | following process cannot reuse by existence |
| E05 | `si_e05_restored_stat_still_captures_new_bytes` | SI-02 | AAAA to BBBB with restored stat fields yields BBBB |
| E06 | `si_e06_selected_io_error_blocks_publication` | SI-02/03 | unreadable selected entry is failure, not omission |
| E07 | `si_e07_prepared_rollback_restores_full_tuple` | SI-03 | all tuple/history components are BEFORE |
| E08 | `si_e08_decision_survives_interrupted_recovery` | SI-03 | phase, not current, governs repeat recovery |
| E09 | `si_e09_gc_excluded_through_publication` | SI-01/03 | contender arrival then WouldBlock before writer release |
| E10 | `si_e10_same_name_serialization_and_parent` | SI-03 | no wait for correct contender to enter held lock |
| E11 | `si_e11_pending_without_current_blocks_sweep` | SI-03 | restart then GC before recovery performs no destructive sweep |
| E12 | `si_e12_independent_tree_fidelity` | SI-02/03 | raw lstat/readlink/bytes oracle, preserve or reject |
| E13 | `si_e13_active_reader_survives_untag_gc` | SI-03 | reader's last read precedes collection |
| E14 | `si_e14_all_root_families_and_consumers` | SI-03 | raw32 build-cache roots as well as LRR1/image roots survive |
| E15 | `si_e15_evidence_rejects_empty_wrong_stale` | SI-00/05 | executable Python controls plus integrated artifact/budget gate |
| E16 | `si_e16_seeded_stress_with_restored_trees` | SI-04/05 | seeds/operations/outcomes retained, not time-only success |
| E17 | `si_e17_composite_oci_generation` | SI-03/04 | A/B/error, never mixed config/tree/manifest |
| E18 | `si_e18_shared_cache_and_resource_recovery` | SI-01/03/04/05 | foreign Store EX cannot reap live shared-cache allocation |
| E19 | `si_e19_legacy_equal_tip_is_unverified` | SI-03/04 | identical records cannot manufacture historical proof |
| E20 | `si_e20_windows_representation_vs_privilege` | SI-02/03/04/05 | native file/dir and dangling kinds; unavailable fixture is not pass |
| E21 | `si_e21_lost_reply_does_not_replay` | SI-03/04 | healthy unknown attribution remains separate |
| E22 | `si_e22_native_path_identity_and_overlap` | SI-00/02/03/04 | native symlink/.. target, not lexical substitute |

Each concurrent test must register checkpoint, contender-arrival message,
nonblocking probe, expected pre-release observation, explicit release, bounded
join and owned cleanup. A watchdog timeout is not automatically a causal kill.
Mutant classification remains KILLED_CAUSALLY, SURVIVED,
EQUIVALENT_WITH_PROOF, NOT_EXERCISED or INVALID_TEST. Equivalent redundancy
requires another witness; never weaken protection to obtain a nominal kill.

## Native bootstrap versus remediation

`expected-tests.json` pins the native hosts/toolchain and a required smoke-test
subset; the collector additionally compares each executable's listed and executed
names. It records the pre-existing ignored GC test explicitly. This baseline allowlist **must not be imported into the
future causal suite**. E01–E22 are not claimed implemented by those baselines.

The published e359bd2 evidence-tools job executed its Python positive/negative
controls. The native workflow retains binary bytes/hashes and named-test logs,
then requires artifacts for all five profiles. Run 35209414026 did not qualify
the matrix: Windows provenance validation failed, even though its tests passed. Missing native evidence
or an unavailable capability remains missing, not a cross-compilation pass.

## Incident investigation registration

Keep `acceptance_r1::g1::a11_gc` and its canonical Rust fixture unchanged.
Initial bounded investigation: 20 serialized attempts on macOS arm64, 20 on
macOS Intel and 20 on Linux x86_64 at each of baseline and first causal candidate;
record diagnostic instrumentation's SHA and whether it changes timing. These
are planned run counts, not performed measurements or a completion-time estimate.
Stop candidate qualification on any unexplained failure. H1 shared staging name,
H2 destination-finalization race, H3 source lifetime/path and H4 CoW fallback must
have different traces/controls. A repaired possible bug does not establish the
historical incident's cause. Preserve HISTORICAL_CAUSE_UNRESOLVED when necessary.

## Fixtures and registered sampling

The published `fixtures.py`/`baseline.py` supply F0–F5 inputs. The
F1 regular contents/layout are derived from `crates/lightr-acceptance/tests/common/mod.rs`;
the canonical Rust fixture remains the incident authority. F5 is a compact
preservation fixture, not the full E20/E22 rejection matrix. SHA-256 describes
raw fixture trees independently of the production manifest codec.

F6 is a registered sequence, not prebuilt fake on-disk history: create an empty
verified name, perform 0/32/256 committed transitions, then measure one update,
history read and GC separately from setup. Include same-root metadata changes
with fixed valid OCI config/manifest inputs. This requires SI-03 semantics;
no generator claims its new envelopes already exist.

Use C10's 3 warmups + 20 paired samples for small fixtures and 1 + 7 for F4,
recorded ordering seed 153 and generator version 1. Separate new-store, warm-store
and hydrate; no cold-cache claim without a proven cache-control operation.
Register numerical fixture/profile budgets after the matching correct reference
exists and before final-candidate measurements. None are invented here.

The evidence arithmetic helper implements an actual median comparison and rejects
empty/missing/duplicate/unknown metrics, wrong units, stale budget identity,
unapproved budgets, nonfinite/boolean/negative values and regression. Its
synthetic tests are not real performance observations. Full C11 artifact/source/
review provenance still surrounds this arithmetic gate; a JSON APPROVED field
alone is not proof of a human approval.

## Observed bootstrap failures

Run 35209414026 completed F0-F4 samples with independent materialized-tree
checks, then F5 failed at its first snapshot with ENOENT. Preserve this as a
baseline failure, not a successful F5 measurement. It contains a Unix literal
backslash and link cases; the exact historical incident cause is not inferred
from a shared errno. Windows validation reported tracked files dirty before and
after execution. Diagnostics at 7706210 retain the precise tracked status; the
gate was not removed. Both findings require disposition before SI-00 acceptance.
