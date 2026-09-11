=== TECHLEAD DISPATCH — WAVE 1 (3 AGENTS PARALELOS) ===

Lead: gustavo (techlead) | Protocol: agent-dispatch-and-landing SKILL.md
Contracts frozen: C-01 (CLI-Flags), C-03 (Runtime-Net-Vol), C-04-NEW (BuildKit-Freeze), C-05 (GPU-Device-Caps)
Plan: .techlead/docker-parity-wave-plan.md (19 WPs corrigido — review: .techlead/docker-parity-wave-plan-REVIEW.md)
No design decisions to agent. Frozen targets only. Mutation-probe required.

---
AGENT 1 — WP-01 (BU-01): BuildKit feature parity
Branch: wp-01 | Contract: C-04-NEW (freeze build/exec.rs) | Sweet-spot: 4 files
Brief: briefs/wp-01.md | Return card format: "WP-01 SEAL: branch wp-01 @ <sha> | files: <list> | clippy -D: pass | mutation: red/restore measured"
Files read FIRST: crates/lightr-cli/src/cli/cmd/subcommands.rs (frozen: BuildCmd enum), crates/lightr-cli/src/cli/dispatch.rs (frozen: match arm), crates/lightr-cli/src/handlers/build.rs (frozen: run() sig), crates/lightr-cli/src/handlers/docker/mod.rs (frozen: translate_build — --build-arg/--target forwarding), crates/lightr-build/src/exec/buildkit/ (new modules: buildkit.rs, secrets.rs, ssh.rs, cache.rs — does NOT edit build/exec.rs due to C-04-NEW freeze)
Target frozen: Add --cache-from parser + --secret file parser + --ssh file parser to docker build shim; wire to native build handler. No interface invention.
Acceptance: A. `lightr docker build --cache-from @t/app --secret id=foo,src=/tmp/secret --ssh default=/tmp/ssh .` exits 0 or 2 (honest). B. If blocked: exit 2 with "not yet supported" + note in return card. C. Mutation: break build/exec.rs (comment out body of run()) → `cargo test -p lightr-acceptance --test acceptance_r3 a23_build_hydrate` RED → restore → measure before/after.
Stop line: open PR from worktree .worktrees/wp-01; wait CI green; STOP at "WP-01 SEAL: PR green, waiting lead". DO NOT MERGE.
Question back: "what did the frozen build/exec.rs interface (C-04-NEW) prevent from designing?"
---
AGENT 2 — WP-03 (RUN-01): Full networking (DNS/VPN/network-create/IPAM)
Branch: wp-03 | Contract: C-03 (runtime-network-volume) | Sweet-spot: 5 files
Brief: briefs/wp-03.md | Return card: "WP-03 SEAL: branch wp-03 @ <sha> | files: <list> | clippy -D: pass | mutation: red/restore measured"
Files FIRST: crates/lightr-run/src/network/mod.rs (frozen: public interface), crates/lightr-run/src/network/registry.rs (frozen: members/subnet), crates/lightr-core/src/network.rs (frozen: NetworkId), crates/lightr-cli/src/cli/cmd/subcommands.rs (frozen: NetworkCmd enum — add ONLY to this enum, not others per C-01), crates/lightr-cli/src/handlers/network.rs (frozen: create/ls/rm/inspect interface — extend, not replace)
Target frozen: Create DnsResolver (dns resolution for compose/network), VpnManager, BridgeCreate (+ connect/rm/inspect extensions), IpamAllocator (subnet/gateway). No edit to shared NetworkId/VolumeId in core. No edit to network/registry.rs internal structure (only add modules: dns.rs, vpn.rs, bridge.rs, ipam.rs; re-export in network/lib.rs).
Acceptance: A. `lightr network create --driver bridge mynet` exits 0; ls shows mynet. B. `lightr network rm mynet` exits 0 (empty) or 1 (members). C. Mutation: remove create() from network/registry.rs → `cargo test -p lightr-acceptance --test acceptance_r3` RED → restore → measure.
Stop: PR from .worktrees/wp-03; STOP at "WP-03 SEAL: PR green, waiting lead".
Question back: "what did C-03 frozen interface prevent from inventing in network/registry.rs?"
---
AGENT 3 — WP-07 (GPU-DEVICE): --privileged/read-only/shm-size/pids-limit/cpus/memory enforcement
Branch: wp-07 | Contract: C-05 (gpu-device-caps) + C-11 (resource-limits) | Sweet-spot: 4 files
Brief: briefs/wp-07.md | Return card: "WP-07 SEAL: branch wp-07 @ <sha> | files: <list> | clippy -D: pass | mutation: red/restore measured"
Files FIRST: crates/lightr-run/src/limits.rs (frozen: ResourceLimits fields + check_native_support + apply_native/apply_cgroup signatures — READ ONLY; agent creates limits/gpu.rs, limits/device.rs, limits/caps.rs NEW modules; does NOT edit limits.rs frozen interface), crates/lightr-engine/src/engine/limits.rs (frozen: engine-side mirror — READ ONLY), crates/lightr-cli/src/cli/cmd/subcommands.rs (frozen: SystemCmd, EngineCmd — agent may add --privileged/--read-only/--shm-size ONLY if C-01 allows new SystemCmd fields; does NOT invent new subcommand enums), crates/lightr-cli/src/cli/dispatch.rs (frozen: SystemCmd dispatch arm)
Target frozen: Implement GpuParser (--gpus string parsing), DeviceParser (--device string parsing), CapParser (--cap-add/--cap-drop parsing). Apply to native (honest Err for unenforceable: cpu share → --engine ns/vz), ns (cgroup v2 device.allow + pids.max + memory.max + cpu.max + memory.swap.max=0 per #101 fix), vz (shim FFI memorySize/cpuCount). --privileged: exit 2 on native engine with message "not a sandbox" (F-203 pattern). --read-only: native RO remount; --shm-size: tmpfs sizing; --pids-limit: cgroup enforcement.
Acceptance: A. `lightr run --engine native --memory 64m -- /bin/sh -c 'echo x'` exits 137 (SIGKILL/OOM-killed) — verifies #101 fix (memory.swap.max=0). B. `lightr run --engine native --cpus 0.2 -- /bin/sh -c 'while true; do :; done'` runs slower (5.05x) — verifies #90 cpu.max throttles. C. Mutation: remove check_native_support() from limits.rs → `cargo test -p lightr-run` RED → restore → measure before/after.
Stop: PR from .worktrees/wp-07; STOP at "WP-07 SEAL: PR green, waiting lead".
Question back: "what did C-05 frozen interface (limits.rs) prevent from inventing?"
