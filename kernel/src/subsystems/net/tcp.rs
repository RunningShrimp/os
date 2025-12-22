//! Transmission Control Protocol (TCP) implementation
//!
//! This module provides TCP protocol support for reliable stream-oriented communication.

extern crate alloc;
use alloc::vec::Vec;

use super::ipv4::Ipv4Addr;

pub mod state;
pub mod manager;

/// TCP header
#[derive(Debug, Clone, Copy)]
pub struct TcpHeader {
    /// Source port
    pub src_port: u16,
    /// Destination port
    pub dst_port: u16,
    /// Sequence number
    pub seq_num: u32,
    /// Acknowledgment number
    pub ack_num: u32,
    /// Data offset and reserved flags
    pub data_offset_reserved: u8,
    /// Flags
    pub flags: u8,
    /// Window size
    pub window_size: u16,
    /// Checksum
    pub checksum: u16,
    /// Urgent pointer
    pub urgent_ptr: u16,
}

impl TcpHeader {
    /// Size of TCP header without options in bytes
    pub const MIN_SIZE: usize = 20;

    /// Maximum header size with options in bytes
    pub const MAX_SIZE: usize = 60;
}

/// TCP flags
pub mod tcp_flags {
    /// FIN flag - No more data from sender
    pub const FIN: u8 = 0x01;
    /// SYN flag - Synchronize sequence numbers
    pub const SYN: u8 = 0x02;
    /// RST flag - Reset the connection
    pub const RST: u8 = 0x04;
    /// PSH flag - Push function
    pub const PSH: u8 = 0x08;
    /// ACK flag - Acknowledgment field significant
    pub const ACK: u8 = 0x10;
    /// URG flag - Urgent pointer field significant
    pub const URG: u8 = 0x20;
    /// ECE flag - ECN-Echo
    pub const ECE: u8 = 0x40;
    /// CWR flag - Congestion Window Reduced
    pub const CWR: u8 = 0x80;
}

impl TcpHeader {
    /// Create a new TCP header
    pub fn new(
        src_port: u16,
        dst_port: u16,
        seq_num: u32,
        ack_num: u32,
        flags: u8,
        window_size: u16,
    ) -> Self {
        Self {
            src_port,
            dst_port,
            seq_num,
            ack_num,
            data_offset_reserved: (Self::MIN_SIZE as u8 / 4) << 4, // Data offset in 32-bit words
            flags,
            window_size,
            checksum: 0,
            urgent_ptr: 0,
        }
    }

    /// Get data offset (header length in 32-bit words)
    pub fn data_offset(&self) -> u8 {
        (self.data_offset_reserved >> 4) & 0x0F
    }

    /// Set data offset
    pub fn set_data_offset(&mut self, offset: u8) {
        self.data_offset_reserved = (offset << 4) | (self.data_offset_reserved & 0x0F);
    }

    /// Get header size in bytes
    pub fn header_size(&self) -> usize {
        (self.data_offset() as usize) * 4
    }

    /// Check if flag is set
    pub fn has_flag(&self, flag: u8) -> bool {
        (self.flags & flag) != 0
    }

    /// Calculate TCP checksum (including pseudo-header)
    pub fn calculate_checksum(
        &self,
        source_addr: Ipv4Addr,
        dest_addr: Ipv4Addr,
        data: &[u8],
    ) -> u16 {
        let mut sum = 0u32;

        // Pseudo-header
        sum += source_addr.to_u32() >> 16;
        sum += source_addr.to_u32() & 0xFFFF;
        sum += dest_addr.to_u32() >> 16;
        sum += dest_addr.to_u32() & 0xFFFF;
        sum += 6; // TCP protocol
        sum += (self.header_size() + data.len()) as u32;

        // TCP header
        sum += self.src_port as u32;
        sum += self.dst_port as u32;
        sum += self.seq_num;
        sum += self.ack_num;
        sum += ((self.data_offset_reserved as u32) << 8) | (self.flags as u32);
        sum += self.window_size as u32;
        sum += self.checksum as u32;
        sum += self.urgent_ptr as u32;

        // Data
        let mut i = 0;
        while i < data.len() {
            if i + 1 < data.len() {
                sum += (((data[i] as u16) << 8) | (data[i + 1] as u16)) as u32;
                i += 2;
            } else {
                sum += ((data[i] as u16) << 8) as u32;
                i += 1;
            }
        }

        // Add carry
        while sum >> 16 != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        !sum as u16
    }

    /// Set checksum
    pub fn set_checksum(
        &mut self,
        source_addr: Ipv4Addr,
        dest_addr: Ipv4Addr,
        data: &[u8],
    ) {
        self.checksum = 0;
        self.checksum = self.calculate_checksum(source_addr, dest_addr, data);
    }

    /// Serialize header to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.header_size());

        bytes.extend_from_slice(&self.src_port.to_be_bytes());
        bytes.extend_from_slice(&self.dst_port.to_be_bytes());
        bytes.extend_from_slice(&self.seq_num.to_be_bytes());
        bytes.extend_from_slice(&self.ack_num.to_be_bytes());
        bytes.push(self.data_offset_reserved);
        bytes.push(self.flags);
        bytes.extend_from_slice(&self.window_size.to_be_bytes());
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes.extend_from_slice(&self.urgent_ptr.to_be_bytes());

        // Add padding for options if needed
        while bytes.len() < self.header_size() {
            bytes.push(0);
        }

        bytes
    }

    /// Parse header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, TcpError> {
        if bytes.len() < Self::MIN_SIZE {
            return Err(TcpError::PacketTooSmall);
        }

        let src_port = u16::from_be_bytes([bytes[0], bytes[1]]);
        let dst_port = u16::from_be_bytes([bytes[2], bytes[3]]);
        let seq_num = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let ack_num = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let data_offset_reserved = bytes[12];
        let flags = bytes[13];
        let window_size = u16::from_be_bytes([bytes[14], bytes[15]]);
        let checksum = u16::from_be_bytes([bytes[16], bytes[17]]);
        let urgent_ptr = u16::from_be_bytes([bytes[18], bytes[19]]);

        let data_offset = (data_offset_reserved >> 4) & 0x0F;
        if data_offset < 5 || data_offset > 15 {
            return Err(TcpError::InvalidHeaderLength);
        }

        let header_size = (data_offset as usize) * 4;
        if bytes.len() < header_size {
            return Err(TcpError::PacketTooSmall);
        }

        Ok(Self {
            src_port,
            dst_port,
            seq_num,
            ack_num,
            data_offset_reserved,
            flags,
            window_size,
            checksum,
            urgent_ptr,
        })
    }
}

/// TCP packet
#[derive(Debug, Clone)]
pub struct TcpPacket {
    /// Header
    pub header: TcpHeader,
    /// Options (if any)
    pub options: Vec<u8>,
    /// Payload data
    pub payload: Vec<u8>,
}

impl TcpPacket {
    /// Create a new TCP packet
    pub fn new(
        src_port: u16,
        dst_port: u16,
        seq_num: u32,
        ack_num: u32,
        flags: u8,
        window_size: u16,
        payload: Vec<u8>,
        source_addr: Ipv4Addr,
        dest_addr: Ipv4Addr,
    ) -> Self {
        let mut header = TcpHeader::new(src_port, dst_port, seq_num, ack_num, flags, window_size);

        // Set checksum (includes options and payload)
        let total_data = [&payload[..]].concat(); // Options would be concatenated here
        header.set_checksum(source_addr, dest_addr, &total_data);

        Self {
            header,
            options: Vec::new(), // Options could be added here
            payload,
        }
    }

    /// Get source port
    pub fn src_port(&self) -> u16 {
        self.header.src_port
    }

    /// Get destination port
    pub fn dst_port(&self) -> u16 {
        self.header.dst_port
    }

    /// Get sequence number
    pub fn seq_num(&self) -> u32 {
        self.header.seq_num
    }

    /// Get acknowledgment number
    pub fn ack_num(&self) -> u32 {
        self.header.ack_num
    }

    /// Get header size
    pub fn header_size(&self) -> usize {
        self.header.header_size()
    }

    /// Get payload size
    pub fn payload_size(&self) -> usize {
        self.payload.len()
    }

    /// Get total packet size
    pub fn len(&self) -> usize {
        self.header_size() + self.options.len() + self.payload.len()
    }

    /// Check if flag is set
    pub fn has_flag(&self, flag: u8) -> bool {
        self.header.has_flag(flag)
    }

    /// Serialize packet to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();
        bytes.extend_from_slice(&self.options);
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    /// Parse packet from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, TcpError> {
        let header = TcpHeader::from_bytes(bytes)?;
        let header_size = header.header_size();

        if bytes.len() < header_size {
            return Err(TcpError::PacketTooSmall);
        }

        let options_end = header_size;
        let options = if options_end > TcpHeader::MIN_SIZE {
            bytes[TcpHeader::MIN_SIZE..options_end].to_vec()
        } else {
            Vec::new()
        };

        let payload = bytes[options_end..].to_vec();

        Ok(Self {
            header,
            options,
            payload,
        })
    }

    /// Verify checksum
    pub fn verify_checksum(
        &self,
        source_addr: Ipv4Addr,
        dest_addr: Ipv4Addr,
    ) -> bool {
        let total_data = [&self.options[..], &self.payload[..]].concat();
        self.header.calculate_checksum(source_addr, dest_addr, &total_data) == 0
    }
}

/// TCP connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    /// CLOSED - No connection is active or pending
    Closed,
    /// LISTEN - Waiting for a connection request from any remote TCP
    Listen,
    /// SYN-SENT - Waiting for a matching connection request after having sent a connection request
    SynSent,
    /// SYN-RECEIVED - Waiting for a confirming connection request acknowledgment after having both received and sent a connection request
    SynReceived,
    /// ESTABLISHED - Connection is established and data can be exchanged
    Established,
    /// FIN-WAIT-1 - Waiting for a connection termination request from the remote TCP
    FinWait1,
    /// FIN-WAIT-2 - Waiting for a connection termination request from the remote TCP
    FinWait2,
    /// CLOSE-WAIT - Waiting for a connection termination request from the local user
    CloseWait,
    /// CLOSING - Waiting for a connection termination request acknowledgment from the remote TCP
    Closing,
    /// LAST-ACK - Waiting for an acknowledgment of the connection termination request
    LastAck,
    /// TIME-WAIT - Waiting for enough time to pass to be sure the remote TCP received the acknowledgment
    TimeWait,
}

/// TCP socket
#[derive(Debug, Clone)]
pub struct TcpSocket {
    /// Local IP address
    pub local_ip: Ipv4Addr,
    /// Local port
    pub local_port: u16,
    /// Remote IP address
    pub remote_ip: Ipv4Addr,
    /// Remote port
    pub remote_port: u16,
    /// Current connection state
    pub state: TcpState,
    /// Send sequence number
    pub snd_nxt: u32,
    /// Send unacknowledged
    pub snd_una: u32,
    /// Receive next sequence number
    pub rcv_nxt: u32,
    /// Receive window
    pub rcv_wnd: u16,
    /// Send window
    pub snd_wnd: u16,
}

impl TcpSocket {
    /// Create a new TCP socket
    pub fn new() -> Self {
        Self {
            local_ip: Ipv4Addr::UNSPECIFIED,
            local_port: 0,
            remote_ip: Ipv4Addr::UNSPECIFIED,
            remote_port: 0,
            state: TcpState::Closed,
            snd_nxt: 0,
            snd_una: 0,
            rcv_nxt: 0,
            rcv_wnd: 8192, // Default receive window
            snd_wnd: 0,
        }
    }

    /// Initialize sequence numbers
    pub fn init_sequence_numbers(&mut self) {
        use core::sync::atomic::{AtomicU32, Ordering};
        static SEQ_COUNTER: AtomicU32 = AtomicU32::new(1);
        let initial_seq = SEQ_COUNTER.fetch_add(1000, Ordering::Relaxed);
        self.snd_nxt = initial_seq;
        self.snd_una = initial_seq;
    }

    /// Check if socket is connected
    pub fn is_connected(&self) -> bool {
        self.state == TcpState::Established
    }

    /// Check if socket can send data
    pub fn can_send(&self) -> bool {
        matches!(
            self.state,
            TcpState::Established | TcpState::CloseWait
        )
    }

    /// Check if socket can receive data
    pub fn can_receive(&self) -> bool {
        matches!(
            self.state,
            TcpState::Established | TcpState::FinWait1 | TcpState::FinWait2
        )
    }
}

impl Default for TcpSocket {
    fn default() -> Self {
        Self::new()
    }
}

/// TCP errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TcpError {
    /// Packet too small
    PacketTooSmall,
    /// Invalid header length
    InvalidHeaderLength,
    /// Invalid checksum
    InvalidChecksum,
    /// Connection reset
    ConnectionReset,
    /// Connection timeout
    ConnectionTimeout,
    /// Connection refused
    ConnectionRefused,
}

/// Well-known TCP ports
pub mod ports {
    /// HTTP
    pub const HTTP: u16 = 80;
    /// HTTPS
    pub const HTTPS: u16 = 443;
    /// FTP
    pub const FTP: u16 = 21;
    /// SSH
    pub const SSH: u16 = 22;
    /// Telnet
    pub const TELNET: u16 = 23;
    /// SMTP
    pub const SMTP: u16 = 25;
    /// DNS
    pub const DNS: u16 = 53;
    /// POP3
    pub const POP3: u16 = 110;
    /// IMAP
    pub const IMAP: u16 = 143;
    /// MySQL
    pub const MYSQL: u16 = 3306;
}

/// Default TCP parameters
pub mod defaults {
    /// Default receive window size
    pub const RECEIVE_WINDOW: u16 = 8192;
    /// Default maximum segment size (MSS)
    pub const MAX_SEGMENT_SIZE: u16 = 1460;
    /// Default initial congestion window
    pub const INITIAL_CWND: u32 = 10;
    /// Default slow start threshold
    pub const INITIAL_SSTHRESH: u32 = u32::MAX;
}