# C-CLI_FLAGS — Contract Frozen (cli-flags)

**Status:** FROZEN by lead (gustavo) — 2026-09-11
**Anchor doc:** docs/build-spec-v2.md / .techlead/parity-contract.md
**Binding:** This contract is the frozen interface. Agents TRANSCRIBE, never invent.
**Disjoint files:** See .techlead/docker-parity-wave-plan.md WP table for this contract.



---
FROZEN SIGNATURE (cli/cmd/subcommands.rs):
```rust
#[derive(Subcommand)]
pub enum ComposeCmd {
    Up { #[arg(short='f', long, default_value="compose.yml")] file: String, ... },
    Down { #[arg(short='f', long)] file: Option<String>, ... },
}
#[derive(Subcommand)]
pub enum NetworkCmd { Create { name: String, ... }, ... }
#[derive(Subcommand)]
pub enum VolumeCmd { Create { name: Option<String> }, ... }
#[derive(Subcommand)]
pub enum PlanCmd { Snapshot { ... }, Hydrate { ... }, Run { ... } }
```
No agent invents new subcommand enum without lead authorization.
