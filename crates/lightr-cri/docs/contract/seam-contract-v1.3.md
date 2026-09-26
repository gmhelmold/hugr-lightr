# Seam Contract v1.3 - additive identity and host aliases

- **Status:** accepted as part of unified `hugr-lightr` repository.
- **Compatibility:** v1.2 remains immutable. All v1.3 fields use
  `serde(default)`, so v1.2 persisted records and vectors deserialize unchanged.

## SandboxConfig

`SandboxConfig` gains:

```rust
#[serde(default)]
pub host_aliases: Vec<HostAlias>,

pub struct HostAlias {
    pub ip: String,
    pub hostnames: Vec<String>,
}
```

Each alias represents one IP and one or more DNS-style hostnames for generated
container `/etc/hosts` content. `ip` must parse as IPv4 or IPv6. Each hostname
must be non-empty ASCII DNS labels separated by dots; labels are 1-63 bytes,
alphanumeric or `-`, and cannot start or end with `-`. Invalid alias data fails
deserialization closed. Empty `host_aliases` means no requested aliases.

## SecurityContext

`SecurityContext` gains:

```rust
#[serde(default)]
pub run_as_user: Option<u64>,
#[serde(default)]
pub run_as_group: Option<u64>,
```

`None` preserves image/runtime identity. `run_as_group` requires
`run_as_user`; group-only persisted data fails deserialization closed. Numeric
IDs are unsigned: negative wire values fail deserialization. Descriptor and
`ns` application are deliberately outside v1.3 seam scope.

## Vectors and versioning

`vectors/v1.3-identity-host-alias-round-trip.json` carries both additions.
v1, v1.1, and v1.2 vectors remain valid unchanged. Canonical vendored and
local transcribed vocabularies serialize the same field names and values.
