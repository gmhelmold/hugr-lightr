//! DNS resolver (WP-03 / C-03 frozen contract stub).
//!
//! Minimal Docker-faithful default: synthesize `/etc/resolv.conf`
//! pointing to the subnet gateway (`nameserver <gateway>`).
//! Ambiguity noted: full DNS query grammar (A/AAAA/CNAME/forward)
//! is NOT frozen in contract C-03; this default keeps the gateway
//! as the DNS server reference (matching Docker's `dns` behavior
//! on user-defined networks) until the grammar is locked.

use super::types::Subnet;

/// Minimal DNS resolver — synthesizes `/etc/resolv.conf` from
/// the network subnet gateway (the L2 switch / embedded DHCP
/// DNS server per ADR-0018 §4).
pub struct DnsResolver;

impl DnsResolver {
    /// Resolve a hostname within this network.
    ///
    /// NOTE: ambiguity — exact DNS grammar (A/AAAA/CNAME,
    /// upstream forward, search domains) is not frozen in C-03.
    /// Default: return gateway IP as the synthesized DNS server
    /// reference; real A-record resolution deferred.
    pub fn resolve(&self, _name: &str, subnet: &Subnet) -> Option<std::net::Ipv4Addr> {
        Some(subnet.gateway)
    }

    /// Synthesize `/etc/resolv.conf` content for a container
    /// attached to this network.
    pub fn synthesize_resolv_conf(&self, subnet: &Subnet) -> String {
        format!("nameserver {}\nsearch local\n", subnet.gateway)
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::Subnet;
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn resolve_uses_subnet_gateway() {
        let subnet = Subnet {
            base: Ipv4Addr::new(10, 69, 7, 0),
            prefix: 24,
            gateway: Ipv4Addr::new(10, 69, 7, 1),
        };
        let resolver = DnsResolver;
        assert_eq!(resolver.resolve("host", &subnet), Some(subnet.gateway));
    }

    #[test]
    fn synthesize_has_gateway_and_search() {
        let subnet = Subnet {
            base: Ipv4Addr::new(10, 69, 3, 0),
            prefix: 24,
            gateway: Ipv4Addr::new(10, 69, 3, 1),
        };
        let resolver = DnsResolver;
        let conf = resolver.synthesize_resolv_conf(&subnet);
        assert!(conf.contains("nameserver "));
        assert!(conf.contains("10.69.3.1"));
        assert!(conf.contains("search local"));
    }
}
