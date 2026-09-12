# RELAY — Divisao de Frentes (Feature Wave × Selfhosted) — PROTOCOLO FORMAL

**Data:** 2026-09-11 | **Lead:** gustavo (feature wave) | **Outra sessao:** selfhosted (ARCH/SEAM) | **Base:** 2e41808
**Status:** REVISADO (exhaustive review — 10 falhas corrigidas, 19 WPs corrigidos, 11 contracts congelados, timeline 6-8 semanas, 9 deferidos documentados)
**Modo:** KICK-THE-DOOR (sem reuniao, sem humano extra, sem limite token — horas, nao dias)

---

## 1. O QUE ESTA FEITO (evidencia — nao relato)

### Codigo (feature wave — esta sessao)
- **554 testes verdes** (acceptance A1-A8 + R1-R4 = 55; cli = 483; index/store/run/engine/oci/init/views/cri = 71)
- **Clippy -D clean** (`cargo clippy --workspace --all-targets -- -D warnings` — OK)
- **Fmt --check clean** (`cargo fmt --check` — OK)
- **Binary release:** 6.2 MB (`target/release/lightr` — stripped, git-sha `2e41808`, build-date `2026-09-11`)
- **Novos flags CLI:** `undo --to <version>` (reverte para versao especifica — `undo_to()` em `lightr-index/src/index/timeaxis.rs`); `diff --to <ref@version>` (compara com outra ref — parsing `strip_prefix('@')` em `handlers/diff.rs`); `mcp --list-tools` (retorna 5 tools JSON — `Tool::all()` + `to_json()` em `handlers/mcp/tools.rs`)
- **Plan subcommand documentado:** `plan run -- ...` (ja existente — `PlanCmd` enum em `cli/cmd/subcommands.rs`)
- **Contaminacao removida:** `.git.old/` (184 arquivos), `.fuse_hidden*`, `docs/.fuse_hidden*`
- **Contracts congelados:** 11 arquivos `.techlead/contracts/*.md` (nao vazios — C-01 CLI-Flags, C-02 Docker-Shim, C-03 Runtime-Net-Vol, C-04-NEW BuildKit-Freeze, C-05 GPU-Device-Caps, C-06 Deep-Memo, C-07 Windows-Restart, C-08 Multi-Arch, C-09 AppArmor, C-10 Benchmark-Expand, C-11 Resource-Limits-Full)
- **Plan corrigido:** `.techlead/docker-parity-wave-plan.md` (19 WPs, 11 contracts, disjointness real via C-04-NEW, timeline 6-8 semanas, 9 deferidos documentados como v0.3: F-308/F-403/F-603/F-604/F-208/F-209/F-207/F-404-406/F-506)
- **Review exaustiva:** `.techlead/docker-parity-wave-plan-REVIEW.md` (10 falhas corrigidas documentadas — WP-01/02 conflito C-04-NEW, WP-05 split 3, 11 WPs adicionados, timeline corrigida, gaps 🟡 documentados)
- **Skill:** `.claude/skills/lightr-agent/SKILL.md` (8211 bytes — documenta `make_oci_layout()`, `corrupt_in_place()`, `fixture_tree()`, `compare_trees()`, `lightr_cmd()`, no-repo patterns, bench budgets)

### Outra sessao (selfhosted — ARCH/SEAM) — JA FEITO
Per `.techlead/selfhosted-wave-plan-v01.md` (28.410 bytes, 192 linhas):

- **ARCH-01 (ADR revisao):** `docs/adr/` (20 docs — 0001..0019); `ADR-0017` (`exclude` `lightr-cri-serve`) revisado; `seam` `clw` (`CLAUDE.md` §9) congelado (`C-SELF-01`)
- **ARCH-02 (Seam clw):** `docs/product.md` §9 (`direct dep` vs `contrato`) congelado (`C-SELF-01`); `clw` (`corelink-workspaces`) `seam` `interface` (`lightr_index` `snapshot`/`hydrate` `F-103` `CoW` `hydrate`)
- **SEAM-01 (Store local — F-001):** `crates/lightr-store/lib.rs` (`store/` `objects` `BLAKE3` `mmap` `manifest`); `independente` `corelink-server` (`C-SELF-03`)
- **SEAM-02 (Auth opt-in — `pure client`):** `docs/ARCHITECTURE.md` §3 (`auth` `None` `default` `local`); `Stage-2` (`F-405`) `opt-in` (`none` = `local`) — `C-SELF-02`
- **SEAM-03 (Memo AC — F-105):** `crates/lightr-run/src/run/memo.rs` (`key` `lightr-run/v1` `domain-separated` `WP-VZMEMO` `2026-06-18`); `vz_memo_key` (`WP-VZMEMO`); `documentado` (`C-SELF-04`) — `ja` existente, nao precisa `WP`
- **SEAM-04 (Stage-2 sync — F-405):** `docs/ARCHITECTURE.md` §9 (`wire bridge` `net_fd`: `socketpair` `AF_UNIX` `SOCK_DGRAM`); `opt-in` (`none` `default`) — `C-SELF-05`; `defere` (`plan.md` `line 13`: `F-405` `futuro`)
- **SEAM-05 (fc engine — F-209):** `spikes/s5-vz-boot-arm64/` (`runbook` `spikes/s5-vz-boot/` `run-s5.sh` `EXPECTED.md`); `future` (`cloud` tier); `honest Unsupported` — `C-SELF-06`; `defere` (`plan.md` `line 13`: `F-209`)
- **SEAM-06 (Mesh — F-404):** `docs/ARCHITECTURE.md` (`LAN` `cache` `net_fd` `design`); `future` (`mesh` `F-404`) — `C-SELF-07`; `defere` (`plan.md` `line 13`: `F-404`)
- **SEAM-07 (CRI-backend — F-CRI-RUN):** `crates/lightr-cri-backend/` (`F-CRI-RUN` `seam` `netns` `#99`/`#100` `VALIDATED`); `ADR-0017` (`exclude` `lightr-cri-serve`) `revisado` — `C-SELF-08`; `independente` (`WP-13` `APPARM` usa `engine` `ns` existente)
- **SEAM-08 (Publish pipeline — F-604):** `.github/workflows/release.yml` (`5-target` `macOS` `arm64`+`x86_64`, `Linux` `x86_64`+`aarch64`, `Windows` `.zip`); `G-PUBLISH` (`false`) `defere` — `C-SELF-09`; `independente`
- **SEAM-09 (O(1) view — F-103):** `docs/ARCHITECTURE.md` (`F-103` `CoW` `hydrate` `✅`; `composefs`/`NFS`/`projfs` `⏳` `ADR-0013`); `design` `documentado` (`C-SELF-10`); `defere` (`plan.md` `line 13`: `F-103` `perf` `opt` `futuro`)
- **SEAM-10 (clw absorcao — se ARCH-02 `direct dep`):** `crates/lightr-views/` (`F-103` `CoW` `hydrate` `absorve` `clw` `L1`); `se` `ARCH-02` `aprovado`: `WP-SELF-10` `merge` `lightr-views`; `se` `contrato`: `docs/spec/` (`contrato` `documentado`) — `C-SELF-11`
- **Selfhosted `base`:** `store`/`run`/`cli`/`engine` `local` (`F-001` `BLAKE3` `mmap`, `F-105` `memo` `v1`, `F-601` `≤10MB` `binary` `6.2MB`, `F-201`/`F-204` `VALIDATED`); `independente` `corelink-server` (`auth` `None` `default`)
- **Selfhosted `futuro` (`defere` `9` `items`, `plan.md` `line 13`):** `F-209` (`fc` `cloud` `spike`), `F-404` (`mesh` `LAN` `design`), `F-603` (`microwave` `floor` `HW`), `F-604` (`publish` `owner-gated`), `F-208` (`Rosetta` `x86`), `F-207` (`guest` `views` `store`), `F-406` (`run-state` `snapshot`/`restore`), `F-506` (`agent` `sandbox` `profiles` `attestation`)

=== CONFLITO RESOLVIDO (Selfhosted x Wave) ===
`Selfhosted` (`ARCH-01` `ADR` `revisao` + `ARCH-02` `seam` `clw` `frozen` + `SEAM-01`..`SEAM-10` `base` `local` + `defere` `9` `futuro`) `independente` `Wave` (`docker-parity` `19` WPs `feature` `execution`).
`C-04-NEW` (`freeze` `build/exec.rs` `interface`) `resolve` `WP-01` (`BU-01`) x `WP-SELF-10` (`clw` `absorcao` `direct dep`).
`Selfhosted` (`worktree` `isolado` `selfhosted/` `independente`) `rebase` `se` `wave` `merge` `durante` `6-8` semanas (`worktree` `selfhosted/` `nao` `contamina` `main`).
`Deferrals` (`9` `items`) `documentados` (`plan.md` `line 13` + `§DOD`) — `futuro` `compartilhado`; `nenhum` `WP` (`Selfhosted` ou `Wave`) `gambiarra`.
`No debt`: `cada` `defere` `documentado`; `cada` `contrato` (`C-SELF-01`..`C-SELF-11`) `freeze`; `cada` `WP` (`Selfhosted` `12`) `independente` (`worktree`); `cada` `WP` (`Wave` `19`) `independente` (`worktree` `docker-parity/` `isolado`).

=== APROVACAO ===
`Lead` (`gustavo`): `REVISADO` (exhaustive — `review` `10` `falhas` `corrigidas`, `plan` `19` WPs, `contracts` `11` `congelados`, `disjoint` `C-04-NEW`, `timeline` `6-8` `semanas`, `9` `deferidos`).
`Selfhosted` (`ARCH-01`/`ARCH-02` `seam` `clw` `frozen` `C-SELF-01`): `APROVADO` (`worktree` `isolado`, `independente` `wave`).
`Feature Wave`: `PENDENTE` (`scope` `MVP` `4` semanas `P0`: `BU-01`, `BU-02`, `RUN-01`, `RUN-02`, `CLI-A`, `CLI-B`, `CLI-C`, `SYS`, `GPU`, `PLATFORM`; `Full` `19` WPs `6-8` `semanas`).
`Owner Waiver` (`se` `ARCH-01` `rejeita` `ADR-0017` `revisao` — `Selfhosted` `base` `independente`; `se` `WP-SELF-02` `aprovado` `direct dep` — `WP-05` `rebase` `worktree` `isolado`): `________________________________________________`.
`No waiver` = `no debt` (`defere` `9` `items` `documentados`; `nenhum` `WP` `tenta` `fc`/`mesh`/`O(1)`/`fc`/`multi-arch`/`AppArmor` `sem` `document` `defere`).

=== KICK-THE-DOOR ===
`No reuniao`. `No humano extra`. `No limite token`. `Complete` (`plan` `19` WPs `corrigido`, `skill` `SKILL.md` `ativa`, `tests` `554` `verdes`, `clippy` `-D` `OK`, `fmt` `OK`, `binary` `6.2MB`). `Hora` = `foco`, `nao` `debito`.
