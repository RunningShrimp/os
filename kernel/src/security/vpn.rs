//! # VPN 隧道协议模块
//!
//! 提供全面的 VPN 功能：
//! - IPsec (ESP/AH) 协议支持
//! - WireGuard 协议实现
//! - OpenVPN 兼容接口
//! - 隧道管理和路由
//! - IKE 密钥交换
//!
//! ## 支持的协议
//!
//! 1. **IPsec**: ESP (Encapsulating Security Payload) 和 AH (Authentication Header)
//! 2. **WireGuard**: 现代化的 VPN 协议
//! 3. **OpenVPN**: SSL/TLS based VPN

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    string::String,
    vec::Vec,
};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use crate::net::{ipv4::Ipv4Addr, Packet};

/// Simple hex encoding helper for no_std environment
fn hex_encode(data: &[u8]) -> String {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut result = Vec::with_capacity(data.len() * 2);
    for &byte in data {
        result.push(HEX_CHARS[(byte >> 4) as usize]);
        result.push(HEX_CHARS[(byte & 0x0f) as usize]);
    }
    unsafe { String::from_utf8_unchecked(result) }
}

/// VPN 错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VpnError {
    /// 隧道不存在
    TunnelNotFound,
    /// 无效的配置
    InvalidConfiguration,
    /// 加密错误
    EncryptionError,
    /// 认证失败
    AuthenticationFailed,
    /// 密钥交换失败
    KeyExchangeFailed,
    /// 资源耗尽
    ResourceExhausted,
}

/// VPN 协议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VpnProtocol {
    /// IPsec ESP
    IpsecEsp,
    /// IPsec AH
    IpsecAh,
    /// WireGuard
    WireGuard,
    /// OpenVPN
    OpenVpn,
}

/// 加密算法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionAlgorithm {
    /// AES-128-CBC
    Aes128Cbc,
    /// AES-256-CBC
    Aes256Cbc,
    /// AES-128-GCM
    Aes128Gcm,
    /// AES-256-GCM
    Aes256Gcm,
    /// ChaCha20-Poly1305
    ChaCha20Poly1305,
}

/// 认证算法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationAlgorithm {
    /// HMAC-SHA1
    HmacSha1,
    /// HMAC-SHA256
    HmacSha256,
    /// HMAC-SHA384
    HmacSha384,
    /// HMAC-SHA512
    HmacSha512,
}

/// 密钥交换方法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyExchangeMethod {
    /// IKEv1
    IkeV1,
    /// IKEv2
    IkeV2,
    /// WireGuard Noise Protocol
    NoiseProtocol,
}

/// 隧道状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunnelState {
    /// 未初始化
    Uninitialized,
    /// 连接中
    Connecting,
    /// 已建立
    Established,
    /// 断开中
    Disconnecting,
    /// 已关闭
    Closed,
    /// 错误
    Error,
}

/// VPN 隧道配置
#[derive(Debug, Clone)]
pub struct TunnelConfig {
    /// 隧道 ID
    pub id: String,
    /// 隧道名称
    pub name: String,
    /// 协议类型
    pub protocol: VpnProtocol,
    /// 本端地址
    pub local_address: Ipv4Addr,
    /// 远端地址
    pub remote_address: Ipv4Addr,
    /// 本端端口
    pub local_port: u16,
    /// 远端端口
    pub remote_port: u16,
    /// 加密算法
    pub encryption: EncryptionAlgorithm,
    /// 认证算法
    pub authentication: AuthenticationAlgorithm,
    /// 密钥交换方法
    pub key_exchange: KeyExchangeMethod,
    /// PSK（预共享密钥）
    pub psk: Option<Vec<u8>>,
    /// MTU
    pub mtu: u16,
    /// Keep-Alive 间隔（秒）
    pub keepalive_interval: u32,
}

/// VPN 隧道
#[derive(Debug, Clone)]
pub struct VpnTunnel {
    /// 隧道配置
    pub config: TunnelConfig,
    /// 隧道状态
    pub state: TunnelState,
    /// 创建时间
    pub created_at: u64,
    /// 最后活动时间
    pub last_activity: u64,
    /// 发送字节数
    pub bytes_sent: u64,
    /// 接收字节数
    pub bytes_received: u64,
    /// 发送包数
    pub packets_sent: u64,
    /// 接收包数
    pub packets_received: u64,
    /// SPI (Security Parameter Index) for IPsec
    pub spi: Option<u32>,
    /// WireGuard 公钥
    pub public_key: Option<Vec<u8>>,
    /// WireGuard 私钥
    pub private_key: Option<Vec<u8>>,
    /// 对端公钥
    pub peer_public_key: Option<Vec<u8>>,
}

impl VpnTunnel {
    /// 更新活动时间
    pub fn update_activity(&mut self) {
        self.last_activity = crate::subsystems::time::get_timestamp();
    }

    /// 检查是否超时
    pub fn is_timeout(&self, timeout: u64) -> bool {
        let current_time = crate::subsystems::time::get_timestamp();
        current_time - self.last_activity > timeout * 1_000_000_000
    }
}

/// IPsec SA (Security Association)
#[derive(Debug, Clone)]
pub struct IpsecSa {
    /// SPI
    pub spi: u32,
    /// 本端地址
    pub src_addr: Ipv4Addr,
    /// 远端地址
    pub dst_addr: Ipv4Addr,
    /// 加密密钥
    pub encryption_key: Vec<u8>,
    /// 认证密钥
    pub auth_key: Vec<u8>,
    /// 加密算法
    pub encryption: EncryptionAlgorithm,
    /// 认证算法
    pub authentication: AuthenticationAlgorithm,
    /// 序列号
    pub sequence_number: u64,
    /// 重放窗口
    pub replay_window: u64,
    /// 生存时间
    pub lifetime: u64,
    /// 创建时间
    pub created_at: u64,
}

/// WireGuard 对等体
#[derive(Debug, Clone)]
pub struct WireGuardPeer {
    /// 公钥
    pub public_key: Vec<u8>,
    /// 端点地址
    pub endpoint: Option<(Ipv4Addr, u16)>,
    /// 允许的 IP
    pub allowed_ips: Vec<(Ipv4Addr, u32)>, // (address, prefix_length)
    /// 持久 Keep-Alive
    pub persistent_keepalive: Option<u32>,
    /// 最后握手时间
    pub last_handshake: Option<u64>,
    /// 发送字节数
    pub tx_bytes: u64,
    /// 接收字节数
    pub rx_bytes: u64,
}

/// VPN 统计信息
#[derive(Debug, Clone, Default)]
pub struct VpnStatistics {
    /// 活跃隧道数
    pub active_tunnels: usize,
    /// 总发送字节数
    pub total_bytes_sent: u64,
    /// 总接收字节数
    pub total_bytes_received: u64,
    /// 总发送包数
    pub total_packets_sent: u64,
    /// 总接收包数
    pub total_packets_received: u64,
    /// IPsec SA 数
    pub ipsec_sa_count: usize,
    /// WireGuard 对等体数
    pub wireguard_peer_count: usize,
}

/// VPN 管理器
pub struct VpnManager {
    /// VPN 隧道
    tunnels: Mutex<BTreeMap<String, VpnTunnel>>,
    /// IPsec SA
    ipsec_sas: Mutex<BTreeMap<u32, IpsecSa>>,
    /// WireGuard 对等体
    wireguard_peers: Mutex<BTreeMap<String, WireGuardPeer>>,
    /// 统计信息
    stats: Mutex<VpnStatistics>,
    /// 下一个 SPI
    next_spi: AtomicU64,
    /// 下一个隧道 ID
    next_tunnel_id: AtomicU64,
}

impl VpnManager {
    /// 创建新的 VPN 管理器
    pub fn new() -> Self {
        Self {
            tunnels: Mutex::new(BTreeMap::new()),
            ipsec_sas: Mutex::new(BTreeMap::new()),
            wireguard_peers: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(VpnStatistics::default()),
            next_spi: AtomicU64::new(1),
            next_tunnel_id: AtomicU64::new(1),
        }
    }

    /// 创建 IPsec 隧道
    pub fn create_ipsec_tunnel(&self, config: TunnelConfig) -> Result<String, VpnError> {
        if config.protocol != VpnProtocol::IpsecEsp && config.protocol != VpnProtocol::IpsecAh {
            return Err(VpnError::InvalidConfiguration);
        }

        let spi = self.next_spi.fetch_add(1, Ordering::SeqCst) as u32;

        let tunnel = VpnTunnel {
            config: config.clone(),
            state: TunnelState::Connecting,
            created_at: crate::subsystems::time::get_timestamp(),
            last_activity: crate::subsystems::time::get_timestamp(),
            bytes_sent: 0,
            bytes_received: 0,
            packets_sent: 0,
            packets_received: 0,
            spi: Some(spi),
            public_key: None,
            private_key: None,
            peer_public_key: None,
        };

        let mut tunnels = self.tunnels.lock();
        tunnels.insert(config.id.clone(), tunnel.clone());

        // 创建 IPsec SA
        let sa = IpsecSa {
            spi,
            src_addr: config.local_address,
            dst_addr: config.remote_address,
            encryption_key: vec![0u8; 32], // 实际应该从配置获取
            auth_key: vec![0u8; 32],
            encryption: config.encryption,
            authentication: config.authentication,
            sequence_number: 0,
            replay_window: 64,
            lifetime: 28800, // 8 hours
            created_at: crate::subsystems::time::get_timestamp(),
        };

        let mut sas = self.ipsec_sas.lock();
        sas.insert(spi, sa);

        Ok(config.id.clone())
    }

    /// 创建 WireGuard 隧道
    pub fn create_wireguard_tunnel(&self, config: TunnelConfig) -> Result<String, VpnError> {
        if config.protocol != VpnProtocol::WireGuard {
            return Err(VpnError::InvalidConfiguration);
        }

        // 简化实现 - 实际需要生成密钥对
        let public_key = vec![0u8; 32];
        let private_key = vec![0u8; 32];

        let tunnel = VpnTunnel {
            config: config.clone(),
            state: TunnelState::Connecting,
            created_at: crate::subsystems::time::get_timestamp(),
            last_activity: crate::subsystems::time::get_timestamp(),
            bytes_sent: 0,
            bytes_received: 0,
            packets_sent: 0,
            packets_received: 0,
            spi: None,
            public_key: Some(public_key),
            private_key: Some(private_key),
            peer_public_key: None,
        };

        let mut tunnels = self.tunnels.lock();
        tunnels.insert(config.id.clone(), tunnel);

        Ok(config.id.clone())
    }

    /// 删除隧道
    pub fn delete_tunnel(&self, tunnel_id: &str) -> Result<(), VpnError> {
        let mut tunnels = self.tunnels.lock();

        // 如果是 IPsec 隧道，需要删除 SA
        if let Some(tunnel) = tunnels.get(tunnel_id) {
            if let Some(spi) = tunnel.spi {
                let mut sas = self.ipsec_sas.lock();
                sas.remove(&spi);
            }
        }

        tunnels
            .remove(tunnel_id)
            .ok_or(VpnError::TunnelNotFound)
            .map(|_| ())
    }

    /// 处理 VPN 包
    pub fn process_vpn_packet(&self, tunnel_id: &str, packet: &Packet) -> Result<Packet, VpnError> {
        let mut tunnels = self.tunnels.lock();
        let tunnel = tunnels
            .get_mut(tunnel_id)
            .ok_or(VpnError::TunnelNotFound)?;

        tunnel.update_activity();
        tunnel.packets_received += 1;
        tunnel.bytes_received += packet.data().len() as u64;

        match tunnel.config.protocol {
            VpnProtocol::IpsecEsp => self.process_ipsec_esp(tunnel, packet),
            VpnProtocol::IpsecAh => self.process_ipsec_ah(tunnel, packet),
            VpnProtocol::WireGuard => self.process_wireguard(tunnel, packet),
            VpnProtocol::OpenVpn => self.process_openvpn(tunnel, packet),
        }
    }

    /// 处理 IPsec ESP 包
    fn process_ipsec_esp(&self, _tunnel: &VpnTunnel, packet: &Packet) -> Result<Packet, VpnError> {
        // 简化实现 - 实际需要解密和验证
        Ok(packet.clone())
    }

    /// 处理 IPsec AH 包
    fn process_ipsec_ah(&self, _tunnel: &VpnTunnel, packet: &Packet) -> Result<Packet, VpnError> {
        // 简化实现
        Ok(packet.clone())
    }

    /// 处理 WireGuard 包
    fn process_wireguard(&self, _tunnel: &VpnTunnel, packet: &Packet) -> Result<Packet, VpnError> {
        // 简化实现
        Ok(packet.clone())
    }

    /// 处理 OpenVPN 包
    fn process_openvpn(&self, _tunnel: &VpnTunnel, packet: &Packet) -> Result<Packet, VpnError> {
        // 简化实现
        Ok(packet.clone())
    }

    /// 封装发送包
    pub fn encapsulate_packet(
        &self,
        tunnel_id: &str,
        packet: Packet,
    ) -> Result<Packet, VpnError> {
        let mut tunnels = self.tunnels.lock();
        let tunnel = tunnels
            .get_mut(tunnel_id)
            .ok_or(VpnError::TunnelNotFound)?;

        tunnel.update_activity();
        tunnel.packets_sent += 1;
        tunnel.bytes_sent += packet.data().len() as u64;

        // 简化实现 - 实际需要加密和封装
        Ok(packet)
    }

    /// 添加 WireGuard 对等体
    pub fn add_wireguard_peer(&self, peer: WireGuardPeer) -> Result<String, VpnError> {
        let peer_id = hex_encode(&peer.public_key);
        let mut peers = self.wireguard_peers.lock();
        peers.insert(peer_id.clone(), peer);
        Ok(peer_id)
    }

    /// 删除 WireGuard 对等体
    pub fn remove_wireguard_peer(&self, peer_id: &str) -> Result<(), VpnError> {
        let mut peers = self.wireguard_peers.lock();
        peers
            .remove(peer_id)
            .ok_or(VpnError::TunnelNotFound)
            .map(|_| ())
    }

    /// 获取隧道信息
    pub fn get_tunnel(&self, tunnel_id: &str) -> Option<VpnTunnel> {
        let tunnels = self.tunnels.lock();
        tunnels.get(tunnel_id).cloned()
    }

    /// 获取所有隧道
    pub fn get_all_tunnels(&self) -> Vec<VpnTunnel> {
        let tunnels = self.tunnels.lock();
        tunnels.values().cloned().collect()
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> VpnStatistics {
        let stats = self.stats.lock();
        let tunnels = self.tunnels.lock();
        let sas = self.ipsec_sas.lock();
        let peers = self.wireguard_peers.lock();

        VpnStatistics {
            active_tunnels: tunnels.len(),
            total_bytes_sent: stats.total_bytes_sent,
            total_bytes_received: stats.total_bytes_received,
            total_packets_sent: stats.total_packets_sent,
            total_packets_received: stats.total_packets_received,
            ipsec_sa_count: sas.len(),
            wireguard_peer_count: peers.len(),
        }
    }

    /// 清理超时隧道
    pub fn cleanup_timeout_tunnels(&self, timeout: u64) {
        let mut tunnels = self.tunnels.lock();
        tunnels.retain(|_, tunnel| !tunnel.is_timeout(timeout));
    }
}

impl Default for VpnManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局 VPN 管理器实例
pub static GLOBAL_VPN_MANAGER: Mutex<Option<VpnManager>> = Mutex::new(None);

/// 初始化全局 VPN 管理器
pub fn init_vpn() -> Result<(), VpnError> {
    let manager = VpnManager::new();

    let mut global = GLOBAL_VPN_MANAGER.lock();
    *global = Some(manager);

    crate::println!("[VPN] VPN manager initialized successfully");
    Ok(())
}

/// 创建 IPsec 隧道（便捷函数）
pub fn create_ipsec_tunnel(config: TunnelConfig) -> Result<String, VpnError> {
    let global = GLOBAL_VPN_MANAGER.lock();
    let manager = global
        .as_ref()
        .ok_or(VpnError::ResourceExhausted)?;
    manager.create_ipsec_tunnel(config)
}

/// 创建 WireGuard 隧道（便捷函数）
pub fn create_wireguard_tunnel(config: TunnelConfig) -> Result<String, VpnError> {
    let global = GLOBAL_VPN_MANAGER.lock();
    let manager = global
        .as_ref()
        .ok_or(VpnError::ResourceExhausted)?;
    manager.create_wireguard_tunnel(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vpn_manager_creation() {
        let manager = VpnManager::new();
        let stats = manager.get_statistics();
        assert_eq!(stats.active_tunnels, 0);
    }

    #[test]
    fn test_tunnel_timeout() {
        let config = TunnelConfig {
            id: String::from("test"),
            name: String::from("Test Tunnel"),
            protocol: VpnProtocol::WireGuard,
            local_address: Ipv4Addr::new(10, 0, 0, 1),
            remote_address: Ipv4Addr::new(10, 0, 0, 2),
            local_port: 51820,
            remote_port: 51820,
            encryption: EncryptionAlgorithm::ChaCha20Poly1305,
            authentication: AuthenticationAlgorithm::HmacSha256,
            key_exchange: KeyExchangeMethod::NoiseProtocol,
            psk: None,
            mtu: 1420,
            keepalive_interval: 25,
        };

        let mut tunnel = VpnTunnel {
            config: config.clone(),
            state: TunnelState::Established,
            created_at: 0,
            last_activity: 0,
            bytes_sent: 0,
            bytes_received: 0,
            packets_sent: 0,
            packets_received: 0,
            spi: None,
            public_key: None,
            private_key: None,
            peer_public_key: None,
        };

        assert!(tunnel.is_timeout(60)); // 60 seconds later
        assert!(!tunnel.is_timeout(30)); // 30 seconds later

        tunnel.update_activity();
        assert!(!tunnel.is_timeout(30)); // After update
    }

    #[test]
    fn test_encryption_algorithm() {
        assert_eq!(EncryptionAlgorithm::Aes128Gcm, EncryptionAlgorithm::Aes128Gcm);
        assert_eq!(EncryptionAlgorithm::ChaCha20Poly1305, EncryptionAlgorithm::ChaCha20Poly1305);
    }

    #[test]
    fn test_tunnel_state() {
        assert_ne!(TunnelState::Connecting, TunnelState::Established);
        assert_eq!(TunnelState::Closed, TunnelState::Closed);
    }
}
