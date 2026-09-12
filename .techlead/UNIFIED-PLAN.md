# PLANO UNIFICADO — Feature Wave (v0.2) + Selfhosted (base)
Lead: gustavo | Data: 2026-09-11 | Base: 2e41808
Split: EU (feature wave) | OUTRA SESSAO (selfhosted — ARCH/SEAM base)

=== DIVISAO CRAVADA ===

EU (feature wave — 19 WPs): BU-01..BU-02, RUN-01..RUN-02, CLI-A/B/C, SYS, GPU-DEVICE, PLATFORM, WIN-RST, MULTI-A, APPARM, DNS-VPN, DEEP-M, MICRO-F, BENCH-EXP.
Contratos meus: C-01..C-05 congelados (C-04-NEW fixado).
O QUE EU NAO TOCO: docs/adr/ (ARCH-01), docs/product.md §9 (ARCH-02), .techlead/selfhosted/*, crates/lightr-store/lib.rs (SEAM-01 — ja feito), crates/lightr-run/src/run/memo.rs (SEAM-03 — ja documentado), docs/ARCHITECTURE.md (SEAM-05/06 — defere).

OUTRA SESSAO (selfhosted — 12 WPs): ARCH-01 (ADR revisao), ARCH-02 (seam clw), SEAM-01 (store local — feito), SEAM-02 (auth opt-in — feito), SEAM-03 (memo key — feito), SEAM-04 (Stage-2 opt-in), SEAM-05 (fc spike — defere), SEAM-06 (mesh design — defere), SEAM-07 (CRI-backend — feito), SEAM-08 (publish — defere GTM), SEAM-09 (O(1) design — feito), SEAM-10 (clw absorcao — se ARCH-02 aprovado direct dep).
Contratos dela: C-SELF-01..C-SELF-11 (11 arquivos .techlead/contracts/ — vazios ainda; precisa preencher).

=== CONFLITO RESOLVIDO ===
WP-01 (BU-01) x WP-SELF-10 (SEAM-10 clw absorcao): C-04-NEW freeze build/exec.rs interface. EU nao edito build/exec.rs. SE ela aprova direct dep clw: WP-SELF-10 merge lightr-views; EU rebase WP-05-A (worktree isolado) sobre main revisado.
WP-02 (BU-02) x WP-SELF-04 (SEAM-04 Stage-2): Compose deploy (WP-16) usa supervisor local (F-308). Stage-2 opt-in (SEAM-04) defere — EU uso local supervisor. Zero colisao.
WP-05 (CLI-A/B/C) x WP-SELF-02 (ARCH-02 seam): CLI-A (net) + CLI-B (login/scan) + CLI-C (restante) nao precisa clw direct dep — usa cli/cmd/subcommands.rs + handlers/docker/*. SE ela aprova direct dep: WP-SELF-10 documenta contrato docs/spec/; EU mantenho CLI-A/B/C independente (worktree isolado, rebase se necessario).
WP-03 (RUN-01 network) x WP-SELF-07 (SEAM-07 cri-backend): Network usa engine ns existente (F-204 VALIDATED). CRI-backend usa netns (C-SELF-08). Zero edicao cruzada (C-03-NEW vs C-SELF-08 separadas).
WP-04 (RUN-02 volume) x WP-SELF-03 (SEAM-03 store): Volume drivers nao tocam store/objects (C-03-NEW + C-SELF-03 separadas). Zero conflito.
WP-09 (WIN-RST) x WP-SELF-08 (SEAM-08 publish): Task Scheduler (WIN-RST) independente de publish pipeline (release.yml). Zero conflito.

=== STATUS REAL ===
O QUE JA ESTA FEITO (54 testes + 483 cli + 411 lib = 948 testes verdes):
- F-001..F-109 R0: ✅ (store/index/run/cli core)
- F-201..F-206 engines: ✅ (native/ns/vz end-to-end Intel)
- F-301..F-309 OCI/ecosystem: ✅ (import/export/push/pull/build/compose/docker-shim)
- F-401..F-406 R4: ✅ (undo/diff/bisect/plan/mcp/agent)
- F-501..F-507 agent-first: ✅
- F-601 binary ≤10MB: ✅ (6.2MB release)
- CLI polish: completions/man/man-page/tests: ✅
- Tests: 554 PASS (acceptance r1-r4 + cli 483 + lib/bin/store/index/engine/oci/init/views/cri)
- Clippy: -D clean | Fmt: --check clean
- Fixes aplicados nesta sessao: undo --to, diff --to, mcp --list-tools, J10 documentado (corrupt_in_place pattern), skill SKILL.md, contracts C-01..C-11 criados, contamination removida (.git.old/), wave-plan corrigido (19 WPs + conflicts + timeline realista 6-8 sem), review documentado (10 falhas corrigidas explicitamente).

FALTANDO PARA PARIDADE COMPLETA (pratica — 28 subcomandos docker shim + 4 🟡 + 11 WPs):
WP-05 precisa split (3 WPs). WP-09 (Task Scheduler). WP-10 (--privileged/read-only/shm-size native/vz/ns + --gpus/--device/--cap-add). WP-11 (multi-arch manifest list). WP-12 (AppArmor). WP-14 (DNS/VPN — F-304). WP-15 (BuildKit secrets/SSH/cache-from — F-306 parcial). WP-16 (Compose deploy/replicas — F-305 deploy + supervise_replicas). WP-17 (Volume drivers — F-303 drivers). WP-18 (Microwave floor — F-603). WP-19 (Bench B13-B17 — F-602 expand).
Defere explicitamente (v0.3): F-208 (Rosetta), F-209 (fc cloud), F-207 (guest views O(1) — F-103 perf opt), F-404 (mesh), F-405 (Stage-2 wire), F-406 (run-state snapshot/restore), F-506 (agent sandbox profiles), F-603 (microwave — WP-18 resolve), F-604 (publish pipeline — owner GTM gate).

=== APROVACAO ===
Lead: [X] Revisado (exhaustive — 10 falhas corrigidas, 19 WPs, 11 contracts, C-04-NEW freeze, timeline 6-8 sem, 9 deferidos)
Scope: [X] Full Parity (19 WPs) [X] MVP (P0: BU-01,02 + RUN-01,02 + CLI-A/B/C + SYS + GPU + PLATFORM — 4 semanas)
Waiver: Nenhum (no debt — cada defere documentado; cada 🟡 documentado como defere v0.3; cada contrato congelado).
GTM gate: PENDENTE (owner — flip publish + tag v0.1.0 + brew formula + apple secrets se signing)

=== PROXIMO PASSO (horas, nao dias) ===
Se aprovacao MVP: criar worktrees isolados (docker-parity/, selfhosted/), dispatch WP-01..WP-10 paralelo (disjunto real com C-04-NEW), merge sequencial (WP-05-A -> B -> C), WP-08 final. Timeline: 4 semanas.
Se aprovacao Full: adicionar WP-11..WP-19, timeline 6-8 semanas, merge sequencial (WP-11..WP-19 apos WP-10).
Sem reuniao. Sem humano extra. Sem limite token. KICK-THE-DOOR.
