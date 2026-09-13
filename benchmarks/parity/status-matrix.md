# S1-A Status Matrix

All 250 original scenario IDs remain. `build-single-stage` is `supported` from
the commit-pinned local `scratch-copy` fixture; all other scenarios remain
typed, non-executable states until their fixture and command evidence exists.

| Category | Unsupported | Hardware gated | Out of scope | Supported | Total |
|---|---:|---:|---:|---:|---:|
| build | 51 | 0 | 0 | 1 | 52 |
| buildkit | 13 | 0 | 0 | 0 | 13 |
| compose | 15 | 0 | 0 | 0 | 15 |
| health | 13 | 0 | 0 | 0 | 13 |
| logging | 6 | 0 | 0 | 0 | 6 |
| network | 17 | 6 | 0 | 0 | 23 |
| plugin | 0 | 0 | 14 | 0 | 14 |
| registry | 37 | 7 | 0 | 0 | 44 |
| resources | 18 | 0 | 0 | 0 | 18 |
| security | 14 | 3 | 0 | 0 | 17 |
| swarm | 0 | 16 | 0 | 0 | 16 |
| volume | 19 | 0 | 0 | 0 | 19 |
| **Total** | **203** | **32** | **14** | **1** | **250** |

`hardware_gated` preserves prior `honest_gated: true` labels while adding typed
status and reason. `plugin` is Sprint-1 owner-policy `out_of_scope`.
