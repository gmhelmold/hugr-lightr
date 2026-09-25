# SI-00 caller, root and resource inventory

**Source reviewed:** `19afb88d94db1d79cfe2a56ce7a99eab84f6abb3`.
**Method:** `scripts/si00/inventory.py` enumerates all tracked `.rs` files and
records every occurrence of the required symbols, including imports, definitions,
comments and test code. The first execution found 490 Rust files / 1,739 symbol
occurrences. These are NOT 1,739 resolved callers. The full file/hash/line inventory
is an Actions artifact; reproduce it instead of committing a giant source dump.
This document classifies the production boundaries inspected by the author.
Any alias/macro/direct-filesystem writer discovered during integration must be
added before G-ACTIVATION; syntactic coverage alone cannot certify completeness.

Paths below are relative to `crates/`, unless stated otherwise. Consumers of an
identically named helper are not assumed to share one lock or implementation.

| Boundary / existing paths | Current behavior to adapt or preserve | WP / decisive experiment |
|---|---|---|
| `lightr-store/src/lib.rs` | Public wrappers; `Store::open` creates directories and probes CoW before returning. Preflight must precede caller-owned initialization. Borrowed leases cannot be introduced only under old recursive wrappers. | SI-01 primitives; SI-03 activation; E09/E22 |
| `lightr-store/src/store/cas/mod.rs`, `lightr-store/src/store/cow.rs` | CAS read/write/materialize, atomic metadata write, permission/flush/install. | SI-01; E01–E04/E09 |
| `lightr-store/src/store/lock.rs` | Existing kernel SH/EX guard. Same-thread/process and canonical-root aliases need explicit proof; cache is a separate domain. | SI-01/02; E09/E18/E22 |
| `lightr-store/src/store/refs.rs` | Current/history/names operations; `ref_get/ref_log/list_refs/ref_remove`. Current must be an independent root; prepared history cannot be exposed. | SI-03; E07/E08/E10/E11 |
| `lightr-store/src/store/ac.rs` | Raw AC values, collection enumeration and per-call write guards. Consumers use two existing encodings, not only LRR1. | SI-03; E11/E14 |
| `lightr-store/src/store/imgmeta.rs`, `imgmeta_codec.rs` | Config/manifest CAS values and name-keyed pointers; copy/remove/get operations currently independent/fail-soft in places. All named tuple readers/writers need the same transaction. | SI-03; E17 |
| `lightr-store/src/store/volume.rs` | Separate ownership/data plane. Volume `_data` is user data, not ordinary CAS scratch. | SI-00 domain inventory; SI-03 regression |
| `lightr-index/src/index/codec.rs`, `scan.rs`, `status.rs` | Index path is shared across Stores by LIGHTR_HOME/source; cache trust, errors and cache save. Snapshot freshness must not alter unrelated status/memo semantics without review. | SI-02; E05/E06/E18/E22 |
| `lightr-index/src/index/snapshot.rs` | Public snapshot scans then publishes; current code nests wrapper guards and skips `exists` objects. Parent lookup, fresh capture and ref transaction switch together. | SI-03; E03/E05/E07/E09/E10 |
| `lightr-index/src/index/hydrate.rs`, `timeaxis.rs` | Materialization, history/undo/bisect and LRR1 parsing. Reader pins only actual object reads, not arbitrary external predicate execution. | SI-03; E12/E13/E14/E19/E21 |
| `lightr-index/src/index/gc.rs` | Mark from names/logs, LRR1 and retained images, then CAS/run-directory cleanup. Strict pending scan must not derive from current; cleanup authorization cannot come from age alone. | SI-03; E08/E11/E13/E18 |
| `lightr-build/src/build/exec.rs`, `exec_fs.rs`, `exec_instr_from.rs`, `platform.rs` | Step AC, snapshot per instruction, FROM/hydrate and filesystem copies. RAW32 build result roots must remain reachable. Snapshot + config producers join one named transaction. | SI-03; E11/E14/E17 |
| `lightr-build/src/build/compose/{lazy,supervise,up}.rs` | Lazy materialization and stack workspaces; preserve workspace owner lifetime, do not hold global store lock during service execution. | SI-03 consumer regression; E13/E14 |
| `lightr-run/src/run/{memo,vzmemo,supervise_native,svz,memo_key,spawn}.rs`, `lightr-run/src/secrets.rs` | LRR1 AC/results, rootfs materialization, secret staging and index consumers. No redesign of runtime keys in this campaign; update affected helper lifetimes only. | SI-01/02 caller tests; SI-03 E14 |
| `lightr-run/src/network/{registry,fsutil}.rs` | Another atomic-write helper, with network-registry state. Do not implicitly enroll it in CAS transaction/GC. | Separate domain; regression only |
| `lightr-cri-backend/src/{container_setup,util}.rs` | CRI materialization; another locally defined atomic-write helper. Preserve engine/CRI contract while adapting actual Store calls. | SI-03 E14 |
| `lightr-views/src/views/backend.rs` | Materializes content through Store. Views must not retain mutable CAS ownership or lose reader lifetime. | SI-03 E13/E14 |
| `lightr-oci/src/oci/{import,pull,retain}.rs` | Import currently writes sidecars before snapshot; retained record references immutable descriptor blobs. Build intended tuple before any named write. | SI-03 E17 |
| `lightr-oci/src/oci/{save,images,history,rmi}.rs`, `lightr-oci/src/oci/push/**` and CLI `handlers/{tag,oci_imageops}.rs` | Resolve complete tuple once, tag source/destination under sorted locks, remove a full named lifetime, export original coherent metadata. | SI-03 E14/E17/E19 |
| `lightr-cli/src/handlers/{snapshot,hydrate,commit,gc,system,tag,rmi,oci_imageops,history,diff,images,info,plan}.rs` | Public errors/JSON, Store initialization and topology preflight; callers cannot silently bypass preservation contracts. | SI-03 E06/E14/E17/E21/E22 |
| CLI `handlers/run/**`, `mcp/**`, `bench/**`, `bench_compare/**` | Indirect entrypoints, measurements and tools must use same root resolution and failure semantics. Separate measured versus skipped/unsupported. | SI-03/05 E14/E15 |

## Root decoder census (do not silently remove a family)

1. Live `refs/<key>` -> current LMF1 tree, independent of log/name index.
2. Retained raw legacy log -> conservative tree roots while live; no fabricated
   provenance. New history envelopes -> tree plus both OCI pointers.
3. Action Cache **LRR1, exactly 72 bytes** -> stdout/stderr CAS digests.
4. Action Cache **BUILD-ROOT32, exactly 32 bytes** -> LMF1 root, then file entries.
   Actual producer: `lightr-build/src/build/exec.rs:338`,
   `store.ac_put(&key, &new_root.0)`. The current GC's LRR1-only parse misses it.
5. `imgmeta` -> config blob; `imgmanifest` -> ImageManifestRecord blob and all
   ordered retained descriptor blobs. Corrupt/inaccessible enumeration blocks
   sweep; no config-less fallback used as destructive authority.
6. New pending is independently enumerated *before* marking and blocks sweep
   even for absent current. Ready receipts/key locks are not roots or scratch.

LMF1 in-tree `.lightr-image.json` is part of tree content, not a newly invented
fourth name-keyed sidecar. A newly discovered version-owned metadata family
blocks the fixed-tuple caller/schema gate until deliberately incorporated.

## Resource-guard census

Store identity anchors CAS/ref/history/AC/image metadata and store-owned scratch.
Shared index root has its own EX leaf lock, including standalone status callers;
never acquire Store/key locks from inside that leaf. Each recognized reaper
checks the matching domain, not merely the Store that initiated the operation.
Run directories, service workspaces, VM-state directories and named volumes
retain their existing ownership rules and cannot be authorized for deletion by
foreign Store EX, timestamps, or a matching path prefix. Public alias/`..`
resolution must identify the actual object opened, not its lexical substitute.

## Explicit review limits

This inventory is reproducible source analysis and author review, not a compiler-
resolved indirect call graph or independent reviewer approval. Native baseline
execution demonstrates the selected entrypoints execute on each target; it does
not prove the future lease/GC/publication conversion is complete. G-ACTIVATION
requires revisiting this map against the actual converted code and all E14/E17
consumer tests; no dangling ownership item may be hidden by a green unit count.
