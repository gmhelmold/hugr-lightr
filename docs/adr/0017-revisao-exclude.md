# ADR-0017 — Revisão proposta: `exclude` `lightr-cri-serve`

- **Status:** Proposto (revisão de `Cargo.toml` `exclude` sob `C-SELF-08`)
- **Data:** 2026-09-11
- **Contratos congelados:** `C-SELF-08` (`ADR-0017` revisado); `C-SELF-01` (`seam` `clw` independente); `C-SELF-07` (`mesh` `defere` independente).
- **Independente de:** `WP-13` (`APPARM` `C-09` frozen); `WP-05` (`CLI` `C-02` frozen); `WP-SELF-09` (`O(1)` `C-SELF-10` `independente` defere).

## Proposta

Revisar o `exclude = ["crates/lightr-cri-serve"]` no `Cargo.toml` do workspace (`docs/ADR-0017` `decision 5`, firewall `tonic/prost/gRPC`).

**Se `aprovado`:** remover `exclude` → `lightr-cri-serve` incluso no workspace; `selfhosted` `base` (`store`/`run`/`cli`/`engine`) `funciona` `net_fd: None` (zero regressão).

**Se `rejeita`:** manter `exclude`; `selfhosted` `base` `independente` `sem` `colisao` (`WP-SELF-07` `seam` `independente`); `Cargo.toml` `linha 22` inalterado.

## Contexto (`selfhosted` `v0.1`)

- `lightr-cri-backend` (`F-CRI-RUN`) `seam` `netns` `VALIDATED` (`#99`/`#100`): `netns_lifecycle` (`netns` `join` `pod` `inode` `equality` `KPI3_NETNS_JOIN`); `exec` `__ns-exec` (`KPI3_EXEC` `PASS` `mnt-ns` `inode` `equal` `PID-1`); `dev` `/dev/pts` (`KPI3_DEVPTS` `PASS` `newinstance` `ptmxmode` `0666`).
- `net_fd` (`ExecSpec.net_fd`) `validado` `independente` `mesh` (`C-SELF-07`): `mesh` (`F-404`) `LAN` `cache` `defere`; `wire bridge` (`F-405`) `nao` `exige` `mesh`. `selfhosted` `base` `funciona` `net_fd: None`.
- `pure client` (`§6`): `selfhosted` `base` = `local`; `free`; `auth` `None` (`C-SELF-02`). `Stage-2` sync `CoreLink` = `futuro` (`lightr-wire` `planned` `async`; `ADR-0011`).

## Decisão (revisada — proposta, não ratificada)

1. **Manter `exclude`** (`rejeita` a proposta de remoção nesta revisão): `WP-SELF-07` (`cri-backend`) `independente`; `WP-13` (`APPARM`) `C-09` `frozen`; nenhum `WP` `tenta` `mesh` (`C-SELF-07` `honest-gated`).
2. `Cargo.toml` `linha 17-22` (`OPT-IN` `OUT-OF-DEFAULT-BUILD`) preservado: `crates/lightr-cri-serve` `excluido` `tonic/prost/gRPC` firewall (`ADR-0017` `decision 5`). `build` explícito: `cargo build --manifest-path crates/lightr-cri-serve/Cargo.toml`.
3. Se `WP-SELF-09` (`seam` `revisada`) `aprovar` `include` no futuro, `rebase` `lightr-cri-serve` `independente`; `C-SELF-08` `revisado` `aprovado` → `include`; `rejeita` → `manter` (`zero` `colisao`).

## Consequências

- `cri-backend` (`F-CRI-RUN`) `seam` `netns` `VALIDATED` (`#99`/`#100`) `funciona` `independente`; `selfhosted` `base` `funciona` `net_fd: None`; `mesh` `defere`; `Stage-2` `futuro`.
- Nenhuma `edicao` `.rs` (`revisao` `independente`); `Cargo.toml` `manter` `exclude`; `ADR-0017` `proposta` `revisada` `documentada` (`remover` `se` `aprovado` `WP-SELF-07`; `manter` `se` `rejeita` — `selfhosted` `base` `independente` `sem` `colisao`).
