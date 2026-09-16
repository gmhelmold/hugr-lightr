# Snapshot integrity remediation — execution contract

**Repository:** `gmhelmold/hugr-lightr`  
**Date:** 2026-09-16  
**Plan version:** 1.0  
**Coordinator:** this ChatGPT session; future execution must resume from this file, not assumed conversation memory.  
**Integration branch:** `fix/snapshot-integrity`; final PR: [#146](https://github.com/gmhelmold/hugr-lightr/pull/146).  
**Authority:** the owner requested a durable plan and agent execution. No authority to merge into `main`, enable auto-merge, release, change billing, or alter sibling repositories.

## 1. Outcome and boundaries

Resolve every finding from the [adversarial self-audit](https://github.com/gmhelmold/hugr-lightr/pull/146#pullrequestreview-5227371146): the unexplained macOS snapshot failure, staging ownership, formatting, insufficient regression evidence, Windows flush correctness, inherited scan omissions, and ref/history/GC concurrency. Include the performance and platform qualification needed to make accurate claims.

This is a storage-correctness campaign, not the earlier project-wide runtime redesign. Memoization keys, output replay, engines, Docker compatibility, seccomp, pricing, whitepaper principles, and CoreLink integration are outside scope. Do not turn this into a framework, daemon, new storage engine, generalized transaction manager, or dependency upgrade.

Preserve public APIs and on-disk encodings unless a proven requirement makes that impossible. Any proposed format/API change requires an explicit decision before implementation; a failing old contract must not be silently preserved for compatibility. Repository guidance is in [CLAUDE.md](https://github.com/gmhelmold/hugr-lightr/blob/e4a53417f6fe7da8c4f44908af9525536443d0fa/CLAUDE.md). Read applicable accepted ADRs and contribution instructions. Repository deliverables use English. Never commit `.techlead/` state.

Non-goals remain explicit: an atomic point-in-time view of a changing directory; repair or full scrub of pre-existing CAS corruption; protection against arbitrary external mutation of private store files; power-loss durability beyond what the actual OS/filesystem guarantees. Do not market this campaign as proving those properties.

## 2. Frozen evidence baseline

| Item | Grounded identifier |
|---|---|
| Pre-fix base | `5c5ca00008ed0d22673be79a7b585517527418af` |
| Audited code head | `e4a53417f6fe7da8c4f44908af9525536443d0fa` |
| CI checkout merge | `609bbf6ed919a3af56f2345444f55b45d8c0ebb9` |
| Audited CI | [run 35139596421](https://github.com/gmhelmold/hugr-lightr/actions/runs/35139596421) |
| macOS arm64 failure | [job 104940488434](https://github.com/gmhelmold/hugr-lightr/actions/runs/35139596421/job/104940488434) |
| Linux x86_64 evidence | [job 104940488689](https://github.com/gmhelmold/hugr-lightr/actions/runs/35139596421/job/104940488689) |
| Pinned toolchain | `rust-toolchain.toml`: Rust `1.96.0` |

Observed: the 12 added unit tests passed in the inspected Linux x86_64 and macOS arm64 logs. Linux builds/unit/acceptance/clippy passed before formatting failed. The Windows job built GNU/MSVC targets and ran clippy, not the new runtime tests. On macOS arm64, `acceptance_r1::g1::a11_gc` failed at its first `snapshot --dir . --name @t/live` with `ENOENT`, before any GC step. Three formatter changes are required in the added CAS test file.

Not established: the cause of that ENOENT; whether staging collisions explain it; whether the new patch caused or exposed it; mutation-test results; performance delta; Windows runtime/crash behavior. A possible collision is a hypothesis, not a diagnosis. Preserve that distinction in issues, commits, and reports.

The authoring container has no `cargo`, `rustc`, `codex`, `opencode`, or `claude` executable on PATH. GitHub write access exists; it does not imply an agent runtime exists. The dispatch ledger below records actual attempts. Never label a task running solely because an issue or branch exists.

## 3. Required invariants

| ID | Contract to establish | Required negative witness |
|---|---|---|
| I1 | Every non-ignored, supported entry selected for capture is preserved or causes an explicit error; IO failure is not an ignore rule. | A selected unreadable/unhashable file cannot yield successful incomplete snapshot output. |
| I2 | A CAS object's published digest identifies the verified staged bytes; every operation owns its staging namespace. | Forced allocation collision and changed staging bytes cannot publish or delete another operation's data. |
| I3 | A detected scan/ingestion failure occurs before advancing the snapshot ref or appending its publication history. | Exact old ref/history and recoverable old tree survive each pre-publication failure. |
| I4 | GC cannot sweep required objects between lookup/ingestion and reference publication. | Pause in-flight publication, contend with GC, remove the operation guard in a mutant, and observe the intended assertion fail. |
| I5 | Concurrent same-ref writers do not overwrite one another's history allocation; the current ref is independently a GC root. | Force the audited writer interleaving and an inconsistent legacy log/current-ref state. Current data must survive GC. |
| I6 | Failed durability operations are propagated; success claims match the supported platform guarantee. | Inject a flush failure and show that publication is not acknowledged as durable success. |
| I7 | Verification stays on the production path, not just in a helper tested in isolation. | Bypass verification in production in a compiling mutant; the public-path test must fail. |
| I8 | Qualification is bound to exact code SHAs, real executed tests, and available evidence. | Missing artifacts, skipped jobs, compilation failures, and undetected mutants cannot become a green completeness claim. |

I3 concerns failures before reference publication. Failures or crashes during a multi-file ref/log commit need an explicit recoverability contract in SI-03: do not pretend file-level rename creates all-or-nothing multi-file durability. Conservatively retained orphan objects are acceptable. Loss of an acknowledged current snapshot is not.

## 4. Execution graph and file ownership

```text
SI-00 baseline / durable plan / issue packets / dispatch receipt
  |
  +--> SI-01 CAS staging + Windows durability + ENOENT investigation ----+
  +--> SI-02 fail-closed capture ---------------------------------------+--> SI-04 publication-boundary evidence
  +--> SI-03 ref concurrency + GC reachability -------------------------+             |
                                                                                   v
                                                                         SI-05 independent qualification
                                                                                   |
                                                                       owner review; NO automatic merge
```

SI-01, SI-02, and SI-03 can start together. Limit the first wave to three workers. SI-04 may design tests earlier, but must implement against the integrated dependencies. SI-05 may reproduce baseline failures read-only earlier; final qualification requires the actual integrated head. Do not dispatch dependency-blocked implementation work as ready.

| Packet | Exclusive code ownership | Initial dependency |
|---|---|---|
| SI-00 | This plan, issue status, PR description; no source-code edits | None |
| SI-01 | `crates/lightr-store/src/store/cas/**`; new private staging helpers under that subtree | SI-00 |
| SI-02 | `crates/lightr-index/src/index/scan.rs`; new scan tests; necessary index-cache internals only after inspection | SI-00 |
| SI-03 | `crates/lightr-store/src/store/refs.rs`, `refs_tests.rs`, `lock.rs`; `crates/lightr-index/src/index/gc.rs`; new ref/GC tests | SI-00 |
| SI-04 | `crates/lightr-index/src/index/snapshot.rs`, `snapshot_tests.rs`; dedicated snapshot acceptance test file | SI-01 + SI-02 + SI-03 integrated |
| SI-05 | Dedicated qualification scripts/results and narrowly scoped CI wiring | SI-04 + all implementation integrated |

SI-01 owns shared `atomic_write`/temporary helpers in CAS; SI-03 consumes their stable interface and must not edit those files concurrently. SI-03 owns lock design; SI-04 consumes it. SI-01 diagnoses the macOS failure without editing the shared acceptance fixture: use a separate reproducer/evidence file. Only SI-05 changes CI configuration. Cargo manifests/lockfile, core error definitions and global test registries are coordinator-owned shared files; request a bounded handoff before editing them. The objective is no new production dependencies; `tempfile` is currently a dev-dependency of `lightr-store`, so do not silently promote it.

Every worker uses its own branch/worktree. Never share a writable checkout. Start from the exact coordinator-published integration SHA, not a stale default-branch search result. Record `git rev-parse HEAD` before work and compare the requested base. Agent-created branches such as `copilot/...` are acceptable, but the worker PR must target `fix/snapshot-integrity`, not `main`. If the executor cannot honor the base/target or isolation contract, stop as BLOCKED; do not implement against a guessed base.

Workers must not push to the integration branch. The coordinator may integrate reviewed worker changes there in dependency order, after checking that the head has not moved unexpectedly. Never force-push over another session. Final merge into `main` is reserved for the owner. An API adapter that cannot control an agent's base must use explicit packet instructions and verify the resulting branch; assignment alone is not sufficient evidence of compliant execution.

## 5. Work packets

### SI-00 — establish baseline and dispatch truth

Owner: coordinator. Persist this file and create linked, self-contained execution issues. Recheck PR/base/head and existing issues to avoid duplicates. Preserve the original failure log and exact baseline SHAs. Keep PR #146 draft; correct its validation description to distinguish tests run by CI from tests not run locally. Do not mark an incomplete campaign accepted.

Dispatch only ready work through an actually available executor. Native subagent spawning is not exposed in this session. A documented alternative is GitHub Copilot issue assignment, subject to the repository/account/integration accepting it. Attempt one ready packet first. Verify the returned assignee plus an execution event/session or agent PR before describing execution as started. If rejected, record the error and leave all undispatched work ready for an authorized executor. Do not install a paid service, request a token in chat, or change permissions/billing to force it.

Done: durable file verified; task IDs/links populated; every dispatch state has an honest receipt. This does not mean any code remediation is done.

### SI-01 — exclusive staging, verified publication, Windows flush, ENOENT diagnosis

Priority: P1; covers A1, A2, A3 and A5, plus the ingestion portion of A4. Read the entire CAS module, CoW ladder, digest reader, existing tests and pinned formatting configuration before editing.

Implementation contract:

1. Make staging allocation exclusive by construction. An atomically created private temporary directory with a still-nonexistent clone destination is one suitable design. `clonefile` must not receive a pre-created destination file. Collision means retry bounded allocation, never remove a path owned by someone else. Clock precision, random naming alone and per-snapshot deduplication are not ownership proofs.
2. Use RAII cleanup restricted to the allocated namespace. Cover successful CoW, failed-CoW fallback, partial copy failure, verifier failure, and rename failure. Never delete a published CAS object as error cleanup. Leave crash-orphan handling conservative; no age-only deletion of potentially active staging areas.
3. Apply the ownership rule to shared CAS temporary-write helpers (`put_bytes` and `atomic_write`) where the same allocator is used, without changing their public contracts. Return contextual stage/path information while preserving error classification and without printing file contents or secrets.
4. Keep staged-byte verification mandatory before publication. Handle concurrent already-present destinations explicitly. No hardlinks to live sources. Verify copy and probed CoW behavior; do not silently equate probed fallback with a tested native CoW facility.
5. For Windows, obtain a correctly authorized private staging handle, handle read-only-source attributes safely, flush it and propagate errors. Prefer standard checked APIs where suitable after verifying platform semantics. Audit all existing unchecked flush call sites in the owned module. On Unix, document permission/fsync/rename order; do not imply directory synchronization on Windows when it is not provided. A reopen roundtrip is not a power-loss test.
6. Correct the three rustfmt differences using the pinned toolchain; do not mechanically reformat unrelated files.
7. Reproduce the original macOS fixture at base, audited head and candidate with stage/path diagnostics. Include the original 200 identical 1 KiB files and large file. Establish causality or leave A1 explicitly unresolved. Disappearance over repeated runs is supporting evidence, not proof of the collision hypothesis.

Required evidence: forced allocation collision preserves the other allocation; concurrent same-digest callers in threads and processes; failure during fallback cleans only owned staging; source mutation between first hash and copy; production-path verification bypass is detected; pre-existing valid destination is not deleted; after-publication source writes cannot change preserved bytes; flush failure prevents acknowledged publication; read-only input and empty/large input coverage. Synchronization uses barriers/channels or scoped injected operations, not sleep-to-win races or environment-variable test hooks.

Deliver separate commits for mechanical formatting and substantive fixes when practical. Submit a draft worker PR with failure-stage attribution, targeted commands/results, platform limits, and unchanged API/format confirmation. No claimed resolution of A1 without evidence.

### SI-02 — fail-closed scan and truthful capture

Priority: P1; covers inherited scan omissions and the scan side of A4. Propagate walker, metadata, read-link and hashing errors for selected supported entries instead of dropping them. Preserve intentional ignore behavior and the supported file-kind contract. Explicitly distinguish an ignored entry from an unreadable entry.

Do not substitute an empty target for a failed symlink read. Do not report a successful empty/incomplete tree because a selected file vanished during hashing. Check per-file metadata/byte consistency where a changing source can produce mismatched size/hash observations; choose bounded retry or explicit failure, not unbounded retry. Keep index data a cache, never evidence that the underlying bytes were preserved. Ensure a failed scan cannot poison a later retry; document that index cache updates need not be rolled back with snapshot refs.

Use the public snapshot/CLI path for at least one negative witness: selected unreadable file, hash/read failure, unreadable subtree, and deletion during capture must not advance an existing ref or its history. Tests must be meaningful under the executing identity: chmod-based tests run in an unprivileged subprocess, or the test must clearly report that the permission scenario was not exercised. Root bypassing permissions is not a pass. Use scoped injected IO for otherwise nondeterministic edges without building a general filesystem abstraction solely for tests.

Acceptance also includes readable retry, preservation/recovery of the previous snapshot, ordinary ignored files remaining excluded, empty directories and valid symlinks still roundtripping, and large-file capture. Removing the new scan-error propagation in a compiling mutant must fail the corresponding test. Preserve original cause and path in diagnostics. Do not change memoization, manifests or ignore semantics to make tests easier.

### SI-03 — same-ref serialization and conservative GC roots

Priority: P1; covers inherited ref/history/GC counterexamples. Define a small explicit concurrency and recovery protocol before editing. Do not treat a shared GC guard as writer serialization.

Serialize read-parent/history-allocation/log/current-ref publication for the same ref where needed. Protect all mutating paths that participate in the protocol, including removal; independently scoped refs should remain able to progress. Specify one lock acquisition order (operation-level GC guard before per-ref serialization, or another justified cycle-free design), account for nested shared locks and waiting exclusive GC across Linux/macOS/Windows, and test same-process as well as cross-process callers. Do not change lock behavior by inference alone.

Replace unsafe count-based history-slot reuse with collision-safe allocation under the protocol. Account for gaps, temp files and corrupted entries. Define the publication point and failure semantics between log write and current-ref write: conservative extra retained history can be acceptable, lost acknowledged current data is not. Existing formats should remain readable; do not invent an implicit migration.

GC must independently mark each current ref in addition to retained history. On inability to read a current root or traverse reachability completely, destructive collection must fail closed or conservatively retain uncertain objects rather than silently sweep them. Preserve expected untag behavior: an intentionally removed ref must not be resurrected just because historical files remain on disk.

Required evidence: deterministic version of the audited A-log/B-log/B-current/A-current race; many same-ref writers without history overwrite; current A with legacy log B survives GC; readable historical versions survive per retention policy; corrupt/missing current root aborts destructive sweep; removal races do not delete an acknowledged live snapshot; different refs still progress; writer/GC contention completes under bounded subprocess timeouts; old encoded refs/manifests remain readable. Kill a writer at selected publication boundaries and demonstrate the documented recoverability behavior; do not describe process termination as a physical power-loss test.

Mutants must detect disabling same-ref serialization and removing current-ref marking independently. Coordinate any necessary CAS atomic helper change with SI-01 rather than editing its files.

### SI-04 — end-to-end publication boundaries and regression-removal tests

Priority: P1 qualification dependency; covers A4 and integration of I1–I7. Work on the integrated SI-01/02/03 head. Keep the useful captured-manifest tests, but add witnesses for the real production entry points and temporal boundaries.

Provide a bounded, deterministic scenario in which publication pauses after ingesting a required object and before writing the ref. A competing GC operation must not delete that object. Continue publication, run GC and hydrate, and compare exact bytes/tree/required modes. Repeat the failure path: old ref/history remain unchanged and GC can progress after failure. A test that only acquires GC after return is insufficient.

Cover existing-object reuse protected during publication, an empty tree, duplicate-content inputs, multiple distinct file failures, source change followed by a successful fresh capture, and restoration of both old and newly acknowledged versions. Assert typed failures, not only nonzero exits. Do not assert rollback of valid orphan CAS objects: their survival until later GC is allowed.

Prove the tests have teeth with targeted compiling mutants: drop/shorten the publication guard; swallow ingestion errors; omit expected-vs-actual digest comparison; bypass staged verification in the production path. A mutant counts as detected only when the designated behavioral assertion fails. Compile errors, unrelated test failures and timeouts without established lock causality are not successful negative evidence. Restore the candidate and rerun to prove no mutant leaked into the deliverable.

No process-global failure hooks. Favor scoped closures/internal seams or child-process synchronization with explicit cleanup. If a test needs platform-specific behavior, report that platform's evidence separately. Independent reviewer reads the test oracle, not just the number of test names.

### SI-05 — independent qualification, evidence integrity and bounded performance study

Owner: a different execution/review session from the implementation author where available; otherwise label self-review honestly. This packet never grants main-merge authority.

Run the pinned formatter, workspace clippy, targeted crate tests and existing acceptance suite on the final integrated SHA. Include actual native Windows runtime tests for the changed store/index code: GNU/MSVC cross-check success is not Windows execution. Use macOS arm64 specifically for the observed failure, macOS x86_64 for compatibility, and Linux x86_64/aarch64. Add only the narrowly scoped missing CI coverage; no removal of tests, `continue-on-error`, or new blanket skips to obtain green status.

Required artifacts must be tied to candidate SHA, base SHA, runner/OS/filesystem, toolchain, exact command, exit code and relevant stdout/stderr. If an aggregate benchmark check has no valid required artifacts, it must say incomplete/fail rather than assert no regression. Do not change measured benchmark ledgers with projections.

Run a bounded stress qualification in addition to deterministic tests; record chosen concurrency/repetition/seed as test parameters, never as measured results in advance. Compare stable fixture results at base, audited head and candidate. Report any baseline failures instead of hiding them. One unexplained candidate snapshot failure blocks qualification.

Measure capture/hydrate on named hardware for distinct versus duplicate small files, the existing acceptance fixture and large files. Separate new-store capture, warm CAS reuse and any OS-cache controls actually achieved. Record samples/median/tail, bytes processed, CPU and peak memory when available, and writer/GC blocking. There is no invented percentage budget: the extra verification read is an accepted correctness cost in principle, but its measured impact still needs review. Do not compare a skipped execution with a full computation as a runtime-speed claim.

Final exit: every packet has reviewed evidence; A1 has a defensible disposition; required native checks pass at the current integration head; negative controls detect their intended mutants; no unreviewed source-file changes or format/API drift; known platform durability limits are explicit; fresh-checkout recovery succeeds. Keep the final PR draft until these conditions hold, and leave merge to the owner.

## 6. Evidence and handoff format

Each worker adds one small report at `docs/plans/snapshot-integrity/evidence/SI-NN.md` (that worker's report only). Large raw logs belong in GitHub Actions artifacts or PR attachments, with SHA-256 checksums and retention/expiry noted. Store a compact durable summary and exact reproducer commands in the report so expiring CI links are not the sole evidence. Never commit secrets, full source-file data from a user's workload, huge logs, or machine-specific paths as required configuration.

Each handoff must contain:

```yaml
packet: SI-NN
status: EVIDENCE_READY # not DONE; use BLOCKED when appropriate
base_sha: exact-commit
head_sha: exact-commit
branch: worker-branch
pull_request: real-url
changed_files: []
invariants_addressed: []
commands: [] # command, runner, exit_code, passed/failed/ignored counts, artifact URL/checksum
negative_controls: [] # mutant, test, observed intended assertion, restored-candidate result
root_cause: established-or-explicitly-unresolved
residual_risks: []
api_or_format_changes: none-or-explicit-proposal
```

No fabricated placeholders may be left as claimed results. A reviewer verifies logs, implementation boundaries and recoverable outputs. Workers may not approve their own results as independent review. Separate tests that exercise a real platform feature from tests that fell back or skipped.

Coordinator integrates one reviewed worker at a time, checks path ownership and head movement, resolves conflicts explicitly, reruns affected tests, then records the integrated commit. Other sessions must not overwrite this file concurrently. Use separate evidence reports and issue comments; the coordinator consolidates the ledger.

## 7. Resume protocol and state machine

States: `PLANNED -> READY -> DISPATCH_REQUESTED -> RUNNING -> EVIDENCE_READY -> REVIEWED -> INTEGRATED -> QUALIFIED`; `BLOCKED` is possible at any step. Assignment accepted without an execution receipt remains DISPATCH_REQUESTED. Creating an issue does not transition it to RUNNING.

On resume: read this plan, PR #146 and linked issues; resolve the live branch SHA; compare with the frozen baseline and last integration receipt; inspect CI for that exact SHA; claim one ready packet; create an isolated worktree; honor file ownership; execute; publish evidence. If the head changed, reconcile rather than assume earlier results validate the new code. Never restart the entire campaign because chat context was lost.

Release criteria are intentionally separate: creating this plan qualifies only the planning portion of SI-00. The code campaign remains unqualified until SI-05 succeeds. No asynchronous ChatGPT monitoring is configured by this document.

## 8. Execution ledger

Packets have been created. This is the planning snapshot, not evidence of execution. The coordinator records actual dispatch results separately in [DISPATCH.md](https://github.com/gmhelmold/hugr-lightr/blob/fix/snapshot-integrity/docs/plans/snapshot-integrity/DISPATCH.md); consult that receipt and live issue events before claiming a worker is active.

| Packet | Dependency state | Issue |
|---|---|---|
| SI-00 | Plan prepared; dispatch qualification requires the receipt | PR #146 / this file |
| SI-01 | READY for an available executor | [#147](https://github.com/gmhelmold/hugr-lightr/issues/147) |
| SI-02 | READY for an available executor | [#148](https://github.com/gmhelmold/hugr-lightr/issues/148) |
| SI-03 | READY for an available executor | [#149](https://github.com/gmhelmold/hugr-lightr/issues/149) |
| SI-04 | BLOCKED on SI-01, SI-02, SI-03 integration | [#150](https://github.com/gmhelmold/hugr-lightr/issues/150) |
| SI-05 | BLOCKED on integrated implementation and SI-04 evidence | [#151](https://github.com/gmhelmold/hugr-lightr/issues/151) |

Ready work without an accepted executor remains undispatched. Dependency readiness and execution status are different fields. The containing documentation commit does not establish a new tested code baseline or resolve the existing CI failures.

## 9. Primary references

- [Audit review and finding IDs](https://github.com/gmhelmold/hugr-lightr/pull/146#pullrequestreview-5227371146).
- [Audited snapshot publisher](https://github.com/gmhelmold/hugr-lightr/blob/e4a53417f6fe7da8c4f44908af9525536443d0fa/crates/lightr-index/src/index/snapshot.rs), [scan](https://github.com/gmhelmold/hugr-lightr/blob/e4a53417f6fe7da8c4f44908af9525536443d0fa/crates/lightr-index/src/index/scan.rs), and [GC](https://github.com/gmhelmold/hugr-lightr/blob/e4a53417f6fe7da8c4f44908af9525536443d0fa/crates/lightr-index/src/index/gc.rs).
- [Audited CAS](https://github.com/gmhelmold/hugr-lightr/blob/e4a53417f6fe7da8c4f44908af9525536443d0fa/crates/lightr-store/src/store/cas/mod.rs), [refs](https://github.com/gmhelmold/hugr-lightr/blob/e4a53417f6fe7da8c4f44908af9525536443d0fa/crates/lightr-store/src/store/refs.rs), and [locks](https://github.com/gmhelmold/hugr-lightr/blob/e4a53417f6fe7da8c4f44908af9525536443d0fa/crates/lightr-store/src/store/lock.rs).
- [Rust File opening semantics](https://doc.rust-lang.org/std/fs/struct.File.html#method.open); [Microsoft FlushFileBuffers requirements](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers). These support the Windows issue; they do not establish tested Windows durability.
- [GitHub cloud-agent API and issue assignment](https://docs.github.com/en/copilot/how-tos/use-copilot-agents/cloud-agent/use-cloud-agent-via-the-api). Assignment support and account eligibility must be verified in the actual connection.
