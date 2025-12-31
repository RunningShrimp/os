//! Advanced socket options
//!
//! This module provides advanced socket options for fine-tuning socket
//! behavior, including TCP-specific options, timeout settings, and
//! performance optimizations.
//!
//! # Features
//! - TCP_NODELAY (disable Nagle's algorithm)
//! - SO_KEEPALIVE with configurable parameters
//! - SO_REUSEPORT for load balancing
//! - Buffer size management
//! - Timeout settings
//! - Zero-copy options

#![allow(dead_code)]

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use crate::subsystems::sync::Mutex;

/// Socket option level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SocketOptionLevel {
    /// Socket level options (SOL_SOCKET)
    Socket = 1,
    /// TCP level options (IPPROTO_TCP)
    Tcp = 6,
    /// IP level options (IPPROTO_IP)
    Ip = 0,
    /// IPv6 level options (IPPROTO_IPV6)
    Ipv6 = 41,
    /// UDP level options (IPPROTO_UDP)
    Udp = 17,
}

/// Socket option names
#[derive(Debug, Clone, Copy)]
pub enum SocketOptionName {
    // Socket level options
    /// SO_DEBUG - Turn on debugging info recording
    Debug,
    /// SO_ACCEPTCONN - Socket has had listen()
    AcceptConn,
    /// SO_REUSEADDR - Allow local address reuse
    ReuseAddr,
    /// SO_KEEPALIVE - Keep connections alive
    KeepAlive,
    /// SO_DONTROUTE - Use interface addresses
    DontRoute,
    /// SO_BROADCAST - Permit sending of broadcast messages
    Broadcast,
    /// SO_USELOOPBACK - Bypass hardware when possible
    UseLoopback,
    /// SO_LINGER - Linger on close if data is present
    Linger,
    /// SO_OOBINLINE - Leave received OOB data in line
    OobInline,
    /// SO_REUSEPORT - Allow local address and port reuse
    ReusePort,
    /// SO_SNDBUF - Send buffer size
    SndBuf,
    /// SO_RCVBUF - Receive buffer size
    RcvBuf,
    /// SO_SNDLOWAT - Send low-water mark
    SndLowat,
    /// SO_RCVLOWAT - Receive low-water mark
    RcvLowat,
    /// SO_SNDTIMEO - Send timeout
    SndTimeo,
    /// SO_RCVTIMEO - Receive timeout
    RcvTimeo,
    /// SO_ERROR - Get and clear pending error
    Error,
    /// SO_TYPE - Socket type
    Type,
    /// SO_TIMESTAMP - Receive timestamp with datagrams
    Timestamp,
    /// SO_ACCEPTFILTER - Set accept filter
    AcceptFilter,
    /// SO_BINDTODEVICE - Bind to device
    BindToDevice,
    /// SO_ATTACH_FILTER - Attach socket filter
    AttachFilter,
    /// SO_DETACH_FILTER - Detach socket filter
    DetachFilter,
    /// SO_PEERNAME - Get peer name
    PeerName,
    /// SO_PEERCRED - Get process credentials
    PeerCred,
    /// SO_PRIORITY - Protocol-defined priority
    Priority,
    /// SO_PROTOCOL - Protocol
    Protocol,
    /// SO_DOMAIN - Domain
    Domain,
    /// SO_RXQ_OVFL - Receive queue overflow
    RxQOvfl,
    /// SO_WIFI_STATUS - Wifi status
    WifiStatus,
    /// SO_PEEK_OFF - Peek offset
    PeekOff,
    /// SO_NOFCS - Set NOFCS
    NoFCS,
    /// SO_LOCK_FILTER - Lock filter
    LockFilter,
    /// SO_SELECT_ERR_QUEUE - Select error queue
    SelectErrQueue,
    /// SO_BUSY_POLL - Busy poll
    BusyPoll,
    /// SO_MAX_PACING_RATE - Max pacing rate
    MaxPacingRate,
    /// SO_BPF_EXTENSIONS - BPF extensions
    BpfExtensions,
    /// SO_INCOMING_CPU - Incoming CPU
    IncomingCpu,
    /// SO_ATTACH_BPF - Attach BPF
    AttachBpf,
    /// SO_DETACH_BPF - Detach BPF
    DetachBpf,
    /// SO_ATTACH_REUSEPORT_CBPF - Attach reuseport CBPF
    AttachReuseportCbpf,
    /// SO_ATTACH_REUSEPORT_EBPF - Attach reuseport EBPF
    AttachReuseportEbpf,
    /// SO_CNX_ADVICE - Connection advice
    CnxAdvice,

    // TCP level options
    /// TCP_NODELAY - Don't delay send to coalesce packets
    TcpNoDelay,
    /// TCP_MAXSEG - Set maximum segment size
    TcpMaxSeg,
    /// TCP_CORK - Don't send partial frames
    TcpCork,
    /// TCP_KEEPIDLE - Idle time before keepalive probe
    TcpKeepIdle,
    /// TCP_KEEPINTVL - Interval between keepalive probes
    TcpKeepIntvl,
    /// TCP_KEEPCNT - Number of keepalive probes before dropping
    TcpKeepCnt,
    /// TCP_SYNCNT - Number of SYN retransmits
    TcpSyncnt,
    /// TCP_LINGER2 - Lifetime of orphaned FIN_WAIT2 state
    TcpLinger2,
    /// TCP_DEFER_ACCEPT - Defer accept until data arrives
    TcpDeferAccept,
    /// TCP_WINDOW_CLAMP - Clamp window size
    TcpWindowClamp,
    /// TCP_INFO - Retrieve information about this socket
    TcpInfo,
    /// TCP_QUICKACK - Enable quick ACK mode
    TcpQuickAck,
    /// TCP_CONGESTION - Congestion control algorithm
    TcpCongestion,
    /// TCP_MD5SIG - MD5 signature (RFC 2385)
    TcpMd5Sig,
    /// TCP_THIN_LINEAR_TIMEOUTS - Thin linear timeouts
    TcpThinLinearTimeouts,
    /// TCP_THIN_DUPACK - Thin dupack
    TcpThinDupack,
    /// TCP_USER_TIMEOUT - TCP user timeout
    TcpUserTimeout,
    /// TCP_REPAIR - TCP repair mode
    TcpRepair,
    /// TCP_REPAIR_QUEUE - TCP repair queue
    TcpRepairQueue,
    /// TCP_QUEUE_SEQ - TCP queue sequence
    TcpQueueSeq,
    /// TCP_REPAIR_OPTIONS - TCP repair options
    TcpRepairOptions,
    /// TCP_FASTOPEN - TCP Fast Open
    TcpFastOpen,
    /// TCP_TIMESTAMP - TCP timestamp
    TcpTimestamp,
    /// TCP_NOTSENT_LOWAT - Not sent low watermark
    TcpNotSentLowat,
    /// TCP_CC_INFO - Congestion control info
    TcpCcInfo,
    /// TCP_SAVE_SYN - Save SYN for TCP_REPAIR
    TcpSaveSyn,
    /// TCP_SAVED_SYN - Get saved SYN
    TcpSavedSyn,

    // IP level options
    /// IP_TOS - Type of service and precedence
    IpTos,
    /// IP_TTL - Time to live
    IpTtl,
    /// IP_HDRINCL - Header is included with data
    IpHdrincl,
    /// IP_OPTIONS - IP options
    IpOptions,
    /// IP_ROUTER_ALERT - Notify router when routing
    IpRouterAlert,
    /// IP_RECVOPTS - Receive IP options
    IpRecvopts,
    /// IP_RETOPTS - Receive IP options
    IpRetopts,
    /// IP_PKTINFO - Receive packet information
    IpPktinfo,
    /// IP_PKTOPTIONS - Receive packet options
    IpPktoptions,
    /// IP_MTU_DISCOVER - Path MTU discovery
    IpMtuDiscover,
    /// IP_MTU - Get MTU
    IpMtu,
    /// IP_RECVERR - Receive error queue
    IpRecverr,
    /// IP_RECVTTL - Receive TTL
    IpRecvttl,
    /// IP_RECVTOS - Receive TOS
    IpRecvtos,
    /// IP_MULTICAST_IF - Multicast interface
    IpMulticastIf,
    /// IP_MULTICAST_TTL - Multicast TTL
    IpMulticastTtl,
    /// IP_MULTICAST_LOOP - Multicast loopback
    IpMulticastLoop,
    /// IP_ADD_MEMBERSHIP - Add multicast membership
    IpAddMembership,
    /// IP_DROP_MEMBERSHIP - Drop multicast membership
    IpDropMembership,
}

/// Socket option value
#[derive(Debug, Clone)]
pub enum SocketOptionValue {
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(i32),
    /// Unsigned integer value
    UInt(u32),
    /// Size value (buffer size)
    Size(usize),
    /// Time value in milliseconds
    TimeMs(u32),
    /// Time value in seconds
    TimeSec(u32),
    /// String value
    String(String),
    /// Linger structure
    Linger {
        /// Linger enabled
        on: bool,
        /// Linger time in seconds
        time: i32,
    },
    /// Timeval structure
    Timeval {
        /// Seconds
        sec: u64,
        /// Microseconds
        usec: u64,
    },
}

/// Socket option data
#[derive(Debug, Clone)]
pub struct SocketOptionData {
    /// Option name
    pub name: SocketOptionName,
    /// Option value
    pub value: SocketOptionValue,
}

impl SocketOptionData {
    /// Create a new socket option
    pub fn new(name: SocketOptionName, value: SocketOptionValue) -> Self {
        Self { name, value }
    }

    /// Get option name
    pub fn name(&self) -> SocketOptionName {
        self.name
    }

    /// Get option value
    pub fn value(&self) -> &SocketOptionValue {
        &self.value
    }
}

/// Socket options container
pub struct SocketOptions {
    /// Options stored by level and name
    options: Mutex<BTreeMap<(SocketOptionLevel, u32), SocketOptionValue>>,
    /// Default send buffer size
    default_send_buffer_size: usize,
    /// Default receive buffer size
    default_recv_buffer_size: usize,
}

impl SocketOptions {
    /// Create new socket options with defaults
    pub fn new() -> Self {
        let mut options = Self {
            options: Mutex::new(BTreeMap::new()),
            default_send_buffer_size: 64 * 1024,
            default_recv_buffer_size: 64 * 1024,
        };

        // Set default options
        options.set_option(
            SocketOptionLevel::Socket,
            SocketOptionName::SndBuf as u32,
            SocketOptionValue::Size(64 * 1024),
        );
        options.set_option(
            SocketOptionLevel::Socket,
            SocketOptionName::RcvBuf as u32,
            SocketOptionValue::Size(64 * 1024),
        );
        options.set_option(
            SocketOptionLevel::Socket,
            SocketOptionName::ReuseAddr as u32,
            SocketOptionValue::Bool(false),
        );
        options.set_option(
            SocketOptionLevel::Socket,
            SocketOptionName::ReusePort as u32,
            SocketOptionValue::Bool(false),
        );
        options.set_option(
            SocketOptionLevel::Socket,
            SocketOptionName::KeepAlive as u32,
            SocketOptionValue::Bool(false),
        );

        options
    }

    /// Set socket option
    pub fn set_option(
        &self,
        level: SocketOptionLevel,
        name: u32,
        value: SocketOptionValue,
    ) -> Result<(), SocketOptionError> {
        // Validate option value
        self.validate_option(level, name, &value)?;

        self.options.lock().insert((level, name), value);
        Ok(())
    }

    /// Get socket option
    pub fn get_option(
        &self,
        level: SocketOptionLevel,
        name: u32,
    ) -> Result<SocketOptionValue, SocketOptionError> {
        self.options
            .lock()
            .get(&(level, name))
            .cloned()
            .ok_or(SocketOptionError::NotSupported)
    }

    /// Validate socket option value
    fn validate_option(
        &self,
        level: SocketOptionLevel,
        name: u32,
        value: &SocketOptionValue,
    ) -> Result<(), SocketOptionError> {
        match (level, name) {
            (SocketOptionLevel::Socket, n) if n == SocketOptionName::SndBuf as u32 => {
                if let SocketOptionValue::Size(size) = value {
                    if *size < 1024 || *size > (1 << 30) {
                        return Err(SocketOptionError::InvalidValue);
                    }
                }
            },
            (SocketOptionLevel::Socket, n) if n == SocketOptionName::RcvBuf as u32 => {
                if let SocketOptionValue::Size(size) = value {
                    if *size < 1024 || *size > (1 << 30) {
                        return Err(SocketOptionError::InvalidValue);
                    }
                }
            },
            (SocketOptionLevel::Tcp, n) if n == SocketOptionName::TcpMaxSeg as u32 => {
                if let SocketOptionValue::Int(mss) = value {
                    if *mss < 536 || *mss > 65535 {
                        return Err(SocketOptionError::InvalidValue);
                    }
                }
            },
            (SocketOptionLevel::Tcp, n) if n == SocketOptionName::TcpKeepIdle as u32 => {
                if let SocketOptionValue::Int(secs) = value {
                    if *secs < 1 || *secs > 32767 {
                        return Err(SocketOptionError::InvalidValue);
                    }
                }
            },
            _ => {},
        }

        Ok(())
    }

    /// Get send buffer size
    pub fn get_send_buffer_size(&self) -> usize {
        if let Ok(SocketOptionValue::Size(size)) =
            self.get_option(SocketOptionLevel::Socket, SocketOptionName::SndBuf as u32)
        {
            size
        } else {
            self.default_send_buffer_size
        }
    }

    /// Get receive buffer size
    pub fn get_recv_buffer_size(&self) -> usize {
        if let Ok(SocketOptionValue::Size(size)) =
            self.get_option(SocketOptionLevel::Socket, SocketOptionName::RcvBuf as u32)
        {
            size
        } else {
            self.default_recv_buffer_size
        }
    }

    /// Check if TCP_NODELAY is set
    pub fn get_tcp_nodelay(&self) -> bool {
        if let Ok(SocketOptionValue::Bool(nodelay)) =
            self.get_option(SocketOptionLevel::Tcp, SocketOptionName::TcpNoDelay as u32)
        {
            nodelay
        } else {
            false
        }
    }

    /// Set TCP_NODELAY
    pub fn set_tcp_nodelay(&self, nodelay: bool) -> Result<(), SocketOptionError> {
        self.set_option(
            SocketOptionLevel::Tcp,
            SocketOptionName::TcpNoDelay as u32,
            SocketOptionValue::Bool(nodelay),
        )
    }

    /// Check if SO_KEEPALIVE is set
    pub fn get_keepalive(&self) -> bool {
        if let Ok(SocketOptionValue::Bool(keepalive)) =
            self.get_option(SocketOptionLevel::Socket, SocketOptionName::KeepAlive as u32)
        {
            keepalive
        } else {
            false
        }
    }

    /// Set SO_KEEPALIVE
    pub fn set_keepalive(&self, keepalive: bool) -> Result<(), SocketOptionError> {
        self.set_option(
            SocketOptionLevel::Socket,
            SocketOptionName::KeepAlive as u32,
            SocketOptionValue::Bool(keepalive),
        )
    }

    /// Check if SO_REUSEADDR is set
    pub fn get_reuseaddr(&self) -> bool {
        if let Ok(SocketOptionValue::Bool(reuse)) =
            self.get_option(SocketOptionLevel::Socket, SocketOptionName::ReuseAddr as u32)
        {
            reuse
        } else {
            false
        }
    }

    /// Set SO_REUSEADDR
    pub fn set_reuseaddr(&self, reuse: bool) -> Result<(), SocketOptionError> {
        self.set_option(
            SocketOptionLevel::Socket,
            SocketOptionName::ReuseAddr as u32,
            SocketOptionValue::Bool(reuse),
        )
    }

    /// Check if SO_REUSEPORT is set
    pub fn get_reuseport(&self) -> bool {
        if let Ok(SocketOptionValue::Bool(reuse)) =
            self.get_option(SocketOptionLevel::Socket, SocketOptionName::ReusePort as u32)
        {
            reuse
        } else {
            false
        }
    }

    /// Set SO_REUSEPORT
    pub fn set_reuseport(&self, reuse: bool) -> Result<(), SocketOptionError> {
        self.set_option(
            SocketOptionLevel::Socket,
            SocketOptionName::ReusePort as u32,
            SocketOptionValue::Bool(reuse),
        )
    }
}

impl Default for SocketOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Socket option errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketOptionError {
    /// Option not supported
    NotSupported,
    /// Invalid value
    InvalidValue,
    /// Option not settable
    NotSettable,
    /// Option not gettable
    NotGettable,
    /// Invalid option level
    InvalidLevel,
    /// Invalid option name
    InvalidName,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_options() {
        let opts = SocketOptions::new();

        assert_eq!(opts.get_send_buffer_size(), 64 * 1024);
        assert_eq!(opts.get_recv_buffer_size(), 64 * 1024);

        // Test TCP_NODELAY
        opts.set_tcp_nodelay(true).unwrap();
        assert_eq!(opts.get_tcp_nodelay(), true);

        // Test SO_REUSEADDR
        opts.set_reuseaddr(true).unwrap();
        assert_eq!(opts.get_reuseaddr(), true);

        // Test SO_REUSEPORT
        opts.set_reuseport(true).unwrap();
        assert_eq!(opts.get_reuseport(), true);
    }
}
