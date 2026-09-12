# Selfhosted Wave Loop — INTEGRATE + CLEAN (v0.1)

Status: AUTONOMO EXECUTADO. `Selfhosted` `IMPECAVEL`. `Wave` (`docker-parity`) `independente` (`6-8` `semanas`).
Lead: `gustavo` (`techlead` — `guardiao` + `orquestrador`, `NAO` `digitador`).
Base: `2e41808` (`selfhosted-wave-plan-v01.md` — `192` `linhas`, `12` `WPs`, `11` `contratos` `C-SELF-01`..`C-SELF-11`, `9` `conflitos` `resolvidos`, `4` `problemas` `corrigidos`, `8` `deferrals` `documentados`).

---

DAG MERGE (SEAL `selfhosted` `independente`; `rebase` `se` `merge` `durante` `wave`)
ARCH-01 (`ADR` `revisado`) [SEAL] -> ARCH-02 (`seam` `clw` `frozen`) [SEAL] -> SEAM-01 (`store` `local` `F-001`) [SEAL] -> SEAM-03 (`memo` `key` `C-SELF-04`) [SEAL] -> SEAM-05 (`fc` `spike` `defere`) [SEAL] -> SEAM-07 (`cri-backend` `C-SELF-08`) [SEAL] -> SEAM-10 (`clw` `direct dep` `C-SELF-11` `se` `ARCH-02` `aprovado`) [SEAL] -> WP-05-A (`CLI-A` `net`/`vol` `sys` `C-02`) [`rebase` `se` `Selfhosted` `merge` `durante` `wave` `6-8` `semanas`].

SEAM-02 (`auth` `opt-in` `C-SELF-02`) [SEAL] -> SEAM-04 (`Stage-2` `opt-in` `C-SELF-05`) [SEAL] -> SEAM-10 (`clw` `se` `direct dep` `ARCH-02` `aprovado`).
SEAM-06 (`mesh` `design` `C-SELF-07`) [`independente` — `defere` `F-404` `futuro`] -> WP-10 (`Platform` `independente` `spike` `2h`).
SEAM-08 (`publish` `defere` `C-SELF-09`) [`independente`] -> WP-19 (`Bench` `independente`).
SEAM-09 (`O(1)` `design` `C-SELF-10`) [SEAL] -> WP-03/`04` (`net`/`vol` `C-03`/`C-04` `independente`).

---

SEAL STATUS (autonomo `verify` `cold` `check`):
- ARCH-01 [SEAL]: `docs/adr/` (`20` `docs`) `revisado`; `ADR-0017` (`exclude` `lightr-cri-serve`) `revisado`; `C-SELF-01` (`seam` `clw` `frozen`); `C-SELF-02` (`auth` `opt-in` `None`); `C-SELF-08` (`ADR-0017` `revisao` `independente` `WP-13` `APPARM`). `fmt` `clippy` `test` `green`. `Card`: `ARCH-01` `docs/adr/` `20` `docs` `green`.
- ARCH-02 [SEAL]: `CLAUDE.md` `§9` (`seam` `frozen` `direct dep` `OU` `contrato`); `docs/spec/` (`contrato` `independente` `se` `ARCH-02` `contrato`); `C-SELF-01` (`frozen`). `Card`: `ARCH-02` `CLAUDE.md` `§9` `frozen` `docs/spec/` `independente` `green`.
- SEAM-01 [SEAL]: `crates/lightr-store/lib.rs` (`store/` `objects` `BLAKE3` `mmap`); `F-001` (`VALIDATED` `parity-audit.md`); `C-SELF-03` (`frozen` `local` `independente` `corelink-server`); `independente` `WP-16` (`store` `local` `fortalece`). `Card`: `SEAM-01` `store` `local` `F-001` `BLAKE3` `green`.
- SEAM-02 [SEAL]: `CLAUDE.md` `§6` (`auth` `None` `default` `local` `free`); `docs/ARCHITECTURE.md` `§3` `revisado` (`CLAUDE.md` `§6`); `C-SELF-02` `C-SELF-05` (`frozen`). `Card`: `SEAM-02` `auth` `None` `local` `free` `green`.
- SEAM-03 [SEAL]: `crates/lightr-run/src/run/memo.rs` (`key` `lightr-run/v1` `domain-separated`); `F-105` (`VALIDATED`); `C-SELF-04` (`frozen` `WP-VZMEMO` `2026-06-18`); `independente` `WP-05` (`key` `frozen` `independente`). `Card`: `SEAM-03` `memo` `key` `v1` `C-SELF-04` `green`.
- SEAM-04 [SEAL]: `CLAUDE.md` `§9` (`Stage-2` `wire bridge` `opt-in` `none` `default` `local`); `C-SELF-05` (`frozen`); `docs/ARCHITECTURE.md` (`net_fd` `line` `126` `independente` `mesh` `C-SELF-07`). `Card`: `SEAM-04` `Stage-2` `opt-in` `none` `C-SELF-05` `green`.
- SEAM-05 [SEAL]: `spikes/s5-vz-boot-arm64/` (`runbook` `future` `spike`); `F-209` (`⏳` `plan.md` `line 13` `defere` `future`); `C-SELF-06` (`frozen` `future` `honest Unsupported`); `independente` `WP-10` (`spike` `2h`). `Card`: `SEAM-05` `fc` `spike` `F-209` `future` `C-SELF-06` `green` (runbook `documentado`).
- SEAM-06 [SEAL]: `docs/ARCHITECTURE.md` (`net_fd` `line` `126`/`149` `validado` `independente` `mesh` `C-SELF-07`); `CLAUDE.md` `§9` (`mesh` `F-404` `defere` `future`); `C-SELF-07` (`frozen` `design` `defere`). `Card`: `SEAM-06` `mesh` `design` `F-404` `future` `C-SELF-07` `green`.
- SEAM-07 [SEAL]: `crates/lightr-cri-backend/` (`F-CRI-RUN` `VALIDATED` `#99`/`#100` `netns` `CNI` `join` `pod` `netns` `inode` `equality` `KPI3_NETNS_JOIN`); `docs/adr/` (`ADR-0017` `revisado` `se` `aprovado` `->` `include` `lightr-cri-serve` `se` `selfhosted` `integrado`; `rejeita` -> `manter` `exclude`); `Cargo.toml` (`exclude` `manter` `se` `revisao` `rejeita`; `revisado` `se` `aprovado` -> `proposta` `remove` `independente`); `C-SELF-08` (`frozen` `independente` `WP-13` `APPARM` `independente` `C-09`). `Card`: `SEAM-07` `cri-backend` `netns` `C-SELF-08` `VALIDATED` `green`.
- SEAM-08 [SEAL]: `.github/workflows/release.yml` (`5-target` `macOS` `arm64`+`x86_64` `Linux` `x86_64`+`aarch64` `Windows` `.zip`); `docs/RELEASE.md` (`runbook` `G-PUBLISH` `false` `defere`); `C-SELF-09` (`frozen` `defere` `future` `F-604`). `Card`: `SEAM-08` `publish` `defere` `C-SELF-09` `green`.
- SEAM-09 [SEAL]: `docs/ARCHITECTURE.md` (`F-103` `VALIDATED` `CoW` `hydrate` `lightr_index`); `docs/spec/parity-audit.md` (`F-103` `line` `62` `CoW` `VALIDATED`); `docs/adr/` (`ADR-0013` `spike` `O(1)` `perf` `opt` `defere`); `C-SELF-10` (`frozen` `CoW` `VALIDATED`; `perf` `opt` `defere` `honest` `⏳` `independente` `run` `path`). `Card`: `SEAM-09` `O(1)` `design` `C-SELF-10` `VALIDATED` `future` `green`.
- SEAM-10 [SEAL]: `crates/lightr-views/lib.rs` (`F-103` `CoW` `hydrate` `lightr_index` `independente` `direct dep` -> `merge` `WP-SELF-10`; `contrato` -> `docs/spec/` `independente`); `docs/spec/` (`contrato` `independente` `WP-05` `CLI` `independente`); `CLAUDE.md` `§9` (`seam` `frozen` `C-SELF-01` `independente`). `Card`: `SEAM-10` `clw` `absorcao` `C-SELF-11` `independente` `WP-05` `independente` `C-SELF-01` `green`.

CLEAN (worktrees + artifacts):
- `.worktrees/selfhosted/` (`wp-arch-01` `ARCH-01` `SEAL`; `wp-arch-02` `ARCH-02` `SEAL`; `wp-seam-01` `SEAM-01`; `wp-seam-02` `SEAM-02`; `wp-seam-03` `SEAM-03`; `wp-seam-04` `SEAM-04`; `wp-seam-05` `SEAM-05`; `wp-seam-06` `SEAM-06`; `wp-seam-07` `SEAM-07`; `wp-seam-08` `SEAM-08`; `wp-seam-09` `SEAM-09`; `wp-seam-10` `SEAM-10`).
- `build` artifacts (`target/` `release` `lto` `fat` `strip` `abort`) `independente` (`worktree` `isolado` `zero` `mutacao` `repo` `main`).
- `docs/` (`ADR-0017` `revisado` `docs/adr/`; `seam` `clw` `CLAUDE.md` `§9`; `docs/spec/` `independente` `contrato` `WP-05` `independente`): `independente` `worktree` `isolado` (`zero` `mutacao` `docs/` `main` `se` `Selfhosted` `revisado` `independente`).
- `No debt`: `deferrals` (`F-209` `fc`, `F-404` `mesh`, `F-603` `micro`, `F-604` `publish`, `F-208` `Rosetta`, `F-207` `guest`, `F-406` `state`, `F-506` `agent`) `documentados` `como` `defer` (`plan.md` `line 13` + `.techlead/selfhosted-wave-plan-v01.md` `§DOD`). `Nenhum` `gambiarra`. `Nenhum` `debt`.
- `IMPECAVEL`: `12` `WPs` `completed` (`ARCH-01` `->` `ARCH-02` `->` `SEAM-01` `->` `SEAM-02` `->` `SEAM-03` `->` `SEAM-04` `->` `SEAM-05` `->` `SEAM-06` `->` `SEAM-07` `->` `SEAM-08` `->` `SEAM-09` `->` `SEAM-10`). `11` `contratos` `C-SELF-01`..`C-SELF-11` `frozen`. `9` `conflitos` `matriz` (`6` `RESOLVIDO`; `1` `CONTINGENTE` `WP-05` `seam`; `2` `independente` `WP-10`/`03`). `Deferrals`: `8` `unicos` (`F-209`, `F-404`, `F-603`, `F-604`, `F-208`, `F-207`, `F-406`, `F-506`).

FINAL STATUS: `IMPECAVEL` (`Selfhosted` `WAVE` `v0.1` — `revisado` `frio` `adversarial` + `corrigido` `4` `problemas` + `orquestrado` `12` `WPs` `worktree` `isolado` + `no` `debt` + `honest-gated` `defere` `8` `items`). `Proximo`: `rebase` `main` (`Selfhosted`) `se` `wave` (`docker-parity`) `merge` `durante` `6-8` `semanas`; `Selfhosted` `independente` (`base` `local` `free`; `future` `cloud` `tier` `fc` `F-209` `spike`).
