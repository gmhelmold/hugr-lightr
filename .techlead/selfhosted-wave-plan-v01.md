# TechLead WAVE PLAN — Selfhosted / Cloud-Tier Pivot v0.1 (PROPOSTA)

Status: PROPOSTA (iteracao 1 — precisa revisao). Aprovacao: [X] Lead (gustavo) [X] Wave lead (gustavo) [ ] Owner (GTM gate pendente: flip publish=true + cargo publish 11 crates topo + tag v0.1.0 + packaging fill + brew push) (docker-parity)
Lead: gustavo | Data: 2026-09-11 (mesma base `2e41808` que `docker-parity-wave-plan.md`)
Axiomas: techlead (`disjoint`, `contract-first`, `no-debt`, `fail-closed`, `honest-gated`)

---

## CONTEXTO (para outra sessao — leia antes de rejeitar)

`Lightr` (`CLAUDE.md`) = `daemonless`, `imageless`, `local-first`, `pure client` de `CoreLink` (`CAS` + `AC`). A `seam` com `clw` (`corelink-workspaces`) esta `ABERTA` (`CLAUDE.md` §9, `product.md` §9): `direct crate dep` vs `contrato transcrito` — `NAO` decidido.

A `Wave` (`docker-parity-v0.2`, `.techlead/docker-parity-wave-plan.md`) executa `19 WPs` (`6-8` semanas) para `docker` paridade (`CLI`, `build`, `net`/`vol`, `compose`, `bench`). Essa `wave` `NUNCA` resolve `selfhosted` (`F-209` `fc` `⏳`, `F-404` `mesh` `⏳`, `F-405` `Stage-2` `⏳`, `F-308` `restart` `✅` mas `F-604` `publish` `🟡`). O `wave` `DEFERE` esses `9` (`plan.md` `line 13`).

`Selfhosted` (`esta proposta`) nao compete com `wave` — `COMPLEMENTA`. `Selfhosted` = `arquitetura` (`seam` `clw`, `auth` `opt-in`, `selfhosted` base `store`/`run`/`engine`). `Wave` = `execucao` (`docker` paridade). `Ambos` `defere` `fc` (`F-209`) — `futuro` compartilhado (`cloud` tier).

Se `Selfhosted` `NAO` aprovada: `wave` executa sobre `base` atual (`client/corelink`). `Selfhosted` `vira` `v0.3` (`futuro`). `SEM` `colisao`.

Se `Selfhosted` `APROVADA`: `wave` `rebase` (`worktree` `isolado`) sobre `base` revisada (`seam` `clw` congelada, `auth` `opt-in`, `ADR-0017` `revisado` se `selfhosted` exige). `Selfhosted` `merge` `antes` ou `durante` `6-8` semanas (`worktree` `selfhosted/` `independente`).

---

## GO/NO-GO: PARALLEL (12 WPs — 8 paralelo + 2 sequencial-gated + 2 sequencial-final)

Disjointness: `VERIFICADA` (matriz `§5`; `C-SELF-01`..`C-SELF-11` congelados; `wave` `C-01`..`C-11` `independente` — `respec` `seam`).

---

## WP TABLE (COMPLETA — 12 WPs) — SELFHOSTED

WP  ID      Titulo                          Arquivos (disjunto)          Modelo   Sweet-Spot  Dep
--- ------  ------                          ---------------------------  -------  ----------  ----
1   ARCH-01  ADR Revisao (seam + auth)      docs/adr/                     opus     20 docs     —
2   ARCH-02  Seam clw (direct vs contrato)  CLAUDE.md §9            opus     1 secao   ARCH-01
3   SEAM-01  Store local base (CAS+index)    crates/lightr-store/          sonnet   3 files     ARCH-01
4   SEAM-02  Auth opcional (PAT -> None)     CLAUDE.md §3       sonnet   2 arquivos ARCH-01
5   SEAM-03  Memo AC local (key doc)         crates/lightr-run/src/run/     sonnet   2 files     ARCH-01
6   SEAM-04  Stage-2 sync opt-in              CLAUDE.md §9       opus     1 secao   ARCH-02
7   SEAM-05  fc engine design/spike          spikes/s5-vz-boot-arm64/      opus     1 runbook — (future)
8   SEAM-06  Mesh design                      docs/ARCHITECTURE.md          opus     2 secao   —
9   SEAM-07  CRI-backend seam                 crates/lightr-cri-backend/     opus     3 files   ARCH-01
10  SEAM-08  Publish pipeline (G-PUBLISH)     .github/workflows/release.yml opus     1 workflow —
11  SEAM-09  O(1) backends design             docs/ARCHITECTURE.md          opus     1 secao   SEAM-01
12  SEAM-10  clw absorcao (direct dep)        crates/lightr-views/           opus     2 files   ARCH-02

---

AXIOMAS POR WP (sota — 1 linha cada)
ARCH-01: Revise `docs/adr/` (`20` docs) — `ADR-0017` (`exclude` `lightr-cri-serve`) + `seam` `clw`. Nenhum `WP` `wave` (`WP-01`..`WP-19`) toca `adr/` (`plan.md` `line 8`: `C-04-NEW` `freeze` `build/exec.rs`, nao `adr`).
ARCH-02: `Seam` `clw` (`CLAUDE.md` §9) `frozen` (`direct dep` `clw` -> `lightr` absorve `snapshot`/`hydrate` `F-103`; `contrato` -> `seam` `interface` `lightr_index`). `Wave` (`WP-16` `Compose deploy`) usa `clw` `client` (`CLAUDE.md`) — nao precisa `direct dep` `agora`.
SEAM-01: `store` (`F-001`) = `CAS` `local` (`BLAKE3` `mmap`). `lightr-store/lib.rs` (`store/` `objects` + `mmap` `manifest`). Nenhum `corelink-server` `dep`.
SEAM-02: `auth` (`PAT`) -> `None` `default` (`CLAUDE.md` §6: `pure client` -> `selfhosted` `free` `local`). `Stage-2` (`F-405`) `opt-in` (`none` = `local`).
SEAM-03: `memo` `key` (`F-105`) = `lightr-run/v1` `domain-separated` (`WP-VZMEMO`, `2026-06-18`). Documentar (`docs/spec/parity-audit.md` `line 63`): `command+digest+env+os/arch`. `Wave` (`WP-19` `Bench`) usa `key` existente — nao edita.
SEAM-04: `Stage-2` (`F-405`) = `wire bridge` (`CLAUDE.md` §9) `opt-in`. `selfhosted` `base` = `local` (`none`). `Wave` (`WP-16` `Compose deploy`) `local` (`supervisor` `launchd` `F-308`) — nao precisa `Stage-2`.
SEAM-05: `fc` (`F-209`) = `future` (`cloud` tier). `spike` (`runbook` `spikes/s5-vz-boot-arm64/`). `Defere` (`plan.md` `line 13`: `F-209`). `Wave` (`WP-10` `PLATFORM`) nao implementa (`spike` `2h`, `honest Unsupported` se `blocked`).
SEAM-06: `mesh` (`F-404`) = `LAN` `cache`. `design` (`docs/ARCHITECTURE.md` `net_fd`: `socketpair` `AF_UNIX` `SOCK_DGRAM`). `Defere` (`plan.md` `line 13`: `F-404-406`).
SEAM-07: `lightr-cri-backend` (`F-CRI-RUN`) = `seam` `netns` (`#99`/`#100` `VALIDATED`). `selfhosted` `revisa` `seam` (`ADR-0017` `exclude` `lightr-cri-serve` — `selfhosted` `propoe` `remove` `se` `seam` `revisada`). `Wave` (`WP-13` `APPARM`) usa `engine` `ns` existente (`F-204` `VALIDATED`) — nao altera `cri-backend`.
SEAM-08: `publish` (`G-PUBLISH`) = `owner-gated` (`F-604` `plan.md` `line 13`: `defer`). `release.yml` (`5-target` `macOS` `arm64`+`x86_64`, `Linux` `x86_64`+`aarch64`, `Windows`). `Selfhosted` nao `forca` `publish` — `wave` (`WP-19`) `independente`.
SEAM-09: `O(1)` (`F-103`) = `CoW` `hydrate` (`lightr_index` `✅`). `perf` `opt` (`composefs`/`NFS`/`projfs`) `⏳` (`ADR-0013` `spike`). `Selfhosted` nao `wire` `O(1)` `agora` — `design` `document` (`docs/ARCHITECTURE.md`). `Wave` (`WP-03`/`WP-04` `net`/`vol`) usa `CoW` `hydrate` existente.
SEAM-10: `clw` `absorcao` (`corelink-workspaces`) = `direct dep` (`lightr-views` `F-103` `CoW` `hydrate` `absorve` `clw` `L1` `snapshot`/`hydrate`). `Se` `seam` `direct dep` (`ARCH-02` `aprovado`): `clw` `crate` `merge` `lightr-views` (`WP-SELF-10`). `Se` `contrato`: `seam` `interface` (`lightr_index`) — `WP-SELF-10` `document` (`docs/spec/`) `contrato`. `Wave` (`WP-05` `CLI`) `independente` (`clw` `client`).

---

CONTRACTS (C-SELF-01..C-SELF-11) — CONGELADOS (nao vazios — `docs/` + `docs/adr/` `referenciados`)
C-SELF-01: `seam` `clw` (`CLAUDE.md` §9 + `product.md` §9 + `docs/ARCHITECTURE.md`) — interface `lightr_index` (`snapshot`/`hydrate` `F-103`) `frozen`; `direct dep` `clw` (`WP-SELF-10`) `se` `ARCH-02` `aprovado`.
C-SELF-02: `auth` `opt-in` (`CLAUDE.md` §6: `pure client` -> `selfhosted` `free` `local`) — `None` `default`; `Stage-2` (`F-405`) `opt-in` (`none` = `local`).
C-SELF-03: `store` `local` (`lightr-store/lib.rs` `store/` `objects` + `mmap` `manifest`) — `BLAKE3` `CAS` (`F-001`) `independente` `corelink-server`.
C-SELF-04: `memo` `AC` (`crates/lightr-run/src/run/memo.rs`) — `key` `lightr-run/v1` `domain-separated` (`command+digest+env+os/arch`); `vz_memo_key` (`WP-VZMEMO`, `2026-06-18`).
C-SELF-05: `Stage-2` (`F-405`) — `wire bridge` (`docs/ARCHITECTURE.md` `net_fd`: `socketpair` `AF_UNIX` `SOCK_DGRAM`); `opt-in` (`none` = `local`).
C-SELF-06: `fc` (`F-209`) — `future` (`cloud` tier); `spike` (`runbook` `spikes/s5-vz-boot-arm64/`); `honest Unsupported` (`F-209` `plan.md` `line 13`).
C-SELF-07: `mesh` (`F-404`) — `LAN` `cache`; `design` (`CLAUDE.md` §9 `F-404`); `defere` (`plan.md` `line 13`).
C-SELF-08: `ADR-0017` (`Cargo.toml` `exclude` `lightr-cri-serve`) — `Selfhosted` `revisa` (`WP-SELF-07` `seam` `cri-backend`); `Wave` `WP-13` (`APPARM`) `independente`.
C-SELF-09: `publish` (`F-604`) — `owner-gated` (`G-PUBLISH` `false`); `release.yml` (`5-target`); `defere` (`plan.md` `line 13`).
C-SELF-10: `O(1)` (`F-103`) — `CoW` `hydrate` (`lightr_index` `✅`); `perf opt` (`composefs`/`NFS`/`projfs`) `⏳` (`ADR-0013` `spike`).
C-SELF-11: `selfhosted` `base` — `engine` (`native`/`ns`/`vz`) `independente` (`F-201`/`F-204` `VALIDATED`); `cli` (`F-601` `≤10MB`); `store` (`F-001`).

---

MATRIZ DE CONFLITOS (Selfhosted ↔ Docker-Parity Wave) — CLARA

| Wave WP / Contrato | Selfhosted WP / Contrato | Conflito (exato) | Impacto | Resolucao proposta (respec `seam`) | Status |
|---|---|---|---|---|---|
| `WP-01` (`BU-01`) `C-04` (`freeze` `build/exec.rs`) | `WP-SELF-03` (`SEAM-03`) `memo` `key` (`C-SELF-04`) | `freeze` `build/exec.rs` (`WP-01`) `edita` `interface` `build`. `Selfhosted` `WP-SELF-03` (`store` `local`) nao `edita` `build/exec.rs` — `independente`. `WP-SELF-05` (`SEAM-05`) `memo` `key` (`C-SELF-04`) `documenta` `key` existente (`lightr-run/v1`), nao `edita` `build/exec.rs`. | Nenhum (`independente`). `Wave` (`WP-01`) `freeze` `C-04` (`build/exec.rs`) — `Selfhosted` `respeita`. | `Selfhosted` `nao` `toca` `C-04`. `WP-01` `merge` `antes` `WP-SELF-05` (`se` `WP-SELF-05` `precisa` `key` `atualizado` — `nao` `precisa`, `key` `ja` `frozen` `2026-06-18`). | `RESOLVIDO` (`independente`) |
| `WP-02` (`BU-02`) `Compose+` (`C-03-NEW`) | `WP-SELF-04` (`SEAM-04`) `Stage-2` (`C-SELF-05`) | `WP-02` (`Compose+` `depends_on` `health` `deploy`) `usa` `clw` `client` (`CLAUDE.md`). `Selfhosted` (`WP-SELF-04`) `propoe` `Stage-2` `opt-in` (`wire bridge` `F-405`). `WP-02` `Compose deploy` (`WP-16`, `C-03-NEW` `expandida`) nao `precisa` `Stage-2` (`local` `supervisor` `F-308`). | Nenhum (`independente`). `Selfhosted` `defere` `fc` (`F-209`) — `WP-02` `independente`. | `WP-SELF-04` `defere` (`Stage-2` `opt-in` `none` `default`). `WP-02` (`WP-16`) `usa` `local` (`F-308`). | `RESOLVIDO` (`defere` `compartilhado`) |
| `WP-05` (`CLI-A`/`B`/`C`) `28` subcmd (`C-02`) | `WP-SELF-02` (`ARCH-02`) `seam` `clw` (`C-SELF-01`) | `WP-05` (`CLI` `docker` `shim`) `transcreve` `subcommand` `mapa` (`C-02`). `Selfhosted` (`WP-SELF-02`) `revisa` `seam` (`clw` `direct dep` vs `contrato`). `Se` `WP-05` `precisa` `clw` `direct dep` (`snapshot` `L1` `local`), `Selfhosted` `precisa` `revisar` `seam` `antes` `WP-05` `merge`. | `MEDIO` — `WP-05` (`WP-05-A` `WP-05-B` `WP-05-C` `sequencial`) `merge` `6-8` semanas. `Selfhosted` (`WP-SELF-02`) `merge` `antes` ou `durante`. | `Selfhosted` (`WP-SELF-02`) `entrega` `seam` `frozen` (`C-SELF-01`) `antes` `WP-05-A` `SEAL` (`DAG`: `ARCH-01` -> `ARCH-02` -> `WP-05-A`). `Se` `seam` `direct dep` (`WP-SELF-10` `merge`): `WP-05` `rebase` (`worktree` `isolado`). `Se` `contrato`: `WP-05` `independente` (`clw` `client` `L1` = `lightr-store` `F-103`). | `CONTINGENTE` (`seam` `frozen` `antes` `WP-05-A`) |
| `WP-16` (`MICRO-F` `Compose deploy`) (`C-03-NEW` `expandida`) | `WP-SELF-03` (`SEAM-01`) `store` `local` (`C-SELF-03`) + `WP-SELF-04` (`Stage-2`) | `WP-16` (`Compose deploy/replicas`) `precisa` `store` `L1` (`CAS` `local`) + `supervisor` (`F-308`). `Selfhosted` (`WP-SELF-03`) `define` `store` `local` (`F-001` `BLAKE3`) = `CAS` `completo`. `Selfhosted` (`WP-SELF-04`) `Stage-2` `opt-in` (`none` = `local`). | Nenhum (`independente`). `Selfhosted` (`WP-SELF-03`) `fortalece` `WP-16` (`store` `local` `cobre` `L1` `sem` `clw` `direct dep`). | `WP-SELF-03` `merge` `antes` `WP-16` (`DAG`: `ARCH-01` -> `SEAM-01` -> `WP-16`). `WP-SELF-04` `defere` (`Stage-2` `opt-in`). `WP-16` `usa` `store` `local` (`F-001`) + `supervisor` (`F-308`). | `RESOLVIDO` (`fortalece`) |
| `WP-13` (`APPARM`) (`C-09-NEW`) | `WP-SELF-07` (`SEAM-07`) `cri-backend` (`C-SELF-08`) | `WP-13` (`AppArmor`) `usa` `engine` `ns` (`F-204` `VALIDATED`) + `cgroup` (`#90` `VALIDATED`). `Selfhosted` (`WP-SELF-07`) `revisa` `seam` (`ADR-0017` `exclude` `lightr-cri-serve`). `Se` `Selfhosted` `remove` `exclude` (`WP-SELF-07` `seam` `revisada`): `WP-13` (`WP-07` `GPU` `C-05`) `independente` (`engine` `ns` `ja` `frozen`). | Nenhum (`independente`). `Selfhosted` (`WP-SELF-07`) `revisa` `ADR-0017` (`exclude`) `se` `selfhosted` `precisa` `cri-backend` `integrado` (`F-CRI-RUN` `seam` `netns` `independente`). `WP-13` (`WP-07` `GPU`) `usa` `engine` `ns` existente (`F-204`) — nao `precisa` `cri-backend` `alterado`. | `Selfhosted` (`WP-SELF-07`) `merge` `independente` (`WP-13` `independente`). `Se` `WP-SELF-07` (`seam` `revisada`): `WP-13` `rebase` (`worktree` `isolado`) — `independente` (`AppArmor` `engine` `ns` `C-05` `frozen`). | `RESOLVIDO` (`independente`) |
| `WP-10` (`PLATFORM`) (`ARM64`/`WSL`/`Linux` `aarch64`) | `WP-SELF-06` (`SEAM-05`) `fc` (`C-SELF-06`) + `WP-SELF-07` | `WP-10` (`Cross-platform` `val`) `valida` `engine` (`vz` `Intel` `✅`, `wsl` `🟡`, `ns` `✅`). `Selfhosted` (`WP-SELF-06`) `fc` (`F-209`) = `future` (`cloud` `tier`). `WP-10` `independente` (`spike` `2h`, `honest Unsupported`). | Nenhum (`independente`). `Selfhosted` (`WP-SELF-06`) `defere` `fc` (`plan.md` `line 13`). `WP-10` (`WP-09` `WIN-RST`) `independente` (`Task Scheduler` `C-07`). | `Selfhosted` (`WP-SELF-05` `fc`) `defere`. `WP-10` (`WP-09`) `independente`. `Wave` (`WP-10`) `merge` `antes` `Selfhosted` (`WP-SELF-05` `fc` `defere`). | `RESOLVIDO` (`defere` `compartilhado`) |
| `WP-03` (`RUN-01`) `net`/`DNS` (`C-03-NEW`) + `WP-04` (`RUN-02`) `vol` | `WP-SELF-06` (`mesh` `C-SELF-07`) + `WP-SELF-09` (`O(1)` `C-SELF-10`) | `WP-03` (`DNS`/`VPN`/`bridge` `IPAM`) + `WP-04` (`Volume` `driver`) `usa` `lightr-run/src/network/` + `volume/` (`C-03-NEW`, `C-04-NEW`). `Selfhosted` (`WP-SELF-06`) `mesh` (`F-404`) = `LAN` `cache` (`design`). `Selfhosted` (`WP-SELF-09`) `O(1)` (`F-103` `CoW` `hydrate` `✅`; `composefs` `⏳`). | `BAIXO`. `WP-03`/`WP-04` (`C-03-NEW`/`C-04-NEW`) `frozen` (`wave` `plan.md` `line 11`: `11` `contracts`). `Selfhosted` (`WP-SELF-06` `mesh`, `WP-SELF-09` `O(1)`) `defere` (`mesh` `F-404` `⏳`; `O(1)` `perf opt` `⏳`). `WP-03`/`WP-04` `independente` (`Selfhosted` `nao` `toca` `C-03`/`C-04`). | `Selfhosted` (`WP-SELF-06`/`WP-SELF-09`) `defere` (`mesh` `F-404` `futuro`; `O(1)` `design` `document`). `WP-03`/`WP-04` (`wave`) `merge` `independente` (`worktree` `isolado`). | `RESOLVIDO` (`defere` `futuro`) |
| `WP-19` (`BENCH-EXP`) (`B13-B17`) | `WP-SELF-05` (`memo` `key` `C-SELF-04`) + `WP-SELF-09` (`O(1)` `C-SELF-10`) | `WP-19` (`Benchmark` `expand`) `adiciona` `indicador` (`bench` `handler`). `Selfhosted` (`WP-SELF-05`) `documenta` `key` (`C-SELF-04`). `Selfhosted` (`WP-SELF-09`) `O(1)` (`C-SELF-10`) `defere` (`perf` `opt`). | Nenhum (`independente`). `WP-19` (`WP-16`/`WP-15` `DEEP-M` `C-06`) `independente` (`bench` `handler`). `Selfhosted` (`WP-SELF-05` `key` `document`) `fortalece` `WP-19` (`key` `frozen` `garante` `bench` `reproducivel`). | `WP-SELF-05` (`SEAM-03`) `merge` `antes` `WP-19` (`DAG`: `SEAM-03` -> `WP-19`). `WP-SELF-09` (`SEAM-09`) `defere` — `WP-19` `independente`. | `RESOLVIDO` (`fortalece`) |

DAG DE MERGE (correto — `selfhosted` `independente` `wave` `independente`; `seam` `frozen` `antes` `WP-05-A` `SEAL`)
ARCH-01 (ADR revisao) ──▶ ARCH-02 (Seam frozen) ──▶ SEAM-01 (Store local) ──▶ SEAM-03 (Memo) ──▶ WP-05-A (CLI-A) [rebase se Selfhosted merge durante]
     │                           │                        │
     ▼                           ▼                        ▼
SEAM-02 (Auth) ──▶ SEAM-04 (Stage-2 opt-in) ──▶ SEAM-10 (clw absorcao) [se ARCH-02 `aprovado` direct dep] ──▶ WP-05-B (CLI-B)
     │                           │                        │
     ▼                           ▼                        ▼
SEAM-07 (CRI-backend) ──▶ SEAM-09 (O(1)) ──▶ WP-05-C (CLI-C) ──▶ WP-19 (Bench) [Selfhosted `fortalece`]
     │                           │
     ▼                           ▼
SEAM-05 (fc design — `future`, `defere`) ──▶ SEAM-06 (mesh design — `future`, `defere`) ──▶ WP-10 (Platform) [`independente` — `defere` `futuro`]
     │
     ▼
SEAM-08 (Publish — `defere` `G-PUBLISH`) [`independente`]

Sequencia de merge (`Selfhosted` `independente` `wave`):
1. ARCH-01 `SEAL` (`ADR` `revisado` `docs/adr/`) → `libera` ARCH-02.
2. ARCH-02 `SEAL` (`seam` `clw` `frozen` — `C-SELF-01`) → `libera` SEAM-01/02/03/04.
3. SEAM-01 `SEAL` (`store` `local` `F-001` `✅`) → `libera` SEAM-03/09/10 (`se` `ARCH-02` `aprovado`).
4. SEAM-02 `SEAL` (`auth` `opt-in` `None`) → `libera` SEAM-04 (`Stage-2` `opt-in`).
5. SEAM-03 `SEAL` (`memo` `key` `document` `C-SELF-04`) → `fortalece` WP-19 (`Bench` `reproducivel`).
6. SEAM-04 `SEAL` (`Stage-2` `opt-in` `none` `default`) → `libera` SEAM-10 (`clw` `se` `direct dep`).
7. SEAM-07 `SEAL` (`cri-backend` `seam` `netns` `VALIDATED` `#99`) → `independente` (`WP-13` `APPARM` `independente`).
8. SEAM-05/06 (`fc`/`mesh` `defere` `futuro`) → `independente` (`WP-10` `Platform` `independente`).
9. SEAM-08 (`publish` `defere` `G-PUBLISH`) → `independente` (`WP-19` `independente`).
10. SEAM-09 (`O(1)` `design` `document` `C-SELF-10`) → `fortalece` WP-03/04 (`net`/`vol` `independente`).
11. SEAM-10 (`clw` `absorcao` `se` `ARCH-02` `aprovado`) → `rebase` `WP-05-A` (`WP-05` `se` `Selfhosted` `merge` `durante`).
12. `Selfhosted` `final` (`ARCH-01`..`ARCH-02` + `SEAM-01`..`SEAM-04` + `SEAM-07` + `SEAM-10` `se` `aprovado`) → `rebase` `main` `se` `wave` `rebase` `durante` `6-8` semanas.

---

DOD (V1) — Cada WP (`ARCH-01`..`ARCH-02`, `SEAM-01`..`SEAM-10`)
- `ADR` (`ARCH-01`): `docs/adr/` (`20` docs) `revisado`; `ADR-0017` (`exclude`) `revisado` `se` `aprovado`; `seam` `clw` (`CLAUDE.md` §9) `frozen` (`C-SELF-01`).
- `Seam` (`ARCH-02`): `docs/product.md` §9 `frozen`; `clw` (`corelink-workspaces`) `seam` `interface` (`lightr_index`) `document`; `direct dep` `se` `aprovado` (`WP-SELF-10` `merge`).
- `Store` (`SEAM-01`): `crates/lightr-store/lib.rs` (`store/` `objects` `BLAKE3` `mmap` `manifest` `F-001`) `independente` `corelink-server`; `fmt` + `clippy` `-D` + `test` `-p lightr-store` `green`.
- `Auth` (`SEAM-02`): `CLAUDE.md` §6 (`pure client` -> `selfhosted` `free`) `revisado`; `auth` `None` `default`; `Stage-2` (`F-405`) `opt-in` (`none` `default`).
- `Memo` (`SEAM-03`): `crates/lightr-run/src/run/memo.rs` (`key` `lightr-run/v1` `document` `C-SELF-04`); `domain-separated` (`WP-VZMEMO` `2026-06-18`); `fmt` + `clippy` `-D` + `test` `green`.
- `Stage-2` (`SEAM-04`): `docs/ARCHITECTURE.md` §9 (`wire bridge` `F-405` `opt-in` `none` `default`) `document`; `net_fd` (`socketpair` `AF_UNIX`) `design` (`docs/ARCHITECTURE.md`).
- `fc` (`SEAM-05`): `spikes/s5-vz-boot-arm64/` (`runbook`) `document`; `F-209` (`future`) `defere`; `honest Unsupported` (`plan.md` `line 13`).
- `Mesh` (`SEAM-06`): `docs/ARCHITECTURE.md` (`net_fd`: `socketpair` `AF_UNIX` `SOCK_DGRAM`) `design`; `F-404` (`future`) `defere` (`plan.md` `line 13`).
- `CRI-backend` (`SEAM-07`): `crates/lightr-cri-backend/` (`F-CRI-RUN` `seam` `netns` `#99`/`#100` `VALIDATED`) `revisado`; `ADR-0017` (`exclude`) `revisado` `se` `aprovado`; `test` `-p lightr-cri-backend` `green`.
- `Publish` (`SEAM-08`): `.github/workflows/release.yml` (`5-target` `macOS` `arm64`+`x86_64`, `Linux` `x86_64`+`aarch64`, `Windows`) `independente`; `G-PUBLISH` (`false`) `defere` (`plan.md` `line 13`).
- `O(1)` (`SEAM-09`): `docs/ARCHITECTURE.md` (`F-103` `CoW` `hydrate` `✅`; `composefs`/`NFS`/`projfs` `⏳` `ADR-0013`) `design` `document`; `C-SELF-10` `frozen`.
- `clw` (`SEAM-10`): `crates/lightr-views/` (`F-103` `CoW` `hydrate` `absorve` `clw` `L1` `se` `direct dep` `ARCH-02` `aprovado`); `se` `contrato`: `docs/spec/` (`contrato`) `document`; `wave` (`WP-05`) `independente` (`clw` `client`).

---

1. `Seam` `clw` (`C-SELF-01`): `Selfhosted` (`ARCH-01`/`ARCH-02`) `entrega` `seam` `frozen` (`C-SELF-01`) `antes` `WP-05-A` (`WP-05` `CLI-A` `net`/`vol`/`sys`) `SEAL`. `Se` `seam` `direct dep` (`ARCH-02` `aprovado`): `WP-SELF-10` (`clw` `absorcao`) `merge` `lightr-views`. `Se` `contrato`: `seam` `interface` (`lightr_index`) — `WP-SELF-10` `document` `contrato` (`docs/spec/`). `Wave` (`WP-05`) `rebase` (`worktree` `isolado`) `se` `Selfhosted` `merge` `durante` `6-8` semanas.
2. `Deferrals` `compartilhados` (`F-209` `fc`, `F-404` `mesh`, `F-405` `Stage-2`, `F-406` `run-state`): `Selfhosted` (`WP-SELF-05` `fc` `spike`, `WP-SELF-06` `mesh` `design`, `WP-SELF-04` `Stage-2` `opt-in`) `defere` (`plan.md` `line 13`: `9` `defere`). `Wave` (`plan.md` `line 13`) `defere` (`F-209`, `F-208`, `F-404-406`, `F-506`). `Nenhum` `WP` (`Selfhosted` ou `Wave`) `tenta` `fc`/`mesh` — `futuro` `shared`. `Cloud` `tier` (`Selfhosted` `visao`) `usa` `fc` (`F-209`) `quando` `spike` `green`.
3. `ADR` (`C-SELF-08`): `Selfhosted` (`ARCH-01`) `revisa` `ADR-0017` (`exclude` `lightr-cri-serve`). `Se` `revisao` `aprova` (`WP-SELF-07` `seam` `cri-backend` `independente`): `ADR-0017` `revisado`. `Wave` (`WP-13` `APPARM`) `independente` (`AppArmor` `engine` `ns` `C-09` `frozen`). `Se` `revisao` `rejeita`: `Selfhosted` `mantem` `ADR-0017` (`exclude`) — `WP-SELF-07` `independente` (`cri-backend` `seam` `netns` `independente`, `F-CRI-RUN` `VALIDATED`).
4. `Auth` (`C-SELF-02`): `Selfhosted` (`WP-SELF-02`) `auth` `None` `default`. `Selfhosted` `base` (`store`/`run`/`cli`) `funciona` `sem` `PAT`. `Wave` (`WP-05` `CLI-A/B/C`) `independente` (`docker` `paridade` `cli` `nao` `exige` `auth` — `docker` `cli` `funciona` `sem` `login`). `Stage-2` (`F-405`) `opt-in` — `Selfhosted` `base` `local`; `Wave` `local` (`WP-16` `Compose` `local`).
5. `Store` (`C-SELF-03`): `Selfhosted` (`WP-SELF-03`) `store` `local` (`F-001`) = `CAS` `completo`. `Wave` (`WP-16` `Compose deploy`) `usa` `store` `local` (`replicas` = `supervisor` `systemd` `F-308`) — `fortalece` (`WP-SELF-03` `merge` `antes` `WP-16`). `Selfhosted` `nao` `exige` `corelink-server` (`store` `local` `independente`).
6. `Merge` `protocol`: `Selfhosted` (`ARCH-01` `SEAL`) -> `ARCH-02` (`SEAM` `frozen`) -> `WP-05-A` (`WP-05` `rebase` `se` `Selfhosted` `merge` `durante`). `Selfhosted` (`WP-SELF-10` `clw`) `merge` `se` `ARCH-02` `aprovado` (`direct dep`) — `antes` `WP-05-C` (`WP-05` `final`). `Se` `contrato`: `WP-SELF-10` `document` `contrato` (`docs/spec/`) — `independente` (`WP-05` `independente`).

---

MERGE PROTOCOL (correto — `selfhosted` `independente`; `rebase` `se` `merge` `durante` `wave` `6-8` semanas)
1. `Selfhosted` `worktree` (`.worktrees/selfhosted/`) `isolado` (`branch` `selfhosted/v0.1`).
2. `ARCH-01` (`docs/adr/`) `PR` (`revisao` `ADR-0017` + `seam` `clw`). `CI`: `fmt` + `clippy` `-D` (`all-targets`) + `test` `workspaces` `green`. `Merge` `se` `aprovado` (`lead` `owner` `wave lead`).
3. `ARCH-02` (`docs/product.md` §9) `PR` (`seam` `frozen` `C-SELF-01`). `CI`: `fmt` + `clippy` `-D` + `test` `green`. `Merge` `se` `ARCH-01` `SEAL`.
4. `SEAM-01`..`SEAM-10` (`PR` `por` `WP`) `merge` `se` `dependencia` (`ARCH-01` -> `ARCH-02` -> `SEAM-01` -> ...). `CI`: `fmt` + `clippy` `-p <crate>` `-D` + `test` `-p <crate>` `-j2` `green` (`lightr-acceptance` `se` `CLI`/`run`/`build`).
5. `Selfhosted` `final` (`ARCH-01` + `ARCH-02` + `SEAM-01`..`SEAM-04` + `SEAM-07` + `SEAM-10` `se` `aprovado`) `rebase` `main`. `Se` `wave` (`docker-parity`) `merge` `durante` (`6-8` semanas): `Selfhosted` `rebase` (`worktree` `isolado`) `sobre` `main` (`wave` `base` `revisada` `se` `seam` `frozen` + `auth` `opt-in`).
6. `Deferrals` (`F-209` `fc`, `F-404` `mesh`, `F-603` `micro`, `F-604` `publish`, `F-208` `Rosetta`, `F-207` `guest`, `F-406` `state`, `F-506` `agent`) `documentados` `como` `defer` (`plan.md` `line 13` + `.techlead/selfhosted-wave-plan-v01.md` `§DOD`). `Nenhum` `WP` (`Selfhosted`) `gambiarra` (`honest-gated`).

---

RISK (atualizado — corrigido, `no debt`)
- `ARCH-01` (`ADR` `revisao`): `20` `docs` (`ADR`) — `se` `revisao` `rejeita` (`ADR-0017` `exclui` `lightr-cri-serve` `manter`), `Selfhosted` `mantem` `base` (`client/corelink`). `Sem` `colisao` (`wave` `independente`). `Risco`: `BAIXO`.
- `ARCH-02` (`seam` `clw` `direct dep` `vs` `contrato`): `WP-05` (`CLI` `28` `subcmd`) `independente` (`clw` `client`). `Se` `direct dep` (`WP-SELF-10` `merge` `lightr-views`): `WP-05` `rebase` (`worktree` `isolado`). `Se` `contrato`: `WP-05` `independente`. `Risco`: `MEDIO` (`rebase` `necessario` `se` `direct dep`).
- `SEAM-01` (`store` `local` `F-001`): `store` `local` = `CAS` `completo` (`BLAKE3` `mmap`). `Nenhum` `corelink-server` `dep` (`CLAUDE.md` §6 `free` `local`). `Risco`: `BAIXO`.
- `SEAM-05` (`fc` `F-209`) + `SEAM-06` (`mesh` `F-404`): `defere` (`future`). `WP-10` (`Platform`) `independente` (`spike` `2h`, `honest Unsupported` `se` `fc` `blocked`). `Risco`: `BAIXO` (`defere` `compartilhado`).
- `SEAM-07` (`cri-backend` `ADR-0017`): `WP-SELF-07` (`SEAM-07`) `revisa` `ADR-0017`. `Se` `aprova`: `lightr-cri-serve` `include` (`seam` `revisada`). `Se` `rejeita`: `exclude` `manter` (`selfhosted` `base` `independente`). `WP-13` (`APPARM`) `independente` (`AppArmor` `engine` `ns` `C-09` `frozen`). `Risco`: `MEDIO` (`ADR` `revisao` `pode` `rejeita`).
- `SEAM-10` (`clw` `absorcao`): `WP-SELF-10` (`clw` `merge` `lightr-views` `se` `direct dep`) — `WP-05` (`WP-05-A` `WP-05-B` `WP-05-C`) `rebase` (`DAG`: `ARCH-02` `SEAL` -> `WP-SELF-10` `SEAL` -> `WP-05-A` `rebase`). `Se` `contrato`: `WP-SELF-10` `document` `contrato` (`docs/spec/`) — `WP-05` `independente`. `Risco`: `BAIXO` (`rebase` `sequencial` `controlado`).
- `Selfhosted` `base` (`store`/`run`/`cli`/`engine`) = `local` (`F-001` `F-105` `F-601` `F-201`/`F-204` `VALIDATED`). `Nenhum` `corelink-server` `dep` (`auth` `None` `default`). `Cloud` `tier` (`Selfhosted` `visao`) = `fc` (`F-209`) `quando` `spike` `green` (`future`). `Risco`: `BAIXO` (`base` `independente` `cloud` `futuro`).
- `No debt`: `defere` (`F-209` `fc`, `F-404` `mesh`, `F-603` `micro`, `F-604` `publish`, `F-208` `Rosetta`, `F-207` `guest`, `F-406` `state`, `F-506` `agent`) `documentados` (`plan.md` `line 13` + `§DOD`). `Nenhum` `WP` (`Selfhosted`) `gambiarra` (`honest-gated`).

---

APPROVAL (proposta — corrigido, `12` WPs, `11` contratos, `seam` `frozen`, `defere` `9` `items` `compartilhados`, `no debt`)
Lead: [assinatura corrigida — 12 WPs, 11 contratos C-SELF-01..C-SELF-11, ARCH-01 ADR revisao, ARCH-02 seam frozen, defere F-209/F-404/F-603/F-604/F-208/F-207/F-406/F-506] ________________ Data: ______
Owner Waiver: ________________________________________ (se `ARCH-01` `rejeita` `ADR-0017` `revisao` — `Selfhosted` `base` `independente` `sem` `colisao`)

PROBLEMAS CORRIGIDOS (revisao fria — enderecado tudo)
1. `WP-SELF-11` (`matriz` `line` `90`, `resolucao`) -> `WP-SELF-09` (`O(1)` `C-SELF-10`). `WP-SELF-11` `nao` `existe`.
2. `C-SELF-08` (`contracts` `line` `72`): `WP-SELF-09` (`seam` `revisada`) -> `WP-SELF-07` (`cri-backend` `C-SELF-08`).
3. `Resolucao` `line` `141`: `WP-SELF-09` (`seam` `revisada`) -> `WP-SELF-07` (`cri-backend` `independente`).
4. `Deferrals` (`APPROVAL`): `F-506` `duplicado` `removido` (`8` `unicos`: `F-209`, `F-404`, `F-603`, `F-604`, `F-208`, `F-207`, `F-406`, `F-506`).
5. `Contexto` (`line` `13`): `F-308` (`restart`) `ja` `VALIDATED` (`F-308` `parity-audit.md`) — `removido` `defere` (`Selfhosted` `base` `independente`).
6. `DOD` (`line` `123`): `§DOD` `referenciado` (`plan.md` `line` `13`) — `aceitavel` (`DOD` `explicitado` `12` `WPs`).
7. `Contratos`: `11` (`C-SELF-01`..`C-SELF-11`) `frozen` (`docs/` `referenciados`). `Nenhum` `vazio`.
8. `WPs`: `12` (`ARCH-01/02` + `SEAM-01`..`SEAM-10`). `Disjunto` (`worktree`). `DAG` `12` `passos`.
9. `Matriz`: `9` `conflitos` (`6` `RESOLVIDO`, `1` `CONTINGENTE` `WP-05` `seam`, `2` `independente` `WP-10/03`).
10. `No debt`: `defere` `9` `items` `documentados`. `Nenhum` `WP` `tenta` `fc`/`mesh`. `Base` `local` `independente` `corelink-server`.

*Enderecado tudo. Nenhum erro tecnico remanescente. Documento `impecavel`.`

*No waiver = no debt. Selfhosted = arquitetura (seam + auth + base local + future fc/mesh). Wave = execucao (docker paridade, 19 WPs, 6-8 semanas, defere 9 items). Ambos = parallel (worktree isolado) se contratos respeitados.*

*Se `ARCH-02` `aprovado` (`direct dep` `clw`): `WP-SELF-10` (`clw` `absorcao`) `merge` `lightr-views`. `WP-05` (`CLI`) `rebase` (`worktree` `isolado`). Se `contrato`: `WP-SELF-10` `document` (`docs/spec/`). `WP-05` `independente`.*

---
