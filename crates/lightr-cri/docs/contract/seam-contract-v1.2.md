# Seam Contract v1.2 — additive security context

- **Status:** accepted as part of the unified `hugr-lightr` repository.
- **Compatibility:** v1.1 state and vectors remain valid; every new field uses
  `serde(default)`.

## ContainerConfig

`ContainerConfig` gains:

```rust
#[serde(default)]
pub security: Option<SecurityContext>;
```

`None` means runtime default/unset. The CRI shell maps
`LinuxContainerSecurityContext` into this field before calling `CriBackend`.

## SecurityContext

```rust
pub struct SecurityContext {
    pub apparmor: Option<SecurityProfile>,
    pub seccomp: Option<SecurityProfile>,
    pub capabilities: Option<Capabilities>,
}

pub struct SecurityProfile {
    pub profile_type: ProfileType,
    pub localhost_ref: String,
}

pub enum ProfileType {
    RuntimeDefault,
    Unconfined,
    Localhost,
}

pub struct Capabilities {
    pub add: Vec<String>,
    pub drop: Vec<String>,
}
```

`Localhost` carries an AppArmor profile name or an absolute seccomp profile
path. `RuntimeDefault` selects host AppArmor policy and the built-in `default`
seccomp profile. `Unconfined` explicitly disables that profile.

The backend applies capabilities before user/seccomp setup. Unsupported
profile grammar, architecture, platform, or privilege combinations fail closed;
the shell never silently drops a requested security field.
