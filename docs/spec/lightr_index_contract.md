# Contract `C-SELF-01` — clw seam frozen (`ARCH-02` approved)

Status: frozen. Direct dependency (`clw` crate path-deps) supersedes wire contract for `v0.1` (`build-spec-v2.md` §2: zero `clw` in `R0` core). `WP-SELF-10` prepares `clw` absorption (`lightr-views`, `F-103` `CoW` `hydrate` → `O(1)` view, `ADR-0013` spike, not wired to run path).

## Interface `lightr_index`

`lightr-index` crate (WP-3) depends on `clw` crates directly (`clw-types`, `clw-cache`, `clw-snapshot`, `clw-hydrate`, `clw-run`, `clw-manifest`). Frozen surface:

- `Store` (`clw-store` / `lightr-store`): `put_bytes`, `get_bytes`, `materialize_file`, `ref_put`/`ref_get`.
- `Manifest` (`clw-manifest`): binary `LMF1` codec (`Digest` `BLAKE3`), path-sorted `Vec<Entry>` (File / Symlink / Dir).
- `RefRecord`: `name` → `root: Digest` + `parent: Option<Digest>` + `created_at_unix` + `tool_version`.
- `snapshot` (`F-102`): `scan` → `ingest` missing objects → `manifest` → `ref_put` (parent = previous ref root). Report: `root`, `files`, `bytes_total`, `objects_new`.
- `hydrate` (`F-103`, `R0` `CoW` clone / `R2` `O(1)` view): `ref_get` → `manifest` decode → `mkdirs` + parallel `materialize_file` (`Clone` → `Reflink` → `CopyRange` → `Copy`) + symlink `target` materialization. `dest` must be empty/absent. Report: `root`, `files`, `bytes_total`, `rung: CowRung`.
- `status` (`F-104`): `scan` vs `ref` manifest diff (path-sorted merge). Report: `clean`, `added`, `removed`, `changed`.

## Dependency rules (`v0.1` core)

One-way: `core` ← `store` ← `index` ← `run` ← `cli`. `clw` crates are direct `path` deps (`../corelink-workspaces/crates/<name>`), `publish = false`, never mutated from this repo (`CLAUDE.md` §9, `ADR-0002` / `ADR-0011` reconciliation). Baseline `corelink-workspaces @ f8f5edf` (`build-spec-v0.1.md`).

## WP-SELF-10 (`clw` absorption)

Prepare `lightr-views` (`ADR-0013`): `ViewPlan` + `Solidifier` pure logic (host-tested); `O(1)` backends (`composefs`/`NFS-loopback`/`projfs`) return `ErrorKind::Unsupported` ("planned spike"); NOT wired to run path. `clw-snapshot`/`clw-hydrate` functions either transcribed into `lightr_index::snapshot`/`hydrate` or kept as crate dependency (frozen by this contract).
