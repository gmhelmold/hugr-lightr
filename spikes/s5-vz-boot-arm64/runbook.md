# SEAM-05 — F-209 (`fc`) Spike Runbook (arm64 vz boot)

**Contract:** `C-SELF-06` frozen — `fc` (`F-209`) = `future` (`cloud` tier); `spike` (`defere`; `plan.md` line 13); `honest Unsupported` (`selfhosted` base `local` `free`).

**Scope:** `spikes/s5-vz-boot-arm64/` (`README.md`, `EXPECTED.md`, `run-s5-arm64.sh`). `independente` `wave` `WP-10` `PLATFORM` (nao tocar).

**Status:** `⏳` `defere` (`future`). `spike` `runbook` `documentado`. Nenhum `boot` `validado` `arm64` — `F-205`/`F-206` `amarelo` (`unvalidated`) ate `run-s5-arm64.sh` `green` `real` `ARM Mac`.

**F-209 (`fc` engine — cloud tier):**
- `future` (`cloud` tier) — `Runners` `fabric`; `Firecracker` `microVM` (`F-209` `spec/feature-tree.md` line 49).
- `honest Unsupported` — `selfhosted` `base` (`store`/`run`/`cli`/`engine`) `funciona` `fc: None` (`zero regression`).
- `spike` (`this runbook`) = `documentar` `arm64 vz boot` (`F-205`/`F-206`) `independente` `fc` (`C-SELF-07` `independente` `mesh` `F-404` `defere`).
- `plan.md` line 13 (`defere`) — `fc` `nao` `construido` `v0.1`; `wave` `WP-10` `independente`.

**Pass criteria (`run-s5-arm64.sh`):**
- `A1`: `lightr run --engine vz @img/alpine -- /bin/echo s5-boot-ok` → `exit 0`, `stdout` `s5-boot-ok`, `exit != 255`.
- `A2`: `lightr run --engine vz @img/alpine -- /bin/sh -c 'exit 7'` → `exit == 7` (nao `0`, nao `255`).
- `255` = `GUEST_NO_REPORT_CODE` (`crates/lightr-engine/src/lib.rs` `vz_impl`). `exit 255` = `vsock` `quebrado`.

**Ambiguities:** `none`. `fc` `future` (`cloud` tier); `wave` `WP-10` `independente` (`spike` `2h`).
