//! IPv6 protocol tests
//!
//! Comprehensive tests for IPv6 protocol implementation

extern crate alloc;

use crate::subsystems::net::ipv6;
use crate::subsystems::net::ipv6::icmpv6::*;

#[cfg(test)]
mod ipv6_tests {
    use super::*;

    #[test]
    fn test_ipv6_addr_constants() {
        // Test unspecified
        assert!(ipv6::Ipv6Addr::UNSPECIFIED.is_unspecified());

        // Test loopback
        assert!(ipv6::Ipv6Addr::LOCALHOST.is_loopback());
        assert_eq!(ipv6::Ipv6Addr::LOCALHOST.octets()[15], 1);

        // Test multicast
        assert!(ipv6::Ipv6Addr::ALL_NODES_MULTICAST.is_multicast());
        assert!(ipv6::Ipv6Addr::ALL_ROUTERS_MULTICAST.is_multicast());
    }

    #[test]
    fn test_ipv6_addr_creation() {
        // Test from octets
        let octets = [0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
        let addr = ipv6::Ipv6Addr::new(octets);
        assert_eq!(addr.octets(), octets);

        // Test from segments
        let segments = [0x2001, 0x0db8, 0, 0, 0, 0, 0, 1];
        let addr = ipv6::Ipv6Addr::from_segments(segments);
        assert_eq!(addr.segments(), segments);
    }

    #[test]
    fn test_ipv6_addr_properties() {
        // Test link-local
        let link_local = ipv6::Ipv6Addr([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        assert!(link_local.is_unicast_link_local());

        // Test unique local
        let ula = ipv6::Ipv6Addr([0xfc, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        assert!(ula.is_unique_local());

        // Test multicast
        let multicast = ipv6::Ipv6Addr([0xff, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        assert!(multicast.is_multicast());
    }

    #[test]
    fn test_ipv6_addr_parsing() {
        // Test basic parsing
        let addr = ipv6::Ipv6Addr::from_str("2001:db8::1").unwrap();
        assert_eq!(addr.segments()[0], 0x2001);
        assert_eq!(addr.segments()[1], 0x0db8);
        assert_eq!(addr.segments()[7], 1);

        // Test loopback parsing
        let loopback = ipv6::Ipv6Addr::from_str("::1").unwrap();
        assert!(loopback.is_loopback());

        // Test compression
        let addr1 = ipv6::Ipv6Addr::from_str("2001:db8:0:0:0:0:0:1").unwrap();
        let addr2 = ipv6::Ipv6Addr::from_str("2001:db8::1").unwrap();
        assert_eq!(addr1, addr2);
    }

    #[test]
    fn test_ipv6_addr_display() {
        let addr = ipv6::Ipv6Addr([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let s = alloc::format!("{}", addr);
        assert!(s.contains("2001") || s.contains("2001:db8::1"));
    }

    #[test]
    fn test_ipv6_prefix() {
        // Test prefix creation
        let network = ipv6::Ipv6Addr([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let prefix = ipv6::Ipv6Prefix::new(network, 32);
        assert_eq!(prefix.prefix_len, 32);

        // Test prefix matching
        let addr1 = ipv6::Ipv6Addr([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let addr2 = ipv6::Ipv6Addr([0x20, 0x01, 0x0d, 0xb9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        assert!(prefix.contains(addr1));
        assert!(!prefix.contains(addr2));

        // Test link-local prefix
        let ll_prefix = ipv6::Ipv6Prefix::link_local();
        let ll_addr = ipv6::Ipv6Addr([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        assert!(ll_prefix.contains(ll_addr));
    }

    #[test]
    fn test_ipv6_header() {
        // Test header creation
        let src = ipv6::Ipv6Addr::LOCALHOST;
        let dst = ipv6::Ipv6Addr([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let mut header = ipv6::Ipv6Header::new(src, dst, ipv6::protocols::TCP, 1000, ipv6::DEFAULT_HOP_LIMIT);

        // Test version
        assert_eq!(header.version(), 6);

        // Test traffic class
        header.set_dscp(0x12);
        assert_eq!(header.dscp(), 0x12);

        header.set_ecn(0x01);
        assert_eq!(header.ecn(), 0x01);

        // Test flow label
        header.set_flow_label(0x12345);
        assert_eq!(header.flow_label(), 0x12345);

        // Test serialization
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), ipv6::Ipv6Header::HEADER_SIZE);

        // Test parsing
        let parsed = ipv6::Ipv6Header::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.version(), 6);
        assert_eq!(parsed.source_addr, src);
        assert_eq!(parsed.dest_addr, dst);
        assert_eq!(parsed.next_header, ipv6::protocols::TCP);
        assert_eq!(parsed.payload_length, 1000);
    }

    #[test]
    fn test_ipv6_packet() {
        // Test packet creation
        let src = ipv6::Ipv6Addr::LOCALHOST;
        let dst = ipv6::Ipv6Addr([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let payload = vec![0x01, 0x02, 0x03, 0x04];
        let packet = ipv6::Ipv6Packet::new(src, dst, ipv6::protocols::TCP, payload.clone(), 64);

        // Test serialization
        let bytes = packet.to_bytes();
        assert!(bytes.len() >= ipv6::Ipv6Header::HEADER_SIZE + payload.len());

        // Test parsing
        let parsed = ipv6::Ipv6Packet::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.header.source_addr, src);
        assert_eq!(parsed.header.dest_addr, dst);
        assert_eq!(parsed.payload, payload);
    }

    #[test]
    fn test_ipv6_extension_header() {
        // Test fragment header
        let ext = ipv6::ExtensionHeader::Fragment {
            next_header: ipv6::protocols::TCP,
            fragment_offset: 123,
            more_fragments: true,
            identification: 0x12345678,
        };

        assert_eq!(ext.next_header(), ipv6::protocols::TCP);
        assert_eq!(ext.len(), 8);

        // Test serialization and parsing
        let bytes = ext.to_bytes();
        let parsed = ipv6::ExtensionHeader::from_bytes(&bytes, ipv6::protocols::FRAGMENT).unwrap();
        match parsed {
            ipv6::ExtensionHeader::Fragment { fragment_offset, more_fragments, identification, .. } => {
                assert_eq!(fragment_offset, 123);
                assert!(more_fragments);
                assert_eq!(identification, 0x12345678);
            }
            _ => panic!("Expected Fragment header"),
        }
    }

    #[test]
    fn test_ipv6_routing_table() {
        let mut table = ipv6::Ipv6RoutingTable::new();

        // Add routes
        let ll_route = ipv6::Ipv6RouteEntry::connected(ipv6::Ipv6Prefix::link_local(), 1);
        table.add_route(ll_route);

        let gateway = ipv6::Ipv6Addr([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);
        let default_route = ipv6::Ipv6RouteEntry::default(gateway, 1, 1);
        table.add_route(default_route);

        // Test lookup
        let ll_addr = ipv6::Ipv6Addr([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let route = table.lookup(ll_addr).unwrap();
        assert!(route.is_connected());
        assert!(route.dest.contains(ll_addr));

        // Test default route lookup
        let internet = ipv6::Ipv6Addr([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let route = table.lookup(internet).unwrap();
        assert!(route.is_default());

        // Test statistics
        let stats = table.stats();
        assert_eq!(stats.total_routes, 2);
        assert!(stats.lookups > 0);
        assert!(stats.cache_hits > 0);
    }

    #[test]
    fn test_ipv6_longest_prefix_match() {
        let mut table = ipv6::Ipv6RoutingTable::new();

        // Add overlapping routes with different prefix lengths
        let prefix32 = ipv6::Ipv6Prefix::new(
            ipv6::Ipv6Addr([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            32,
        );
        table.add_route(ipv6::Ipv6RouteEntry::connected(prefix32, 1));

        let prefix48 = ipv6::Ipv6Prefix::new(
            ipv6::Ipv6Addr([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            48,
        );
        table.add_route(ipv6::Ipv6RouteEntry::connected(prefix48, 1));

        // Lookup should match the /48 (longer prefix)
        let addr = ipv6::Ipv6Addr([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let route = table.lookup(addr).unwrap();
        assert_eq!(route.dest.prefix_len, 48);
    }
}

#[cfg(test)]
mod icmpv6_tests {
    use super::*;
    use crate::subsystems::net::ipv6 as ipv6_mod;

    #[test]
    fn test_icmpv6_packet_creation() {
        // Test echo request
        let echo_req = Icmpv6Packet::echo_request(12345, 1, vec![0x01, 0x02]);
        assert_eq!(echo_req.header.message_type, Icmpv6Type::EchoRequest);
        assert_eq!(echo_req.identifier(), 12345);
        assert_eq!(echo_req.sequence(), 1);

        // Test echo reply
        let echo_reply = Icmpv6Packet::echo_reply(12345, 1, vec![0x01, 0x02]);
        assert_eq!(echo_reply.header.message_type, Icmpv6Type::EchoReply);
    }

    #[test]
    fn test_icmpv6_neighbor_solicitation() {
        let target = ipv6_mod::Ipv6Addr([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let ns = Icmpv6Packet::neighbor_solicitation(target);
        assert_eq!(ns.header.message_type, Icmpv6Type::NeighborSolicitation);
        assert!(ns.payload.len() >= 16);
    }

    #[test]
    fn test_icmpv6_neighbor_advertisement() {
        let target = ipv6_mod::Ipv6Addr([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let na = Icmpv6Packet::neighbor_advertisement(target, true, true, false);
        assert_eq!(na.header.message_type, Icmpv6Type::NeighborAdvertisement);
    }

    #[test]
    fn test_icmpv6_packet_too_big() {
        let invoking_packet = vec![0x01, 0x02, 0x03];
        let ptb = Icmpv6Packet::packet_to_big(1280, &invoking_packet);
        assert_eq!(ptb.header.message_type, Icmpv6Type::PacketTooBig);
        assert!(ptb.payload.len() >= 4);
    }

    #[test]
    fn test_icmpv6_checksum() {
        let src = ipv6_mod::Ipv6Addr::LOCALHOST;
        let dst = ipv6_mod::Ipv6Addr([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let mut packet = Icmpv6Packet::echo_request(12345, 1, vec![0x01, 0x02]);

        // Set checksum
        packet.set_checksum(src, dst);

        // Verify checksum
        assert!(packet.verify_checksum(src, dst));
    }

    #[test]
    fn test_icmpv6_processor() {
        let processor = Icmpv6Processor::new();
        let src = ipv6_mod::Ipv6Addr([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let dst = ipv6_mod::Ipv6Addr::LOCALHOST;

        let mut echo_req = Icmpv6Packet::echo_request(12345, 1, vec![0x01, 0x02]);
        echo_req.set_checksum(src, dst);

        // Process echo request - should generate reply
        let reply = processor.process_packet(src, dst, echo_req);
        assert!(reply.is_some());

        let reply = reply.unwrap();
        assert_eq!(reply.header.message_type, Icmpv6Type::EchoReply);
        assert_eq!(reply.identifier(), 12345);
        assert_eq!(reply.sequence(), 1);
    }

    #[test]
    fn test_ndp_cache() {
        let mut cache = NdpCache::new(10);

        // Add entry
        let entry = NdpEntry {
            address: ipv6_mod::Ipv6Addr([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]),
            lladdr: [0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
            state: NdpState::Reachable,
            created_at: 1000,
            expires_at: 2000,
        };
        cache.insert(entry.clone());

        // Lookup
        let found = cache.lookup(entry.address).unwrap();
        assert_eq!(found.lladdr, entry.lladdr);
        assert_eq!(found.state, NdpState::Reachable);

        // Remove
        assert!(cache.remove(entry.address));
        assert!(cache.lookup(entry.address).is_none());
    }

    #[test]
    fn test_ndp_cache_expiration() {
        let mut cache = NdpCache::new(10);

        let entry = NdpEntry {
            address: ipv6_mod::Ipv6Addr([0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]),
            lladdr: [0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
            state: NdpState::Reachable,
            created_at: 1000,
            expires_at: 2000,
        };
        cache.insert(entry);

        // Prune expired entries
        cache.prune_expired(3000);
        assert!(cache.lookup(entry.address).is_none());
    }
}
