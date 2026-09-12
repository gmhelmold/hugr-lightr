# FROZEN PARITY CONTRACT — HuGR Lightr ↔ Docker 100% parity

Status: **FROZEN by the lead, owner-approved.** Source: 7-axis SOTA research wave +
lead ratification (3 high-risk roots verified line-by-line against the code).
This is the single canonical spec the 30+ agent wave transcribes. Agents do NOT
design — they transcribe a WP's row + the root contract it binds to.

Totals: **105 WPs — 34 P0 / 44 P1 / 27 P2** across 7 axes (A1 CLI-lifecycle,
A2 Dockerfile, A3 Compose, A4 Networking, A5 Volumes, A6 Images/registry,
A7 Runtime-axioms).

---

## 0. FREEZE-GATE — Level-0 root edits (LEAD authors, ONE atomic green commit, before ANY WP dispatches)

Each root is a single additive edit; every existing construction/call site is
compile-forced to a default (`None`/`&[]`/`Vec::new()`). The freeze-gate commit
must be `cargo build/clippy --all-targets -D + cargo test` green before Wave-A.

### R-SPECDISK — `crates/lightr-run/src/run/types.rs`
Add to `SpecOnDisk`, ALL `#[serde(default)]` (back-compat with existing spec.json):
`name: Option<String>`, `rm: bool`, `restart: Option<String>`,
`labels: Vec<(String,String)>`, `workdir: Option<String>`, `user: Option<String>`,
`entrypoint: Option<Vec<String>>`, **`env_explicit: Vec<(String,String)>`**,
`stop_signal: Option<String>`, `network: Option<String>`,
`network_alias: Vec<String>`, `hostname: Option<String>`,
`add_host: Vec<(String,String)>`, `dns: Vec<String>`,
`mounts2: Vec<MountOnDisk2>` (tagged enum, see R-MOUNT), `ports2: Vec<PortOnDisk>` (proto-tagged).
**LEAD ARBITRATION (env-split):** the EXISTING `env: Vec<(String,String)>`
(compose discovery, `<PEER>_HOST`) stays UNKEYED. The new `env_explicit` (user
`-e`/`--env-file`) is the ONLY env that enters the memo key. Two distinct
channels — never merge them. Legacy `mounts`/`ports` fields stay for read
back-compat; `mounts2`/`ports2` are the go-forward tagged shapes.

### R-EXECSPEC — `crates/lightr-engine/src/engine/spec.rs`
`ExecSpec<'a>` gains: `mounts: &'a [ResolvedMount]`, `env: &'a [(String,String)]`,
`workdir: Option<&'a str>`, `user: Option<&'a str>`, `hostname: Option<&'a str>`,
`add_host: &'a [(String,String)]`, `dns: &'a [String]`, `mesh_ip: Option<std::net::Ipv4Addr>`.
(`net_fd`/`net_mac` already present.) Every construction site (native/ns/vz/wsl
tests + run/build call sites) sets `&[]`/`None`. KEYING STAYS IN memo.rs — ExecSpec
only carries values to the engine (verified: the key is computed pre-ExecSpec).
**FREEZE-GATE LANDED (462c11a):** the 8 fields landed; 8 construction sites
defaulted (3 in lightr-engine tests, 2 in cli/run/paths.rs, 1 build/exec.rs,
1 run/svz.rs, 1 vz-switch example).

### R-KEY — `crates/lightr-run/src/run/memo.rs` + `crates/lightr-build/src/build/memo.rs`
Document + enforce the partition (verified against the existing fold-order:
domain → inputs → args → env → OS/arch → mounts → secrets):
- **IN key:** explicit env (`env_explicit`, folded `key=value\0`), image ENV,
  CAS-ref content, ro-bind fingerprint, and (BUILD only) workdir/user/entrypoint-when-set
  + post-interpolation instruction text.
- **OUT of key (runtime):** caps, restart, health, ports, labels, network, tty,
  workdir/user/hostname at RUN time, discovery `env`.
- **NON-memoizable (force-MISS, no AC write):** rw-bind, named, anon, tmpfs mounts.
- **LEAD ARBITRATION (v2 bump):** bump the domain tag PER-KEY-DOMAIN and ONLY when
  that key's input format changes. BUILD key → `lightr/build/v2` (interp text +
  workdir/user/entrypoint). RUN key STAYS `lightr/run/v1` (env format unchanged).
  Each bump is a documented one-time Action-Cache invalidation.

### R-MOUNT — `crates/lightr-run/src/run/mount.rs` (new) + `types.rs`
`enum MountKind { CasRef, HostBind, NamedVolume, AnonVolume, Tmpfs }` +
`struct MountSpec { kind, source: Option<String>, target: String, readonly: bool, opts }`.
Unified parser `parse_v` / `parse_mount_long` / `parse_tmpfs`. `ResolvedMount`
(post-resolution, what ExecSpec carries). Absolute-target rule: native keeps the
relative-CasRef law; bind variants accept absolute (ns/vz). `MountOnDisk2` tagged
enum mirrors it for SpecOnDisk.
**FREEZE-GATE RATIFIED DEVIATION (462c11a):** because `lightr-run` depends on
`lightr-engine` (not the reverse), `ExecSpec` cannot borrow a `lightr-run` type
without a dep cycle. So `MountKind` + `ResolvedMount` are DEFINED in
`lightr-engine/src/engine/spec.rs` and RE-EXPORTED from `lightr-run::run::mount`
(`pub use lightr_engine::{MountKind, ResolvedMount}`) — that file stays the
canonical surface and owns `MountSpec` + the parser stubs. WP-VOL-* bind to
`lightr_run::mount::{MountKind, ResolvedMount, MountSpec}` as named here.

### R-VARENG — `crates/lightr-build/src/build/vars.rs` (new)
`pub fn interpolate(s: &str, scope: &VarScope, escape: bool) -> Result<String>` +
`struct VarScope { args: Map, env: Map }`. Bash modifiers (`${A:-d}`, `${A:?msg}`,
`${A:+v}`), `\$` literal escape, ENV-over-ARG precedence. **LEAD DECISION:** compose
CONSUMES `lightr_build::vars::interpolate` directly (no fork). If the crate-dep
direction is wrong, lift `vars` to `lightr-core`; freeze the signature regardless.

### R-IMGREC — `crates/lightr-store/src/store/imgmeta.rs` + `gc.rs`
`ImageManifestRecord` (retained raw blobs + original manifest bytes + ordered
descriptors + platform), `Store::image_manifest_put/get`, length-prefixed binary
codec. **gc mark-walk extended in the SAME commit** to mark retained blobs
reachable (else gc reaps faithful-push blobs).

### R-IMGCFG — `crates/lightr-build/src/build/memo.rs` + `run/mod.rs`
`ImageConfig` sidecar (`.lightr-image.json`: entrypoint/cmd/env/workdir/user/expose/
volume/labels/healthcheck/stopsignal/shell/onbuild) + single shared
`effective_argv()` combination fn (run + compose + shim all call it).

---

## 1. WP LEDGER (the transcription list — see the research synthesis for full per-WP spec)

The complete 105-WP ledger (id · item · key files · spec one-liner · effort ·
priority · depends_on · disjoint?) is the research wave's §3 output, reproduced
into per-axis dispatch cards at fan-out time. Each agent receives: its WP row +
the frozen root(s) it binds to (verbatim from §0) + the invariants below.

**Universal invariants (every WP):** behavior-preserving where it touches shipped
paths; fail-closed on bad input (honest exit-2, never silent); tense-law (no
unmeasured number claimed measured); <400 LOC/file (split tests if needed);
EDIT-ONLY + push-to-gate (no local cargo beyond one `cargo check -p <crate>`);
commit to the worktree's CURRENT branch; touch ONLY the WP's named files.

---

## 2. DISPATCH DAG

### WAVE-A — massive fan-out (33 disjoint WPs, dispatch after freeze-gate seals)
Disjoint or depend only on a frozen Level-0 root. Taproot self-contained roots
(8): WP-LIFE-01, WP-DF-01, CMP-P0-ENVFILE-SVC, CMP-P0-PORTS-FULL, WP-VOL-1,
WP-IMG-01, F4-9, WP-VOL-4. Disjoint feature WPs (25): WP-LIFE-10; CMP-P1-{ENTRYPOINT,
PROFILES, RESTART, DEPLOY-RES, HEALTH-FULL, PROJECT, CONFIG-CMD}, CMP-P2-{SECRETS-CONFIGS,
BUILD, EXTRA-FIELDS, MERGE-EXTRAS}; F4-8, F4-10; WP-IMG-{04,06,08,11,12}; RC-4, RC-7.
Compose P1/P2 WPs dispatch against the frozen `ServiceDef` model stub (a Level-1
contract anchor) and integration-merge after CMP-P0-PARSER seals.

### WAVE-B — gated on a single taproot (dispatch the instant it SEALs green)
- WP-LIFE-01 → LIFE-02,03,04,05,06,08,09,12,13,14,19
- WP-DF-01 → DF-02,04,05,09 ; WP-DF-02 → DF-03,05,08
- CMP-P0-PARSER → CMP-P0-INTERP, CMP-P0-MERGE
- F4-1+F4-2 → F4-3,4,5,6,7,8(wire),11
- WP-VOL-1 → VOL-2,3,6,7,10,11 ; WP-IMG-01 → IMG-02,03,05,07,09
- R-EXECSPEC → RC-1,2,3, WP-VOL-8, F4-6

### WAVE-C+ — multi-dependency convergent (later rings; see synthesis §4)
LIFE-07/11/15/16/17/18/20; DF-06/07/10/11/12/13/14/15; CMP-P0-VOLUMES,
CMP-P0-DEPENDS, CMP-P1-NETWORKS, CMP-P2-LIFECYCLE-VERBS; VOL-5/9/12; IMG-10/13;
F4-12; RC-5/6/8/9/10/11/12.

### OWNER-GATED (split: fleet builds the code half behind a gated test; owner runs the real-iron acceptance — NEVER on the critical path)
F4-1/2/5/6/7/8/11/12 (real vz boot), WP-VOL-8 (Linux mount-ns), WP-VOL-9/12 (vz),
WP-IMG-02/05/10/11/13 (registry round-trip), WP-IMG-12 (cred helper), RC-3/7/8
(Linux cgroup/uid), RC-10 (tty), CMP-P1-NETWORKS (real name-DNS). Plus the pure
owner items: Apple notarization, beta users, AS-hardware absolute benchmarks.

---

## 2b. DAG SEQUENCING NOTES (lead, discovered during execution)

- **IMG-09 gates IMG-02 (HARD).** WP-IMG-01 (landed) retains original layer blobs
  into the CAS, referenced only by the `imgmanifest` sidecar — which the current
  gc reachability walk (lightr-index/gc + cli/gc) does NOT traverse, so gc would
  reap them. No live regression today (nothing consumes retained blobs until
  faithful push). But **WP-IMG-09 (gc marks retained blobs reachable) MUST land
  before WP-IMG-02 (faithful push) is relied upon** — else a pull→gc→push loses
  layers. Freeze-gate's R-IMGREC landed the record/codec but NOT the gc walk
  (gc lives outside lightr-store); IMG-09 closes it.

- **Run-flag WPs CONVERGE on `cli/dispatch.rs` (serialize them).** The CLI-surface
  freeze put ALL new `run` flags behind one `new_flag_set` guard in dispatch.rs
  that returns the `WP-RUNFLAGS` stub. Each run-flag WP (LIFE-15 workdir/user,
  LIFE-16 env/label, RC-1/2/3, VOL-10 `-v`/`--tmpfs`, F4-3/7 network/hostname/
  add-host/dns) must REMOVE its flags from that guard and wire behavior — so they
  all touch dispatch.rs's guard and CANNOT run in parallel. Either serialize them
  or have ONE WP own a guard refactor first. NOT disjoint despite different axes.
- **`--mount` / `--env` collisions.** Docker `--mount type=…` and `-e/--env KEY=VAL`
  collide with the EXISTING lightr `run` `--mount REF:TARGET` and `--env` memo-KEYS.
  Freeze added only docker short `-e` (`env_set`); the long `--env` and docker
  `--mount` grammar are deferred to the env/volume run-flag WPs (reconcile there).

- **Compose interp is built but NOT CLI-wired.** CMP-P0-INTERP added
  `parse_compose_with_scope(yaml, &VarScope)` + `scope_from_project_dir`, but the
  compose CLI handler (`lightr-cli/handlers/compose.rs`) still calls plain
  `parse_compose` (no interpolation). A follow-up WP must wire the handler to
  `scope_from_project_dir` + `parse_compose_with_scope` and add the
  `--env-file`/`--project-directory` flags (CLI-surface change). Until then
  `lightr compose up` does not interpolate `${VAR}`. Tracked.

- **Compose ports: grammar accepted, proto/host_ip not yet honored.**
  CMP-P0-PORTS-FULL parses the full ports grammar (udp/ranges/host_ip/long-form)
  into a rich `ParsedPort`, but the runtime model `Service.ports` is
  `Vec<(u16,u16)>` (TCP, loopback) so proto/host_ip/auto-assign are DROPPED at
  lowering (compatibility: compose files load without error; behavior = TCP
  host:container as before). A downstream WP must widen the runtime port model
  (+ UDP publish) to honor proto/host_ip; `ParsedPort` is ready. Until then a
  `/udp` mapping silently runs as TCP — acceptable for file-compat, tracked here.

- **oci save→load preserves content, not the ref name.** IMG-04 `save` emits no
  RepoTags, so IMG-05 `load` of a save output lands under a content-fallback name
  (`@loaded/img-<digest12>`) — root+blob digests are byte-for-byte lossless
  (re-pushable), only the human ref label differs. Follow-up: have `save` emit
  RepoTags (manifest.json) so load recovers the original name. Tracked.
- **DF interpolation now memo-correct (WP-DF-BUILDKEY).** Build key = v2, hashes
  POST-interpolation text; ENV updates the VarScope; proven no-false-hit. The DF
  consumer ring (DF-03 multi-stage, DF-05 ENV multi-pair, DF-08 ARG/--build-arg,
  DF-09 SHELL) now extends the VarScope/key SAFELY — they converge on
  build/exec.rs + build/memo.rs, so SERIALIZE them (one DF WP per batch).

## 3. SCHEDULING LAW (techlead-loop)
Rolling, not barriered: the DAG is the MERGE order, not a dispatch barrier.
Dispatch-on-ready (Wave-A all at once = 33 in flight, ≤16 concurrent by the cap),
merge-on-green under change-scoped CI, SEAL + recovery per branch, post-flight
off-baseline/leak sweep. Each taproot SEAL unblocks its Wave-B fan-out. Combined
gate per batch (per-crate or per-axis) to keep each build tractable on the box.
Owner-gated halves merge on their unit tests; integration acceptance is a separate
off-critical-path checklist.
