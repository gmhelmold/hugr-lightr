//! VPN manager (WP-03 / C-03 frozen contract stub).
//!
//! Basic tunnel interface configuration.
//! Ambiguity noted: VPN config format (tunnel type,
//! authentication, key exchange) is NOT frozen in contract C-03.
//! Default: minimal tunnel interface declaration (Docker-faithful
//! `tunnel` interface reference) to keep the surface available
//! without inventing unvalidated auth grammar.

/// Minimal VPN manager — tunnel interface config only.
/// Full auth/key grammar deferred until C-03 is completed.
pub struct VpnManager;

impl VpnManager {
    /// Configure a tunnel interface by name.
    ///
    /// NOTE: ambiguity — full VPN config grammar (tunnel mode,
    /// auth, keys) not frozen in C-03. Default returns basic
    /// tunnel declaration; real auth wired when contract locks.
    pub fn configure_tunnel(&self, interface: &str) -> String {
        format!("dev {}\n", interface)
    }

    /// Produce a minimal tunnel interface reference string.
    pub fn tunnel_ref(&self, interface: &str) -> String {
        format!("tunnel:{}", interface)
    }
}
