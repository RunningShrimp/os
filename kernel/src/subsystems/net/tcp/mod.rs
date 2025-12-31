//! Advanced TCP optimizations
//!
//! This module implements modern TCP enhancements including:
//! - BBR congestion control
//! - RACK-TLP loss recovery
//! - TCP Fast Open
//! - TLS 1.3 integration

// Core TCP submodules (existing)
pub mod batch_ack;
pub mod congestion;
pub mod manager;
pub mod state;

// Advanced TCP optimizations (new)
pub mod bbr;
pub mod fast_open;
pub mod rack;
pub mod tls;

// Re-export base TCP types from tcp_base.rs
pub use crate::subsystems::net::tcp_base::{
    EnhancedTcpStats, MssOption, SackBlock, SackOption, TcpConfig, TcpError, TcpHeader,
    TcpOption, TcpOptionKind, TcpPacket, TcpSocket, TcpState, TimestampOption, WindowScaleOption,
    ports, tcp_flags,
};

// Re-export advanced optimization types
pub use self::bbr::{BbrState, BbrStateEnum};
pub use self::fast_open::{TfoError, TfoState, TFO_COOKIE_LEN, TFO_COOKIE_MAX_AGE};
pub use self::rack::RackState;
pub use self::tls::{
    CipherSuite, TlsConfig, TlsContentType, TlsError, TlsHandshakeState, TlsRecordHeader, TlsState,
    TlsVersion,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbr_integration() {
        let bbr = BbrState::new(1460);
        assert_eq!(bbr.state, BbrStateEnum::Startup);
    }

    #[test]
    fn test_rack_integration() {
        let rack = RackState::new();
        assert!(rack.enabled);
    }

    #[test]
    fn test_tfo_integration() {
        let tfo = TfoState::new();
        assert!(tfo.enabled);
    }

    #[test]
    fn test_tls_integration() {
        let tls = TlsState::new();
        assert_eq!(tls.version, TlsVersion::Tls13);
    }
}
