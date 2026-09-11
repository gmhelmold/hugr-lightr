//! IPAM allocator — subnet/IP allocation (WP-03 / C-03 frozen contract stub).
//!
//! Uses frozen `alloc::subnet_for` (`alloc.rs`) for deterministic
//! `/24` derivation (`10.69.<k>.0/24`, `k` = `blake3(id)[0]`) and
//! frozen `types::Subnet` / `types::Member` for in-memory model.
//! Integration point: `NetworkRegistry::join()` consumes this
//! allocator (existing frozen path, preserved unchanged).

use super::alloc::subnet_for;
use super::types::{NetworkId, Subnet};

/// IPAM allocator — deterministic subnet + host IP assignment.
pub struct IpamAllocator;

impl IpamAllocator {
    /// Allocate a deterministic `/24` subnet for a network id.
    /// Matches `alloc::subnet_for` exactly (same hash source).
    pub fn allocate_subnet(id: &NetworkId) -> Subnet {
        subnet_for(id)
    }

    /// Check if a candidate host IP is available within the subnet,
    /// skipping the gateway (`.1`). Delegates to frozen registry logic.
    pub fn is_available(
        &self,
        subnet: &Subnet,
        candidate: std::net::Ipv4Addr,
        taken: &[std::net::Ipv4Addr],
    ) -> bool {
        if candidate == subnet.gateway {
            return false;
        }
        if candidate.octets()[3] == 0 {
            return false; // network address reserved
        }
        if candidate.octets()[3] == 255 {
            return false; // broadcast reserved
        }
        taken.iter().all(|t| *t != candidate)
    }
}
