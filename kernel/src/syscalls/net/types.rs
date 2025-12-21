//! 网络模块类型定义
//! 
//! 本模块定义了网络相关的类型、枚举和结构体，包括：
//! - 套接字类型和地址
//! - 网络协议和状态
//! - 网络配置参数
//! - 网络统计信息

use alloc::string::String;
use alloc::vec::Vec;

/// 套接字类型枚举
/// 
/// 定义套接字的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    /// 流套接字（TCP）
    Stream,
    /// 数据报套接字（UDP）
    Datagram,
    /// 原始套接字
    Raw,
    /// 顺序数据包套接字
    SeqPacket,
    /// 数据包套接字
    Packet,
}

/// 套接字地址族
/// 
/// 定义套接字的地址族。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressFamily {
    /// IPv4
    IPv4,
    /// IPv6
    IPv6,
    /// Unix域套接字
    Unix,
    /// 数据链路层
    Packet,
}

/// 套接字协议
/// 
/// 定义套接字使用的协议。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketProtocol {
    /// TCP协议
    TCP,
    /// UDP协议
    UDP,
    /// ICMP协议
    ICMP,
    /// 原始IP协议
    RawIP,
    /// 自定义协议
    Custom(u8),
}

/// 网络地址结构体
/// 
/// 表示网络地址的通用结构。
#[derive(Debug, Clone)]
pub enum NetworkAddress {
    /// IPv4地址
    IPv4 {
        address: [u8; 4],
        port: u16,
    },
    /// IPv6地址
    IPv6 {
        address: [u8; 16],
        port: u16,
        flow_info: u32,
        scope_id: u32,
    },
    /// Unix域套接字地址
    Unix {
        path: String,
    },
    /// 数据链路层地址
    LinkLayer {
        interface: u32,
        protocol: u16,
    },
}

impl NetworkAddress {
    /// 创建IPv4地址
    pub fn ipv4(address: [u8; 4], port: u16) -> Self {
        Self::IPv4 { address, port }
    }

    /// 创建IPv6地址
    pub fn ipv6(address: [u8; 16], port: u16) -> Self {
        Self::IPv6 { 
            address, 
            port, 
            flow_info: 0, 
            scope_id: 0 
        }
    }

    /// 创建Unix域地址
    pub fn unix(path: String) -> Self {
        Self::Unix { path }
    }

    /// 获取地址字符串表示
    pub fn to_string(&self) -> String {
        match self {
            Self::IPv4 { address, port } => {
                format!("{}.{}.{}.{}:{}", address[0], address[1], address[2], address[3], port)
            }
            Self::IPv6 { address, port, .. } => {
                let addr_str = address.iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<_>>()
                    .join(":");
                // println removed for no_std compatibility
                format!("[{}]:{}", addr_str, port)
            }
            Self::Unix { path } => {
                format!("unix:{}", path)
            }
            Self::LinkLayer { interface, protocol } => {
                format!("link:{}:{}", interface, protocol)
            }
        }
    }
}

/// 套接字状态
/// 
/// 定义套接字的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    /// 未连接
    Unconnected,
    /// 连接中
    Connecting,
    /// 已连接
    Connected,
    /// 监听中
    Listening,
    /// 关闭中
    Closing,
    /// 已关闭
    Closed,
}

/// 套接字信息
/// 
/// 包含套接字的详细状态和属性。
#[derive(Debug, Clone)]
pub struct SocketInfo {
    /// 套接字描述符
    pub fd: i32,
    /// 套接字类型
    pub socket_type: SocketType,
    /// 地址族
    pub address_family: AddressFamily,
    /// 协议
    pub protocol: SocketProtocol,
    /// 本地地址
    pub local_address: Option<NetworkAddress>,
    /// 远程地址
    pub remote_address: Option<NetworkAddress>,
    /// 套接字状态
    pub state: SocketState,
    /// 发送缓冲区大小
    pub send_buffer_size: u32,
    /// 接收缓冲区大小
    pub recv_buffer_size: u32,
    /// 套接字选项
    pub options: Vec<SocketOption>,
}

/// 套接字选项
/// 
/// 定义可设置的套接字选项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketOption {
    /// 调试模式
    Debug(bool),
    /// 重用地址
    ReuseAddr(bool),
    /// 保持连接
    KeepAlive(bool),
    /// 禁用Nagle算法
    NoDelay(bool),
    /// 发送缓冲区大小
    SendBufSize(u32),
    /// 接收缓冲区大小
    RecvBufSize(u32),
    /// 广播模式
    Broadcast(bool),
    /// 多播TTL
    MulticastTTL(u32),
    /// 多播回环
    MulticastLoop(bool),
}

/// 网络接口信息
/// 
/// 包含网络接口的详细信息。
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    /// 接口名称
    pub name: String,
    /// 接口索引
    pub index: u32,
    /// MAC地址
    pub mac_address: [u8; 6],
    /// MTU大小
    pub mtu: u32,
    /// 是否启用
    pub is_up: bool,
    /// 是否支持多播
    pub supports_multicast: bool,
    /// IPv4地址列表
    pub ipv4_addresses: Vec<NetworkAddress>,
    /// IPv6地址列表
    pub ipv6_addresses: Vec<NetworkAddress>,
}

/// 网络统计信息
/// 
/// 包含网络使用的统计信息。
#[derive(Debug, Clone)]
pub struct NetworkStats {
    /// 接收的字节数
    pub bytes_received: u64,
    /// 发送的字节数
    pub bytes_sent: u64,
    /// 接收的数据包数
    pub packets_received: u64,
    /// 发送的数据包数
    pub packets_sent: u64,
    /// 错误的数据包数
    pub packets_dropped: u64,
    /// 活跃连接数
    pub active_connections: u32,
    /// 监听套接字数
    pub listening_sockets: u32,
}

/// 网络连接参数
/// 
/// 包含建立网络连接所需的参数。
#[derive(Debug, Clone)]
pub struct ConnectionParams {
    /// 本地地址
    pub local_address: Option<NetworkAddress>,
    /// 远程地址
    pub remote_address: NetworkAddress,
    /// 套接字类型
    pub socket_type: SocketType,
    /// 协议
    pub protocol: SocketProtocol,
    /// 连接超时（秒）
    pub timeout: Option<u32>,
    /// 非阻塞模式
    pub non_blocking: bool,
}

impl Default for ConnectionParams {
    fn default() -> Self {
        Self {
            local_address: None,
            remote_address: NetworkAddress::ipv4([127, 0, 0, 1], 80),
            socket_type: SocketType::Stream,
            protocol: SocketProtocol::TCP,
            timeout: Some(30),
            non_blocking: false,
        }
    }
}

/// 网络错误类型
/// 
/// 定义网络模块特有的错误类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkError {
    /// 地址已在使用
    AddressInUse,
    /// 地址不可用
    AddressNotAvailable,
    /// 网络不可达
    NetworkUnreachable,
    /// 连接被拒绝
    ConnectionRefused,
    /// 连接超时
    ConnectionTimeout,
    /// 连接被重置
    ConnectionReset,
    /// 连接已关闭
    ConnectionClosed,
    /// 无效套接字
    InvalidSocket,
    /// 套接字已用完
    SocketTableFull,
    /// 协议不支持
    ProtocolNotSupported,
    /// 地址族不支持
    AddressFamilyNotSupported,
    /// 权限不足
    PermissionDenied,
    /// 缓冲区空间不足
    BufferSpaceInsufficient,
    /// 系统调用不支持
    UnsupportedSyscall,
    /// 无效参数
    InvalidArgument,
}

impl NetworkError {
    /// 获取错误码
    pub fn error_code(&self) -> i32 {
        match self {
            NetworkError::AddressInUse => -98,
            NetworkError::AddressNotAvailable => -99,
            NetworkError::NetworkUnreachable => -101,
            NetworkError::ConnectionRefused => -111,
            NetworkError::ConnectionTimeout => -110,
            NetworkError::ConnectionReset => -104,
            NetworkError::ConnectionClosed => -104,
            NetworkError::InvalidSocket => -88,
            NetworkError::SocketTableFull => -24,
            NetworkError::ProtocolNotSupported => -92,
            NetworkError::AddressFamilyNotSupported => -97,
            NetworkError::PermissionDenied => -13,
            NetworkError::BufferSpaceInsufficient => -105,
            NetworkError::UnsupportedSyscall => -38,
            NetworkError::InvalidArgument => -22,
        }
    }

    /// 获取错误描述
    pub fn error_message(&self) -> &str {
        match self {
            NetworkError::AddressInUse => "Address already in use",
            NetworkError::AddressNotAvailable => "Address not available",
            NetworkError::NetworkUnreachable => "Network is unreachable",
            NetworkError::ConnectionRefused => "Connection refused",
            NetworkError::ConnectionTimeout => "Connection timed out",
            NetworkError::ConnectionReset => "Connection reset by peer",
            NetworkError::ConnectionClosed => "Connection closed",
            NetworkError::InvalidSocket => "Invalid socket",
            NetworkError::SocketTableFull => "Socket table full",
            NetworkError::ProtocolNotSupported => "Protocol not supported",
            NetworkError::AddressFamilyNotSupported => "Address family not supported",
            NetworkError::PermissionDenied => "Permission denied",
            NetworkError::BufferSpaceInsufficient => "Insufficient buffer space",
            NetworkError::UnsupportedSyscall => "Unsupported syscall",
            NetworkError::InvalidArgument => "Invalid argument",
        }
    }
}

/// 网络接口管理特征
/// 
/// 定义网络接口管理的基本操作接口。
pub trait NetworkInterfaceManager: Send + Sync {
    /// 获取所有网络接口
    fn get_interfaces(&self) -> Vec<NetworkInterface>;
    
    /// 获取指定接口
    fn get_interface(&self, name: &str) -> Option<NetworkInterface>;
    
    /// 启用接口
    fn enable_interface(&mut self, name: &str) -> Result<(), NetworkError>;
    
    /// 禁用接口
    fn disable_interface(&mut self, name: &str) -> Result<(), NetworkError>;
    
    /// 设置接口MTU
    fn set_interface_mtu(&mut self, name: &str, mtu: u32) -> Result<(), NetworkError>;
    
    /// 添加接口地址
    fn add_interface_address(&mut self, name: &str, address: NetworkAddress) -> Result<(), NetworkError>;
    
    /// 删除接口地址
    fn remove_interface_address(&mut self, name: &str, address: NetworkAddress) -> Result<(), NetworkError>;
}

/// 套接字管理特征
/// 
/// 定义套接字管理的基本操作接口。
pub trait SocketManager: Send + Sync {
    /// 创建套接字
    fn create_socket(&mut self, family: AddressFamily, socket_type: SocketType, protocol: SocketProtocol) -> Result<i32, NetworkError>;
    
    /// 关闭套接字
    fn close_socket(&mut self, fd: i32) -> Result<(), NetworkError>;
    
    /// 绑定套接字
    fn bind_socket(&mut self, fd: i32, address: NetworkAddress) -> Result<(), NetworkError>;
    
    /// 监听套接字
    fn listen_socket(&mut self, fd: i32, backlog: i32) -> Result<(), NetworkError>;
    
    /// 接受连接
    fn accept_connection(&mut self, fd: i32) -> Result<(i32, NetworkAddress), NetworkError>;
    
    /// 连接到远程地址
    fn connect_socket(&mut self, fd: i32, address: NetworkAddress) -> Result<(), NetworkError>;
    
    /// 发送数据
    fn send_data(&mut self, fd: i32, data: &[u8], flags: i32) -> Result<usize, NetworkError>;
    
    /// 接收数据
    fn receive_data(&mut self, fd: i32, buffer: &mut [u8], flags: i32) -> Result<usize, NetworkError>;
    
    /// 发送数据到指定地址
    fn send_to(&mut self, fd: i32, data: &[u8], address: NetworkAddress, flags: i32) -> Result<usize, NetworkError>;
    
    /// 从指定地址接收数据
    fn receive_from(&mut self, fd: i32, buffer: &mut [u8], flags: i32) -> Result<(usize, NetworkAddress), NetworkError>;
    
    /// 获取套接字信息
    fn get_socket_info(&self, fd: i32) -> Option<SocketInfo>;
    
    /// 设置套接字选项
    fn set_socket_option(&mut self, fd: i32, option: SocketOption) -> Result<(), NetworkError>;
    
    /// 获取套接字选项
    fn get_socket_option(&self, fd: i32, option_name: &str) -> Option<SocketOption>;
    
    /// 获取网络统计
    fn get_network_stats(&self) -> NetworkStats;
}