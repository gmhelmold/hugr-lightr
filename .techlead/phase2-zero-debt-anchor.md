# Phase-2 anchor — ZERO DEBT → THE HUMILIATION BENCHMARK
> gitignored session state; the lead's external memory for phase 2.

## GOAL (owner-set, 2026-06-18)
Owner reframed: "Expansão = dívida; deveria estar entregue." Close EVERY buildable
gap to the canonical bar (whitepaper v2 + perf-bar.md + feature-parity.md) to ZERO
debt — no mock/stub/bypass/gambiarra — then THE FINAL STEP: assemble + run the
"humilhação total" benchmark vs Docker/OrbStack/Apple-container. Autonomous, techlead
fleet, warp speed, SOTA. Lead owns all decisions/commits/merges.

## CONTEXT ANCHOR (canon — don't drift)
- Vision: docs/whitepaper/hugr-lightr-v2.md (canonical)
- Bar: docs/spec/performance-bar.md (8 indicators) + feature-parity.md (parity × 0-weight)
- Bench doctrine: ADR-0012 (bench = CI gate; tense law: only measured numbers)
- Views: ADR-0013 (O(1) materialization; CoW is R0 fallback forever)
- Truth ledger: docs/spec/parity-audit.md (every F-id)
- Principles: CLAUDE.md (no daemon ever; no images; fail closed; tense law)

## THE DEBT LEDGER (honest, 3 buckets)
### Bucket A — buildable parity debt (CLOSE; production, no stub)
- F-308 restart-via-OS-supervisor (launchd/systemd unit-gen) — buildable on mac NOW
- F-309 healthcheck/secrets/configs (run-spec features) — buildable NOW
- F-203 resource limits — native rung (rlimits/taskpolicy) NOW; vz VM config; ns cgroups (validate HW-gated)
### Bucket B — the views-O(1) frontier (indicator #4 software half)
- perf-bar #4 (re-run ≤10ms) NOT met on shipped path: bench measures 35–50ms;
  bench.rs admits "≤10ms binds to views + Apple Silicon". This is a HEADLINE I wrote.
- Build: macOS NFS-loopback view (ADR-0013) + composefs in vz Linux guest.
  Honest: EdenFS-class; build real+tested; AS perf number stays HW-gated.
### Bucket C — physically gated (NOT debt; honest-mark, don't sweep)
- fc/cloud, Stage-2 CoreLink sync, cross-tenant dedup, LAN mesh (need fabric/server)
- Rosetta + Apple-Silicon perf numbers (need Silicon)
- arm64 vz / Windows wsl / Linux ns runtime VALIDATION (need HW/CI)
- registry push (Stage-2), fs-verity (Linux)

## THE BENCHMARK (the final step)
Indicators = perf-bar.md 8, collapsed to humiliation axes:
  pull/materialize 1GB (≤100ms vs 30-60s) · idle RAM (0 vs 2-4GB) · install (1.9MB vs 1.5GB)
  · re-run (≤10ms vs full) ← the views+AS gap · disk 10ws (~1× vs 10×) · snapshot 10k (≤100ms)
  · cold-start (ms vs s) · exec overhead (~0 vs dockerd+runc)
Harness EXISTS (bench.rs B1-B11, --check, budgets) BUT --vs-docker is shallow
(only times `docker --version`). MISSING: real head-to-head running the SAME workload
through Lightr vs Docker vs OrbStack vs Apple container → side-by-side table.
Tense law: only measured-on-this-Intel numbers published; AS headline marked "binds-on-AS".

## WAVE PLAN (sequence: debt → bench last; A/B/C build in parallel, final RUN waits)
- Wave A: parity debt (Bucket A) — disjoint by feature, run-spec contract frozen FIRST
- Wave B: views-O(1) (Bucket B) — the frontier spike
- Wave C: benchmark head-to-head harness — parallel with A/B
- FINAL: run the full measured benchmark → humiliation table → truth-up docs

## SOTA CHECKLIST (per WP, inviolable)
[ ] no mock/stub/bypass/gambiarra; root-cause fixes only
[ ] tests real (not vacuous); gate green (fmt + clippy -D + test) at merge
[ ] tense law: no unmeasured number claimed; HW-gated marked honest
[ ] disjoint slices OR frozen contract; lead cold-reviews every SEAL; linear merge
[ ] no debt/deferral without explicit owner waiver (logged verbatim)
[ ] repo impeccable: worktrees pruned, anchor + parity-audit truth-synced

## WAVE LOG
- 2026-06-18: phase-2 opened. main @ cdad144 (go-live wave FF'd, single-branch, clean).
  Recon dispatched (read-only): parity-state / bench-depth / views-seam.
- 2026-06-18: build-spec-parity.md frozen + committed (67e1f47). branch wave/zero-debt.
- 2026-06-18: WP-A0.1 (ResourceLimits in core, +parser tests) by lead (76e5f62).
- 2026-06-18: WP-A0 (contract freeze) SEALED by agent + VERIFIED by lead. Cold-read
  diff: spec-conformant, no hidden decisions (off-spec docker.rs/main.rs = forced sig
  propagation; index.rs = pinned-rustfmt rewrap; Supervise→SuperviseDetached rename
  forced by clap name-collision, flagged). Memo-key no-op CONFIRMED (contribute_to_key
  doesn't touch hasher). Honest stubs (limits/secrets inert Ok; supervise/bench_compare
  honest Unsupported err). FF-merged @ 1dbf9d8. Full gate in warm tree: fmt✓
  clippy-default✓ clippy-vz✓ test 415/415✓ (incl. 43 acceptance). Agent's "pre-existing
  acceptance fail" = fresh-worktree assert_cmd bin-resolution artifact, NOT a regression.
  CARRY: (1) A0 makes --memory/--cpus parse-but-not-enforce → A0+A1 land together before
  main (no silent-no-op to users); (2) add lightr-cli dev-dep to lightr-acceptance so
  CARGO_BIN_EXE_lightr is always set (kills worktree bin-resolution fragility) — fold in.
- 2026-06-18: fanned impl fleet (background, 2-at-a-time for machine health): WP-A1
  (resource limits) + WP-C (bench-compare) off 1dbf9d8. WP-A2 (restart) + WP-A3
  (health/secrets/configs) queued next.
- 2026-06-18: WP-A1 came back BLOCKED — agent correctly refused to invent (AP-3):
  frozen §2.2 premise "macOS enforces memory via RLIMIT_AS/DATA" was FALSE (Darwin
  EINVAL, verified). LEAD DECISION: macOS/Windows native memory → honest Err →
  --engine vz (VM RAM = Docker's Mac mechanism); Linux native → rlimit; cpu-share
  native → honest Err → ns/vz; validate EARLY (pre-AC, no cache-hit bypass). Lead
  finished A1 in the worktree (SendMessage tool unavailable) + amended build-spec
  §2.2 (own the error). WP-C SEALED clean (22 tests, real Docker sanity-run).
- 2026-06-18: INTEGRATED A0+A1+C → wave/zero-debt (FF A1=5234e15, cherry-pick
  C=348043d, truth-up=0e38e3e). Gate in warm main tree: fmt✓ clippy-default✓
  clippy-vz✓ test 445/445✓ (all 43 acceptance incl a24_compose_lazy passing
  UNLOADED → confirmed C's flag was host-load, not regression). A0+A1 both landed →
  parse-but-not-enforce carry RESOLVED (limits now enforce / honest-err).
- 2026-06-18: fanned WP-A2 (restart) + WP-A3 (health/secrets/configs) off 348043d
  (background). PENDING after they land: integrate + gate; prune 3 merged worktrees
  (a6c0e76/a3b9b446/a61c9acc); acceptance dev-dep hardening (lightr-cli dev-dep →
  CARGO_BIN_EXE robust in fresh worktrees); then FINAL benchmark run + merge wave→main.
- 2026-06-18: A2 + A3 SEALED green; integrated (cherry-pick A2=557c999, A3=e9a564c;
  resolved parity-audit F-308/F-309 adjacency). GATE CAUGHT A REAL REGRESSION: a24_
  compose_lazy failed once the bin was built fresh — `image: scratch` was hydrated as
  a store ref (RefNotFound) not treated as the empty base, so the lazy service never
  started (supervisor stderr=/dev/null swallowed it). ROOT: latent since the go-live
  compose-hydrate change, MASKED by the stale-bin trap (`cargo test --workspace` builds
  test-bins, not the runnable target/debug/lightr that assert_cmd execs). Fixed
  prepare_service_cwd to mirror the Dockerfile FROM-scratch path + fixed CI to build
  the bin before tests (b742b9c). dev-dep hardening idea retired: a plain dev-dep builds
  the lib not the bin and doesn't set CARGO_BIN_EXE on stable → build-first is the right
  fix. FULL GATE GREEN: 482 tests, fmt, clippy -D (default+vz), build-first. Worktrees
  pruned → main + wave/zero-debt only.
- 2026-06-18: Bucket-A wave FF-merged to main @ b742b9c (confirmed single-branch, clean).
  Phase-2 Bucket-A = DONE. Remaining buildable pendência = the FULL Docker head-to-head
  (§5 bench-compare only raced `idle`; the 4 timed axes SKIP'd Docker, no spawn).

=== WAVE-D — THE FULL DOCKER HEAD-TO-HEAD (obliterate docker, every adversarial axis) ===
- 2026-06-18: owner: "deploya seu time pra matar todas as pendencias ate dar um benchmark e
  obliterar o docker em todos os indicadores adversariais." Recon (AP-5, not trusting own
  summary): main @ b742b9c clean; DOCKER PRESENT + daemon live (28.3.2, linux) → real
  head-to-head physically possible NOW. OrbStack/Apple-container ABSENT → honest SKIP cols.
  LEAD SCOPE DECISION: Wave-D = make the head-to-head real on the 6 axes with an honest
  docker mirror (install#1 NEW, materialize#3, cold-run#8, re-run#4, idle#2, build#4/8).
  Snapshot#5/disk#6 = no fair cheap mirror → Lightr-side only, honest. ≤10ms#4 binds to
  views+AS (Bucket-B, separate; we already obliterate docker re-run on the shipped path).
- 2026-06-18: branch wave/humiliation-bench off main. WP-D0 (lead scaffold) committed dc8140e:
  install-footprint axis measured now (du Docker.app=1.9GB vs lightr bin); ProbePolicy
  spawn-guard (real CLI=Spawn, tests/CI=NeverSpawn → present docker still SKIPs, locked by
  test); bench_compete_docker module (install impl+4 portable tests; cold/re/build/materialize
  honest-SKIP stubs); shared fixtures+timing exposed pub(crate). Gate green: clippy-D clean,
  28 bench tests. Contract recorded build-spec-parity §7 + benchmark-results.md (methodology
  frozen, numbers ⏳ pending authoritative run — tense law).
- 2026-06-18: WP-D1 dispatched (background, worktree, opus) off dc8140e — fills the 4 spawn
  probes per the frozen fairness doctrine. WP-D2 (docs/methodology) done by LEAD in parallel.
- 2026-06-18: WP-D1 SEALED + cold-reviewed (AP-5): run_op real wall-clock timeout (poll+kill),
  fallible sample_median → SKIP not fabricate, shared fixtures (fair), seam bench_compare.rs
  UNTOUCHED, no hidden decisions. Cherry-picked 5813573. Full gate green (498 tests).
  AUTHORITATIVE RUN exposed 2 honest SKIPs (probe SKIP'd correctly — the fix was fairness-design,
  LEAD's call): build fixture is FROM-scratch+RUN (docker can't build scratch+RUN → /bin/sh
  exit1); materialize built a 1GB image = ~7min, blew the 180s setup budget. LEAD fixes (51fad2a):
  build → equivalent FROM-alpine 3-step; materialize → docker create + cp-IN (untimed ingest,
  fits budget) + timed cp-OUT (the clonefile mirror); OP_TIMEOUT 60→120 for the ~50s 1GB cp.
  Re-ran AUTHORITATIVE → ALL 6 AXES MEASURED, every one a Lightr win:
    install 451.7x · materialize(1GB) 160.6x · cold-run 8.3x · re-run 48.1x · idle 0-vs-7 (∞) · build 69.6x
  Filled benchmark-results.md + parity F-602 truth-up (d6814ef). FULL GATE GREEN (fmt, clippy
  default+vz, build --workspace, 498 tests). FF-merged wave/humiliation-bench → main @ d6814ef.
  Single branch, clean, worktrees pruned. **WAVE-D COMPLETE** — owner's "obliterar o docker em
  todos os indicadores adversariais" = DONE, measured, on a release binary vs the live daemon.
- HONEST REMAINING (post Wave-D):
  (1) absolute perf-bar targets on THIS Intel box (distinct from the head-to-head wins): install
      ≤10MB MET (4.3MB); materialize 1GB ≤100ms APPROACHED (248.9ms, 1024×1MB files); re-run ≤10ms
      NOT met on shipped CoW (106.5ms) — binds to views-O(1) on Apple Silicon.
  (2) Bucket-B views-O(1) = the ≤10ms frontier: lightr-views crate is pure-logic + UNWIRED
      (solidify_step never called); EdenFS-class (macOS NFS-loopback + composefs in vz guest),
      partly HW-gated. NOT needed to beat docker (we win re-run 48x on CoW); it's the ABSOLUTE bar.
  (3) snapshot#5 / disk#6 = no faithful/cheap docker mirror → Lightr-side only, honest, not a claim.
  (4) owner-gated: G-PUBLISH (no remote/creds), G-HW-RUNTIME (arm64/Win/Linux validate), ratify ADRs.

=== WAVE-NET — NETWORKING (ship as "complete Docker + more, but lightR") ===
- 2026-06-18: variance hardening of the bench: 3 runs, EVERY axis wins EVERY run (worst-case:
  install 452x, materialize ≥100x, build ≥80x, re-run ≥30x, cold-run ≥7x, idle 0-vs-7..9).
  benchmark-results.md hardened (median+range, tamper-evident) @ 887ed69. Reddit verdict: GO with
  "macOS vs Docker Desktop, reproducible harness" framing. ROOT CAUSE of the binary-purging during
  hardening = disk 94% full (29GB free) → macOS purges target/ artifacts; docker ~17GB reclaimable
  but entangled w/ sibling corelink buildkit → flagged to owner, did NOT prune (sibling rule).
- 2026-06-18: owner: "ataca rede" (the #1 functional gap for "Docker completo"). LEAD ARCHITECTURE
  DECISION (frozen): Docker `-p` needs net isolation = an ENGINE concern; the daemonless SOTA is a
  USERSPACE forward-proxy (exactly how rootless docker/podman publish ports — slirp/pasta/gvproxy are
  userspace). A published service = long-running server → DETACHED + supervised (like compose). The
  detached path is NATIVE-ONLY by construction (run.rs early-returns for engine!=native before the
  detach check). So Phase 1 = native-detached `run -d -p`. Memo-key: ports NOT in key (runtime, like
  limits). Grounding (AP-5, read myself): no `-p` today; RunSpec no ports; compose ALREADY has
  proxy_bidirectional + ports model (lightr-build ~1438/~1295); supervise() (~796) spawns a
  long-running child + loops → server-capable; the proxy lives there.
- 2026-06-18: WAVE-NET Phase 1 dispatched (1 opus agent, worktree — disk-conscious, prune after; NOT
  a fan-out: cohesive feature + disk-constrained machine). Contract: PortMap + RunSpec.ports (+ all
  ctors), SpecOnDisk persist (serde default back-comat), NEW portforward.rs (generalize compose proxy,
  multi-connection), supervise hook after child spawn, CLI `-p/--publish`, run.rs parse + 2 honest
  guards (`-p` requires `-d`; `-p`+engine=vz/ns → Phase-2 err), tests (key-exclusion, portforward
  round-trip, parse, acceptance: python3 http.server reachability+teardown, SKIP if no python3).
  DONE: WP-NET1 SEALED (bd5ebe8) + cold-reviewed (AP-5, read myself): portforward.rs correct —
  Drop sets stop + pokes port to unblock accept + joins (no hang); per-conn threads short-lived;
  multi-connection. Lifecycle: _forwarders held in supervise() fn scope (lives through the loop,
  torn down on exit). Memo-key EXCLUDED (ports_excluded_from_key test, k1==k2). SpecOnDisk serde-
  default (back-compat). 2 honest guards (-p requires -d; -p+vz/ns→Phase2). Mechanical files verified
  (ports: vec![] in every ctor; docker.rs &[] passthrough). #[allow(large_enum_variant)] justified
  (once-per-process Cmd enum; boxing clap is worse). FF-merged main → bd5ebe8. FULL GATE GREEN in my
  tree: fmt, clippy -D (default+vz), build --workspace, 511 tests (498+13 net). Worktree pruned.
  F-304 truth-up'd (52c2e00). main @ 52c2e00, single branch, clean.
  NET Phase 1 = `lightr run -d -p HOST:CONTAINER` works (native-detached, userspace forward-proxy,
  daemonless, real reachability acceptance via python3 http.server). The 90% native publish case.
  HONEST: this is host-process publishing; the macOS "run a Linux image + -p" case = NET Phase 2 (vz
  host↔VM forward, buildable on Intel since vz boots). Reported to owner; Phase 2 plan pending owner go.

=== WAVE-VZ — FULL PRODUCT (run Linux image + -p on Mac) ===
- 2026-06-18: owner: "só vou shipar o produto FULL... temos que conseguir montar. Por que só Mac novo?
  não tem lógica." HE'S RIGHT — I over-hedged. VZ runs Linux VMs on Intel (Docker/OrbStack do it);
  only VM save/restore is arm64-only, NOT boot. Corrected.
- 2026-06-18 ★ MILESTONE: vz BOOT PROVEN ON INTEL by me. Built kernel pack (linux 6.18.5 bzImage via
  scripts/build-kernel-x86.sh in docker — reused the existing build/linux-pack-x86/kernel.bzImage to
  skip the recompile; lightr-init via build-init.sh) → installed ~/.lightr/packs/linux → codesigned
  target/debug/lightr with packaging/vz.entitlements (ad-hoc `codesign -s - --entitlements`, REQUIRED)
  → imported alpine (docker save|oci import). 3 PROOFS: `run --engine vz --rootfs alpine -- /bin/echo`
  =exit0+stdout; `exit 7`=exit7 (real code flows, not faked); `uname`=Linux 6.18.5 x86_64 + alpine
  3.24.1. The S5 spike (spikes/s5-vz-boot/run-s5.sh) is GREEN on Intel. Fixed a real bug: build-init
  cargo-zigbuild probe (committed b8ceead). main @ b8ceead.
- WAVE-VZ NETWORKING PLAN (frozen architecture, lead):
  Engine::run is SYNCHRONOUS one-shot (boot→run→stop→return; Swift lightr_vz_run blocks until VM
  .stopped; CMD_FILE/EXIT_FILE channel; NO network device). For `run --engine vz --rootfs <img> -p`:
  (1) Swift shim: add VZNATNetworkDeviceAttachment + MAC → guest gets a NAT IP (vmnet subnet,
      host-reachable). [moderate]
  (2) DETACHED vz lifecycle: a server stays up → the VM must stay running + be supervised + stoppable.
      Currently detach is native-only (run.rs early-returns for engine!=native). THE BIG PIECE — new
      supervised-VM path (VM held by a supervisor process, like compose holds services). [big]
  (3) guest IP discovery: DHCP lease by MAC (/var/db/dhcpd_leases) OR guest reports via file channel.
  (4) host proxy: reuse Phase-1 portforward, target guest_IP:CONTAINER (not 127.0.0.1).
  (5) wire -p for the vz path + guest binds 0.0.0.0.
  NEXT STEP: add the network device to the shim + prove the guest gets a reachable IP (smallest
  networking primitive) BEFORE the detached lifecycle. Then build up to port-publish.

=== WAVE-VZ NETWORKING DONE (host↔guest proven) + COLD-RUN GRIND + THE ∞ INSIGHT ===
- 2026-06-18: vz networking PROVEN end-to-end on Intel: NAT NIC + ip=dhcp (committed ef4a744) →
  guest leased 192.168.64.3 (host=gw .1) → a server bound 0.0.0.0:8080 INSIDE the alpine guest was
  reachable FROM THE HOST (host curled it → HTTP 200). The whole `-p`-via-vz path is de-risked.
- COLD-RUN GRIND (owner: "obliterar o docker na modalidade VM"): cut `run --engine vz` from 7s → ~1.9s
  reliable. Commits: 84537ca (conditional NAT via LIGHTR_VZ_NET + host force-stop on durable EXIT_FILE),
  d0691c1 (real-time console exit channel: guest PID1 prints LIGHTR_EXIT:<n>, shim taps the console +
  force-stops live → killed the virtiofs visibility lag; variance 1.3-4s → 1.7-2.3s; shim return -1/
  0..255/-2). Profiling: env-gated LIGHTR_VZ_TIMING (947201c). Breakdown: VZ-start ~0.3s + guest boot
  ~1.4s (driver-heavy kernel) + host ~0.3s. NOTE: build-init probe fixed twice (b8ceead cargo-zigbuild;
  d0691c1 rustc-sysroot instead of rustup). MACHINE: disk hit 92% repeatedly PURGING toolchain mid-build
  (release bin, cargo-zigbuild, rustup, musl target all evicted) — owner freed to 85GB; re-add target +
  zigbuild in the SAME command (no idle gap) to beat purges.
- ★ THE INSIGHT (owner challenge — "raciocinar, a resposta pro 1000x-∞ está em plain sight"):
  I was playing DOCKER'S GAME (race to boot a VM fast = parity, physics floor ~1s). The ∞ lever is
  MEMOIZATION: lightr REMEMBERS, Docker has NO memory. Re-run = ~0.06s CONSTANT regardless of work;
  Docker redoes the full work every time. PROVEN: 3s job→67x, 10s job→166x (re-run flat ~0.06s) →
  UNBOUNDED (10min build ~10,000x; 1h CI → ∞), scales with work AND reuse (fleet sharing the CAS).
- ★ STRATEGIC SYNTHESIS (owner: "mas o norte é SUBSTITUIR o docker — não conflita?"): memo is the
  MOAT, NOT the product. LAYERED: (1) PARITY (run/build/network/compose/services/OCI) = the foundation
  that makes "drop Docker, lose nothing" POSSIBLE; (2) MEMO + structural (daemonless 0-idle, CAS/CoW,
  4MB install) = the moat that makes switching INEVITABLE. Not everything is memoizable (servers/
  interactive/first-run) — there lightr STILL wins structurally (0 idle vs Docker's 24/7 VM, instant
  materialize/restart) + parity on run. So replace-completely holds across ALL use cases. Do NOT drop
  parity work chasing the memo. The vz-boot grind = table stakes (servers need a decent run), not the
  ∞ differentiator. Benchmark must show BOTH (∞ where memo applies, structural 40x+ where it doesn't).
- 2026-06-18: WP-VZMEMO dispatched (opus worktree, frozen contract): vz_memo_key + run_vz_memoized
  (lightr-run, mirror run_memoized_with's CAS/AC) + init captures cmd stdout/stderr→files (fsync before
  the console marker) + handler memoizes the vz+rootfs+!detach path (HIT replays from CAS, NO boot).
  Agent does code+unit-tests (can't validate vz boot); LEAD validates end-to-end (boot/hit/miss/∞).
- 2026-06-18 WP-VZMEMO DONE + VALIDATED + HARDENED + DOCUMENTED (main @ 99868de):
  cold-reviewed run_vz_memoized (faithful mirror of run_memoized_with; domain-tagged key) →
  FF e4f9c9a. End-to-end validated on Intel: MISS 2.1s (boots, captures, stores), HIT 0.014s
  (replays from AC, NO boot), byte-identical, exit-correct. Fixed a real UX bug (MISS leaked
  kernel console to stdout → route console to /dev/null on the memo miss, a00bd3e). Hardened the
  env-lockstep the agent flagged: GUEST_PATH = one source of truth in lightr_init, re-exported via
  lightr_engine, used by both engine + handler key (468ed49). Documented: benchmark-results.md
  container-modality table + parity F-105 (99868de). MEASURED: container re-run HIT 0.014s vs
  docker 1.30s = 93× (unbounded). Gate green throughout: 517 tests, clippy -D (default+vz),
  build-first. The ∞ moat is now in the Linux-container modality. NOTE the recurring stale-bin
  trap: my manual `cargo test --workspace` (no build-first) showed 8 acceptance FAILs = MISSING
  target/debug/lightr (disk-purged), NOT a regression — build-first → 517 green.
- CAMPAIGN STATE (honest, owner's "40x-or-idiot" bar): ∞ on re-run (native 48x + container 93x,
  both unbounded); 40-452x structural (install 452x, idle ∞, materialize 119-160x, build 70x);
  cold-run-once ~PARITY (VM-boot physics — any VM runtime; Lightr matches Docker's warm start with
  ZERO idle = strictly better). No axis Docker wins. Parity-floor axis can't be 40x (physics);
  honest. NEXT candidates: cold-image (CoW vs pull on a real image) · vz-build-memo · slim kernel
  (nudge cold-run below Docker) · vz networking Phase 2 (`-p` for Linux images) · the bench-compare
  harness could add a vz-memo re-run row (currently the container numbers are measured manually).
- WAVE-NET Phase 2 (honest remaining for full Docker networking; report to owner):
  vz host↔VM port-forward (the USEFUL macOS case: Linux image + `-p`; buildable on Intel since vz
  boots green) · ns netns+veth+bridge (Linux, NOT testable on this Intel Mac → HW-gated) · DNS /
  service-discovery via /etc/hosts (compose named services) · foreground `-p` · udp · container↔
  container networks · `-P`/random host port · `--add-host`/`--hostname`/`--dns`.

═══════════════════════════════════════════════════════════════════════════════
WP-NET2 — `-p` for a Linux IMAGE on vz — DONE + VALIDATED + GREEN (2026-06-18)
═══════════════════════════════════════════════════════════════════════════════
The flagship Docker-parity gap CLOSED: `run -d -p HOST:CONT --engine vz --rootfs
<img> -- <server>` boots a Linux container in a microVM + forwards host→guest.

ARCHITECTURE (lead-frozen contract, hybrid wave):
- IP discovery = deterministic file channel (guest PID1 getifaddrs → IP_FILE,
  sibling of EXIT_FILE). NOT the /var/db/dhcpd_leases-by-MAC heuristic (staleness
  + OS-file dependency = rejected as gambiarra). Guest reports its REAL IP.
- stop = write guest EXIT_FILE → shim pollExit force-stops the VM. ZERO new shim
  code (reused LIGHTR_VZ_EXITFILE, confirmed polls continuously @15ms).
- detached vz lifecycle = supervisor (lightr-run) boots VM in a worker thread
  (engine.run blocks), reads IP, starts forwarders, serves ctl.sock. VM is
  in-process → killing the supervisor tears it down (daemonless invariant holds).
- de-risked FIRST: kernel has CONFIG_IP_PNP_DHCP=y + VIRTIO_NET=y (checked the
  built kernel.config) → ip=dhcp works → no kernel rebuild needed.

WAVE (A+B agents disjoint+contract-bound; C/D/E lead core; F lead validation):
- WP-A (agent): lightr-init IP_FILE + InitSpec.net + GuestOps::publish_ip +
  primary_ipv4 (getifaddrs). cold-reviewed clean.
- WP-B (agent): portforward start_to(host, target_host, container) + start
  delegates 127.0.0.1. cold-reviewed clean.
- WP-C (lead): ExecSpec.net → VzEngine sets InitSpec.net + LIGHTR_VZ_NET (set_var
  safe: only main thread, no concurrent getenv, before VM boot). 6 call-sites.
- WP-D (lead): lightr-run → lightr-engine + lightr-init deps (acyclic, verified);
  SpecOnDisk.engine+rootfs_ref (serde default = native back-compat, NOT in RunSpec
  so NOT in memo key); spawn_detached_engine; supervise_vz branch.
- WP-E (lead): run handler routes (Vz,Some(rootfs),detach=true) → spawn_detached
  _engine (old engine path ignored -d, blocked sync — fixed); guard now allows
  vz+rootfs, blocks ns/wsl/vz-no-rootfs honestly.
- WP-F (lead): spikes/s5-vz-net/run.sh + EXPECTED.md. Also fixed CLI `vz` feature
  (docs referenced `--features vz` but CLI lacked it → added forwarding feature).

VALIDATION (real boot, Intel x86_64): spikes/s5-vz-net/run.sh GREEN end-to-end —
guest IP 192.168.64.3, `curl 127.0.0.1:18080` → `lightr-vz-net` (busybox nc in
the VM), `stop` → port closed + `exited 143` + 0 leaked supervisor (exits 0s on a
clean single run). Two script bugs found+fixed (false negatives, not product):
unescaped backtick; pgrep-exits-1-on-0-matches vs set -e/pipefail.

GATES: build default+vz green · 523 tests/0 (was 517; +4 agent unit tests, +2
SpecOnDisk serde back-compat/roundtrip) · clippy -D default+vz · fmt clean ·
spike GREEN. Hygiene: 0 strays, disk 122Gi free, gc'd, temp logs cleaned,
build/* gitignored.

HONEST remaining (Phase 2): ns veth/bridge (Linux HW-gated) · DNS/service-disc ·
foreground -p · udp · container↔container nets · -P/--add-host/--hostname/--dns ·
`lightr logs` for a detached vz container (guest output is on the rootfs share,
not run_dir/stdout.log — noted, not wired). `lightr stop` exits with the run's
code (143), like native — a docker-compat 0 is a separate (existing) choice.

===============================================================================
PARALLEL WAVE — cold-image + oci push + compose discovery — DONE+VALIDATED (2026-06-19)
===============================================================================
Owner: "ataca tudo em paralelo com teu time." Disciplined (NOT AP-1): parallel
INVESTIGATION (3 read-only agents) -> lead froze 3 contracts + scoping -> parallel
IMPLEMENTATION (3 agents, WORKTREE-ISOLATED since all touch lightr-cli + DISC
changes the spawn-detached signature -> concurrent compiles would collide) ->
lead cold-reviewed (AP-5) -> cherry-picked (disjoint files) -> full gate -> on-box
validation -> docs (lead-only, shared-file).

WP-CI cold-image (->57381b2): bench-compare cold-image workload. Lightr CoW from
CAS vs docker pull, DISTINCT image (busybox not shared alpine) so the cold-ness
guard doesnt sabotage other probes. CI-safe. VALIDATED 63ms vs 2429ms = 38.5x.

WP-PUSH oci push (->600906d + fix d14e7f4): synthesize single-layer OCI from the
CAS tree (store has no blobs), upload via the pull machinery. Honest: faithful-fs
snapshot not byte-identical. VALIDATED via local registry:2 (port 15000 since
:5000 is macOS AirPlay -> 403): push alpine, docker pull back, run cat
/etc/alpine-release = 3.24.1. BUG caught in validation (AP-5 win): config os was
the host (macos) -> fixed to linux. docker inspect os=linux/arch=amd64.

WP-DISC compose discovery (->a666366): env-var peer discovery (links convention),
plumbed via the on-disk spec + explicit child env, KILLED the racy global env
mutation (real data-race fix). Native services share loopback -> peer reached
directly, no proxy. VALIDATED a24b acceptance. HONEST: true name-DNS needs vz/ns.

GATES (main @ d14e7f4): build default+vz, 536 tests/0, clippy -D default+vz, fmt.
Docs: parity-audit F-302 push done, F-304 +disc, F-602 +cold-image;
benchmark-results +cold-image row.

REMAINING: name-DNS (vz/ns), container-to-container nets, foreground -p, udp,
push fidelity (preserve original config needs a store change), logs for detached vz.

===============================================================================
PUSH-FIDELITY — runnable-faithful push (rigor sota) — DONE+VALIDATED (2026-06-19)
===============================================================================
Owner: "rigor sota" → closed the lossy-push caveat I shipped in WP-PUSH.
Decision (schema rigor): a CAS-native sidecar (imgmeta keyed by ref, content-
addressed) — NOT a RefRecord codec change (the codec has a strict trailing-bytes
check; touching it risks every existing store). Store gains image_config_put/get.
pull + both import paths capture the original image config blob (best-effort: the
filesystem is already snapshotted, so a capture hiccup never fails the pull).
push re-emits it, preserving entrypoint/cmd/environment/workingdir/os/arch and
rewriting only rootfs.diff_ids for the one synthesized layer (history dropped).
Fallback to minimal Linux config when no original was captured.
VALIDATED on Intel via local registry:2: pull nginx:alpine (8 layers) -> push ->
docker inspect Entrypoint/Cmd/WorkingDir IDENTICAL to the original. (b478e95)
GATES: build default+vz, 537 tests/0 (+ store sidecar roundtrip), clippy, fmt.
Note: a24b flaked ONLY when two `cargo test --workspace` ran concurrently (port
contention from my duplicate bg run) — passes clean single-suite. Not a regression.
main @ b478e95.
