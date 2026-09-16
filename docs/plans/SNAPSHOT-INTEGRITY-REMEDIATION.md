# Snapshot integrity remediation — execution specification v2.0

**Repository:** `gmhelmold/hugr-lightr`  
**Date:** 2026-09-16  
**Integration branch / PR:** `fix/snapshot-integrity` / [#146](https://github.com/gmhelmold/hugr-lightr/pull/146)  
**Revision scope:** planning documents and execution packets only. This revision does not implement or validate Rust fixes.  
**Status:** DESIGN SPECIFIED; execution NOT STARTED. Read the current `DISPATCH.md` before acting.  
**Supersedes:** v1.0 at `1ab1fcd638525d824b78137c522305036a6382bd`, including its unconditional parallel READY states.  
**Code baseline:** `e4a53417f6fe7da8c4f44908af9525536443d0fa`; pre-fix base `5c5ca00008ed0d22673be79a7b585517527418af`; planning head inspected before this revision `19b28d7ad2ffff66e176b589f75c6592bd47ebe8`.

## 0. Use, authority and meaning of completion

This document specifies the observable behavior first and derives work packages and experiments from it. The protocol below is a selected design for implementation and review, not a menu delegated to competing agents and not an already proven implementation. A demonstrably incompatible primitive blocks the affected package and requires an explicit amendment; it is not permission to invent a different commit protocol locally.

The owner's latest instruction is **fix the plan, not the source code**. Publishing this revision does not authorize launching workers, editing Rust/CI, creating new execution services, merging, enabling auto-merge, releasing, changing billing/permissions or modifying sibling repositories. Future implementation needs a separate execution instruction and the gates below. Read `CLAUDE.md`, `CONTRIBUTING.md` and applicable accepted ADRs; this plan does not silently change their acceptance status.

Keep the original five implementation issues, #147–#151, and SI-00 as coordinator/bootstrap work on PR #146. There are six work packages, not a new orchestration framework. Each contains five distinct, mandatory axioms:

| Axiom | Question it answers |
|---|---|
| **Success Criteria** | Which observable outcome must the package produce? |
| **Quality Standards** | Under which implementation and evidence constraints is that outcome acceptable? |
| **Completeness Criteria** | Which exact surfaces, scenarios and deliverables must be covered? |
| **DoD** | What evidence, review and integration transitions allow the package to close? |
| **Invariants** | Which properties must remain true throughout all its transitions? |

These sections are cumulative, not interchangeable headings. A green test count is neither completeness nor DoD. The protocol can be specified while execution prerequisites remain unmet. Do not label either implementation or qualification complete from a documentation commit.

### Scope and limits

In scope: owned staging, CAS publication and reuse, capture freshness, ref/history publication and recovery, authoritative GC roots, active materialization, affected metadata consumers, causal tests, native-platform evidence and performance acceptance.

Out of scope: runtime/memoization redesign, new engines, Docker feature expansion, seccomp, CoreLink changes, full CAS scrub/repair, arbitrary hostile modifications to private store files, network filesystems and unsupported mounts, atomic snapshots of an actively mutating entire directory, universal power-loss certification. A corruption encounter must fail safely even though repairing all historical corruption is out of scope.

Existing manifest/ref encodings remain readable. The selected protocol adds **small versioned object-readiness receipts and per-ref pending-operation sidecars**, not a database or a generic transaction framework. These are additive on-disk protocol changes and must be recorded in an accepted ADR before their implementing packages start. No claim of mixed-old/new-writer compatibility is made. Minimal internal lease/result types and caller adaptations are allowed after SI-00 freezes their contracts; public CLI/schema changes require an explicit compatibility decision there.

## 1. Evidence baseline — facts, not current qualification

[Original audit](https://github.com/gmhelmold/hugr-lightr/pull/146#pullrequestreview-5227371146). Original [CI run 35139596421](https://github.com/gmhelmold/hugr-lightr/actions/runs/35139596421) tested checkout merge `609bbf6ed919a3af56f2345444f55b45d8c0ebb9`, not merely a branch name. The inspected Linux x86_64/macOS arm64 logs showed the 12 added tests passing. Formatting failed. macOS arm64 [job 104940488434](https://github.com/gmhelmold/hugr-lightr/actions/runs/35139596421/job/104940488434) failed in `acceptance_r1::g1::a11_gc` during the FIRST snapshot, before GC, with ENOENT. Windows builds/clippy were not native execution of the new tests. Rust is pinned to `1.96.0` in the baseline.

The plan adversarial review identified R01–R14. Its auxiliary filesystem experiment and two transition models were not Rust or crash-durability tests. The historical ENOENT cause remains unproved. No new passing CI, available executor, platform capability or performance number is asserted by v2.

## 2. Selected contracts

### C01 — Success, failure and platform assurance

Separate **content integrity**, **fresh capture**, **visibility**, and **durability**. They must not share an ambiguous boolean.

Internally, publication reports its operation ID, phase, original error/cause and one of these outcomes. Names are semantic requirements; SI-00 maps them to the existing error/CLI surface without discarding OS error information.

| Outcome | Meaning and permitted caller behavior |
|---|---|
| `NotPublished` | This operation did not install its new current ref. Valid orphan objects may exist. Its pending metadata must be resolved before the affected history is used. A corrected capture can be retried after resolution. |
| `CommitUncertain` | Current-ref replacement/removal happened or cannot safely be ruled out, but required confirmation failed. Never claim rollback. Do not automatically repeat snapshot/undo/untag; inspect/recover the recorded operation first. |
| `CommittedMaintenanceFailure` | Required payload and current-ref barriers succeeded; final journal cleanup or ancillary maintenance failed. The logical change is retained, never rolled back. Surface the error and required recovery without pretending the operation had no effect. |
| `Published` | Required payload, ref/history and journal-resolution steps completed at the platform's declared assurance level. Only this is ordinary successful completion. |
| `RecoveryRequired` | A pending/malformed/ambiguous metadata state prevents safe progress. No destructive GC or guessed history. The original cause remains attached. |

The assurance baseline is process-crash recoverability plus checked file synchronization. Linux/macOS additionally require directory-entry barriers on validated local filesystems. Windows must check writable-handle file flushing, but **must not claim directory-entry power-loss durability merely from `FlushFileBuffers`**. A stronger requested guarantee on an unsupported profile fails explicitly; no silent downgrade. SI-00 records each supported profile and how its guarantee is exposed without inventing a new CLI flag as an implementation shortcut.

A process kill/reopen test establishes process recovery only. File/directory ordering plus fault-injection tests establish implementation obligations under documented OS assumptions, not proof against lying devices or arbitrary filesystem corruption. A failed flush is never dismissed by simply retrying that syscall and calling the old bytes durable. [P1–P4]

### C02 — One GC lease and an explicit lock order

Top-level operations acquire one store-scoped GC lease: SHARED for capture/publication and materialization; EXCLUSIVE for collection or explicit metadata recovery. Low-level calls borrow that lease rather than reopening/reacquiring the global lock. Parallel child work must finish before its parent lease is released.

Order: **global lease → ref lock(s), sorted by ref key when multiple are needed → digest lock(s), sorted when multiple are needed**. A digest lock must never be held while acquiring a ref lock. Payload preparation may precede ref locking only when all its digest locks are released first. No lock upgrades, recursive acquisition, or call from an exclusive-lease path into a wrapper that takes a shared lease.

Use store-canonical identity, stable lock files and both thread/process-correct exclusion. Do not delete lock files on unlock: replacing a lock inode can split the lock domain. Prove same-process behavior rather than assuming an OS lock also serializes threads. Keyed in-process mutexes may complement kernel locks. Independent refs must progress concurrently. Lock waits need cancellation/cleanup; correctness tests use bounded watchdogs, not timing assumptions about fairness.

`PreparedObject` is an internal proof tied to the live lease and digest, produced only after C03 completes. It cannot be forged from `exists`, a stat-index entry or an arbitrary path. The per-lease dedup map contains only completed preparations; failures wake all waiters with an error, not a permanently pending promise. Borrowing/scope prevents use after lease release. Persistent readiness receipts are checked under C03; they are not a substitute for a live lease protecting an object from collection.

### C03 — Owned staging and safe reuse after failed finalization

Allocate a private temporary directory atomically on the destination filesystem. Place a **nonexistent** payload path inside it for CoW operations. Use exclusive creation and bounded collision retries; PID/time/randomness may choose names but are not the ownership guarantee. Reserve at most 32 candidate names before returning a contextual allocation error. Cleanup owns only that directory and never a published object or another writer's allocation.

A valid object-readiness receipt is stored separately under a sharded private `objects-ready` namespace. Its bounded, versioned contents bind digest, length, protocol version, assurance profile and checksum. It is written **only after** the corresponding payload and required namespace barriers complete. Receipt and object reads reject symlinks, malformed records and mismatched identity. Receipts are not GC roots. They do not certify a copied/moved store on an unqualified destination filesystem; an offline move/import needs target-side qualification. All participating writers must use this protocol; mixed legacy writers are unsupported.

The selected publication algorithm is:

1. Capture bytes from the selected live source or, for a digest-only operation, use a verified streaming read of the existing CAS object. Reject wrong type, missing bytes, digest mismatch and read errors. Do not follow a substituted store symlink.
2. Stage privately. New live-source capture may use a supported CoW clone; fallback is a complete checked copy. On failed CoW, remove only the operation's own incomplete payload before fallback. Hash the final owned staged file, derive its length there and compare any externally supplied expected identity. Retain checked handles/ownership through publication.
3. Under the digest lock, first consult an existing valid receipt and validate its object. A completed proof in the same lease can be reused directly; a cross-operation receipt requires identity/content verification before creating this lease's proof. **A receipt-confirmed object is immutable: later normal writers do not replace it**, even with identical bytes. This prevents another writer's failed replacement from invalidating an earlier successful publisher's proof.
4. If no valid receipt exists, path presence is not readiness. An existing unconfirmed or legacy object must validate against the expected digest, then be requalified by a full ordinary byte rewrite into a fresh owned file, not a hardlink, reflink of the potentially failed object, or fsync-only retry. Re-hash the final replacement. A malformed receipt or corrupt object is explicit RecoveryRequired/Integrity, not silent repair. Requalification touches only the required object, not a whole-store scrub.
5. Apply final payload permissions/attributes before its last data/metadata synchronization, retaining rights needed to flush on Windows. Never mutate live-source attributes. Install verified bytes atomically under the digest lock; synchronize the destination directory and each newly created ancestor needing a barrier. No fallback unlinks a destination before replacement. Only unconfirmed objects may be replaced by this path.
6. After payload confirmation, atomically write the readiness receipt and perform its required synchronization. Only then report ordinary successful preparation and create `PreparedObject`. If receipt finalization fails, report the phase; do not roll back/delete the now-confirmed payload. A following caller either validates a fully readable receipt (whose publication was ordered after payload confirmation) and reconstructs the receipt in a fresh private file with checked barriers, or requalifies without relying on a missing receipt. It never upgrades an unconfirmed payload by existence alone.

Receipt synchronization is not confused with payload synchronization: a valid visible receipt can only have been constructed after successful payload barriers. Loss of that receipt after a crash loses an optimization/proof, not the data guarantee; requalification is then required. Bad/missing payload despite a receipt fails integrity checks. A subsequent caller may use validated immutable bytes only under the declared assurance profile, never claim that retrying a failed data fsync repaired them.

The safe staging and phase-reporting rules apply to `ingest_file`, `put_bytes` and shared atomic metadata writes. Metadata writes report whether replacement happened before error; `Err` does not always mean no effect. Standalone low-level calls do not protect a later ref write after their lease ends; transaction-aware callers borrow one lease across the entire operation. Preserve public wrappers where possible.

**Post-payload-rename failure:** the object may remain but no receipt/readiness proof is issued. This operation cannot publish a ref. A following operation must perform fresh requalification or fail. A mere read-back is insufficient. Known storage I/O faults stop the affected operation; no automatic loops claim the device is healthy. Successful fresh requalification is a new transaction, not repair certification for unrelated storage.

**GC and receipts:** under the EXCLUSIVE lease, invalidate/remove and confirm the receipt before deleting an unreachable payload. If that prerequisite fails, do not delete the payload. A crash between those steps leaves an unconfirmed orphan, not a stale readiness certificate. Receipt absence never authorizes deleting a reachable object. The object/receipt pair is not claimed to be an atomic two-file transaction; the ordered states deliberately remain safe.

Within a capture, identical digests share a completed preparation and waiters receive the same success/failure, not N racing publications. Cross-operation reuse retains content verification and lease protection. The cost of receipts, verification and legacy adoption is measured under C10; no unreviewed persistent fast path is added to meet a benchmark.

### C04 — Freshness: snapshot derives identity from captured bytes, not stat guesses

All snapshot-producing entry points use a verified capture path. For a source stable for the duration of capture, the result must represent its current selected contents even when size, inode and mtime equal an earlier scan. The stat index may accelerate enumeration/planning/status heuristics; it may not supply a snapshot's final digest without reading/capturing this invocation's bytes.

Preferred dataflow: `capture selected file → owned stage → hash/size of stage → PreparedObject → manifest entry`. Streaming avoids loading whole files into memory. The internal capture/requalification/materialization pipeline must use a verified streaming reader rather than a whole-payload `get_bytes()` allocation; public byte-vector getters need not change their existing API. The manifest is built from these actual results; do not attach fresh metadata to an old cached digest. Preserve an expected-digest comparison where an API explicitly supplies a previously captured manifest. No implementation may bypass C03 through `Store::exists`.

Record file identity/type and metadata around capture. An observed identity/type change, vanished selected entry or inconsistent capture is `CaptureChanged`/a typed error, not omission. Default is one attempt and an explicit retryable error, not an unbounded rescan. Captured bytes must remain independent of subsequent source mutations. Undetectable concurrent rewrites and a globally atomic directory view are outside the guarantee; this does not excuse stale capture of an already stable source.

Traversal, metadata, read-link and hashing failures propagate for selected entries. Existing explicit ignore rules remain exclusions, not errors. Never turn a failed read-link into an empty target. Selected unsupported special files are rejected with a path/type error unless explicitly excluded by ignore policy. Permission experiments must prove the tested identity actually lacked access.

Build candidate index updates separately and publish them only after a successful scan. An unreadable/corrupt cache can be discarded and recomputed; a failed cache save is classified as cache maintenance, not proof that a ref failed after commit. Keep cache writes before ref commit where they remain required by an existing API. A later retry must work without manual cache deletion. Concurrent cache writers cannot share/truncate a staging file.

### C05 — Ref publication, history and bounded recovery

`refs/<key>` remains authoritative for whether a named ref exists and what it currently denotes. `refs-names` is a rebuildable name index, not authority for existence or GC. `refs-log` contains the user-visible committed version sequence only after pending operations are resolved. Preparations are not versions.

Use one versioned pending descriptor per ref key, outside numeric history filenames. Its bounded payload records: format version/checksum, operation ID/kind, validated name/key, exact previous current-record bytes or absence, exact proposed bytes or absence, the reserved history slot and expected record hash, and owned preparation paths. For untag/recreation it also identifies the exact history namespace to retire, so interrupted retirement cannot target another lifetime's history. Validate all paths relative to the store; never execute arbitrary descriptor paths. Use canonical serialization with checked lengths, not unbounded deserialization. The descriptor is additive recovery metadata, not a change to LMF1 or existing ref-record encoding. SI-00 records its concrete field encoding and size bounds with the readiness-receipt schema; any later wire-format disagreement blocks integration rather than being resolved independently by workers.

Every ref read/write/history read checks pending state under the ref lock and the global lease. An unresolved descriptor returns `RecoveryRequired`; it cannot expose a prepared log slot as committed history. Explicit recovery runs under the EXCLUSIVE global lease before ref/digest locks and uses only the guarded low-level APIs.

**Normal update sequence:**

1. Prepare all new-root dependencies under the lease. Then acquire the ref lock, read/revalidate current and derive `parent` there. Snapshot orchestration and `ref_put` must change together. Exact-record no-ops do not append history. A repeated snapshot of the same root is a documented no-op; it does not add timestamp-only versions. History-specific callers retain their defined operation semantics.
2. Validate legacy current/history consistency. Choose `max(existing numeric slots)+1` with checked overflow and exclusive allocation, never file count. Gaps are permitted; collision or unreadable history is not permission to overwrite.
3. Durably publish the pending descriptor **before** modifying the committed namespace. Write and synchronize the proposed log record at its reserved slot. Its pending status excludes it from normal history reads.
4. Atomically replace current with the proposed record. This is the visibility/linearization point. Synchronize the current directory and required new ancestors. No ordinary reader can run across this boundary without taking the ref lock.
5. Remove the pending descriptor and synchronize its directory. Only then return ordinary `Published`. Names-index repair may occur separately because names do not determine truth. A cache-repair failure is visible maintenance information, not grounds to roll back a committed ref.

**Recovery decision, not guesswork:**

| Observed current versus descriptor | Recovery action |
|---|---|
| Exact previous value/absence; update never became visible | Abort that update: remove only its matching prepared log slot, flush the removal, then clear/flush the descriptor. Previous committed history remains. |
| Exact proposed value; update may have committed | Never roll back. Requalify proposed dependencies using C03, rewrite/confirm the recorded log and current bytes through fresh checked writes, then clear the descriptor. Report `RecoveredCommitted`, even if the original caller saw an error. |
| Neither value, malformed descriptor, mismatched owned slot, or missing required data | `RecoveryRequired` with no destructive action. Do not infer chronology from timestamps or arbitrary file counts. |

If required namespace flushing fails, retain enough pending information for recovery and preserve the original error. An error after current replacement is not `NotPublished`. An error after successful current confirmation but during journal cleanup is `CommittedMaintenanceFailure`. A descriptor that reappears after a crash is resolved idempotently using the same table. No automatic second `undo` after an ambiguous first `undo`.

**Untag and retention:** untag is a journaled operation with proposed current=absent. Delete/confirm current before retiring that name's history and name index. If current still equals the previous record, no history retirement is permitted. If absence is the committed outcome, recovery finishes retirement, never resurrects the name. Completed untag ends the retention promise for its history, except for other live roots and active readers. Before recreating a name, retire any legacy orphan log namespace under the same protocol so old history is not silently resurrected. Retired metadata may remain diagnostic; it is not a retention root.

**Legacy stores:** existing unambiguous current/log sequences remain readable and can enter the protocol. A current record that disagrees with the latest legacy log is readable as a current root, but history navigation/updates fail with `LegacyHistoryAmbiguous` until explicit reconciliation. Do not invent missing chronology. GC conservatively retains the decodable legacy history and current; an unreadable record aborts destructive collection. Missing names-index entries do not make existing refs disappear. Mixed old/new writers are unsupported; SI-00 must document exclusive upgrade/downgrade handling and accept the sidecar ADR before implementation. No automatic lossy format migration.

### C06 — Strict root discovery and non-destructive uncertainty

GC takes the EXCLUSIVE lease and completes a full mark phase before deleting anything. Enumerate physical authoritative `refs` shards, decode each record, and verify the ref key/name relationship. ENOENT meaning an absent ref is different from an inaccessible shard or a decode error. Never use an empty `list_refs()` result as proof that all roots are absent.

For each live name, mark current independently of retained committed/legacy history. Also preserve every existing root family: Action Cache records and retained OCI metadata/blobs, plus any additional family found by the SI-00 caller/root inventory. Each family needs a strict collector-facing enumeration path: display APIs may remain fail-soft, but their swallowed errors cannot feed destructive GC. Unknown pointer-bearing record formats, incomplete enumeration, pending ref operations, or unreadable required metadata abort the sweep. Do not silently remove an existing root family to simplify this campaign.

A missing referenced object is an integrity error, not an invitation to delete its remaining siblings. An absent optional root-family directory is allowed only when its absence is actually observed, not inferred from a failed read. Removed names do not regain roots merely because old log files exist. Sweep reports must distinguish candidates, objects actually removed, and failed removals.

### C07 — Active readers and exact materialization

A hydrate operation acquires a SHARED lease before resolving its ref. Under the ref lock it obtains a stable committed record or fails on pending state. It releases the ref lock but retains the global lease through the last required object read/materialization. Concurrent untag may succeed; GC must wait until this materialization finishes. The output then consists of independent bytes, not borrowed mutable CAS storage.

Do not hold a store lease while running an arbitrary user command. `bisect` pins each materialization step, not the externally executing predicate; removal between steps may produce an explicit unavailable-history error. This limited guarantee must be documented. `undo` chooses its target and current parent under the same ref transaction and publishes a new transition, rather than blindly copying stale parent metadata.

Tree equality means entry kinds and relative component names, exact regular-file bytes/length, exact stored symlink target text, and explicitly supported metadata. Use an independent lstat/readlink/byte comparator, not the production manifest codec or path-normalizer as the sole oracle. Do not follow symlinks when comparing or writing descendants.

| Surface | Preservation or rejection rule |
|---|---|
| Names | Exact valid UTF-8 components; no lossy conversion or Unicode normalization. Relative manifest paths only; reject traversal, duplicate paths and file/link ancestor conflicts. Check encoding length limits. |
| Unix backslash | Preserve as a literal filename character; never rewrite it to a separator. |
| Destination aliases | Preflight case/Unicode/reserved-name collisions against the actual target filesystem using an owned reservation area; reject unrepresentable trees without overwriting user data. Do not assume all macOS/Windows volumes have the same case behavior. |
| Regular files | Exact bytes and stored Unix permission bits where supported. Windows preserves the supported read-only attribute, not fictional POSIX ownership/modes. |
| Symlinks | Preserve target text and link kind when supported. Otherwise explicit Unsupported; no silent copy or dangling-link omission. Test Windows with and without actual link privilege/capability. |
| Directories | Preserve representable directory paths, including empty directories. Existing format does not promise directory ownership, ACLs, xattrs or arbitrary directory modes. |
| Other metadata | Timestamps, ownership, ACLs, xattrs, hardlink identity and special files are outside exact-tree equality unless separately contracted. No hidden success claim for their preservation. |

Hydrate must validate names/types before user-output writes. On later failure it reports failure and the partial-output/owned-cleanup policy, never success with omissions. It must not delete a pre-existing user directory during cleanup. Atomic all-or-nothing destination replacement is not promised by this campaign.

### C08 — Observable failure boundaries

The table applies to one operation's effects. Under concurrency, a failed operation must not restore old state over another successfully committed operation.

| Failure boundary | CAS / ref / history state | Next legal action |
|---|---|---|
| Capture/staging/verification fails | No new ref/history; only owned staging and possibly already prepared orphan objects | Cleanup owned staging; correct source and recapture |
| Payload installed; required barrier fails | Object visible but no lease-local readiness proof; no new ref/history | Fresh qualified rewrite or fail; existence-only reuse prohibited |
| Dependencies prepared; descriptor not published | Old current/history; data may be orphaned | Retry after error cause addressed |
| Descriptor or prepared log exists; current remains old | Old logical version; affected normal history blocked while unresolved | Abort matching preparation via explicit recovery; no ghost version |
| Current replaced; confirmation fails | `CommitUncertain`; pending descriptor tracks transition | Recover by exact before/after comparison; never assumed rollback |
| Current confirmed; pending cleanup fails | `CommittedMaintenanceFailure`; retain new logical version | Finish recovery/cleanup without replaying the operation |
| Process dies before reply after all barriers | Publication may already have completed | Inspect current/operation evidence; lost reply is not proof of failure |
| Root discovery incomplete | GC does not start destructive sweep | Restore access/resolve pending state and retry enumeration |
| Reader active; untag commits | Materialization protected by its lease; future reads may see absence | Complete active reader, then allow collection |

## 3. Global invariants

| ID | Required property |
|---|---|
| I01 | Stable-source capture is fresh; stat equality never substitutes for this invocation's bytes. |
| I02 | Published digest/length refer to owned verified bytes; cleanup cannot affect another allocation. |
| I03 | No readiness receipt/proof before required payload barriers; confirmed objects are immutable to normal writers; existence is not readiness. |
| I04 | Each operation holds one correctly scoped global lease; no nested global acquisition or reversed key-lock order. |
| I05 | Failed precommit work contributes no committed history; operation-local rollback never overwrites another commit. |
| I06 | Current, parent and committed history are interpreted under the same per-ref protocol; uncertainty is explicit. |
| I07 | Authoritative root discovery is complete before sweeping; no root-family error becomes empty success. |
| I08 | Acknowledged current/history data remain retained while the name is live, except an explicit retention/untag action; active materialization stays protected. |
| I09 | File/tree compatibility is preserve-or-reject, never lossy success. |
| I10 | Evidence identifies the code, binaries, tests and platform actually exercised. |
| I11 | Resource/performance acceptance is decided from preregistered measurements; no optimization weakens integrity to pass. |
| I12 | Scope, ownership and authorization remain explicit; no source execution or merge is implied by this plan. |

## 4. Work graph and integration ownership

```text
SI-00: contract ratification + caller inventory + platform/test capability
   ├── SI-01: shared CAS/lease/result foundation ── G-FOUNDATION ──┐
   └── SI-02: pure capture/oracle work against frozen seam ──────┤
                                                              v
            SI-03: ref/history/GC + publisher/readers integrated together
                                                              v
                     SI-04: integrated adversarial proof suite
                                                              v
                  SI-05: independent qualification + perf gate
                                                              v
                              owner review (no auto-merge)
```

SI-02 may implement its pure traversal/oracle portion in parallel with SI-01 only after SI-00 accepts the seam. Its production integration and DoD depend on G-FOUNDATION. SI-03 does not start implementation against changing CAS/lock semantics. Ref locking and the snapshot caller are integrated in SI-03, not deferred to a later test-only package. SI-04 may review/design experiments from SI-00 onward, but cannot qualify nonexistent integrated behavior. Test infrastructure is delivered in SI-00, not postponed to SI-05.

At most two implementation workers run in the initial wave; read-only review can be a third role. One writable worktree/branch per worker. Every worker targets a draft PR at `fix/snapshot-integrity`, never main. Coordinator integrates one reviewed change at a time after checking the exact expected head; no force-push. Rebase/reconcile moved inputs and rerun affected tests. Package artifacts stay isolated; issue prose cannot override this specification.

### Work package SI-00 — freeze seams and make evidence executable

**Tracking:** PR #146. **Owner:** coordinator. **Entry:** later execution authorization. **Dependencies:** none.  
**Ownership:** accepted design/compatibility records, caller/root inventory, dedicated test-support and minimal CI wiring, evidence validator and baseline fixtures. No remediation source edits except explicitly reviewed internal seam declarations if needed before fan-out; these cannot change behavior.

#### Success Criteria

- SC00.1: C01–C08 map to named functions/types and callers, with no conflict between helper and transaction outcomes.
- SC00.2: each required native platform executes a smoke witness and returns readable logs/results before its dependent implementation claims readiness.
- SC00.3: incident diagnostics, test-oracle recipes and performance methodology are available to implementers from the start. The accepted foundation seam includes bounded receipt and pending-descriptor formats, not just Rust signatures.

#### Quality Standards

- QS00.1: read applicable ADRs; record acceptance of the readiness-receipt and pending-sidecar protocols and intentional semantic changes before implementation. No silent format/dependency promotion.
- QS00.2: add narrowly scoped CI/support only; preserve unrelated workflows, security boundaries, tests and failure visibility. Historical skips are inventoried, not copied into causal suites.
- QS00.3: capabilities require receipts, not runner labels or API permissions. Pin the baseline toolchain; resource costs are disclosed.

#### Completeness Criteria

- CC00.1: enumerate all callers of `atomic_write`, `ingest_file`, `put_bytes`, `ref_put`, `ref_get`, `ref_log`, `ref_remove`, `list_refs`, GC root loaders and `materialize_file` at the frozen SHA; classify mutation/read behavior and test ownership.
- CC00.2: include snapshot, build snapshotting, tag/import metadata, history/undo/bisect, hydrate, AC and image sidecars. Out-of-scope consumer behavior still receives regression coverage if a shared helper affects it.
- CC00.3: deliver `platforms`, `seams`, `caller-map`, `expected-tests`, incident hypothesis table and fixture hashes as compact evidence records. All relevant CI/bootstrap paths have an owner.
- CC00.4: establish privilege-aware Windows symlink and Unix permission tests, actual CoW/fallback reporting, artifact collection, exact-SHA checkout verification and a nonzero-test smoke run. Create the performance baseline and budget-registration step in C10.

#### DoD

- DOD00.1: contract/compatibility review is recorded; mandatory profiles have capability receipts or clearly blocked downstream work. An unavailable platform is not a pass.
- DOD00.2: inventory and test identifiers are reviewed; evidence-validator negative fixtures reject wrong SHA, empty suite and missing artifacts.
- DOD00.3: coordinator publishes one exact foundation input SHA and accepts the seam definitions. Only then can SI-01 and SI-02's isolated portion become READY. Nothing is marked implemented by SI-00 alone.

#### Invariants

- INV00.1: I10/I12; no credentials, sibling changes, invented runners or fictitious agents.
- INV00.2: authoring the plan is distinct from executing this bootstrap; current capability entries remain NOT VERIFIED until measured.
- INV00.3: shared semantics do not change while independent workers consume them; an amendment invalidates affected readiness/evidence.

### Work package SI-01 — CAS, staging, lease and phase-result foundation

**Tracking:** [#147](https://github.com/gmhelmold/hugr-lightr/issues/147). **Owner:** storage foundation implementer. **Entry:** SI-00 accepted.  
**Ownership:** `lightr-store/src/store/cas/**`, required CoW helper changes, global lease/digest-lock primitives, Store wrappers and minimal phase-result definitions. Those shared files are exclusively owned here until G-FOUNDATION. Per-ref transaction logic belongs to SI-03. Mechanical formatting is a separate commit.

#### Success Criteria

- SC01.1: a collision or failing copy cannot touch another operation's staging, and the public entry point cannot bypass staged verification.
- SC01.2: a failed post-install barrier cannot become a later successful existence-only reuse; C03 requalification or an explicit failure occurs.
- SC01.3: checked native flushing, single-lease composition and concurrent same-digest publication satisfy C01–C03 without changing payload encoding; confirmed objects cannot be replaced by later writers.

#### Quality Standards

- QS01.1: RAII ownership, atomic reservation, bounded retries, streaming buffers and typed contextual errors; no full-file memory buffer, unsafe destination unlink, hardlink to live input or fsync-only recovery.
- QS01.2: no process-global failure hooks. Use scoped test seams sharing the production path and retain original error kind/phase. New unsafe or platform-specific code receives line-level review.
- QS01.3: native clone and copy-fallback outcomes are reported separately; Windows source read-only attributes and flush-handle rights are exercised, not inferred from a build.

#### Completeness Criteria

- CC01.1: implement all C03 call sites, not only `finish_ingest`; update the caller inventory and preserve metadata-writer behavior required by SI-03.
- CC01.2: E01–E04 and E09: forced allocation collision, threads/processes, failed fallback, source/staged mutation, wrong digest/length, zero/large/read-only files, valid/missing/malformed readiness receipts, existing valid/corrupt destinations, immutable confirmed-object reuse, post-rename failure/retry, flush failure and nested-lease negative controls.
- CC01.3: mechanical rustfmt fixes and diagnostics-only incident probes are separately attributable. Preserve original fixture; use C09 to distinguish the ENOENT hypotheses.
- CC01.4: test lease-local dedup failure wake-up, cancellation, object count semantics and progress across distinct digests. Orphan cleanup must not require a new daemon.

#### DoD

- DOD01.1: targeted store tests/formatter/clippy and required native witnesses pass at the exact worker SHA; mutants have their specified dispositions.
- DOD01.2: independent or explicitly labeled self-review checks phase propagation and lock lifetime, including shared helper consumers. No surviving causal mutant is ignored.
- DOD01.3: coordinator integrates the reviewed foundation, verifies its seam matches SI-00, and records G-FOUNDATION. Incident status may remain unresolved, but must remain an explicit final gate; no false causal closure.

#### Invariants

- INV01.1: I02/I03/I04; a proof is issued only for fully prepared bytes and cannot outlive its lease.
- INV01.2: errors after visibility are not rewritten as no-effect failures; already committed user refs are untouched by CAS cleanup.
- INV01.3: platform failures propagate; no performance shortcut reintroduces stat/existence trust across operations.

### Work package SI-02 — fresh, fail-closed capture and faithful path semantics

**Tracking:** [#148](https://github.com/gmhelmold/hugr-lightr/issues/148). **Owner:** capture implementer. **Entry:** SI-00 seam accepted. **Production integration dependency:** G-FOUNDATION.  
**Ownership:** scan/capture implementation and index cache internals in `lightr-index`, pure path validation/oracle fixtures and capture tests. No CAS, global locks, refs, GC or snapshot orchestrator changes. SI-03 wires the verified path into every publisher.

#### Success Criteria

- SC02.1: warmed-index AAAA→BBBB with restored stat fields captures BBBB on a stable source; all snapshot identities come from this invocation's captured bytes.
- SC02.2: selected traversal/read/metadata/link failures cannot yield a successful incomplete capture; correcting the cause permits a fresh retry.
- SC02.3: supported names/types are preserved exactly; invalid/unrepresentable paths are rejected rather than silently normalized.

#### Quality Standards

- QS02.1: stage-derived length/digest, bounded memory and one-attempt changed-source policy; no blanket retries or special-casing fixtures.
- QS02.2: preserve explicit ignore rules. Permission tests run under an identity proven unable to read the target; root bypass is not success evidence.
- QS02.3: the oracle uses independent filesystem observations and bytes, not the implementation's index/codec as its only truth source.

#### Completeness Criteria

- CC02.1: E05/E06/E12 and capture parts of E03: restored mtime/inode/size, stale/corrupt cache, failed cache save/retry, vanished entry, unreadable subtree, failed read-link, valid/dangling links, empty directories and source changes.
- CC02.2: Unix literal backslashes, UTF-8 rejection, alias collisions, path traversal/ancestor conflicts, codec length limits and intentionally ignored files are explicit tests.
- CC02.3: produce a verified capture result against the real SI-01 seam plus compatibility tests for existing scan/status consumers. Pure seam doubles are not final production-path evidence.

#### DoD

- DOD02.1: pure tests may be reviewed early; package closure waits for G-FOUNDATION integration and real prepare/capture tests on supported profiles.
- DOD02.2: removing freshness/error propagation must fail the designated negative witnesses or receive a justified equivalent-mutant classification after redesign.
- DOD02.3: coordinator records the integrated capture SHA and hands the actual function/call-site list to SI-03. No source-data identity decision remains implicit.

#### Invariants

- INV02.1: I01/I02/I09; the index never certifies a snapshot's bytes, and a failed read never becomes an ignore rule.
- INV02.2: no silent change of filename, link kind or target text; unsupported metadata is not advertised as preserved.
- INV02.3: failed capture does not mutate ref/history; unrelated concurrent commits are not rolled back.

### Work package SI-03 — one coherent ref/history/GC and reader integration

**Tracking:** [#149](https://github.com/gmhelmold/hugr-lightr/issues/149). **Owner:** transaction integration implementer. **Entry:** SI-01 foundation and SI-02 production seam integrated; pending-protocol ADR accepted.  
**Ownership:** refs/per-ref locks/pending descriptor, strict root loaders and readiness-receipt invalidation, GC, `snapshot.rs` plus its caller adapters, hydrate and timeaxis/history consumers. Core/shared signature changes require coordinator review. SI-01 global/CAS primitives are consumed, not independently redefined. This deliberately replaces the v1 split between SI-03 and the later SI-04 publisher edits.

#### Success Criteria

- SC03.1: current/parent/history implement C05/C08, including interrupted updates and untag/recreation. A failed prepared B never turns `[A,P]` into a false history `[B,A,P]`.
- SC03.2: authoritative refs cannot disappear from GC because a name index was omitted or unreadable; no destructive sweep follows incomplete root discovery.
- SC03.3: the real snapshot entry point uses verified capture/preparation; active hydrate survives concurrent untag+GC under C07.

#### Quality Standards

- QS03.1: one bounded per-ref journal, explicit outcomes and fixed lock order; no generic WAL/database or silent on-disk migration. All consumers obey pending-state checks.
- QS03.2: preserve original error and operation identity; legacy ambiguity is explicit. Do not invent chronology, reinterpret attempted versions as committed, or resurrect untagged roots.
- QS03.3: keep unrelated ref progress and existing AC/image-root retention. Names/log display conveniences never determine destructive authority.

#### Completeness Criteria

- CC03.1: E07–E11/E13/E14 plus all C08 boundaries, exercised through both public APIs and CLI where applicable.
- CC03.2: include `ref_put`, removal, same-name recreation, snapshot parent read, tag/import root publication, history/undo/bisect, hydrate, names-index discovery, all existing GC root-family adapters and legacy fixture decoding.
- CC03.3: verify log gaps/overflow/exclusive allocation, pending checksum/path validation, before/after/neither recovery, interrupted retirement, missing names, malformed current/log/sidecars and incomplete shard enumeration.
- CC03.4: implement targeted recovery entry/diagnostics and demonstrate that recovery itself is idempotent after interruption; no manual deletion of pending files is presented as recovery.

#### DoD

- DOD03.1: no partial integration declares ref serialization complete while its snapshot caller still reads parent outside the protocol. All affected consumer tests accompany the same reviewed integration unit.
- DOD03.2: each C08 row has an observed correct result/recovery; old/new trees are compared against independent fixture bytes, not merely return codes.
- DOD03.3: coordinator reviews compatibility, recovery and caller/root completeness, integrates the unit and publishes the candidate SHA for SI-04. Known unresolved integrity failures block this transition.

#### Invariants

- INV03.1: I03–I09; prepared logs are never visible committed versions and current remains an independent GC root.
- INV03.2: no ref-level error handling undoes another writer's commit; uncertain visibility is never reported as guaranteed rollback.
- INV03.3: a reader's lease covers its last required read, not an arbitrary external command; untag defines an explicit retention boundary.

### Work package SI-04 — causal proof of the integrated behavior

**Tracking:** [#150](https://github.com/gmhelmold/hugr-lightr/issues/150). **Owner:** adversarial test implementer/reviewer. **Entry:** integrated SI-03 candidate; test design can start earlier read-only.  
**Ownership:** dedicated adversarial tests, isolated mutant patches, evidence and small test-support adapters. No unilateral production fixes, shared workflow changes or weakening of earlier tests; findings return to their owning package.

#### Success Criteria

- SC04.1: each invariant has a witness capable of distinguishing the candidate from a relevant faulty implementation.
- SC04.2: tests prove contenders reached the intended boundary without requiring a correct implementation to enter a mutually excluded region.
- SC04.3: process-death/retry/reader scenarios match C08 and exact output-tree checks, not just helper behavior.

#### Quality Standards

- QS04.1: scoped seams, barriers/channels and nonblocking acquisition probes; sleeps are not scheduling proof. Watchdogs bound failures but do not by themselves establish mutant causality.
- QS04.2: classify every mutant as KILLED_CAUSALLY, SURVIVED, EQUIVALENT_WITH_PROOF, NOT_EXERCISED or INVALID_TEST. Do not weaken redundant protections to force a kill.
- QS04.3: no global environment mutation hooks, leaked children, dirty mutant delivery tree or production-only bypasses hidden behind mocks.

#### Completeness Criteria

- CC04.1: execute E01–E16, using real implementations/public paths where specified, and cover every C08 failure row. Any equivalent control gets an alternate causal witness for the invariant.
- CC04.2: separately remove the relevant verification path, freshness rule, lease interval, history filtering/recovery and strict-root behavior; verify intended assertions, then restore and rerun the candidate.
- CC04.3: exercise multiple writers and processes, independent refs, failure cancellation, exact old/new bytes, bounded retries and no source mutation after successful preservation affecting CAS.

#### DoD

- DOD04.1: the reviewer inspects each oracle and actual synchronization trace. Empty/filtered-away suites, compilation failures and unrelated test failures do not qualify.
- DOD04.2: no unexplained surviving causal mutant; all execution receipts identify source/binary hashes and restored-tree checks. Production defects reopen SI-01/02/03 and invalidate dependent evidence.
- DOD04.3: coordinator integrates only test/support/evidence changes, revalidates the full integrated candidate and hands SI-05 a frozen code/test/workflow identity.

#### Invariants

- INV04.1: I05/I10/I12; test controls cannot become delivered production behavior or harm real user data.
- INV04.2: a correct lock design must terminate under the test harness; causality is established by checkpoints, not guessed timing.
- INV04.3: independent review is not claimed when the implementation author reviewed itself.

### Work package SI-05 — native qualification and product acceptance

**Tracking:** [#151](https://github.com/gmhelmold/hugr-lightr/issues/151). **Owner:** reviewer distinct from implementation when available. **Entry:** SI-04 candidate frozen and C10 budget registered.  
**Ownership:** qualification runs, compact evidence records and comparison results. CI infrastructure already exists from SI-00. Missing wiring reopens SI-00; source defects return to their owners rather than being silently fixed during certification.

#### Success Criteria

- SC05.1: mandatory native profiles execute the expected causal/acceptance tests at the actual candidate identity; qualified claims match platform capability.
- SC05.2: C09 incident disposition and C10 performance acceptance are explicit, supported and reviewed; no unexplained candidate snapshot failure is ignored.
- SC05.3: a fresh checkout can reproduce recovery/materialization and reject invalid evidence using the documented commands.

#### Quality Standards

- QS05.1: pinned toolchain, exact checkout and binary hashes; distinguish tested PR merge from branch head. Existing failing baselines remain visible.
- QS05.2: no blanket skips, `continue-on-error`, retrospective budget relaxation or benchmark claims from missing artifacts. Physical power-loss guarantees are not inferred from process kills.
- QS05.3: review residual risk, performance and backward compatibility, not only build/check statuses. Self-review cannot be labeled independent.

#### Completeness Criteria

- CC05.1: targeted store/index tests; real CLI acceptance; pinned formatting/clippy; impacted consumer regressions; E01–E16 dispositions; Linux x86_64/aarch64, macOS x86_64/arm64 and native Windows store/index execution with capability-specific rows.
- CC05.2: compare base, audited patch and final candidate fixtures; include checked failure/recovery, native CoW versus copy and the existing macOS failure reproducer. Execute full workspace checks on supported feature/target combinations established in SI-00.
- CC05.3: fill evidence schema, retention/expiry, independent review identity, budget verdict, incident disposition and all R01–R14 implementation-evidence links. Design closure is not implementation closure.

#### DoD

- DOD05.1: all mandatory rows have valid evidence, no unresolved integrity blocker or causal survivor remains, and the fresh-checkout reproduction passes.
- DOD05.2: performance thresholds registered before final candidate measurement are met; otherwise BLOCKED_PERFORMANCE or an explicit owner-reviewed plan amendment followed by new measurements.
- DOD05.3: coordinator records QUALIFIED for that exact identity only. PR remains draft until the owner decides it is ready for review; main merge/release require separate owner action. No automatic merge.

#### Invariants

- INV05.1: I01–I12; qualification cannot weaken any contract to create a green status.
- INV05.2: missing native capability/evidence is not cross-compilation success, and an expected Unsupported test is not an exercised feature.
- INV05.3: later code/test/workflow changes invalidate affected qualification; documentation-only provenance is handled explicitly under C11.

## 5. Experiments and causal oracles

Each E-row is a required experiment family. SI-00 resolves it to exact test names; SI-04 checks that they actually execute. A short implementation-specific subcase list is allowed, but no E-row disappears silently.

| ID | Setup/checkpoint and observable oracle | Negative control / owner |
|---|---|---|
| E01 | Force the same allocation candidate twice; first payload remains byte-identical, second retries/errors without unlinking it. Cover failed clone and partial fallback. | Nonexclusive reserve/unlink-other control; SI-01 |
| E02 | Synchronize threads AND subprocesses preparing identical digests; read all resulting bytes, prove one lease-local completion is shared and every waiter terminates on failure. | Shared-temp and premature-ready controls; SI-01 |
| E03 | Change supplied source/stage at the actual expected-digest verification boundary; mismatch fails before readiness/ref publication. For stage-first capture, prove the final digest denotes that owned stage, not an earlier stat observation. | Bypass production verification, not just its helper; SI-01/02 |
| E04 | Fail directory confirmation AFTER object install, end A, then run B in another process. B must perform fresh qualification or fail; it cannot return success on path existence. Also prove a later writer cannot replace an already receipt-confirmed object. Fail payload and receipt flush separately. | Exists-only and unchecked-flush controls; SI-01 |
| E05 | Warm index with AAAA; rewrite BBBB at same length/inode and restore mtime before capture. Decode via independent oracle and hydrate BBBB. | Stat-only digest reuse; SI-02 |
| E06 | Select unreadable/unhashable file/subtree, vanished entry and failed read-link. Public capture/snapshot fails; fix cause and recover previous then new tree. Verify permission premise under actual identity. | Swallowed-IO/empty-link controls; SI-02/03 |
| E07 | Begin with current A/history [A,P]; fail B after prepared log but before current. Normal history refuses unresolved state; recovery produces [A,P], and undo targets P. | Expose prepared history as committed; SI-03 |
| E08 | Fail after current replacement and at each journal cleanup/retirement boundary. Recover by before/after table; no double undo, ghost version or untag resurrection. | Treat all errors as rollback; SI-03 |
| E09 | Pause publication after readiness but before ref commit. Collector signals its attempt OUTSIDE the guarded region, performs a nonblocking EX probe and reports WouldBlock. Release writer, then collector completes; hydrate exact bytes. | Shorten every effective protection of interval; redundant guard removal may be equivalent; SI-01/03/04 |
| E10 | Force history allocation contention; B reports a failed nonblocking ref-lock probe rather than being required to enter A's region. After A releases, both complete with distinct history slots and parent order. | Remove serialization/allocation exclusion; SI-03 |
| E11 | Current A with log B; missing/corrupt/inaccessible names index; failed authoritative shard iteration; unreadable current/log/AC/image metadata. No destructive sweep on uncertainty; A survives when state is decodable. | Names-only/skip-error/current-not-marked controls; SI-03 |
| E12 | Fixtures with links, empty dirs, literal backslashes, case/Unicode aliases and invalid paths. lstat/readlink/raw-byte comparator checks preservation or explicit preflight rejection. | Lossy normalize/follow-link comparator controls; SI-02/03 |
| E13 | Reader resolves A and pauses before final object read; untag commits; collector nonblocking probe is excluded until reader completes. Future new reader sees absence. | Drop reader lease; SI-03 |
| E14 | Existing tag/import/build/history/undo/bisect and root-family fixtures run through changed helpers; repeated snapshot/no-op and recreate-after-untag policy are explicit. | Consumer-bypass/missing-root fixtures; SI-03 |
| E15 | Give evidence validator a wrong checkout/binary, zero tests, skipped required test, absent artifact, dirty mutant or stale workflow. Each is rejected; valid receipt accepted. | Permissive evidence gate; SI-00/05 |
| E16 | Bounded seeded stress complements deterministic schedules; record concurrency, operations, seed and output trees. Compare incident diagnostic/control variants under C09. | Original faulty fixture/control where applicable; SI-04/05 |

Every concurrent experiment has: fixture identity; checkpoint outside/inside the relevant critical section; proof of contender arrival; expected observations before release; explicit release; bounded join; owned cleanup. Never wait for a correct contender to enter a region the lock intentionally excludes. Nonblocking probes demonstrate exclusion; watchdog timeouts only report harness/system failure until a trace explains causality.

For each mutant record compilation result, actual executed test names, failing assertion/trace, and restored-candidate rerun. `SURVIVED` on a relevant behavioral mutation blocks closure. `EQUIVALENT_WITH_PROOF` requires reviewed control-flow/redundancy reasoning and an alternative witness, not relabeling a weak test. Crashes of the test harness, compile errors and unrelated failures are not killed mutants.

## 6. C09 — ENOENT incident investigation without causal overclaim

Use the existing `a11_gc` fixture unmodified: 200 identical 1 KiB files, nested directories, executable file, symlink, empty directory and its large file. Keep formatting-only, diagnostics-only and causal implementation commits separable. Diagnostics record operation/staging identity, syscall stage, paths, digest prefix, actual copy rung/fallback and original error; never payload contents.

| Hypothesis | Distinguishing observation / controlled intervention |
|---|---|
| H1 staging name collision | Two operations claim the same temp path; forced name collision reproduces ownership violation; exclusive reservation changes that exact behavior. |
| H2 destination-finalization race | Different staging paths, same destination; failure at install/attribute/barrier. Serialize finalization without changing source allocation to isolate it. |
| H3 source/path failure | Original source open/stat/read fails; staging/destination changes alone do not explain it. Reproduce source lifetime/path resolution independently. |
| H4 CoW fallback failure | Trace identifies failed native clone and fallback/partial destination handling; force copy-only versus actual clone with identical fixture. |

These hypotheses are not asserted equally likely or exhaustive. Add one only with a discriminating observation. Compare base, audited patch and candidate using the same diagnostics patch where behavior-preserving. Record when instrumentation alters timing.

Disposition is `CAUSE_CONFIRMED` only with a causal trace plus a reproducer/control that distinguishes the repair. Otherwise use `HISTORICAL_CAUSE_UNRESOLVED`, even when a real allocator bug is fixed. An unexplained candidate failure always blocks qualification. If the historical incident cannot be reproduced after a bounded investigation, only an explicit owner-reviewed risk disposition can replace causal closure; do not stall indefinitely or silently call repeated passes proof. The investigation budget is preregistered in SI-00 as run counts/platforms, not an invented completion-time promise.

## 7. C10 — Performance evidence and acceptance before final measurement

Correctness is mandatory; unlimited cost is not automatically acceptable. C03 is a safe reference path, not a claim of optimal warm-cache performance. Do not revert freshness or cross-operation qualification to match an unsafe baseline.

Preregister deterministic fixtures: F0 empty; F1 the unchanged existing acceptance fixture; F2 10,000 distinct 4 KiB files; F3 10,000 identical 1 KiB files; F4 one 256 MiB file; F5 the link/name/metadata fixture; F6 histories of 0, 32 and 256 committed transitions, measuring one additional update, history read and GC without confusing setup cost with operation latency. These sizes are chosen test inputs, not measured results. Record generator version/seed and raw fixture hash. Local stress may add 1 GiB files after capacity check without replacing the mandatory set.

For small fixtures use 3 warmups plus 20 measured paired runs; for F4 use 1 warmup plus 7 measured runs. Record all samples, median, maximum and dispersion; do not sell a p99 from a tiny sample. Larger tail studies require a separately registered sample count. Interleave base/audited/reference/candidate ordering with a recorded seed. Separate new-store capture, warmed-store reuse, hydrate, and any OS-cache control actually achieved. Do not call a cache cold unless that condition was established.

Measure wall time, bytes read/written/hashed where instrumented, actual copy rung, peak memory, metadata/fsync calls, unique versus logical object counts and GC wait after writers quiesce. Selected initial resource targets: streaming buffers at most 1 MiB per active payload worker; no whole-file buffer; bounded payload worker count recorded per run. These are engineering limits, not benchmark observations; metadata storage is separately O(number of entries).

**Two mandatory gates:** (1) SI-00 freezes the method and gathers baseline/capability data. (2) After the first correct SI-01/02 reference implementation is available, before final candidate performance results are collected, the coordinator and owner/reviewer record numeric fixture-specific latency/memory/GC-wait budgets and their justification in `performance-budget.json`. Compare to the correct reference and disclose deltas against the unsafe historical base as context. The implementer cannot approve its own unexplained budget expansion.

The budget record includes reference code/binary hashes, samples, metric/unit/direction, absolute threshold, workload profile, rationale and approving identity. Missing budgets mean BLOCKED_PERFORMANCE; they do not mean unlimited acceptance. Threshold changes after seeing final results require a versioned plan amendment and new measurements. This revision intentionally contains no fabricated baseline or numeric latency budget without hardware evidence.

## 8. C11 — Minimal, verifiable evidence and resume protocol

Store compact evidence at `docs/plans/snapshot-integrity/evidence/SI-NN.md` plus small structured receipts; raw logs remain artifacts with checksums/expiry and a durable summary. No giant transcript, secret, user workload payload or machine-specific required path is committed.

Each receipt must identify:

```yaml
packet: SI-NN
plan_version: '2.0'
status: EVIDENCE_READY
requested_head_sha: actual-head
checkout_sha: actual-tested-commit
checkout_parents: []
source_tree_sha: actual-tree
code_input_fingerprint: hash-of-reviewed-code-tests-lockfile-build-and-workflow-inputs
plan_commit: actual-plan-commit
binary_sha256: actual-built-binary
runner: {os: actual, arch: actual, filesystem: actual, toolchain: actual, profile: actual}
commands: [] # exact argv, cwd, exit, actual named tests and pass/fail/ignored counts
required_test_inventory_sha256: actual-inventory
artifacts: [] # URL, SHA-256, retention, durable summary
negative_controls: [] # mutant patch hash, classification, causal assertion, restored rerun
incident_disposition: actual-or-unresolved
performance_budget_sha256: actual-or-blocked
review: {identity: actual, independent: false, findings: []}
residual_limits: []
```

Placeholders above are schema examples, never acceptable results. A small validator checks exact checkout/parents, build-before-run ordering and binary path/hash; nonzero expected named-test execution; required profile/capability rows; artifact presence/checksums; mutant restoration; and matching frozen budget. Cargo exit 0 with every causal test filtered out fails validation.

A GitHub PR merge checkout is a separate commit; compare its tree/parents with the intended candidate rather than relabeling it the branch head. Later source, test, dependency, build-script or workflow changes invalidate affected evidence. Documentation-only additions may inherit execution evidence only after a reviewed path-diff plus code-input fingerprint proves those inputs unchanged; record both SHAs. Do not demand a self-referential commit hash in its own evidence file.

States: `PLANNED → PREREQUISITES_VERIFIED → READY → DISPATCH_REQUESTED → RUNNING → EVIDENCE_READY → REVIEWED → INTEGRATED → QUALIFIED`, with precise BLOCKED reasons. No assignment/session means no RUNNING. Resume reads live PR, this version, dispatch receipt and issue comments; records exact head and ownership; reconciles changes; then claims only permitted work. No background monitoring or automatic future dispatch is established by this plan.

## 9. R01–R14 design-resolution matrix

These are **planning resolutions**, not proof the corresponding code defects are fixed.

| Review finding | Selected resolution | Package / required evidence |
|---|---|---|
| R01 failed-finalization reuse | C01/C03: ordered readiness receipts, immutable confirmed objects and fresh rewrite of unconfirmed payloads | SI-01; E04 |
| R02 ghost committed history | C05: pending descriptor, before/after recovery, guarded consumers | SI-03; E07/E08/E14 |
| R03 stale stat cache | C04: snapshot identity from this invocation's owned bytes | SI-02; E05 |
| R04 hidden GC roots | C06: authoritative refs enumeration, strict family loaders, abort incomplete sweep | SI-03; E11 |
| R05 false parallel independence | C02 and work graph: freeze seam, G-FOUNDATION, ref plus caller same integration | SI-00/01/03; E09/E10 |
| R06 late evidence infrastructure | SI-00 provides and verifies native/CI capability before dependent work | SI-00; platform receipts/E15 |
| R07 ambiguous failure outcomes | C01/C05/C08: selected visibility point, recovery, retention and operation-local effects | SI-03; E07/E08/E13 |
| R08 weak temporal/mutant oracle | Experiment recipes, nonblocking contender proof and explicit mutant dispositions | SI-04; E01–E16 |
| R09 undefined tree fidelity | C07 domain and independent comparator; preserve-or-reject profiles | SI-02/03; E12 |
| R10 active readers | C07 per-materialization lease, explicit untag and bisect boundaries | SI-03; E13 |
| R11 ENOENT over-attribution | C09 diagnostic/control matrix and honest bounded disposition | SI-00/01/05; E16 |
| R12 unbounded performance acceptance | C10 baseline, fixed methodology and preregistered numeric budget gate | SI-00/05; budget/measurement receipts |
| R13 incomplete impact map | SI-00 caller/root inventory; SI-03 owns actual consumers together | SI-00/03; E14 |
| R14 weak evidence association | C11 exact checkout/binary/test validation, negative evidence fixtures | SI-00/05; E15 |

## 10. Exit decision

Design review may accept this specification without asserting runtime correctness. Implementation cannot be dispatched merely because R01–R14 now have design responses. SI-00's ADR/compatibility, platform, seam and evidence prerequisites must be real; future execution must be explicitly authorized.

Campaign qualification requires all six packages' Success Criteria, Quality Standards, Completeness Criteria, DoD and Invariants satisfied; all mandatory C08/E-rows and profile receipts present; no unreviewed causal survivor or integrity failure; C09/C10 dispositions accepted; and current-head evidence validated under C11. Main merge/release remains the owner's separate decision.

## Primary sources and code anchors

Design decisions above are proposed here; sources document the underlying constraints, not a claim that this protocol is already proven.

- [Baseline snapshot publisher](https://github.com/gmhelmold/hugr-lightr/blob/e4a53417f6fe7da8c4f44908af9525536443d0fa/crates/lightr-index/src/index/snapshot.rs), [scan](https://github.com/gmhelmold/hugr-lightr/blob/e4a53417f6fe7da8c4f44908af9525536443d0fa/crates/lightr-index/src/index/scan.rs), [index codec](https://github.com/gmhelmold/hugr-lightr/blob/e4a53417f6fe7da8c4f44908af9525536443d0fa/crates/lightr-index/src/index/codec.rs).
- [CAS](https://github.com/gmhelmold/hugr-lightr/blob/e4a53417f6fe7da8c4f44908af9525536443d0fa/crates/lightr-store/src/store/cas/mod.rs), [refs](https://github.com/gmhelmold/hugr-lightr/blob/e4a53417f6fe7da8c4f44908af9525536443d0fa/crates/lightr-store/src/store/refs.rs), [GC](https://github.com/gmhelmold/hugr-lightr/blob/e4a53417f6fe7da8c4f44908af9525536443d0fa/crates/lightr-index/src/index/gc.rs), [hydrate](https://github.com/gmhelmold/hugr-lightr/blob/e4a53417f6fe7da8c4f44908af9525536443d0fa/crates/lightr-index/src/index/hydrate.rs), [timeaxis](https://github.com/gmhelmold/hugr-lightr/blob/e4a53417f6fe7da8c4f44908af9525536443d0fa/crates/lightr-index/src/index/timeaxis.rs).
- P1: [Rust File synchronization/opening](https://doc.rust-lang.org/std/fs/struct.File.html).
- P2: [Microsoft FlushFileBuffers handle rights and return semantics](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers).
- P3: [Linux fsync file versus directory barriers](https://man7.org/linux/man-pages/man2/fsync.2.html).
- P4: [Rebello et al., Can Applications Recover from fsync Failures?, USENIX ATC 2020](https://www.usenix.org/conference/atc20/presentation/rebello).
- [Rust rename platform behavior](https://doc.rust-lang.org/std/fs/fn.rename.html), [SQLite atomic-commit explanation](https://www.sqlite.org/atomiccommit.html), [Git Racy Git](https://git-scm.com/docs/racy-git). These are constraints/precedents, not mandates to replace the store with another system.
