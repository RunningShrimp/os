//! POSIX Socket Types and Constants

/// Socket address family constants
pub const AF_UNSPEC: i32 = 0;
pub const AF_UNIX: i32 = 1;
pub const AF_INET: i32 = 2;
pub const AF_INET6: i32 = 10;

/// Socket address structure (generic)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Sockaddr {
    /// Address family
    pub sa_family: u16,
    /// Address data (14 bytes for most address types)
    pub sa_data: [u8; 14],
}

impl Default for Sockaddr {
    fn default() -> Self {
        Self {
            sa_family: AF_UNSPEC as u16,
            sa_data: [0; 14],
        }
    }
}

/// Socket address for IPv4
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SockaddrIn {
    /// Address family (AF_INET)
    pub sin_family: u16,
    /// Port number (network byte order)
    pub sin_port: u16,
    /// IPv4 address (network byte order)
    pub sin_addr: u32,
    /// Padding
    pub sin_zero: [u8; 8],
}

/// Socket address for IPv6
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SockaddrIn6 {
    /// Address family (AF_INET6)
    pub sin6_family: u16,
    /// Port number (network byte order)
    pub sin6_port: u16,
    /// Flow information
    pub sin6_flowinfo: u32,
    /// IPv6 address
    pub sin6_addr: [u16; 8],
    /// Scope ID
    pub sin6_scope_id: u32,
}

