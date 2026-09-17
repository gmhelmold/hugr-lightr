# SI-00 seams, root families and activation inventory

Source input: `19afb88d94db1d79cfe2a56ce7a99eab84f6abb3`; archive-verified
Rust tree matches PR #154's first commit `8a3098c6`. Proposed refinements are
subject to ADR-0020; this is not an implementation receipt.

## Selected implementation seams

| Contract | Existing surface | Internal implementation obligation / owner |
|---|---|---|
| C01/C03 | `Store::{put_bytes,ingest_file}`, `cas::atomic_write` | `PublicationOutcome` preserves phase, OS cause and operation ID. Preparation returns `PreparedObject<'lease>` only after required barriers. SI-01. |
| C02 | `Store::{write_guard,gc_guard}`, `store/lock.rs` | `StoreLease`, borrowed by private methods; typed cache-resource section and keyed locks. No implicit nested wrapper calls. SI-01. |
| C04 | `Index::{load_for,save_for}`, `scan`, `snapshot` | `capture_verified(source, lease, policy) -> CapturedManifest<'lease>`; identity from owned bytes, not cached stat digest. SI-02; public wiring SI-03. |
| C05 | `ref_put/get/log/remove`, `imgmeta` writers/getters | `resolve_named(lease, RefGuard)` returns one coherent `NamedVersion`; `publish_named` operates on all three tuple components. SI-03. |
| C05 recovery | no equivalent integrated baseline API | `inspect_pending` is read-only; `recover_pending(ExclusiveLease)` acts from journal phase, never guesses from current. SI-03. |
| C06 | `gc`, `list_refs`, `list_ac`, `list_image_reachable_blobs` | strict pending pre-scan independent of refs; strict typed root visitors before sweep. Display-list convenience APIs are not authority. SI-03. |
| C07 | `hydrate`, `hydrate_verified`, `undo_to`, `bisect` | resolve tuple under ref lock, keep lease through final materialization read, never across arbitrary predicate execution. SI-03. |
| C12 | CLI Store initialization / path joins | anchored native-resolution `ValidatedTopology`; cannot be forged by an internal-looking path or lexical collapse of unresolved symlink/.. . SI-02 validation; SI-03 callers. |
| C13 | transient files; existing age-based run-dir pruning | `reap_owned_scratch(ResourceGuard)` only in registered staging domains; run directories, volumes and metadata are not owned scratch. SI-03. |

These are semantic signature requirements, not exported Rust types. SI-00 must
not commit behavior-changing seam declarations before the proposed ADR is
accepted. Receipt/history/pending schema bounds are in ADR-0020.

## Root authority and byte formats

| Family | Baseline writer/reader | Required mark policy |
|---|---|---|
| `objects/` | `store/cas/mod.rs` | sweep targets, never roots by mere existence. Validate manifest/payload digests. |
| `refs/<key>` | `store/refs.rs::{ref_put,ref_get}` | authoritative current; independently retain tree and coherent named OCI metadata. |
| `refs-log/<key>/<slot>` | same module; `index/timeaxis.rs` | retain current-lifetime legacy raw records conservatively; new envelopes retain tree plus historical OCI tuple. Never certify raw history as committed. |
| `refs-names/` | `refs.rs::list_refs` | rebuildable presentation index; absence cannot hide authoritative refs. |
| `ac/`, 72-byte LRR1 | `run/ac.rs`, `run/{memo,vzmemo}.rs` | stdout and stderr digests at offsets 8..40/40..72; exact length/magic; typed strict discovery. |
| `ac/`, 32 raw bytes | `build/exec.rs:338` | **build manifest root** plus its referenced file payloads; do not discard because it is not LRR1. Validate the pointed manifest. |
| `imgmeta/` | `store/imgmeta.rs` | 32-byte pointer to original OCI config CAS object, part of named tuple. |
| `imgmanifest/` | `store/imgmeta.rs`, `imgmeta_codec.rs` | pointer to retained record; retain the record and all descriptor CAS dependencies. Strict decode/length checks. |
| future `objects-ready/` | SI-01 | readiness evidence, not retention root; ordered invalidation before unreachable payload deletion. |
| future pending namespace | SI-03 | any unresolved/unknown/unreadable entry stops sweep, even when refs is empty. |
| index cache | `index/codec.rs` | not a CAS root; shared resource lock protects its filesystem operations/scratch. |
| volumes, run/workspace state | `store/volume.rs`, `run/` consumers | not disposable CAS scratch; preserve existing ownership/retention surface. Do not apply age-only scratch reaping to live user workspaces. |

Unknown AC values are an explicit strict-root blocker, not unconditionally
ignored. This does not redefine memoization keys or add effect replay; only
reachability and lifecycle are in scope. Missing descriptors or a new named
metadata family discovered during caller review blocks activation.

## Producer and consumer groups

- Snapshot: `index/snapshot.rs`, CLI snapshot handler, build step snapshots,
  OCI layer `apply_and_snapshot`, CRI/engine callers. Parent read moves inside
  the same ref transaction as publication.
- Named image operations: `oci/importer`, `oci/retain`, CLI OCI tag/remove/
  inspect/push and build image configuration. Resolve whole tuples once.
- History: CLI diff/history/undo/bisect plus `index/timeaxis.rs`; no caller may
  treat the first raw log entry as proof of current generation.
- Materializers: index hydrate, build FROM/cached steps, CLI run preparation,
  compose/CRI/engine adapters and views. Each needs explicit ownership transfer
  when a lease ends; no borrowed mutable CAS path escapes.
- Shared helpers: store wrappers, CAS primitives, AC and image pointer writes;
  caller compatibility is tested even when that caller's feature is otherwise
  out of campaign scope.

`python scripts/si00/inventory.py --output caller-inventory.json` enumerates all
tracked Rust textual occurrences with file hashes and line context. The published e359bd2 scanner enumerated 490 Rust files and 1,739 conservative
symbol occurrences in run 35209414026. Its broader symbol-reference scan is
different from an earlier local 934-occurrence call-pattern prototype. **This is not type-resolved or cfg-resolved**;
`Path::exists`, test modules and aliases require review. Line context is navigation, not a proven caller. Raw inventory is delivered as an
artifact, not misrepresented as an exhaustive semantic graph.

## Failure, resources and support surface

Inspection returns tuple state, journal phase and attribution separately.
Recovery outcomes are NotPublished, CommitUncertain, CommittedMaintenanceFailure,
Published, RecoveryRequired or RecoveryBlockedResources, retaining OS causes.
Windows checked file flush is not directory-entry power-loss certification.

The operator sequence is read-only inspection -> quiesce participants -> guarded
scratch reaper -> free unrelated space/increase quota if necessary -> phase-based
recovery -> byte-verified hydrate -> strict full GC. Never delete pending/current/
CAS manually to manufacture space. Headroom estimate is the sum of dependencies
needing ordinary rewrite plus bounded metadata staging and native directory/inode
overhead. Concrete per-profile overhead requires disposable resource testing;
no universal minimum free-space number is approved by this document.

Legacy adoption records a new present-day ADOPTED_BASELINE without relabeling old
history. Default navigation stays in the verified suffix. Snapshot-producing
entry points must fail rather than silently inherit OCI sidecars from another
generation. Lost reply with no matching pending is AttributionUnknown, not an
automatic replay or corruption verdict.

## Closure boundary

This inventory and the proposed schema must receive recorded compatibility and
caller review before G-FOUNDATION inputs are accepted. Infrastructure/baseline
execution does not establish typed-proof safety or close the storage defects.
