# Docker Managed-Plugin Adapter: Feasibility Study

- **Status:** S1-F research only, 2026-09-12.
- **Target:** Docker CLI/Engine `28.3.2`; Engine API version must be captured by
  the probe, never inferred.
- **Scope:** Docker managed plugins and their Engine plugin RPC interfaces.
- **Decision:** no implementation and no compatibility claim. Current Lightr
  cannot safely host a Docker managed volume, network, authorization, or log
  plugin. A Linux-only volume lifecycle probe can determine whether a sharply
  constrained adapter is worth a design decision.

## Framing

Docker plugins are extensions of a resident Engine. Docker manages plugin
distribution, privilege consent, enable/disable state, discovery, and calls
from Engine subsystems. Lightr is a one-shot CLI with run-scoped supervisors;
at idle it must have no Lightr process or socket. Replaying Docker's plugin
surface therefore requires more than HTTP endpoint translation. It requires an
explicit owner for plugin process, persistent state, privileged host resources,
and cleanup. [D1][D2][L1]

This study addresses managed plugins. Docker's older external plugin discovery
also uses same JSON-over-HTTP RPC protocol, but its `.sock`, `.spec`, and
`.json` files are daemon-host discovery mechanisms, not an Lightr feature.
They are not an alternative implementation path. [D3]

## Docker Contract

### Managed-plugin lifecycle and privilege boundary

`docker plugin install` pulls or finds plugin, prompts for declared privileges
unless `--grant-all-permissions` is passed, then installs and enables it by
default. Managed-plugin CLI also exposes create, enable, disable, inspect, set,
upgrade, push, remove, and list. A compatible adapter would need an explicit
answer for every state transition; treating a plugin image as an OCI runtime
image loses its config and privilege contract. [D2][D4][D5]

Plugin config declares interface types, entrypoint, rootfs workdir, network
mode, mounts, environment and arguments, host IPC/PID access, propagated mount,
Linux capabilities, device access, and devices. `PropagatedMount` is specifically
used to expose mounts made below that path outside plugin rootfs. Docker's
published `vieux/sshfs` install example asks for host networking, `/dev/fuse`,
and `CAP_SYS_ADMIN`; this is privileged host code, not a normal application
extension. [D2][D4]

At use, Engine activates an external plugin with `POST /Plugin.Activate`, whose
response names implemented subsystems. Requests are JSON-over-HTTP `POST` calls
with `Accept: application/vnd.docker.plugins.v1+json`. Docker documents
on-demand activation and retries failed plugin calls with exponential backoff
for up to 30 seconds. Lifecycle ownership otherwise remains outside protocol:
external plugins should start before Docker and stop after it; socket activation
is an optional OS service pattern. [D3]

Plugin RPC is typed by subsystem, not one generic operation:

| Type | Engine lifecycle expected by plugin | Adapter consequence |
|---|---|---|
| `VolumeDriver` | `Create`, `Get`, `List`, `Remove`, `Mount(name,id)`, `Unmount(name,id)`, `Path`, `Capabilities`; `Mount` can be repeated and must be paired with `Unmount` per consumer. | Adapter needs durable volume-to-driver config and a mount lease ledger. Returned mountpoint must become a safe bind source before child spawn. |
| `NetworkDriver` | Network allocation/create/delete; endpoint create/delete; join/leave, external-connectivity, discovery and capability calls. `Join` receives Docker sandbox identity and returns interface names/routes. | Adapter must emulate libnetwork endpoint and network-namespace semantics, not only Lightr network naming. |
| `authz` | `AuthZPlugin.AuthZReq` before, `AuthZPlugin.AuthZRes` after each Docker HTTP API request. Plugin sees user/auth method, URI, headers and permitted bodies. | Only meaningful at an Engine API mediation boundary. Lightr CLI invocation is not an equivalent request stream. |
| `LogDriver` | `StartLogging` receives Engine-created FIFO file, driver consumes framed protobuf stream; `StopLogging` drains before reply. Optional `ReadLogs` is Docker `logs` backend. | Adapter must preserve backpressure, framing and read-back semantics. Existing stdout/stderr files are not this contract. |

Sources: volume RPC [D6]; network RPC [D7][D8]; authorization RPC [D9]; log
RPC [D10].

### State, cleanup, and failure semantics

Any candidate adapter must persist an fsync-backed operation ledger before each
externally visible state change. Minimum record: adapter schema version, plugin
reference and immutable content identity, normalized config digest, granted
privileges, plugin instance ID/PID, volume name, Lightr run ID, plugin mount ID,
mountpoint after canonicalization, state, and last cleanup error. Secret values
must never enter this ledger or logs.

Required volume state machine:

```text
absent --Create--> created --Mount(run,id)--> mounted --spawn--> attached
attached --child exit/stop--> unmounting --Unmount(run,id)--> created
created --Remove--> absent
```

- Crash before child spawn: call `Unmount` if `Mount` returned success; retain
  failed cleanup record and return nonzero.
- Child start failure: do not claim a run; unmount synchronously.
- Child exit, `lightr stop`, SIGTERM, or supervisor restart: exactly one
  idempotent cleanup owner calls `Unmount`; adapter host remains alive until it
  finishes or reports a typed cleanup failure.
- Plugin exit, malformed JSON, unexpected endpoint, unavailable socket, or
  mountpoint escaping approved root: fail closed. Do not fall back to Lightr
  local volume or a bind path.
- Recovery: next explicit adapter invocation may inspect ledger and offer
  cleanup only. It must not replay `Mount`, guess a plugin state, or remove a
  resource without matching plugin identity and lease ID.

This does not establish Docker equivalence. It defines minimum safety behavior
for an experiment.

## Daemonless Architecture

### Option A: run-scoped plugin host

`lightr run` starts plugin host after AC lookup and before a run that explicitly
requests a plugin driver. Host exposes private Unix socket in run directory,
performs activation and required RPCs, then is owned by existing detached-run
supervisor. Supervisor executes cleanup and kills host only after final
`Unmount`. Synchronous run keeps host as child and applies same `finally` path.

Advantages: no resident Lightr service; socket and process die with run;
failure ownership matches `spec.json`, PID, logs and stop lifecycle already used
by Lightr. [L1]

Limits: unsuitable for a general Docker managed-plugin contract. `volume
create`, `inspect`, and `rm` may be invoked with no container; a plugin may
maintain durable remote state or a FUSE mount across consumers. A per-run host
cannot safely claim driver-wide `List`, `Get`, `Path`, concurrent mount refcount,
or cross-run durability without a new persistent control plane. It is viable
only for a future explicit experimental *run-volume-driver* surface, one
volume-plugin process per run, Linux namespace engine, and no shared lifecycle.

### Option B: explicitly installed OS-supervised plugin process

User explicitly installs plugin under systemd or launchd. OS owns service
restart/socket activation; Lightr only connects to configured private socket at
invocation. This follows Lightr's existing policy for restart: generate or use
OS supervision, never ship a resident coordinator. [D3][L1]

Advantages: matches external plugin lifecycle and allows durable plugin state
and multi-run volume lease accounting.

Costs: third-party process can be resident, privileged, and independently
upgradeable. It is incompatible with "no Lightr processes at idle" only if
Lightr owns installation or starts it implicitly. It also does not implement
Docker managed-plugin distribution, sandboxing, config mutation, or Engine
driver discovery. Use only after owner accepts OS service as explicit opt-in
and security review approves exact plugin config.

### Recommendation

Do not add either architecture now. If probe passes, choose no broader than
Option A for a Linux-only, one-run, volume-only experimental adapter. Promote
to Option B only after independent approval of persistent privileged third-party
processes, config/provenance UX, cross-run lease recovery, and an exit-cleanup
test under process crash. No default plugin discovery directory, automatic
service install, or implicit plugin start.

## Support Tiers

| Surface | Current state | Earliest eligible tier | Unsafe or impossible path |
|---|---|---|---|
| Volume | Unsupported. `lightr volume` has no driver/options surface; named volumes are not wired into detached materialization. [L2][L3] | Probe only: Linux `ns`, one explicit plugin, one run, one volume, privileged host approved. | Native engine has no mount namespace and realizes rw binds as symlinks/ro as snapshots. VZ/WSL require separate mount propagation proof. Do not accept plugin mountpoint as arbitrary host path. [L4] |
| Network | Unsupported. CLI records only bridge-style local registry; supplied `--driver` is discarded. Hot connect/disconnect exits 2 by design. [L5] | None until separate network-driver and endpoint lifecycle design. | Remote drivers require libnetwork sandbox/interface/routes; Lightr's VZ switch and ns network paths do not expose equivalent driver hooks. Never map a remote driver to `bridge` or silently ignore it. [D7][D8] |
| Authorization | Unsupported. | None until opt-in Docker-compatible API socket defines request identity and full API semantics. | AuthZ authorizes Engine HTTP API, including pre/post request behavior. CLI translation, direct Rust calls, gRPC, upgraded streams, or partial body inspection are not equivalent policy enforcement. [D9] |
| Log | Unsupported. | None until stream writer, FIFO lifecycle, protobuf framing, backpressure and `ReadLogs` semantics are specified and proven. | Wrapping current run-dir stdout/stderr files loses FIFO/framing/drain semantics. Never advertise driver or accept `--log-driver` until it is real. [D10][L1] |

## Security Requirements

1. **Explicit root boundary.** Managed plugin permissions can include host
   networking, host PID/IPC, devices, `CAP_SYS_ADMIN`, mount propagation, and
   unrestricted device access. Adapter must parse config before execution,
   present exact privilege set, require noninteractive explicit consent, and
   reject undeclared or changed config. Rootless mode rejects any plugin needing
   unavailable privilege. [D4]
2. **No Docker socket pass-through.** Plugin gets no `/var/run/docker.sock`,
   Lightr compat socket, or raw Engine API credential by default. If a future
   adapter needs mediation, expose narrow versioned RPC operations, enforce
   allowlist, timeouts and request-size limits, and audit operation metadata
   without secret values.
3. **Least privilege.** Run host with empty ambient capability set; add only
   approved declared capability/device/mount/network access. Plugin request to
   self-escalate, access host namespaces, or return a mountpoint outside its
   approved propagation root is hard failure.
4. **Credentials.** Supply credentials through an explicit per-plugin secret
   reference or inherited OS credential store, never command line, environment
   dump, config ledger, or `inspect` output. Redact plugin RPC bodies in logs.
   Plugin may receive storage credentials; that makes its process a secret
   boundary.
5. **Trust and provenance.** Pin plugin reference to digest; retain config and
   rootfs/content digest plus publisher identity evidence in probe artifact.
   Reject mutable tag-only execution. Docker install's privilege prompt is not
   a Lightr trust policy or signature verification result. Signature/verifier
   requirements need an owner-approved supply-chain decision before any install
   UX exists. [D2][D4]
6. **Isolation honesty.** `native` is not sandboxed. Rootless `ns` is not a
   hostile-tenant boundary. A plugin granted host mount or device access remains
   trusted host code even when workload runs in a namespace. [L4][L6]

## Compatibility Risks

- **Engine API mismatch:** plugin callbacks receive Docker-specific IDs,
  container metadata, sandbox keys, API versions, request bodies, FIFO paths,
  and sequencing. Lightr `RunSpec` and run directories are different state
  model. Translation must be versioned, explicitly documented, and tested per
  RPC; generic JSON forwarding is insufficient. [D6][D7][D9][D10][L1]
- **Discovery mismatch:** Docker resolves driver names through managed-plugin
  state or daemon-host `.sock`/`.spec`/`.json` lookup. Lightr has neither a
  `plugin` command nor driver registry. Reusing `/run/docker/plugins` would
  collide with Docker host state and creates unreviewed discovery. [D3][L2][L5]
- **Config mismatch:** Docker config controls entrypoint, rootfs, env, mounts,
  host namespaces, capabilities/devices and propagated mount. Lightr's OCI
  import turns images into CAS trees; it does not preserve a managed-plugin
  installation/config lifecycle. [D4][L7]
- **Mount propagation:** Docker v2 volume mountpoints must be under declared
  `PropagatedMount`; Engine bind-mounts returned path into container. Current
  Lightr named-volume materialization is intentionally skipped and native has
  no mount namespace. This is a hard blocker, not a path-string conversion.
  [D6][L3][L4]
- **Network state:** Docker remote network driver creates endpoints and joins
  its returned interface to Engine sandbox. Current Lightr network registry
  tracks members for its own switch; it cannot import arbitrary interface names,
  routes, IPAM or external-connectivity behavior. [D7][D8][L5]
- **Process persistence:** Docker plugin drivers may survive container calls;
  Lightr per-run supervisor self-exits. Shared volume semantics require crash
  recovery and multi-run ownership, which Option A deliberately excludes.

## Narrow Falsifiable Probe

### Question

Can a single Docker-documentation volume plugin (`vieux/sshfs`) complete one
`Create -> Mount -> use -> Unmount -> Remove` lifecycle through a future
run-scoped Linux adapter, with mountpoint containment and no leaked plugin
process, FUSE mount, or adapter ledger entry?

This plugin is selected because Docker documentation uses it as privilege-prompt
example (`host` network, `/dev/fuse`, `CAP_SYS_ADMIN`). Probe is deliberately
host-privileged and must never run in shared CI or on developer workstation with
non-disposable mount state. [D4]

### Prerequisites

1. Disposable Linux x86_64 VM, systemd, kernel FUSE support, root access,
   user namespaces enabled, and `findmnt`, `ps`, `sha256sum`, `jq`, and `curl`.
2. Docker CLI and Server both exactly `28.3.2`; capture full
   `docker version --format '{{json .}}'` and daemon API version before setup.
3. A test-only SSH/SFTP server and account, reachable from VM, with an empty
   remote directory. Credentials come from protected test secret store; neither
   URI nor password enters committed evidence.
4. Docker plugin reference `vieux/sshfs` resolved to immutable installed plugin
   ID and content digest. Record `docker plugin inspect` JSON, plugin config,
   requested privileges and `docker plugin ls --no-trunc` output. If reference
   cannot be resolved to immutable identity, do not run.
5. Future adapter binary built from recorded Lightr revision and configured with
   a fresh `LIGHTR_HOME` plus an approved plugin socket/root. No existing Docker
   plugin sockets, mounts, or Lightr runs with probe name.

### Setup and execution

These commands define Docker baseline and evidence capture only. Adapter command
is a placeholder contract, not an existing Lightr command or claimed syntax.

```bash
# Baseline setup on disposable VM; operator reviews Docker privilege prompt.
sudo docker plugin install vieux/sshfs
sudo docker plugin inspect vieux/sshfs > artifacts/docker-plugin-inspect.json
sudo docker plugin ls --no-trunc > artifacts/docker-plugin-ls.txt
docker version --format '{{json .}}' > artifacts/docker-version.json
docker version --format '{{.Server.APIVersion}}' > artifacts/docker-api-version.txt

# Operator supplies endpoint options from protected test environment.
sudo docker volume create --driver vieux/sshfs \
  --opt sshcmd="$PROBE_SSHCMD" --opt password="$PROBE_PASSWORD" \
  lightr-plugin-probe
sudo docker run --rm -v lightr-plugin-probe:/data alpine:3.20 \
  sh -ceu 'printf docker-baseline >/data/baseline; test "$(cat /data/baseline)" = docker-baseline'
sudo docker volume rm lightr-plugin-probe
```

Future adapter test uses same installed plugin identity and equivalent endpoint
options. It must record every RPC request/response shape with secrets redacted:

```text
lightr experimental plugin-volume-run \
  --plugin vieux/sshfs@<recorded-immutable-identity> \
  --volume lightr-plugin-probe:/data \
  --secret-ref <protected-reference> --engine ns -- \
  sh -ceu 'printf lightr-probe >/data/probe; test "$(cat /data/probe)" = lightr-probe'
```

### Expected behavior and controls

- Baseline succeeds. Adapter calls `Plugin.Activate`, `VolumeDriver.Create`,
  `Mount` with unique Lightr run ID, binds only returned canonical mountpoint
  below approved root, then child reads/writes `/data/probe`.
- Adapter exits nonzero unless post-run ledger shows matched successful
  `Unmount` and `Remove`, no live plugin-host child, and no FUSE mount below
  approved root. A successful child with failed cleanup is probe failure.
- **Negative control:** change configured expected propagation root so plugin's
  returned mountpoint is outside it. Adapter must reject before child spawn;
  sentinel file must not exist; ledger must show cleanup attempt; no fallback to
  Lightr local volume. This controls path containment, not plugin availability.

### Cleanup and evidence artifacts

Always run cleanup in finally path, then separately verify:

```bash
sudo docker plugin disable vieux/sshfs
sudo docker plugin rm vieux/sshfs
findmnt -R -t fuse,fuse.sshfs
ps -ef
```

Retain outside repository: Docker version/API artifacts; full plugin inspect and
privilege transcript; immutable identity/config/rootfs digests; redacted RPC
trace ordered by operation; Lightr revision/binary digest; `spec.json` and
adapter ledger; child stdout/stderr/exit; pre/post `findmnt`; pre/post process
list; cleanup command exits; negative-control trace; host/kernel identity. Hash
all raw artifacts and record manifest digest.

### Exit criteria

Probe is **GREEN** only if baseline and adapter lifecycle both pass, adapter
returns no leaked process/mount/ledger, negative control fails before workload
spawn, and artifact review confirms plugin identity and privilege set remained
unchanged. Any other result is **RED**. Green proves one narrow lifecycle on
named disposable hardware; it proves neither general volume support nor Docker
compatibility.

## Non-Claims and Decision Gates

This study does not claim support for Docker plugins, managed-plugin install,
plugin discovery, plugin signatures, volume drivers, network drivers,
authorization plugins, logging drivers, FUSE, mount propagation, Docker API
compatibility, or safe execution of `vieux/sshfs`. It reports no measurement.

Do not create implementation WP unless all gates pass:

1. Owner approves explicit experimental scope, root/privilege UX, disposable
   host requirement, and no automatic service install.
2. Independent security review accepts capability/device/mount/socket/credential
   mediation model and immutable provenance policy.
3. Falsifiable probe is GREEN with complete artifact manifest and negative
   control.
4. Linux namespace mount-injection contract, cleanup/recovery ledger, and
   crash-kill test are accepted. MacOS VZ, Windows WSL, and native remain
   unsupported unless independently proven.
5. Parity corpus lists plugin scenarios as `out_of_scope` until a future
   accepted WP produces per-operation evidence. [L8]

## Sources

- **[D1]** Docker Plugin API: <https://docs.docker.com/engine/extend/plugin_api/>
  (discovery, activation, HTTP protocol, retries, lifecycle).
- **[D2]** `docker plugin` command: <https://docs.docker.com/reference/cli/docker/plugin/>
  (managed-plugin lifecycle commands).
- **[D3]** Docker Plugin API, discovery and socket activation:
  <https://docs.docker.com/engine/extend/plugin_api/#plugin-discovery>
  and <https://docs.docker.com/engine/extend/plugin_api/#systemd-socket-activation>.
- **[D4]** `docker plugin install`: <https://docs.docker.com/reference/cli/docker/plugin/install/>
  (privilege grant; `vieux/sshfs` example).
- **[D5]** Docker Plugin config v1: <https://docs.docker.com/engine/extend/config/>
  (interface, entrypoint, mounts, propagated mount, namespaces, Linux devices
  and capabilities).
- **[D6]** Docker volume plugin protocol:
  <https://docs.docker.com/engine/extend/plugins_volume/>.
- **[D7]** Docker network plugin protocol:
  <https://docs.docker.com/engine/extend/plugins_network/> and libnetwork remote
  driver contract: <https://github.com/moby/moby/blob/master/daemon/libnetwork/docs/remote.md>.
- **[D8]** Docker `go-plugins-helpers` network interface:
  <https://raw.githubusercontent.com/docker/go-plugins-helpers/main/network/api.go>.
- **[D9]** Docker authorization plugin protocol:
  <https://docs.docker.com/engine/extend/plugins_authorization/>.
- **[D10]** Docker log driver plugin protocol:
  <https://docs.docker.com/engine/extend/plugins_logging/>.
- **[L1]** `docs/ARCHITECTURE.md` §§1, 4, 11 (current run supervisor, log
  files, daemonless invariant).
- **[L2]** `crates/lightr-cli/src/handlers/volume.rs` lines 1-10 (local named
  volume registry; no mount wiring).
- **[L3]** `crates/lightr-run/src/run/bindmat.rs` lines 74-121 (named/anonymous
  volumes intentionally skipped on detached path).
- **[L4]** `crates/lightr-run/src/run/bindmat.rs` lines 1-24, 33-58 and
  `docs/ARCHITECTURE.md` §§2, 11 (native bind semantics and isolation boundary).
- **[L5]** `crates/lightr-cli/src/handlers/network.rs` lines 1-24, 223-235 and
  `crates/lightr-cli/src/cli/cmd/subcommands.rs` lines 169-210 (local bridge
  registry; no driver dispatch; no hot-plug).
- **[L6]** `docs/spec/parity-audit.md` F-204 and platform coverage (rootless
  namespace isolation is not hostile-tenant boundary).
- **[L7]** `docs/ARCHITECTURE.md` §7 (OCI import is CAS bridge, not runtime
  image model).
- **[L8]** `benchmarks/SPRINT-01-EXECUTION-PLAN.md` lines 65-76 and
  `benchmarks/TECHLEAD-SPRINT-01.md` lines 8-12, 96-100 (plugins out of scope;
  research-only until lifecycle evidence).
