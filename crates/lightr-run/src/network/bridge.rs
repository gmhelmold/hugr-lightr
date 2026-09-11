//! Bridge creation — network create with bridge/overlay drivers (WP-03 / C-03).
//!
//! Uses the frozen `NetworkRegistry` (C-01, `registry.rs`) for
//! on-disk persistence and deterministic subnet allocation.
//! Driver selection: `bridge` (default) or `overlay`.
//! Unknown drivers fail closed (`InvalidInput`), never silently
//! mapped to `bridge`.

use std::path::Path;

/// Bridge/network creation handler — validates driver then delegates
/// to `NetworkRegistry::create` (frozen interface, never inventing
/// a parallel registry).
pub struct BridgeCreate;

impl BridgeCreate {
    /// Create a named network with the given driver.
    ///
    /// `driver`: `Some("bridge")` or `Some("overlay")`; `None` ⇒ `bridge`.
    /// Errors: unsupported driver (`InvalidInput`), registry I/O (`Io`).
    pub fn create_network(home: &Path, name: &str, driver: Option<&str>) -> std::io::Result<()> {
        let drv = driver.unwrap_or("bridge");
        // Fail-closed: only `bridge` and `overlay` accepted (Docker parity).
        if drv != "bridge" && drv != "overlay" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("unsupported network driver: {drv} (want bridge or overlay)"),
            ));
        }
        // Delegate to frozen registry — same atomic write + flock path.
        use super::registry::NetworkRegistry;
        NetworkRegistry::create(home, &name.to_string())?;
        Ok(())
    }
}
