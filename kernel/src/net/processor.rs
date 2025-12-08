//! Network packet processor
//!
//! This module coordinates the processing of network packets across different
//! protocol layers (Ethernet, IP, TCP, UDP, ICMP).

extern crate alloc;
use alloc::vec::Vec;
use super::packet::{Packet, PacketType};
use super::device::{NetworkDevice, MacAddr};
use super::interface::Interface;
use super::arp::{ArpHeader, ArpProcessor, ArpPacket};
use super::ipv4::{Ipv4Addr, Ipv4Packet, Ipv4Header};
use super::icmp::{IcmpPacket, IcmpProcessor};
use super::udp::{UdpPacket, UdpSocket, UdpSocketState};
use super::tcp::{TcpPacket, TcpSocket, TcpState};
use super::route::RoutingTable;
use super::fragment::FragmentReassembler;

/// Packet processing result
#[derive(Debug, Clone)]
pub enum PacketResult {
    /// Packet successfully processed
    Success,
    /// Packet should be forwarded
    Forward(Packet),
    /// Packet should be dropped
    Drop,
    /// Packet requires a response
    Respond(Packet),
}

/// Network packet processor
pub struct NetworkProcessor {
    /// ARP processor
    arp_processor: ArpProcessor,
    /// ICMP processor
    icmp_processor: IcmpProcessor,
    /// Fragment reassembler
    reassembler: FragmentReassembler,
    /// Routing table
    routing_table: RoutingTable,
    /// UDP socket manager
    udp_sockets: Vec<UdpSocket>,
    /// TCP socket manager
    tcp_sockets: Vec<TcpSocket>,
}

impl NetworkProcessor {
    /// Create a new network processor
    pub fn new() -> Self {
        Self {
            arp_processor: ArpProcessor::new(),
            icmp_processor: IcmpProcessor::new(),
            reassembler: FragmentReassembler::new(),
            routing_table: RoutingTable::new(),
            udp_sockets: Vec::new(),
            tcp_sockets: Vec::new(),
        }
    }

    /// Process an incoming packet
    pub fn process_incoming_packet(
        &mut self,
        packet: Packet,
        interface: &Interface,
    ) -> Result<PacketResult, ProcessorError> {
        match packet.packet_type() {
            PacketType::Ethernet => self.process_ethernet_packet(packet, interface),
            PacketType::Arp => self.process_arp_packet(packet, interface),
            PacketType::Ipv4 => self.process_ipv4_packet(packet, interface),
            _ => Ok(PacketResult::Drop),
        }
    }

    /// Process an outgoing packet
    pub fn process_outgoing_packet(
        &mut self,
        mut packet: Packet,
        src_interface: Option<&Interface>,
    ) -> Result<PacketResult, ProcessorError> {
        // Determine routing for outgoing packet
        let dest_ip = self.extract_dest_ip(&packet)?;

        if let Some(route) = self.routing_table.lookup_route(dest_ip) {
            // Find the appropriate interface
            let interface = if let Some(src_interface) = src_interface {
                if src_interface.id() == route.interface_id {
                    src_interface
                } else {
                    return Err(ProcessorError::InvalidInterface);
                }
            } else {
                // Would need to look up interface by ID
                return Err(ProcessorError::InvalidInterface);
            };

            // Apply interface-specific processing
            self.apply_interface_rules(&mut packet, interface)?;

            Ok(PacketResult::Success)
        } else {
            // No route to host
            Ok(PacketResult::Drop)
        }
    }

    /// Process Ethernet packet
    fn process_ethernet_packet(
        &mut self,
        packet: Packet,
        interface: &Interface,
    ) -> Result<PacketResult, ProcessorError> {
        // For now, assume the packet is already parsed and has the correct type
        // In a real implementation, we would parse the Ethernet header here
        match packet.packet_type() {
            PacketType::Arp => self.process_arp_packet(packet, interface),
            PacketType::Ipv4 => self.process_ipv4_packet(packet, interface),
            _ => Ok(PacketResult::Drop),
        }
    }

    /// Process ARP packet
    fn process_arp_packet(
        &mut self,
        packet: Packet,
        interface: &Interface,
    ) -> Result<PacketResult, ProcessorError> {
        let _arp_packet = ArpPacket::from_bytes(packet.data())
            .map_err(|_| ProcessorError::InvalidPacket)?;

        let response = {
            let mut arp_cache = interface.arp_cache().lock();
            self.arp_processor.process_packet(
                packet.data(),
                interface.ipv4_addr().unwrap_or(Ipv4Addr::UNSPECIFIED),
                interface.mac_address(),
                &mut *arp_cache,
            )?
        };

        if let Some(response_header) = response {
            let response_data = response_header.to_bytes();
            let response_packet = Packet::from_bytes(&response_data, PacketType::Arp)
                .map_err(|_| ProcessorError::InvalidPacket)?;

            Ok(PacketResult::Respond(response_packet))
        } else {
            Ok(PacketResult::Success)
        }
    }

    /// Process IPv4 packet
    fn process_ipv4_packet(
        &mut self,
        packet: Packet,
        interface: &Interface,
    ) -> Result<PacketResult, ProcessorError> {
        // Parse IPv4 packet
        let ipv4_packet = Ipv4Packet::from_bytes(packet.data())
            .map_err(|_| ProcessorError::InvalidPacket)?;

        // Check if packet is for this interface
        let is_for_me = interface.is_my_address(ipv4_packet.header.dest_addr);

        if !is_for_me {
            // Check if we should forward this packet
            if ipv4_packet.header.ttl > 1 {
                return Ok(PacketResult::Forward(packet));
            } else {
                // TTL exceeded, send ICMP Time Exceeded
                return self.send_icmp_error(
                    ipv4_packet.header.source_addr,
                    interface.ipv4_addr().unwrap_or(Ipv4Addr::UNSPECIFIED),
                    super::icmp::IcmpType::TimeExceeded,
                    super::icmp::IcmpCode::TtlExceeded,
                    &ipv4_packet.to_bytes(),
                );
            }
        }

        // Handle fragmentation and reassembly
        let reassembled_data = self.reassembler.process_fragment(
            &ipv4_packet.header,
            &ipv4_packet.payload,
        )?;

        let payload = match reassembled_data {
            Some(data) => {
                // Packet was reassembled from fragments
                let reassembled_packet = Ipv4Packet::from_header_and_payload(
                    ipv4_packet.header.clone(),
                    data,
                );
                reassembled_packet.payload
            }
            None => {
                // Not fragmented or incomplete
                return Ok(PacketResult::Success);
            }
        };

        // Process based on protocol
        match ipv4_packet.header.protocol {
            super::ipv4::protocols::ICMP => {
                self.process_icmp_packet(
                    ipv4_packet.header.source_addr,
                    ipv4_packet.header.dest_addr,
                    &payload,
                    interface,
                )
            }
            super::ipv4::protocols::TCP => {
                self.process_tcp_packet(
                    ipv4_packet.header.source_addr,
                    ipv4_packet.header.dest_addr,
                    &payload,
                )
            }
            super::ipv4::protocols::UDP => {
                self.process_udp_packet(
                    ipv4_packet.header.source_addr,
                    ipv4_packet.header.dest_addr,
                    &payload,
                )
            }
            _ => Ok(PacketResult::Drop),
        }
    }

    /// Process ICMP packet
    fn process_icmp_packet(
        &mut self,
        src_addr: Ipv4Addr,
        dest_addr: Ipv4Addr,
        data: &[u8],
        interface: &Interface,
    ) -> Result<PacketResult, ProcessorError> {
        let icmp_packet = IcmpPacket::from_bytes(data)
            .map_err(|_| ProcessorError::InvalidPacket)?;

        let response = self.icmp_processor.process_packet(
            src_addr,
            dest_addr,
            icmp_packet,
        );

        if let Some(response_packet) = response {
            // Create IPv4 packet for ICMP response
            let ipv4_response = Ipv4Packet::new(
                interface.ipv4_addr().unwrap_or(Ipv4Addr::UNSPECIFIED),
                src_addr,
                super::ipv4::protocols::ICMP,
                response_packet.to_bytes(),
                64, // Default TTL
            );

            let packet = Packet::from_bytes(&ipv4_response.to_bytes(), PacketType::Ipv4)
                .map_err(|_| ProcessorError::InvalidPacket)?;

            Ok(PacketResult::Respond(packet))
        } else {
            Ok(PacketResult::Success)
        }
    }

    /// Process TCP packet
    fn process_tcp_packet(
        &mut self,
        src_addr: Ipv4Addr,
        dst_addr: Ipv4Addr,
        data: &[u8],
    ) -> Result<PacketResult, ProcessorError> {
        let tcp_packet = TcpPacket::from_bytes(data)
            .map_err(|_| ProcessorError::InvalidPacket)?;

        // Find matching TCP socket
        let matching_socket_idx = self.tcp_sockets.iter_mut().position(|socket| {
            socket.local_ip == dst_addr && socket.local_port == tcp_packet.src_port()
        });

        if let Some(idx) = matching_socket_idx {
            // Process the packet with the matching socket
            let socket = &mut self.tcp_sockets[idx];

            // Update socket state based on TCP flags and sequence numbers
            if tcp_packet.has_flag(super::tcp::tcp_flags::SYN) {
                if socket.state == TcpState::Listen {
                    // Transition to SYN_RECEIVED
                    socket.state = TcpState::SynReceived;
                    // TODO: Send SYN-ACK
                    return Ok(PacketResult::Drop);
                }
            }

            // Handle data packets
            if tcp_packet.payload.len() > 0 {
                // TODO: Buffer received data
                crate::log_info!("TCP received {} bytes from {}", tcp_packet.payload.len(), src_addr);
            }

            return Ok(PacketResult::Drop);
        }

        // No matching socket found
        Ok(PacketResult::Drop)
    }

    /// Process UDP packet
    fn process_udp_packet(
        &mut self,
        src_addr: Ipv4Addr,
        dst_addr: Ipv4Addr,
        data: &[u8],
    ) -> Result<PacketResult, ProcessorError> {
        let udp_packet = UdpPacket::from_bytes(data)
            .map_err(|_| ProcessorError::InvalidPacket)?;

        // Find matching UDP socket index first
        let matching_socket_idx = self.udp_sockets.iter_mut().position(|socket| {
            socket.is_bound() &&
            socket.local_port == udp_packet.dst_port() &&
            (socket.local_ip == Ipv4Addr::UNSPECIFIED || socket.local_ip == dst_addr)
        });

        if let Some(idx) = matching_socket_idx {
            // Extract socket from vector to avoid borrow conflicts
            let mut socket = self.udp_sockets.swap_remove(idx);
            let result = self.deliver_udp_packet(&mut socket, &udp_packet, src_addr);
            // Put the socket back
            self.udp_sockets.push(socket);
            return result;
        }

        // No matching socket found, send ICMP Port Unreachable
        self.send_icmp_port_unreachable(src_addr, dst_addr, data)
    }

    /// Handle TCP packet for a specific socket
    fn handle_tcp_socket_packet(
        &mut self,
        socket: &mut TcpSocket,
        packet: &TcpPacket,
        _src_addr: Ipv4Addr,
    ) -> Result<PacketResult, ProcessorError> {
        // Update socket state based on TCP flags and sequence numbers
        // This is a simplified implementation
        if packet.has_flag(super::tcp::tcp_flags::SYN) {
            if socket.state == TcpState::Listen {
                // Transition to SYN_RECEIVED
                socket.state = TcpState::SynReceived;
                socket.rcv_nxt = packet.seq_num() + 1;

                // Send SYN-ACK response
                return Ok(PacketResult::Success); // Would create response packet
            }
        } else if packet.has_flag(super::tcp::tcp_flags::ACK) {
            // Handle ACK
            socket.snd_una = packet.ack_num();
        }

        // Handle data payload
        if !packet.payload.is_empty() {
            // Process received data
            socket.rcv_nxt = packet.seq_num() + packet.payload.len() as u32;
        }

        Ok(PacketResult::Success)
    }

    /// Deliver UDP packet to socket
    fn deliver_udp_packet(
        &mut self,
        _socket: &mut UdpSocket,
        _packet: &UdpPacket,
        _src_addr: Ipv4Addr,
    ) -> Result<PacketResult, ProcessorError> {
        // In a real implementation, this would queue data for the socket
        // For now, just acknowledge receipt
        Ok(PacketResult::Success)
    }

    /// Send ICMP error message
    fn send_icmp_error(
        &mut self,
        dest_addr: Ipv4Addr,
        src_addr: Ipv4Addr,
        icmp_type: super::icmp::IcmpType,
        icmp_code: super::icmp::IcmpCode,
        original_data: &[u8],
    ) -> Result<PacketResult, ProcessorError> {
        // Create ICMP error packet
        let mut error_data = Vec::new();
        // Include original IP header + first 8 bytes of data
        error_data.extend_from_slice(&original_data[..core::cmp::min(original_data.len(), 28)]);

        let icmp_packet = super::icmp::IcmpPacket::new(
            icmp_type,
            icmp_code,
            0, // Rest of header (would be set appropriately)
            error_data,
        );

        // Create IPv4 packet
        let ipv4_packet = Ipv4Packet::new(
            src_addr,
            dest_addr,
            super::ipv4::protocols::ICMP,
            icmp_packet.to_bytes(),
            64,
        );

        let packet = Packet::from_bytes(&ipv4_packet.to_bytes(), PacketType::Ipv4)
            .map_err(|_| ProcessorError::InvalidPacket)?;

        Ok(PacketResult::Respond(packet))
    }

    /// Send ICMP Port Unreachable
    fn send_icmp_port_unreachable(
        &mut self,
        src_addr: Ipv4Addr,
        dst_addr: Ipv4Addr,
        original_data: &[u8],
    ) -> Result<PacketResult, ProcessorError> {
        self.send_icmp_error(
            src_addr,
            dst_addr,
            super::icmp::IcmpType::DestinationUnreachable,
            super::icmp::IcmpCode::PortUnreachable,
            original_data,
        )
    }

    /// Extract destination IP from packet
    fn extract_dest_ip(&self, packet: &Packet) -> Result<Ipv4Addr, ProcessorError> {
        match packet.packet_type() {
            PacketType::Ipv4 => {
                let ipv4_packet = Ipv4Packet::from_bytes(packet.data())
                    .map_err(|_| ProcessorError::InvalidPacket)?;
                Ok(ipv4_packet.header.dest_addr)
            }
            _ => Err(ProcessorError::UnsupportedPacketType),
        }
    }

    /// Apply interface-specific rules to packet
    fn apply_interface_rules(
        &self,
        packet: &mut Packet,
        interface: &Interface,
    ) -> Result<(), ProcessorError> {
        // Apply MTU limits, adjust headers, etc.
        if packet.len() > interface.mtu() {
            return Err(ProcessorError::PacketTooLarge);
        }

        Ok(())
    }

    /// Clean up expired reassembly entries
    pub fn cleanup(&mut self) {
        self.reassembler.cleanup();
    }

    /// Get processor statistics
    pub fn stats(&self) -> ProcessorStats {
        ProcessorStats {
            udp_sockets: self.udp_sockets.len(),
            tcp_sockets: self.tcp_sockets.len(),
            reassembly_stats: self.reassembler.stats(),
            routing_stats: self.routing_table.stats(),
        }
    }

    /// Get routing table reference
    pub fn routing_table(&self) -> &RoutingTable {
        &self.routing_table
    }

    /// Get mutable routing table reference
    pub fn routing_table_mut(&mut self) -> &mut RoutingTable {
        &mut self.routing_table
    }
}

impl Default for NetworkProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Processor statistics
#[derive(Debug, Clone)]
pub struct ProcessorStats {
    /// Number of UDP sockets
    pub udp_sockets: usize,
    /// Number of TCP sockets
    pub tcp_sockets: usize,
    /// Reassembly statistics
    pub reassembly_stats: super::fragment::FragmentReassemblerStats,
    /// Routing statistics
    pub routing_stats: super::route::RoutingTableStats,
}

/// Packet processing errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessorError {
    /// Invalid packet format
    InvalidPacket,
    /// Unsupported packet type
    UnsupportedPacketType,
    /// Invalid interface
    InvalidInterface,
    /// Packet too large
    PacketTooLarge,
    /// Routing error
    RoutingError,
    /// Fragmentation error
    FragmentationError,
}

// Convert from ArpError to ProcessorError
impl From<super::arp::ArpError> for ProcessorError {
    fn from(_error: super::arp::ArpError) -> Self {
        ProcessorError::InvalidPacket
    }
}

// Convert from FragmentError to ProcessorError
impl From<super::fragment::FragmentError> for ProcessorError {
    fn from(_error: super::fragment::FragmentError) -> Self {
        ProcessorError::FragmentationError
    }
}
