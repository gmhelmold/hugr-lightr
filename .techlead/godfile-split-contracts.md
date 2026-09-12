# Godfile-split — FROZEN CONTRACTS (lead-owned design)

These are the lead's decisions. Agents TRANSCRIBE; they do not design. Each
submodule file lists EXACTLY the items (by name) it must contain — verification
is "does the file contain exactly these items?", deterministic.

INVARIANTS (all WPs): pure move (no logic change) · crate public API IDENTICAL
(re-export every former-`pub`/`pub(crate)` item at the crate root) · move each
`use` + `#[cfg(test)]` with its code · every RESULTING file < 400 LOC INCLUDING
tests (split test mods to mirror their source modules) · keep doc-comments with
items · move `#[cfg(...)]` blocks intact with their attributes.

Re-export rule: the crate's `lib.rs` becomes a thin facade — `mod <sub>;` decls +
`pub use <sub>::{…}` for every former-public name, so external crates compile
unchanged. Pre-existing sibling modules (e.g. lightr-run's healthcheck/limits/
portforward/restart/secrets/network/vswitch; lightr-engine's limits/pack) STAY
declared in lib.rs — only the code currently INSIDE lib.rs moves.

═══════════════════════════════════════════════════════════════════════════════
## G1 — crates/lightr-run/src/lib.rs (3567) → run/
run/types.rs    ← PortMap, RunSpec, Mount, StoreFile, RunOutcome, RunHandle,
                  RunInfo, LogStream, MountOnDisk, SpecOnDisk, VzMemoKey,
                  DeepMemoConfig, default_engine
run/ac.rs       ← AC_MAGIC, AC_RECORD_LEN, encode_ac_record, decode_ac_record
run/paths.rs    ← lightr_home, run_dir_for_id, new_run_id, read_spec_on_disk,
                  write_spec_json, read_pid_file, read_status_file,
                  parse_exit_code_from_status, pid_alive (both cfg), win_terminate
run/ctl.rs      ← ctl_sock_path (both cfg), ctl_pipe_name, send_ctl_op (both cfg)
run/memo.rs     ← validate_mount_target, assemble_key, build_key, run_memoized,
                  run_memoized_with, predict
run/vzmemo.rs   ← vz_memo_key, run_vz_memoized
run/deepmemo.rs ← deep_memo_available, run_memoized_deep
run/spawn.rs    ← spawn_detached, spawn_detached_with_health, spawn_detached_engine
run/supervise.rs← supervise, win_pipe_server_loop, win_pipe_nudge
run/svz.rs      ← supervise_vz (both cfg: unix impl + non-unix stub)
run/ps.rs       ← ps
run/logs.rs     ← logs
run/stop.rs     ← stop
run/exec.rs     ← exec_in
run/tests/*.rs  ← the `mod tests` block, SPLIT by area (memo/spawn/ps/etc.) so
                  no test file > 400 LOC; gate behind #[cfg(test)]
lib.rs (thin)   ← keep `pub mod {healthcheck,limits,portforward,restart,secrets,
                  network(cfg unix),vswitch(cfg unix)}` + `mod run;` + re-exports

## G2 — crates/lightr-oci/src/lib.rs (3384) → oci/
oci/model.rs    ← ImportReport, PushReport, OciDescriptor, OciPlatform, OciIndex,
                  OciManifest, DockerSaveItem, TokenResponse, ManifestList
oci/util.rs     ← path_is_safe, sha256_hex, sha256_hex_of, verify_sha256,
                  hex_to_digest, hex_nibble, hasher_to_hex, host_arch, TempDirGuard
oci/layer.rs    ← LayerBlob(+impl), PendingEntry, layer_timeout_secs, apply_layers
oci/import.rs   ← import_layout, import_oci_layout_dir, import_docker_save_tar,
                  apply_and_snapshot
oci/http.rs     ← net_agent, RegistryCreds, read_creds_for_registry,
                  parse_docker_config_for_registry, map_ureq_error, ureq_status,
                  retry_request, stream_blob_to_file, read_response_bytes,
                  registry_scheme
oci/reference.rs← parse_image_ref, pick_from_manifest_list, fetch_docker_token
oci/pull.rs     ← pull
oci/push.rs     ← push, build_layer_tar_gz, blob_exists, begin_blob_upload,
                  upload_put_url, upload_blob_from_bytes, upload_blob_from_file
oci/tests/*.rs  ← `mod tests` split < 400 each
lib.rs (thin)   ← mod + re-exports

## G3 — crates/lightr-build/src/lib.rs (2267) → build/   [REDO compose]
build/parse.rs  ← Instr, BuildStep, parse_dockerfile, parse_argv_or_shell
build/memo.rs   ← ImageMeta, IMAGE_META_FILE, load_meta, save_meta, step_key,
                  hash_copy_source, collect_dir_paths, TempDirGuard
build/exec.rs   ← materialize_from_digest, BuildReport, build, copy_dir_recursive,
                  step_reads_clock_or_net
build/compose/model.rs    ← Service, Compose, ComposeHandle, StackSpec,
                            ServiceSpec, empty_service, parse_duration_secs
build/compose/parse.rs    ← parse_compose
build/compose/up.rs       ← compose_up, prepare_service_cwd
build/compose/supervise.rs← compose_supervise, start_service_detached,
                            proxy_bidirectional, discovery_key
build/compose/down.rs     ← compose_down
build/compose/mod.rs      ← `pub mod` + re-exports of the compose public API
build/*/tests             ← split < 400 each
NOTE: G3's finished output kept compose.rs at 1296 — REJECTED; compose MUST split
per the above. parse/memo/exec from G3 may be kept if they match.
lib.rs (thin)   ← mod + re-exports

## G4 — crates/lightr-index/src/lib.rs (2219) → index/
index/codec.rs  ← INDEX_MAGIC, INDEX_VERSION, IndexEntry, Index(+impl), fsync_dir,
                  index_dir, index_path_for
index/scan.rs   ← WalkReport, WalkCandidate, stat_fields, scan
index/snapshot.rs ← SnapshotReport, snapshot
index/hydrate.rs← HydrateReport, hydrate_verified, hydrate, hydrate_impl
index/status.rs ← StatusReport, status, entries_differ
index/gc.rs     ← GcReport, gc, TempDirGuard
index/timeaxis.rs← DiffReport, diff_manifests, parse_lrr1, undo, bisect
index/tests/*.rs← `mod tests` + `mod r1_tests` split < 400 each (TEST_ENV_LOCK here)
lib.rs (thin)   ← mod + re-exports

## G5 — crates/lightr-store/src/lib.rs (1653) → store/   [VERIFY: split cas tests]
store/lock.rs   ← WriteGuard, GcGuard
store/cow.rs    ← CowRung, probe_rung, try_ladder_probe, cow_clone, cow_reflink,
                  cow_copy_range, cow_refs_block_clone, cow_copy_file, try_cow_at_rung
store/paths.rs  ← shard_parts, object_path, ref_path, refs_names_path,
                  refs_log_dir, imgmeta_path, ac_path, temp_suffix, fsync_dir,
                  atomic_write, set_mode
store/cas.rs    ← Store struct + the put/get/object methods (impl Store block 1)
                  — if > 400 incl tests, move tests to store/tests/
store/refs.rs   ← the refs/lineage/imgmeta/ac methods (impl Store block 2)
store/tests/*.rs← split < 400 each
lib.rs (thin)   ← mod + re-exports (Store, CowRung, WriteGuard, GcGuard)
NOTE: G5 finished good (cas 479 incl tests / 260 prod); ACCEPT if cas tests are
moved so no file > 400 incl tests — else minor test-split fix.

## G6 — crates/lightr-engine/src/lib.rs (1353) → engine/   [LIKELY ACCEPT]
engine/kind.rs  ← EngineKind(+FromStr+impl)
engine/spec.rs  ← ExecSpec (incl. net_fd/net_mac)
engine/probe.rs ← EngineCaps, pack_dir, dirs_home, probe, pack_status,
                  probe_ns/vz/wsl (all cfg), wsl_list_distros
engine/native.rs← NativeEngine, exit_code (both cfg)
engine/ns.rs    ← ns_impl, ns_engine_box, NsEngineStub
engine/vz.rs    ← vz_impl (incl. the `extern "C" lightr_vz_run` + net_mac),
                  vz_engine_box, VzEngineStub
engine/wsl.rs   ← wsl_impl, wsl_engine_box, WslEngineStub
engine/dispatch.rs ← Engine trait, engine_for
engine/tests.rs ← `mod tests` (< 400; else split)
lib.rs (thin)   ← keep `pub mod {limits,pack}` + `mod engine;` + re-exports
NOTE: G6 finished with kind/spec/probe/native/ns/vz/wsl/mod all < 400, green —
ACCEPT pending diff review; ensure tests file < 400.

## G7 — crates/lightr-cli/src/main.rs (1813) → cli/   [RATIFIED]
cli/version.rs  ← LIGHTR_VERSION, AFTER_HELP, now_ms, emit_event helpers
cli/cmd.rs      ← Cli, ComposeCmd, PlanCmd, EngineCmd, OciCmd, SuperviseCmd,
                  Shell, Cmd (all clap derives verbatim)
cli/dispatch.rs ← dispatch, generate_completions, generate_man
cli/tests.rs    ← `mod tests` (split if > 400)
main.rs (thin)  ← fn main + mod decls + the PlanCmd root re-export shim
NOTE: G7 finished exactly this (cmd 419/dispatch 192/version 27), green, handlers
untouched — ACCEPTED; only ensure the test mod is < 400 (split if needed).

## G8 — handlers/bench_compare.rs (1621) → handlers/bench_compare/
bench_compare/model.rs   ← MaterializeSize, Workload, Runtime, Unit, Cell, CmpRow,
                           Detected, ProbePolicy (+impls), parse_runtimes,
                           which_on_path, which_in, detect_all
bench_compare/measure.rs ← median_of, dur_ms, time_lightr, SAMPLES,
                           build_materialize_fixture, make_bench_dockerfile,
                           lightr_* measurement fns, comm_is_lightr_binary,
                           build_layer_buf, make_tiny_oci_tar
bench_compare/competitor.rs← measure_competitor, competitor_idle_processes
bench_compare/report.rs  ← render_cell, render_factor, header_line, render_table,
                           *Json structs, build_report_json
bench_compare/mod.rs     ← run_workload, run (entry) + re-exports;
                           handlers/mod.rs `pub mod bench_compare;` UNCHANGED
bench_compare/tests.rs   ← `mod tests` (< 400; split if needed)
NOTE: G8 was killed but left mod.rs 612 (tests 486) — REDO per the above so the
test mod splits and run_workload/run live in mod.rs.

═══════════════════════════════════════════════════════════════════════════════
## Verify/redo disposition
- RATIFY (review diff, keep): G7. Likely-accept: G6, G5 (test-split nit).
- REDO per contract: G3 (compose), G8 (tests), G1, G2, G4 (killed).
- All redos: edit-only, worktree, transcribe THIS contract, `cargo check -p <crate>`.
- Lead combines into one integration branch → one full gate → merge.

═══════════════════════════════════════════════════════════════════════════════
# WAVE 2 — middle tier (≤1000 LOC) FROZEN CONTRACTS

## Tier 2a — TEST-EXTRACTION ONLY (prod already <400; move the `#[cfg(test)] mod`
## to a sibling tests file/dir, #[cfg(test)]; split the test mod if it alone >400;
## NOTHING else moves). Pure-move, lowest risk.
M-schema  handlers/schema.rs (280p)   → schema/{mod.rs(all schema_*+KNOWN_VERBS+schema_for+run), tests.rs}
M-docker  handlers/docker.rs (370p)   → docker/{mod.rs(all fns), tests.rs}
M-restart lightr-run/src/restart.rs(250p)→ restart/{mod.rs(RestartPolicy+launchd_plist+systemd_unit+helpers), tests.rs}
M-init    lightr-init/src/lib.rs(172p) → keep lib.rs prod; `#[cfg(test)] mod tests;`→ tests.rs
M-vswmod  lightr-run/src/vswitch/mod.rs(374p)→ mod.rs keeps prod; move `mod tests`→ vswitch/mod_tests.rs (path attr)
M-dns     lightr-run/src/vswitch/dns.rs(391p)→ dns/{mod.rs(prod), tests.rs}  (dir module; `pub mod dns;` unchanged)

## Tier 2b — REAL PROD SPLIT (item→file; lib.rs/mod.rs thin re-export of ALL former-pub)
M-pack  lightr-engine/src/pack.rs → pack/{
   mod.rs ← PackManifest,PackInfo,assemble_pack,verify_pack,sha256_hex,FirstEntry,parse_first_newc_entry,consts
   cpio.rs ← build_initrd_cpio,write_newc_entry,write_newc_trailer,write_newc_header,write_hex8,pad4,pad4_from
   tests.rs }   (lib.rs keeps `pub mod pack;`)
M-mcp   handlers/mcp.rs → mcp/{
   mod.rs ← run,run_mcp_loop,handle_method,handle_initialize,tool_result,handle_tools_list
   tools.rs ← handle_tools_call
   tests.rs }
M-views lightr-views/src/lib.rs → views/{
   plan.rs ← EntryKind,PlanEntry,ViewPlan(+impl),plan_view
   backend.rs ← ViewBackend,FakeBackend(+impl),FileState,Solidifier(+impl),solidify_step
   composefs.rs ← (extract inline `pub mod composefs`)
   nfsloopback.rs ← (extract inline `pub mod nfsloopback`)
   projfs.rs ← (extract inline `pub mod projfs`)
   tests.rs }   lib.rs thin + re-export plan/backend public items + `pub mod {composefs,nfsloopback,projfs}`
M-core  lightr-core/src/lib.rs → core/{   ⚠ FOUNDATIONAL — re-export EVERY pub item identically
   consts.rs ← OUTPUT_CAP_BYTES,MANIFEST_MAGIC,REF_KEY_DOMAIN
   limits.rs ← ResourceLimits(+impl),parse_memory,parse_cpus (+resource_limits_tests)
   digest.rs ← Digest(+impl,+Debug),hex_nibble
   manifest.rs ← Entry(+impl),Manifest(+impl),KIND_FILE/SYMLINK/DIR
   refrecord.rs ← RefRecord(+impl),validate_ref_name,ref_key
   error.rs ← LightrError(+Display+Error+From<io::Error>),Result
   tests.rs }   lib.rs thin: `mod core_impl;`(or modules) + `pub use` ALL
M-bcd   handlers/bench_compete_docker.rs → bench_compete_docker/{
   mod.rs ← Outcome,consts(OP_TIMEOUT,SETUP_TIMEOUT,NAME_COUNTER,TINY_IMAGE,COLD_IMAGE_REF),unique_name,run_op,docker,setup_ok,sample_median,median_outcome,ensure_tiny_image
   probes.rs ← cold_run_ms,re_run_ms,build_ms,cold_image_ms,materialize_ms,install_footprint_mb,create_container,dir_size_bytes,docker_app_candidates
   tests.rs }   (handlers/mod.rs `pub mod bench_compete_docker;` unchanged; keep pub(crate) entry paths)
M-network lightr-run/src/network.rs → network/{
   types.rs ← NetworkId,MacAddr,Member,Subnet,SubnetOnDisk,MemberOnDisk,From impls
   fsutil.rs ← FlockGuard(+impl,+Drop),fsync_dir,atomic_write
   alloc.rs ← subnet_for,mac_for
   registry.rs ← NetworkRegistry(+impl)
   tests.rs }   (`#[cfg(unix)] pub mod network;` unchanged; dir module)
M-dhcp  lightr-run/src/vswitch/dhcp.rs → dhcp/{
   mod.rs ← handle,LeaseStore(+impl),DhcpRequest,all consts
   parse.rs ← parse_dhcp,parse_bootp,parse_option_53 (+be16/be32 helpers if local)
   build.rs ← build_reply,build_bootp_reply (+checksum/u16 helpers)
   tests.rs }

## Tier 2c — FUNCTION DECOMPOSITION (NOT pure-move; LEAD designs the helper
## boundaries — a single ~440-LOC fn each). Held for careful lead-authored split.
D-bench handlers/bench.rs — run() (lines 181-622, 441 LOC) → extract per-row helpers
D-runh  handlers/run.rs — run() (lines 145-573, 429 LOC) → extract engine/path helpers

# Tests tier (separate, lower priority, ask owner): acceptance_r1/r2/r3/r4 + acceptance.rs

