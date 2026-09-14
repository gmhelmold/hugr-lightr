//! DNS resolver (WP-03 / C-03 frozen contract stub).
//!
//! Minimal Docker-faithful default: synthesize `/etc/resolv.conf`
//! pointing to the subnet gateway (`nameserver <gateway>`).
//! Ambiguity noted: full DNS query grammar (A/AAAA/CNAME/forward)
//! is NOT frozen in contract C-03; this default keeps the gateway
//! as the DNS server reference (matching Docker's `dns` behavior
//! on user-defined networks) until the grammar is locked.

use super::types::{Member, Subnet};

/// Minimal DNS resolver — synthesizes `/etc/resolv.conf` from
/// the network subnet gateway (the L2 switch / embedded DHCP
/// DNS server per ADR-0018 §4).
pub struct DnsResolver;

impl DnsResolver {
    /// Names production switch must register for one spawn-time membership.
    pub fn member_names<'a>(
        &self,
        name: &'a str,
        aliases: &'a [String],
    ) -> impl Iterator<Item = &'a str> {
        std::iter::once(name).chain(aliases.iter().map(String::as_str))
    }

    /// Resolve spawn-time membership names and aliases from registry snapshot.
    /// No record means no answer; callers must not substitute gateway address.
    pub fn resolve_member(&self, name: &str, members: &[Member]) -> Option<std::net::Ipv4Addr> {
        members
            .iter()
            .find(|member| member.name == name || member.aliases.iter().any(|alias| alias == name))
            .map(|member| member.ip)
    }

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

    #[test]
    fn s3_network_dns_resolves_member_name_and_alias() {
        let resolver = DnsResolver;
        let api = Member {
            name: "api".to_string(),
            aliases: vec!["backend".to_string()],
            mac: super::super::types::MacAddr([0x0a, 0, 0, 0, 0, 2]),
            ip: Ipv4Addr::new(10, 69, 7, 2),
            ports: Vec::new(),
        };

        assert_eq!(resolver.resolve_member("api", &[api.clone()]), Some(api.ip));
        assert_eq!(
            resolver.resolve_member("backend", &[api]),
            Some(Ipv4Addr::new(10, 69, 7, 2))
        );
        assert_eq!(resolver.resolve_member("missing", &[]), None);
        assert_eq!(
            resolver
                .member_names("api", &["backend".to_string()])
                .collect::<Vec<_>>(),
            ["api", "backend"]
        );
    }
}
