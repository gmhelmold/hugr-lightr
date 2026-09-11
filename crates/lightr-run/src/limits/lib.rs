//! Limits submodule re-export — frozen C-05 interface.
//!
//! Ambiguity (noted in return card): `limits/lib.rs` conflicts with the frozen
//! `limits.rs` file at the same module path (`lightr_run::limits`). The frozen
//! `limits.rs` interface (`ResourceLimits`, `check_native_support`, `apply_native`,
//! `apply_cgroup`, `apply_native_ulimits`) is untouched; these new submodules
//! (`caps`, `device`, `gpu`) and their parsers (`CapParser`, `DeviceParser`,
//! `GpuParser`) are created per C-05 but cannot be compiled into the `limits`
//! module without either:
//!   (a) renaming `limits.rs` → `limits/lib.rs` (edits frozen entry), or
//!   (b) adding `pub mod caps; pub mod device; pub mod gpu;` to `limits.rs`
//!       (edits frozen file, but only module declarations, not interface bodies).
//! Lead decision deferred; current state = files present, compilation deferred.

pub mod caps;
pub mod device;
pub mod gpu;

// Re-export the frozen parser interfaces per C-05.
pub use caps::{CapParser, apply_cgroup as cap_apply_cgroup, apply_native as cap_apply_native, apply_vz as cap_apply_vz};
pub use device::{DeviceMapping, DeviceMode, DeviceParser, apply_cgroup as device_apply_cgroup, apply_native as device_apply_native, apply_vz as device_apply_vz};
pub use gpu::{GpuParser, GpuRequest, apply_cgroup as gpu_apply_cgroup, apply_native as gpu_apply_native, apply_vz as gpu_apply_vz};
