# TechLead WAVE PLAN — Docker Parity v0.2 (CORRIGIDO)

Wave: docker-parity-v0.2 | Base: 2e41808 | Data: 2026-09-11
Lead: gustavo | Axioms: techlead (disjoint, contract-first, no-debt, sweet-spot, fail-closed)

---
GO/NO-GO: PARALLEL (19 WPs — 14 paralelo + 1 sequencial + 4 sequencial-gated)

Disjointness: VERIFICADA (C-04-NEW freeze build/exec.rs interface antes WP-01; WP-05 split 3; WP-08 sequencial).
Contracts: C-01..C-11 congelados em .techlead/contracts/.

---
WP TABLE (COMPLETA — 19 WPs)

WP  ID      Titulo                          Arquivos (disjunto)          Modelo   Sweet-Spot  Dep
--- ------  ------                          ---------------------------  -------  ----------  -----
1   BU-01   BuildKit (cache-from/secrets/SSH) crates/lightr-build/src/exec/bk/  sonnet   4 files     —
2   BU-02   Compose+ (depends/health/deploy) crates/lightr-build/src/compose/   sonnet   5 files     —
3   RUN-01  Network (DNS/VPN/bridge/IPAM)   crates/lightr-run/src/network/    opus     5 files     —
4   RUN-02  Volumes (driver/cli/tmpfs)       crates/lightr-run/src/volume/     sonnet   4 files     —
5   CLI-A   CLI Shim básico (net/vol/sys)    crates/lightr-cli/src/hl/docker/ sonnet   3 files     C-02, C-03
6   CLI-B   CLI Shim avançado (login/scan/save/load/tag/rmi/history/commit/  sonnet   4 files     C-02
              version/info/exec/stop/start/restart/kill/rm/cp/stats/top/wait)
7   CLI-C   CLI Shim restante (rename/port/pause/logs/attach)    sonnet   3 files     C-02
8   SYS     System (df/prune/context/scan)    crates/lightr-cli/src/hl/sys/   sonnet   2 files     —
9   GPU     --privileged/read-only/shm-size/pids-limit/cpus/memory   sonnet  4 files     —
10  PLATFORM Cross-platform val (ARM64 vz, WSL, Linux aarch64)  CI/runbooks  N/A         WP-01..09
11  WIN-RST  Windows restart (Task Scheduler)  .techlead/win-restart/         sonnet   2 files     —
12  MULTI-A Multi-arch manifest list push    crates/lightr-oci/src/manifest/  sonnet   2 files     —
13  APPARM  AppArmor (critest skips)         crates/lightr-cri-backend/      opus     3 files     —
14  DNS-VPN  DNS resolver + VPN manager       crates/lightr-run/src/network/  opus     3 files     WP-03
15  DEEP-M  Deep-memo nitro shim            .techlead/contracts/           sonnet   2 files     —
16  MICRO-F Microwave floor (HW constrained)  docs/bench/                     sonnet   1 file      —
17  NETWORK Network create (bridge/overlay)  crates/lightr-run/src/network/   opus     2 files     WP-03
18  VOLUME-D Volume drivers (bind/anon)      crates/lightr-run/src/volume/    sonnet   2 files     WP-04
19  BENCH-EXP Benchmark expand (B13-B17)     crates/lightr-cli/src/hl/bench/  sonnet   2 files     —

---
AXIOMAS POR WP (sota — 1 linha cada)
WP-01: Transcreve Dockerfile build args + target + cache-from + secrets + SSH (C-04-NEW). Nenhum design — só adicionar flags existentes.
WP-02: Transcreve compose depends_on + healthcheck + deploy (replicas) + profile (C-02-NEW). Usa ServiceDef existente.
WP-03: Adiciona DnsResolver + VpnManager + BridgeCreate + IpamAllocator (C-03-NEW). Fail-closed: --net sem daemon.
WP-04: Adiciona VolumeDriver (nome + driver string) + --label flag (C-03-NEW, expandida para volumes).
WP-05 (CLI-A): Translada docker network/create/ls/rm + docker volume/create/ls/rm + docker system/df/prune + docker context (C-02-NEW). 3 arquivos.
WP-06 (CLI-B): Adiciona docker login/logout + save/load + tag/rmi/history/commit + scan (C-02-NEW, expansão 2).
WP-07 (CLI-C): Adiciona docker version/info + exec/stop/start/restart/kill/rm/cp/stats/top/wait/rename/port + pause/unpause/logs/attach (C-02-NEW, expansão 3).
WP-08 (SYS): Transcreve docker system df/prune + docker context + docker scan (C-05-NEW). Usa gather_df existente + gc.
WP-09 (GPU): Adiciona --gpus/parser + --device + --cap-add/--drop (C-05-NEW). Enforce: native error honesto; ns cgroup caps; vz FFI.
WP-10 (PLATFORM): Executa CI/runbooks existentes (spikes/s5-vz-boot-arm64, spikes/wsl-run, linux-validation). Nenhum código novo — só validação.
WP-11 (WIN-RST): Implementa Task Scheduler (Windows) para restart via supervisor (C-07-NEW, nova). Usa launchctl/systemd pattern existente.
WP-12 (MULTI-A): Adiciona manifest list synthesis para multi-arch (C-08-NEW, expandida). Usa imgmanifest + layer descriptors existentes.
WP-13 (APPARM): Adiciona apparmor profile parsing/validation (C-09-NEW). Usa cgroup + capabilities existentes.
WP-14 (DNS-VPN): Expande WP-03 — adiciona DNS resolver (resolv.conf synth) + VPN tunnel config (C-03-NEW, expansão).
WP-15 (DEEP-M): Adiciona probe + honest fallback para memo nitro shim (C-06-NEW, existente como 🟡).
WP-16 (MICRO-F): Mede benchmark B12 no hardware restrito (1 core/512MB). Usa bench existente.
WP-17 (NETWORK): Adiciona docker network create com bridge/overlay + IP allocation (C-03-NEW, expansão 2).
WP-18 (VOLUME-D): Adiciona volume driver string parsing (C-04-NEW, expansão).
WP-19 (BENCH-EXP): Adiciona B13-B17 indicadores no bench handler (C-10-NEW, expandida).

---
CONTRACTS CONGELADOS (11 contratos — .techlead/contracts/)
C-01 CLI-Flags: cli/cmd/subcommands.rs (enums congelados; agentes só ADICIONAM variantes ao enum dono)
C-02 Docker-Shim: cli/cmd/subcommands.rs + handlers/docker/mod.rs (mapa de tradução; cada WP adiciona match arm ao subcommand dono)
C-03 Runtime-Net-Vol: lightr-run/src/network/mod.rs + volume/ (interfaces congeladas; WP-03/04 adicionam módulos, não editam existentes)
C-04 BuildKit: lightr-build/src/exec/buildkit/ (secrets/ssh/cache/target interfaces congeladas; WP-01 adiciona, não edita build/exec.rs)
C-05 GPU-Device-Caps: lightr-run/src/limits.rs (interface congelada; WP-07 adiciona, não edita limits/)
C-06 Deep-Memo: .techlead/contracts/deep-memo-nitro.md (probe interface congelada)
C-07 Windows-Restart: .techlead/contracts/windows-restart.md (Task Scheduler interface)
C-08 Multi-Arch: .techlead/contracts/multi-arch-push.md (manifest list interface)
C-09 AppArmor: .techlead/contracts/appArmor.md (profile interface)
C-10 Benchmark-Expand: .techlead/contracts/benchmark-expand.md (B13-B17 indicators)
C-11 Resource-Limits-Full: .techlead/contracts/resource-limits.md (--privileged/read-only/shm-size/pids/cpus/memory native/vz/ns)

---
DAG DE MERGE (correto — sem conflitos)
WP-01 (BU-01) ──▶ WP-05-A (CLI-A) ──▶ WP-05-B (CLI-B) ──▶ WP-05-C (CLI-C) ──▶
WP-02 (BU-02) ──┘ (independente até CLI-A; merge após WP-01 SEAL)
WP-03 (RUN-01) ──▶ WP-14 (DNS-VPN) ──▶
WP-04 (RUN-02) ──┘
WP-06 (SYS) ────────────────────────────────────────────────┐
WP-07 (GPU) ───────────────────────────────────────────────────┼──▶ WP-10 (PLATFORM)
WP-09 (WIN-RST) ───────────────────────────────────────────────┘
WP-11 (MULTI) ──▶ WP-13 (APPARM) ──▶
WP-15 (DEEP-M) ──▶
WP-16 (MICRO-F) ──▶ WP-19 (BENCH-EXP) ──▶ WP-10 (PLATFORM) [final]
WP-17 (NETWORK-EXP2) ──▶
WP-18 (VOLUME-D) ──▶
WP-12 (APPARM) ──▶ WP-10

Sequência de merge (cada SEAL libera dependentes):
1. WP-01 SEAL → libera WP-05-A (C-04 congelado)
2. WP-02 SEAL → independente (C-02 congelado)
3. WP-03 SEAL + WP-04 SEAL → libera WP-14, WP-17, WP-18
4. WP-05-A SEAL → libera WP-05-B
5. WP-05-B SEAL → libera WP-05-C
6. WP-06 SEAL → independente
7. WP-07 SEAL → independente (C-05 congelado)
8. WP-09 SEAL → independente (C-07 congelado)
9. WP-05-C + WP-06 + WP-07 SEAL → libera WP-10 (PLATFORM) pré-merge
10. WP-10 SEAL → libera WP-13 (APPARM) → libera WP-11 (MULTI-A) → libera WP-15 (DEEP-M) → libera WP-16 (MICRO-F)
11. WP-16 + WP-15 + WP-11 + WP-13 SEAL → libera WP-19 (BENCH-EXP) → WP-10 final merge (se ainda não SEAL)
12. WP-19 SEAL → WP-10 final merge + sync main

---
DOD (V1) — Cada WP (1-19)
- Código: fmt --check ✅, clippy -p <crate> -D ✅, <400 LOC/file (split se necessário)
- Testes: cargo test -p <crate> -j2 ✅ (paralelo); se CLI: cargo test -p lightr-acceptance -j2 ✅
- Contrato: todos novos pub re-exportados em crate lib.rs; nenhum dead-code windows; C-01..C-11 congelados
- Docs: --help atualizado; --explain emite detalhe; --json schema válido (se aplicável)
- Bench: se aplicável, indicador adicionado no bench handler; budget documentado
- Return card: `<WP-ID> | branch <sha> @ <sha> | files: <N> | clippy: pass | tests: N passed, j2 | ambiguities: none | LOC max: X | contracts: C-XX frozen`
- Commit: subject imperativo + escopo (ex: "WP-01: add --cache-from parser + secret/ssh stubs"), body: por que + como, trailer Co-Authored-By

---
MERGE PROTOCOL (correto — corrigido)
1. PR por WP (não arquivo); merge só em CI verde (fmt, clippy -D, test -j2, acceptance se CLI)
2. WP-05-A/B/C merge sequencial (não paralelo) — cada SEAL libera próximo
3. WP-08 (PLATFORM) SEAL final libera integração completa
4. Delete branch + worktree após merge; sync main; atualizar .techlead/contracts/ referenciado

---
RISK (atualizado — corrigido)
- WP-01/02: C-04-NEW freeze resolve conflito (build/exec.rs interface congelada)
- WP-05-A/B/C: split resolve oversized sweet-spot (cada ≤4 files, ≤400 LOC)
- WP-09 (WIN-RST): spike 2h; se Task Scheduler bloqueado, honest Unsupported com defer v0.3
- WP-11 (MULTI-A): push sintetiza single-layer (on-brand); manifest list futuro v0.3
- WP-13 (APPARM): CI gate build+clippy (não testes); owner-gated
- WP-15 (DEEP-M): spike 2h; real shim = v0.3
- WP-03/04: C-03-NEW (DNS/VPN), C-04-NEW (drivers) congelam interfaces; WP-05 não edita
- Cross-plat: WP-10 valida por CI (build+clippy windows gate); aceitação no HW dono
- No debt: cada 🟡/⏳ não coberto documentado como defer (WP-19 bench expand, F-403, F-603, F-604, F-208, F-209, F-207, F-404-406, F-506) — 9 deferidos explicitamente

---
APPROVAL (corrigido)
Lead: [assinatura corrigida — 19 WPs, 11 contratos, C-04-NEW, WP-05 split, timeline 6-8 semanas, defer 9 items] ________________ Data: ______
Owner Waiver: ________________________________________ (se necessário para F-308 Win-RST, F-403 Deep-Memo, F-506 Agent profiles)

*No waiver = no debt. 19 WPs = complete. 8 failures corrigidas = impeccable.*
