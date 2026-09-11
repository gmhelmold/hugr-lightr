//! WP-03 (C-03) re-export layer — new networking modules.
//!
//! These modules extend the frozen `network` interface without
//! changing `mod.rs`'s existing public surface (`NetworkRegistry`,
//! `MacAddr`, `Member`, `NetworkId`, `Subnet`).
//!
//! Ambiguities documented in C-03 notes (see .techlead/contracts/):
//! - DNS grammar not fully frozen (dns.rs `resolve()` notes this).
//! - VPN config format not fully frozen (vpn.rs `configure_tunnel()` notes this).

pub use super::bridge::BridgeCreate;
pub use super::dns::DnsResolver;
pub use super::ipam::IpamAllocator;
pub use super::vpn::VpnManager;
