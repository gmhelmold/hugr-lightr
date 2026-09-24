# SI-00 direct binding review — 2026-09-17

Source revision: `98f5d3bc1146cbff964f8495a515e40c8e621c16`, tree
`ef908135d5f490e9c49fa67efc8a1b6ef9f7de71`. Reviewer: ChatGPT, author/coordinator;
not a separate human or coding agent. No production Rust changed in this delivery.

The regenerated inventory contains 491 Rust files and 1,739 textual occurrences.
The added Rust file is the test-only hash example. These are not 1,739 callers.
For the eleven named surfaces below, the direct-call census is 73 sites in 35
files: 70 bind to Store/CAS/ref/image/root surfaces; three bind to unrelated
network or CRI helpers. Imports, local variables, cfg(test) modules and aliases
were checked separately. No `as` alias for these eleven names was found.

## Direct call sites

All paths below are relative to `crates/`. Numbers are lines in the pinned source,
not future implementation locations. The existing `caller-map.md` covers the
transitive verb groups, additional image getters/setters and raw filesystem users.

| File | Line and direct surface | Role / owner |
|---|---|---|
| `lightr-build/src/build/exec_fs.rs` | 77 materialize_file | CAS reader; SI-03/E14 |
| `lightr-cli/src/handlers/diff.rs` | 24,72 ref_log | History reader; SI-03/E19 |
| `lightr-cli/src/handlers/docker/mod.rs` | 233 list_refs | Presentation; SI-03 regression |
| `lightr-cli/src/handlers/docker/run_args.rs` | 171 list_refs | Presentation; SI-03 regression |
| `lightr-cli/src/handlers/history.rs` | 57 ref_get;66 ref_log | Current/history consistency; SI-03 |
| `lightr-cli/src/handlers/images.rs` | 79 ref_get | Named reader; SI-03/E17 |
| `lightr-cli/src/handlers/info.rs` | 62 list_refs;65 list_ac | Diagnostics, not GC authority |
| `lightr-cli/src/handlers/mcp/tools.rs` | 219 ref_log | MCP history; SI-03/E14 |
| `lightr-cli/src/handlers/oci_imageops.rs` | 53 ref_get;70 ref_put | Tag writer, then sidecar copy; SI-03/E17 |
| `lightr-cli/src/handlers/plan.rs` | 74 ref_get | Named planning reader; SI-03 |
| `lightr-cli/src/handlers/rmi.rs` | 66 ref_get;69 ref_remove | Untag then sidecar removal; SI-03/E17 |
| `lightr-cli/src/handlers/run/paths_vz.rs` | 31 ref_get | VZ preparation, not VM qualification |
| `lightr-cli/src/handlers/system.rs` | 63 list_refs;66 list_ac | Presentation; strict GC is separate |
| `lightr-cli/src/handlers/tag.rs` | 60 ref_get;76 ref_put | Tag writer, then sidecar copy; SI-03/E17 |
| `lightr-cri-backend/src/util.rs` | 102 atomic_write | Local CRI helper, NOT CAS |
| `lightr-index/src/index/gc.rs` | 37 list_refs;38 ref_log;58 list_ac;73 list_image_reachable_blobs | Collector root authority; SI-03/E11/E14 |
| `lightr-index/src/index/hydrate.rs` | 32 ref_get;97 materialize_file | Reader lifetime; SI-03/E13 |
| `lightr-index/src/index/snapshot.rs` | 41 ref_get;61 ingest_file;74 put_bytes;89 ref_put | Parent/capture/publication; SI-01/02/03 |
| `lightr-index/src/index/status.rs` | 17 ref_get | Status, not fresh-capture proof |
| `lightr-index/src/index/timeaxis.rs` | 113,159 ref_log;144 ref_put;190 materialize_file | Undo/bisect; SI-03/E13/E19 |
| `lightr-oci/src/oci/history.rs` | 89 ref_get | Also reads both OCI sidecars; SI-03/E17 |
| `lightr-oci/src/oci/images.rs` | 54 list_refs;60 ref_get | Named image reader; SI-03 |
| `lightr-oci/src/oci/push/mod.rs` | 52 ref_get | Export, also config read; SI-03/E17 |
| `lightr-oci/src/oci/retain.rs` | 61 put_bytes | Retained OCI dependencies; SI-03/E14 |
| `lightr-oci/src/oci/rmi.rs` | 40 ref_get;56 ref_remove | OCI untag; SI-03/E17 |
| `lightr-oci/src/oci/save.rs` | 47 ref_get | Export, also retained manifest read; SI-03/E17 |
| `lightr-run/src/network/registry.rs` | 62,116 atomic_write | Imports network/fsutil, NOT CAS |
| `lightr-run/src/run/memo.rs` | 179 ref_get;339,340 put_bytes | Snapshot and LRR1 outputs; preserve memo semantics |
| `lightr-run/src/run/vzmemo.rs` | 112,113 put_bytes | LRR1 output blobs; no VZ execution claim |
| `lightr-run/src/secrets.rs` | 73 ref_get | Named input preparation; no secret-value exposure |
| `lightr-store/src/lib.rs` | 86 put_bytes;91 ingest_file;108 materialize_file;121 ref_get;127 ref_put;133 ref_log;139 list_refs;149 ref_remove;215 list_image_reachable_blobs;232 list_ac | Ten concrete public-to-private wrappers; SI-01/03 |
| `lightr-store/src/store/ac.rs` | 39 atomic_write | CAS metadata helper under Store guard |
| `lightr-store/src/store/imgmeta.rs` | 81,216 put_bytes;87,155,222 atomic_write | Config/manifest values and pointers; SI-03/E17 |
| `lightr-store/src/store/refs.rs` | 65,90,97 atomic_write | Name/log/current writes; SI-03 |
| `lightr-views/src/views/backend.rs` | 226 materialize_file | Optional solidification driver, not wired into hydrate/CLI |

The excluded `handlers/testref.rs` is included only behind its parent module's
`cfg(test)`. `store/ac.rs` calls after line 100 and `store/lock.rs` test calls
belong to inline test modules. Local variables named `ref_log` in diff/MCP are
not additional calls. `healthcheck::{load_for,save_for}` are not Index methods.

## Direct namespace writers and additional consumers

A source scan of literal `refs`, `refs-log`, `refs-names`, `imgmeta`,
`imgmanifest`, `objects`, `ac` and `index` joins found Store initialization,
CAS/refs/imgmeta/AC, index publication, the collector, diagnostic usage, and the
old CLI benchmark. `bench/measure.rs` directly removes its disposable objects,
index and AC directories between samples. The future protocol must reset the
whole owned benchmark store coherently, not leave stale receipts/refs after
wiping only objects. The old benchmark also calls `.ok()` on snapshot/hydrate;
its numbers are not this campaign's performance acceptance evidence. The new
SI00 baseline checks every command and independently compares restored trees.

Build `exec.rs:225/338` stores a raw32 manifest root in AC. Run memo and VZ memo
store LRR1 stdout/stderr records. Both root formats must survive strict marking.
OCI import/pull/retain and every tag/remove/save/push/history/config reader
remain enrolled in the full-tuple conversion. Shared index load/save users,
including standalone status and memo-key computation, must use one separate resource
lock; CRI/network/health state does not become scratch by sharing a helper name.

This closes the baseline direct-name disambiguation and records the inspected
caller boundaries. It is not compiler-resolved whole-program or future cfg
conversion proof, independent review, ADR acceptance or permission to activate.
The final compatibility/activation review must check these rows against the
actual converted code, public error surface and E14/E17 witnesses.
