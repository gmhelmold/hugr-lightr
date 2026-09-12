# Split Protocol — Selfhosted (arch) vs Docker-Parity Wave (exec)

Status: proposta. Aprovacao: usuario relays. Conflito: resolvido se contratos respeitados.

## 1. Separacao de trabalho

| Dimensao | Selfhosted (esta sessao) | Wave (outra sessao — .techlead/docker-parity-wave-plan.md) |
|---|---|---|
| Objetivo | Arquitetura: `seam` `clw`, `auth` opcional, `selfhosted` base, `CoreLink` `tier` cloud | Execucao: `19 WPs`, `docker` paridade, `6-8 semanas` |
| Worktree | `.worktrees/selfhosted/` | `.worktrees/wave-parity/` (ou direto `main` se `wave` nao usa `worktree`) |
| Crates foco | `lightr-store`, `lightr-engine` (`F-209` `fc`), `lightr-cri-backend`, `docs/adr/` | `lightr-cli` (`WP-05` `28` subcmd), `lightr-run` (`WP-01`/`WP-10`), `lightr-oci` (`WP-11`), `lightr-build` (`WP-15`), `lightr-views` (`WP-03`?) |
| Contratos | `ADR-0017` (`exclude` `lightr-cri-serve`), `seam` `clw` (`CLAUDE.md` §9) | `C-01`..`C-11` (`.techlead/contracts/`), `build/exec.rs` freeze (`WP-01`/`WP-02`) |
| Deferrals compartilhados | `F-209` (`fc` `cloud` — precisa), `F-404` (`mesh` — precisa), `F-405` (`Stage-2` — `opt-in` `wire bridge`) | `F-209`, `F-603`, `F-604`, `F-208`, `F-207`, `F-404-406`, `F-506` (`plan.md` `line 13`) — nao implementa |

## 2. Regra de nao-colisao (hard contract)

- `Wave` nao modifica `docs/adr/` (`20` `ADR`). `Selfhosted` propede revisao `ADR` (`seam` `clw`, `auth` `opt-in`).
- `Wave` nao altera `Cargo.toml` `exclude` (`lightr-cri-serve`) (`ADR-0017`). `Selfhosted` pode propor `remove` `exclude` se `seam` revisada.
- `Wave` nao implementa `F-209`/`F-404`/`F-405`. `Selfhosted` usa como `target` `future` (`cloud` tier).
- `Clw` (`corelink-workspaces`): `wave` (`WP-05` `CLI-A/B/C`) usa `clw` como `client` (`CLAUDE.md`). `Selfhosted` decide `seam` (`direct dep` vs `contrato`). Se `wave` precisa `clw` `L1` (`snapshot`/`hydrate`), `F-103` (`CoW` `lightr_index`) ja cobre — `wave` nao precisa `clw` `crate` se `seam` mantida.
- `CI` (`6` `.github/workflows/`): `wave` adiciona `test` (`WP-19` `bench`). `Selfhosted` nao remove `linux-validation.yml` (`F-204` `ns` `VALIDATED`).

## 3. Protocolo de merge

```
Selfhosted PR (arch)  ->  Revisao (ADR + seam)  ->  Merge (se aprovado)
     |                                      |
     v                                      v
Wave PR (exec)     ->  Revisao (C-01..C-11)  ->  Rebase sobre Selfhosted (se Selfhosted mergeou) ou Merge independente (se Selfhosted pendente)
```

Risco: se `Selfhosted` nao aprovada (`seam` `clw` nao resolvida), `Wave` executa sobre `base` atual (`client/corelink`). `Selfhosted` vira `future` `v0.3`.

## 4. Avaliacao honesta — da certo?

Sim, `se` e `so` se:
- `Wave` aceita `contratos` (`C-01`..`C-11`) como `interface` estavel, nao `arquitetura`.
- `Selfhosted` entrega `PR` curto (`seam` `clw` + `ADR` proposta + `auth` `opt-in`), nao `19 WPs`.
- `Deferrals` (`F-209`) mantidos `defer` (`plan.md` `line 13`) — `wave` nao tenta `fc`.
- `Merge` `Selfhosted` `antes` ou `durante` `wave` (`worktree` `isolado` evita `conflict` `git`).

Se `Wave` modifica `seam` (`clw` absorvido sem `contrato`) ou `ADR-0017` (`lightr-cri-serve` incluido), `Selfhosted` quebra. Entao `sim`, `da certo numa boa` — `se` `protocol` respeitado. Se `nao`, `colide` (`WP-16` `Compose deploy` + `F-405` `Stage-2` = `overlap` `corelink` `tier`).

## 5. Documentacao para relay

Envie para outra sessao (`wave`):
- `Split Protocol` (este doc) = `worktree` + `crates` + `contratos`.
- `Regra`: `wave` nao toca `docs/adr/`, `Cargo.toml` `exclude`, `seam` `clw`.
- `Deferrals` (`F-209`/`F-404`) = `futuro` compartilhado — `wave` nao implementa.
- `Selfhosted` entregara `PR` (`seam` + `ADR`) antes ou `durante` `6-8` semanas.

Se `wave` aceita: `parallel` `limpo`. Se `wave` rejeita (`wave` precisa `clw` `direct dep` `agora`): `colisao` (`WP-05` `CLI` + `Selfhosted` `seam`). Entao `sim`, `funciona` — `condicional`.
