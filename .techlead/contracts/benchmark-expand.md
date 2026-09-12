# C-BENCH_EXPAND — Contract Frozen (benchmark-expand)

**Status:** FROZEN by lead (gustavo) — 2026-09-11
**Anchor doc:** crates/lightr-cli/src/handlers/bench/mod.rs / docs/benchmarks/RESULTS.md
**Binding:** Benchmark indicator B13-B17 expansion (install footprint, build, materialize, volume, network, system) must document budget in bench/mod.rs constants before adding.
**Disjoint files:** See .techlead/docker-parity-wave-plan.md WP-19 (BENCH-EXP).

```rust
// FROZEN SIGNATURE — benchmark handler budgets
const BUDGET_B13_INSTALL_FOOTPRINT_MS: u64 = ...; // Not yet defined — WP-19
const BUDGET_B14_VOLUME_MS: u64 = ...;
const BUDGET_B15_NETWORK_CREATE_MS: u64 = ...;
const BUDGET_B16_SYSTEM_DF_MS: u64 = ...;
const BUDGET_B17_MULTI_ARCH_MS: u64 = ...;
```
