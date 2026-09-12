# Project map — hugr-lightr

Generated 2026-06-11 (reader-merged; repo is docs-only, no parseable code).

## Areas
- **Vision & business** — `docs/whitepaper/hugr-lightr-v1.md` (canonical, source of truth), `docs/VISION.md` (funnel), `docs/product/product.md` (ICPs, pricing posture, open decisions)
- **Architecture** — `docs/ARCHITECTURE.md` (engines, execution flow, lazy rootfs, seams)
- **MVP** — `docs/MVP-v0.1.md` (v0.1 scope + DoD)
- **Agent context** — `CLAUDE.md` (principles, tense law, conventions)
- **TechLead state** — `.techlead/` (gitignored, never committed)

## Entry points
`README.md` → `CLAUDE.md` → `docs/whitepaper/hugr-lightr-v1.md`

## Invariants (must not break)
1. **Design phase** — no code; no number claimed as measured (tables are targets from cited precedents).
2. **Tense law** — CoreLink dedup intra-tenant at GA; cross-tenant **staged** (`CAP-DEDUP-CROSS-TENANT`); never "live".
3. **Whitepaper §9 principles are decided** — no daemon · no images · free local no-account · isolation à la carte (native ≠ sandbox, said loudly) · memoize-first · pure CoreLink client · fail-closed · ship after Runners M1 · never bill memoized runs. Owner-only to relitigate.
4. **Pure client** — zero corelink-server changes, ever.
5. **Sibling fence** — never mutate other HuGR repos.
6. **Naming** — crate `hugr-lightr`, binary `lightr`.

## Cross-repo deps
- `corelink-server` — CAS/AC API consumer (tenancy, PAT).
- `corelink-workspaces` (clw) — distribution pipeline; seam form open (product.md §9).
- `corelink-runners` — `Engine` lineage; in cloud, Lightr is what a lease executes.
- Runners **M1** gates the public free-tier launch.
