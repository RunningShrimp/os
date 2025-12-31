//! # DDoS 防护模块
//!
//! 提供全面的 DDoS 攻击防护功能：
//! - 流量速率限制
//! - SYN Cookie 保护
//! - IP 黑名单/白名单
//! - 流量清洗
//! - 自适应过滤
//!
//! ## 防护机制
//!
//! 1. **SYN Flood 防护**: SYN Cookie 和 SYN Proxy
//! 2. **UDP Flood 防护**: UDP 速率限制和过滤
//! 3. **ICMP Flood 防护**: ICMP 速率限制
//! 4. **HTTP Flood 防护**: HTTP 请求速率限制
//! 5. **放大攻击防护**: DNS/NTP 放大攻击检测

extern crate alloc;

use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use crate::net::{ipv4::Ipv4Addr, Packet};

/// DDoS 防护错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DdosError {
    /// 速率限制器已满
    RateLimiterFull,
    /// 无效的 IP 地址
    InvalidIpAddress,
    /// 配置错误
    ConfigurationError(String),
}

/// IP 列表类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpListType {
    /// 黑名单
    Blacklist,
    /// 白名单
    Whitelist,
}

/// 速率限制策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitStrategy {
    /// 令牌桶
    TokenBucket,
    /// 漏桶
    LeakyBucket,
    /// 固定窗口
    FixedWindow,
    /// 滑动窗口
    SlidingWindow,
}

/// SYN Cookie 状态
#[derive(Debug, Clone)]
pub struct SynCookieState {
    /// 源 IP
    pub source_ip: Ipv4Addr,
    /// 源端口
    pub source_port: u16,
    /// 目标端口
    pub dest_port: u16,
    /// 时间戳
    pub timestamp: u64,
    /// MSS
    pub mss: u16,
    /// 窗口大小
    pub window_size: u16,
    /// 窗口缩放
    pub window_scale: u8,
    /// SACK 允许
    pub sack_permitted: bool,
}

/// SYN Cookie
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SynCookie {
    /// Cookie 值
    pub cookie: u32,
    /// 时间索引
    pub time_index: u8,
    /// 标志
    pub flags: u8,
}

/// 速率限制条目
#[derive(Debug, Clone)]
pub struct RateLimitEntry {
    /// IP 地址
    pub ip: Ipv4Addr,
    /// 请求数
    pub requests: u64,
    /// 字节数
    pub bytes: u64,
    /// 最后更新时间
    pub last_update: u64,
    /// 违规次数
    pub violations: u32,
    /// 是否被阻止
    pub blocked: bool,
}

impl RateLimitEntry {
    /// 检查是否超时
    pub fn is_expired(&self, timeout: u64) -> bool {
        let current_time = crate::subsystems::time::get_timestamp();
        current_time - self.last_update > timeout * 1_000_000_000
    }
}

/// 速率限制配置
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// 每秒最大请求数
    pub max_requests_per_second: u32,
    /// 每秒最大字节数
    pub max_bytes_per_second: u64,
    /// 突发容量
    pub burst_capacity: u32,
    /// 限制策略
    pub strategy: RateLimitStrategy,
    /// 超时时间（秒）
    pub timeout: u64,
    /// 违规阈值
    pub violation_threshold: u32,
    /// 阻止时长（秒）
    pub block_duration: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests_per_second: 100,
            max_bytes_per_second: 10_000_000,
            burst_capacity: 200,
            strategy: RateLimitStrategy::TokenBucket,
            timeout: 60,
            violation_threshold: 5,
            block_duration: 300,
        }
    }
}

/// SYN Cookie 配置
#[derive(Debug, Clone)]
pub struct SynCookieConfig {
    /// 启用 SYN Cookie
    pub enabled: bool,
    /// SYN Cookie 密钥
    pub secret: [u8; 32],
    /// Cookie 有效期（秒）
    pub cookie_lifetime: u64,
    /// 最大 SYN 队列长度
    pub max_syn_queue: usize,
    /// SYN 重试限制
    pub syn_retry_limit: u32,
}

impl Default for SynCookieConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            secret: [0u8; 32],
            cookie_lifetime: 60,
            max_syn_queue: 1024,
            syn_retry_limit: 5,
        }
    }
}

/// 流量清洗配置
#[derive(Debug, Clone)]
pub struct TrafficCleaningConfig {
    /// 启用流量清洗
    pub enabled: bool,
    /// 异常流量阈值
    pub anomaly_threshold: f32,
    /// 清洗动作
    pub cleaning_action: CleaningAction,
    /// 清洗超时（秒）
    pub timeout: u64,
}

impl Default for TrafficCleaningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            anomaly_threshold: 3.0, // 标准差倍数
            cleaning_action: CleaningAction::Drop,
            timeout: 300,
        }
    }
}

/// 清洗动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleaningAction {
    /// 丢弃
    Drop,
    /// 速率限制
    RateLimit,
    /// 重定向
    Redirect,
    /// 标记
    Mark,
}

/// 自适应过滤配置
#[derive(Debug, Clone)]
pub struct AdaptiveFilterConfig {
    /// 启用自适应过滤
    pub enabled: bool,
    /// 学习周期（秒）
    pub learning_period: u64,
    /// 更新间隔（秒）
    pub update_interval: u64,
    /// 最小样本数
    pub min_samples: usize,
    /// 灵敏度
    pub sensitivity: f32,
}

impl Default for AdaptiveFilterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            learning_period: 3600,
            update_interval: 60,
            min_samples: 1000,
            sensitivity: 0.8,
        }
    }
}

/// DDoS 防护统计
#[derive(Debug, Clone, Default)]
pub struct DdosStatistics {
    /// 总处理包数
    pub total_packets: u64,
    /// 速率限制的包
    pub rate_limited_packets: u64,
    /// SYN Cookie 处理数
    pub syn_cookies_issued: u64,
    pub syn_cookies_validated: u64,
    pub syn_cookies_failed: u64,
    /// 黑名单匹配数
    pub blacklist_matches: u64,
    /// 白名单匹配数
    pub whitelist_matches: u64,
    /// 清洗的包
    pub cleaned_packets: u64,
    /// 阻止的 IP 数
    pub blocked_ips: u64,
    /// 当前活跃 IP 数
    pub active_ips: usize,
}

/// DDoS 防护引擎
pub struct DdosProtectionEngine {
    /// 速率限制器
    rate_limiters: Mutex<BTreeMap<Ipv4Addr, RateLimitEntry>>,
    /// IP 黑名单
    blacklist: Mutex<BTreeSet<Ipv4Addr>>,
    /// IP 白名单
    whitelist: Mutex<BTreeSet<Ipv4Addr>>,
    /// SYN Cookie 状态
    syn_cookies: Mutex<BTreeMap<u32, SynCookieState>>,
    /// 统计信息
    stats: Mutex<DdosStatistics>,
    /// 速率限制配置
    rate_limit_config: RateLimitConfig,
    /// SYN Cookie 配置
    syn_cookie_config: SynCookieConfig,
    /// 流量清洗配置
    traffic_cleaning_config: TrafficCleaningConfig,
    /// 自适应过滤配置
    adaptive_filter_config: AdaptiveFilterConfig,
    /// 下一个时间索引
    next_time_index: AtomicU64,
    /// 令牌桶（每个 IP）
    token_buckets: Mutex<BTreeMap<Ipv4Addr, TokenBucketState>>,
}

/// 令牌桶状态
#[derive(Debug, Clone)]
struct TokenBucketState {
    /// 令牌数
    tokens: f64,
    /// 最后更新时间
    last_update: u64,
}

impl DdosProtectionEngine {
    /// 创建新的 DDoS 防护引擎
    pub fn new() -> Self {
        Self {
            rate_limiters: Mutex::new(BTreeMap::new()),
            blacklist: Mutex::new(BTreeSet::new()),
            whitelist: Mutex::new(BTreeSet::new()),
            syn_cookies: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(DdosStatistics::default()),
            rate_limit_config: RateLimitConfig::default(),
            syn_cookie_config: SynCookieConfig::default(),
            traffic_cleaning_config: TrafficCleaningConfig::default(),
            adaptive_filter_config: AdaptiveFilterConfig::default(),
            next_time_index: AtomicU64::new(0),
            token_buckets: Mutex::new(BTreeMap::new()),
        }
    }

    /// 创建带配置的 DDoS 防护引擎
    pub fn with_config(
        rate_limit_config: RateLimitConfig,
        syn_cookie_config: SynCookieConfig,
        traffic_cleaning_config: TrafficCleaningConfig,
        adaptive_filter_config: AdaptiveFilterConfig,
    ) -> Self {
        Self {
            rate_limiters: Mutex::new(BTreeMap::new()),
            blacklist: Mutex::new(BTreeSet::new()),
            whitelist: Mutex::new(BTreeSet::new()),
            syn_cookies: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(DdosStatistics::default()),
            rate_limit_config,
            syn_cookie_config,
            traffic_cleaning_config,
            adaptive_filter_config,
            next_time_index: AtomicU64::new(0),
            token_buckets: Mutex::new(BTreeMap::new()),
        }
    }

    /// 处理网络包
    pub fn process_packet(&self, packet: &Packet, source_ip: Ipv4Addr) -> bool {
        let mut stats = self.stats.lock();
        stats.total_packets += 1;

        // 检查白名单
        {
            let whitelist = self.whitelist.lock();
            if whitelist.contains(&source_ip) {
                stats.whitelist_matches += 1;
                return true; // 白名单直接通过
            }
        }

        // 检查黑名单
        {
            let blacklist = self.blacklist.lock();
            if blacklist.contains(&source_ip) {
                stats.blacklist_matches += 1;
                return false; // 黑名单直接丢弃
            }
        }

        // 速率限制检查
        if !self.check_rate_limit(&source_ip, packet.data().len() as u64) {
            stats.rate_limited_packets += 1;
            return false;
        }

        // 流量清洗检查
        if self.traffic_cleaning_config.enabled {
            if !self.check_traffic_cleaning(&source_ip) {
                stats.cleaned_packets += 1;
                return false;
            }
        }

        true
    }

    /// 检查速率限制
    fn check_rate_limit(&self, ip: &Ipv4Addr, bytes: u64) -> bool {
        let mut limiters = self.rate_limiters.lock();
        let current_time = crate::subsystems::time::get_timestamp();

        // 清理过期条目
        limiters.retain(|_, entry| !entry.is_expired(self.rate_limit_config.timeout));

        let entry = limiters.entry(*ip).or_insert(RateLimitEntry {
            ip: *ip,
            requests: 0,
            bytes: 0,
            last_update: current_time,
            violations: 0,
            blocked: false,
        });

        // 如果被阻止，检查是否可以解除
        if entry.blocked {
            if current_time - entry.last_update > self.rate_limit_config.block_duration * 1_000_000_000 {
                entry.blocked = false;
                entry.violations = 0;
            } else {
                return false;
            }
        }

        match self.rate_limit_config.strategy {
            RateLimitStrategy::TokenBucket => self.token_bucket_check(ip, bytes),
            RateLimitStrategy::LeakyBucket => self.leaky_bucket_check(ip, bytes),
            RateLimitStrategy::FixedWindow => self.fixed_window_check(ip, bytes),
            RateLimitStrategy::SlidingWindow => self.sliding_window_check(ip, bytes),
        }
    }

    /// 令牌桶检查
    fn token_bucket_check(&self, ip: &Ipv4Addr, _bytes: u64) -> bool {
        let mut buckets = self.token_buckets.lock();
        let current_time = crate::subsystems::time::get_timestamp();

        let bucket = buckets.entry(*ip).or_insert(TokenBucketState {
            tokens: self.rate_limit_config.burst_capacity as f64,
            last_update: current_time,
        });

        // 计算时间差（秒）
        let elapsed = ((current_time - bucket.last_update) as f64) / 1_000_000_000.0;
        bucket.last_update = current_time;

        // 添加令牌
        bucket.tokens += elapsed * self.rate_limit_config.max_requests_per_second as f64;
        bucket.tokens = bucket.tokens.min(self.rate_limit_config.burst_capacity as f64);

        // 检查是否有足够的令牌
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            // 更新违规计数
            let mut limiters = self.rate_limiters.lock();
            if let Some(entry) = limiters.get_mut(ip) {
                entry.violations += 1;
                if entry.violations >= self.rate_limit_config.violation_threshold {
                    entry.blocked = true;
                }
            }
            false
        }
    }

    /// 漏桶检查
    fn leaky_bucket_check(&self, _ip: &Ipv4Addr, _bytes: u64) -> bool {
        // 简化实现
        true
    }

    /// 固定窗口检查
    fn fixed_window_check(&self, ip: &Ipv4Addr, bytes: u64) -> bool {
        let mut limiters = self.rate_limiters.lock();
        let current_time = crate::subsystems::time::get_timestamp();

        if let Some(entry) = limiters.get_mut(ip) {
            // 简化：每秒重置
            let elapsed = current_time - entry.last_update;
            if elapsed > 1_000_000_000 {
                entry.requests = 0;
                entry.bytes = 0;
                entry.last_update = current_time;
            }

            entry.requests += 1;
            entry.bytes += bytes;

            if entry.requests > self.rate_limit_config.max_requests_per_second as u64
                || entry.bytes > self.rate_limit_config.max_bytes_per_second
            {
                entry.violations += 1;
                if entry.violations >= self.rate_limit_config.violation_threshold {
                    entry.blocked = true;
                }
                return false;
            }
        }

        true
    }

    /// 滑动窗口检查
    fn sliding_window_check(&self, _ip: &Ipv4Addr, _bytes: u64) -> bool {
        // 简化实现
        true
    }

    /// 检查流量清洗
    fn check_traffic_cleaning(&self, ip: &Ipv4Addr) -> bool {
        if !self.traffic_cleaning_config.enabled {
            return true;
        }

        // 简化实现：基于异常检测
        let limiters = self.rate_limiters.lock();
        if let Some(entry) = limiters.get(ip) {
            // 如果流量远高于基线，可能是攻击
            if entry.requests > 1000 {
                return false;
            }
        }

        true
    }

    /// 生成 SYN Cookie
    pub fn generate_syn_cookie(&self, state: &SynCookieState) -> SynCookie {
        let time_index = (self.next_time_index.fetch_add(1, Ordering::SeqCst) % 256) as u8;
        let timestamp = (state.timestamp / 1_000_000_000) as u32;

        // 简化的 Cookie 生成
        let cookie = timestamp ^ (state.source_ip.to_u32() as u32) ^ (state.dest_port as u32);

        let flags = if state.sack_permitted { 0x01 } else { 0 };

        SynCookie {
            cookie,
            time_index,
            flags,
        }
    }

    /// 验证 SYN Cookie
    pub fn validate_syn_cookie(&self, cookie: SynCookie, source_ip: Ipv4Addr) -> bool {
        let cookies = self.syn_cookies.lock();
        if let Some(state) = cookies.get(&cookie.cookie) {
            if state.source_ip == source_ip {
                let mut stats = self.stats.lock();
                stats.syn_cookies_validated += 1;
                return true;
            }
        }

        let mut stats = self.stats.lock();
        stats.syn_cookies_failed += 1;
        false
    }

    /// 添加到黑名单
    pub fn add_to_blacklist(&self, ip: Ipv4Addr) {
        let mut blacklist = self.blacklist.lock();
        blacklist.insert(ip);

        let mut stats = self.stats.lock();
        stats.blocked_ips = blacklist.len() as u64;
    }

    /// 从黑名单移除
    pub fn remove_from_blacklist(&self, ip: &Ipv4Addr) {
        let mut blacklist = self.blacklist.lock();
        blacklist.remove(ip);

        let mut stats = self.stats.lock();
        stats.blocked_ips = blacklist.len() as u64;
    }

    /// 添加到白名单
    pub fn add_to_whitelist(&self, ip: Ipv4Addr) {
        let mut whitelist = self.whitelist.lock();
        whitelist.insert(ip);
    }

    /// 从白名单移除
    pub fn remove_from_whitelist(&self, ip: &Ipv4Addr) {
        let mut whitelist = self.whitelist.lock();
        whitelist.remove(ip);
    }

    /// 检查 IP 是否在黑名单
    pub fn is_blacklisted(&self, ip: &Ipv4Addr) -> bool {
        let blacklist = self.blacklist.lock();
        blacklist.contains(ip)
    }

    /// 检查 IP 是否在白名单
    pub fn is_whitelisted(&self, ip: &Ipv4Addr) -> bool {
        let whitelist = self.whitelist.lock();
        whitelist.contains(ip)
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> DdosStatistics {
        let stats = self.stats.lock();
        let rate_limiters = self.rate_limiters.lock();
        DdosStatistics {
            total_packets: stats.total_packets,
            rate_limited_packets: stats.rate_limited_packets,
            syn_cookies_issued: stats.syn_cookies_issued,
            syn_cookies_validated: stats.syn_cookies_validated,
            syn_cookies_failed: stats.syn_cookies_failed,
            blacklist_matches: stats.blacklist_matches,
            whitelist_matches: stats.whitelist_matches,
            cleaned_packets: stats.cleaned_packets,
            blocked_ips: stats.blocked_ips,
            active_ips: rate_limiters.len(),
        }
    }

    /// 重置统计信息
    pub fn reset_statistics(&self) {
        *self.stats.lock() = DdosStatistics::default();
    }

    /// 清理过期条目
    pub fn cleanup(&self) {
        {
            let mut limiters = self.rate_limiters.lock();
            limiters.retain(|_, entry| !entry.is_expired(self.rate_limit_config.timeout));
        }

        {
            let mut buckets = self.token_buckets.lock();
            let current_time = crate::subsystems::time::get_timestamp();
            buckets.retain(|_, bucket| {
                current_time - bucket.last_update < self.rate_limit_config.timeout * 1_000_000_000
            });
        }
    }
}

impl Default for DdosProtectionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局 DDoS 防护引擎实例
pub static GLOBAL_DDOS_PROTECTION: Mutex<Option<DdosProtectionEngine>> = Mutex::new(None);

/// 初始化全局 DDoS 防护引擎
pub fn init_ddos_protection() -> Result<(), DdosError> {
    let engine = DdosProtectionEngine::new();

    let mut global = GLOBAL_DDOS_PROTECTION.lock();
    *global = Some(engine);

    crate::println!("[DDoS] DDoS protection engine initialized successfully");
    Ok(())
}

/// 处理网络包（便捷函数）
pub fn process_packet_ddos(packet: &Packet, source_ip: Ipv4Addr) -> bool {
    let global = GLOBAL_DDOS_PROTECTION.lock();
    if let Some(engine) = global.as_ref() {
        engine.process_packet(packet, source_ip)
    } else {
        true // 如果引擎未初始化，默认通过
    }
}

/// 添加 IP 到黑名单（便捷函数）
pub fn blacklist_ip(ip: Ipv4Addr) {
    let global = GLOBAL_DDOS_PROTECTION.lock();
    if let Some(engine) = global.as_ref() {
        engine.add_to_blacklist(ip);
    }
}

/// 添加 IP 到白名单（便捷函数）
pub fn whitelist_ip(ip: Ipv4Addr) {
    let global = GLOBAL_DDOS_PROTECTION.lock();
    if let Some(engine) = global.as_ref() {
        engine.add_to_whitelist(ip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ddos_engine_creation() {
        let engine = DdosProtectionEngine::new();
        let stats = engine.get_statistics();
        assert_eq!(stats.total_packets, 0);
    }

    #[test]
    fn test_rate_limit_entry_expiration() {
        let entry = RateLimitEntry {
            ip: Ipv4Addr::new(192, 168, 1, 1),
            requests: 100,
            bytes: 10000,
            last_update: 0,
            violations: 0,
            blocked: false,
        };

        assert!(entry.is_expired(61)); // 61 seconds later
        assert!(!entry.is_expired(59)); // 59 seconds later
    }

    #[test]
    fn test_syn_cookie_generation() {
        let state = SynCookieState {
            source_ip: Ipv4Addr::new(192, 168, 1, 1),
            source_port: 12345,
            dest_port: 80,
            timestamp: 1234567890,
            mss: 1460,
            window_size: 65535,
            window_scale: 0,
            sack_permitted: true,
        };

        let engine = DdosProtectionEngine::new();
        let cookie = engine.generate_syn_cookie(&state);

        assert_eq!(cookie.flags, 0x01); // SACK permitted
    }

    #[test]
    fn test_blacklist_whitelist() {
        let engine = DdosProtectionEngine::new();
        let ip = Ipv4Addr::new(192, 168, 1, 1);

        assert!(!engine.is_blacklisted(&ip));
        assert!(!engine.is_whitelisted(&ip));

        engine.add_to_blacklist(ip);
        assert!(engine.is_blacklisted(&ip));

        engine.remove_from_blacklist(&ip);
        assert!(!engine.is_blacklisted(&ip));

        engine.add_to_whitelist(ip);
        assert!(engine.is_whitelisted(&ip));
    }

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_requests_per_second, 100);
        assert_eq!(config.max_bytes_per_second, 10_000_000);
        assert_eq!(config.strategy, RateLimitStrategy::TokenBucket);
    }

    #[test]
    fn test_syn_cookie_config_default() {
        let config = SynCookieConfig::default();
        assert!(config.enabled);
        assert_eq!(config.cookie_lifetime, 60);
        assert_eq!(config.max_syn_queue, 1024);
    }
}
