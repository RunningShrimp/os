//! # 防火墙和包过滤引擎
//!
//! 提供企业级防火墙功能，包括：
//! - 包过滤规则（iptables/nftables 风格）
//! - 连接跟踪（conntrack）
//! - NAT 转换（SNAT/DNAT/MASQUERADE）
//! - 状态检测
//! - 规则链和匹配

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    fmt,
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use crate::net::{
    ipv4::Ipv4Addr,
    packet::Packet,
};

/// 防火墙错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirewallError {
    /// 规则不存在
    RuleNotFound,
    /// 链不存在
    ChainNotFound,
    /// 无效的规则
    InvalidRule,
    /// 连接跟踪表已满
    ConntrackTableFull,
    /// NAT 表已满
    NatTableFull,
    /// 权限被拒绝
    PermissionDenied,
    /// 内部错误
    InternalError(String),
}

impl fmt::Display for FirewallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FirewallError::RuleNotFound => write!(f, "Rule not found"),
            FirewallError::ChainNotFound => write!(f, "Chain not found"),
            FirewallError::InvalidRule => write!(f, "Invalid rule"),
            FirewallError::ConntrackTableFull => write!(f, "Connection tracking table full"),
            FirewallError::NatTableFull => write!(f, "NAT table full"),
            FirewallError::PermissionDenied => write!(f, "Permission denied"),
            FirewallError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

/// 规则目标动作
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuleTarget {
    /// 接受
    Accept,
    /// 拒绝
    Drop,
    /// 拒绝并通知
    Reject,
    /// 跳转到另一条链
    Goto(String),
    /// 调用用户空间
    Return,
    /// MASQUERADE
    Masquerade,
    /// REDIRECT
    Redirect,
    /// SNAT
    SNAT,
    /// DNAT
    DNAT,
    /// 记录日志
    Log,
}

/// 协议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    /// TCP
    Tcp,
    /// UDP
    Udp,
    /// ICMP
    Icmp,
    /// 所有协议
    All,
    /// 协议编号
    Other(u8),
}

/// 连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConntrackState {
    /// 新连接
    New,
    /// 已建立
    Established,
    /// 相关连接
    Related,
    /// 无效连接
    Invalid,
}

/// 地址匹配条件
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressMatch {
    /// IP地址
    pub addr: Ipv4Addr,
    /// 子网掩码
    pub netmask: Ipv4Addr,
    /// 是否取反
    pub inverted: bool,
}

impl AddressMatch {
    /// 创建新的地址匹配
    pub fn new(addr: Ipv4Addr, netmask: Ipv4Addr) -> Self {
        Self {
            addr,
            netmask,
            inverted: false,
        }
    }

    /// 创建取反的地址匹配
    pub fn inverted(mut self) -> Self {
        self.inverted = true;
        self
    }

    /// 检查地址是否匹配
    pub fn matches(&self, target: Ipv4Addr) -> bool {
        let matches = (target.to_u32() & self.netmask.to_u32()) == (self.addr.to_u32() & self.netmask.to_u32());
        if self.inverted {
            !matches
        } else {
            matches
        }
    }
}

/// 端口匹配条件
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortMatch {
    /// 起始端口
    pub port_start: u16,
    /// 结束端口
    pub port_end: u16,
    /// 是否取反
    pub inverted: bool,
}

impl PortMatch {
    /// 创建单个端口匹配
    pub fn single(port: u16) -> Self {
        Self {
            port_start: port,
            port_end: port,
            inverted: false,
        }
    }

    /// 创建端口范围匹配
    pub fn range(start: u16, end: u16) -> Self {
        Self {
            port_start: start,
            port_end: end,
            inverted: false,
        }
    }

    /// 创建取反的端口匹配
    pub fn inverted(mut self) -> Self {
        self.inverted = true;
        self
    }

    /// 检查端口是否匹配
    pub fn matches(&self, port: u16) -> bool {
        let matches = port >= self.port_start && port <= self.port_end;
        if self.inverted {
            !matches
        } else {
            matches
        }
    }
}

/// 包匹配条件
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketMatch {
    /// 协议
    pub protocol: Option<Protocol>,
    /// 源地址
    pub source_addr: Option<AddressMatch>,
    /// 目标地址
    pub dest_addr: Option<AddressMatch>,
    /// 源端口
    pub source_port: Option<PortMatch>,
    /// 目标端口
    pub dest_port: Option<PortMatch>,
    /// 入站接口
    pub in_interface: Option<String>,
    /// 出站接口
    pub out_interface: Option<String>,
    /// 连接状态
    pub conntrack_state: Option<ConntrackState>,
    /// MAC 地址
    pub mac_source: Option<[u8; 6]>,
    pub mac_dest: Option<[u8; 6]>,
    /// 包长度
    pub packet_length: Option<(usize, usize)>,
    /// TCP 标志
    pub tcp_flags: Option<TcpFlags>,
    /// 限制
    pub limit: Option<RateLimit>,
}

impl Default for PacketMatch {
    fn default() -> Self {
        Self {
            protocol: None,
            source_addr: None,
            dest_addr: None,
            source_port: None,
            dest_port: None,
            in_interface: None,
            out_interface: None,
            conntrack_state: None,
            mac_source: None,
            mac_dest: None,
            packet_length: None,
            tcp_flags: None,
            limit: None,
        }
    }
}

/// TCP 标志
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TcpFlags {
    /// SYN 标志
    pub syn: bool,
    /// ACK 标志
    pub ack: bool,
    /// FIN 标志
    pub fin: bool,
    /// RST 标志
    pub rst: bool,
    /// PSH 标志
    pub psh: bool,
    /// URG 标志
    pub urg: bool,
    /// 标志掩码（哪些标志需要检查）
    pub mask: u8,
}

impl TcpFlags {
    /// 创建新的 TCP 标志匹配
    pub fn new() -> Self {
        Self {
            syn: false,
            ack: false,
            fin: false,
            rst: false,
            psh: false,
            urg: false,
            mask: 0,
        }
    }

    /// 设置 SYN 标志
    pub fn syn(mut self, syn: bool) -> Self {
        self.syn = syn;
        self.mask |= 0x02;
        self
    }

    /// 设置 ACK 标志
    pub fn ack(mut self, ack: bool) -> Self {
        self.ack = ack;
        self.mask |= 0x10;
        self
    }

    /// 设置 FIN 标志
    pub fn fin(mut self, fin: bool) -> Self {
        self.fin = fin;
        self.mask |= 0x01;
        self
    }

    /// 设置 RST 标志
    pub fn rst(mut self, rst: bool) -> Self {
        self.rst = rst;
        self.mask |= 0x04;
        self
    }

    /// 检查 TCP 标志是否匹配
    pub fn matches(&self, flags: u8) -> bool {
        if self.mask == 0 {
            return true;
        }

        let mut result = true;
        if self.mask & 0x02 != 0 {
            result &= ((flags & 0x02) != 0) == self.syn;
        }
        if self.mask & 0x10 != 0 {
            result &= ((flags & 0x10) != 0) == self.ack;
        }
        if self.mask & 0x01 != 0 {
            result &= ((flags & 0x01) != 0) == self.fin;
        }
        if self.mask & 0x04 != 0 {
            result &= ((flags & 0x04) != 0) == self.rst;
        }
        result
    }
}

impl Default for TcpFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// 速率限制
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimit {
    /// 时间间隔（秒）
    pub interval: u64,
    /// 间隔内允许的包数
    pub burst: u32,
    /// 平均速率（包/秒）
    pub rate: u32,
}

impl RateLimit {
    /// 创建新的速率限制
    pub fn new(rate: u32, burst: u32, interval: u64) -> Self {
        Self {
            interval,
            burst,
            rate,
        }
    }
}

/// 防火墙规则
#[derive(Debug, Clone)]
pub struct FirewallRule {
    /// 规则 ID
    pub id: u64,
    /// 规则序号
    pub priority: u32,
    /// 匹配条件
    pub match_condition: PacketMatch,
    /// 目标动作
    pub target: RuleTarget,
    /// 规则描述
    pub description: String,
    /// 规则统计
    pub stats: RuleStats,
    /// 是否启用
    pub enabled: bool,
}

/// 规则统计
#[derive(Debug, Clone, Default)]
pub struct RuleStats {
    /// 匹配次数
    pub matches: u64,
    /// 字节数
    pub bytes: u64,
    /// 最后匹配时间
    pub last_match: Option<u64>,
}

/// 防火墙链
#[derive(Debug)]
pub struct FirewallChain {
    /// 链名称
    pub name: String,
    /// 链类型
    pub chain_type: ChainType,
    /// 规则列表
    pub rules: Vec<FirewallRule>,
    /// 默认策略
    pub policy: RuleTarget,
    /// 规则计数器
    next_rule_id: AtomicU64,
}

impl Clone for FirewallChain {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            chain_type: self.chain_type,
            rules: self.rules.clone(),
            policy: self.policy.clone(),
            next_rule_id: AtomicU64::new(self.next_rule_id.load(core::sync::atomic::Ordering::Relaxed)),
        }
    }
}

/// 链类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainType {
    /// 内置链
    BuiltIn,
    /// 用户自定义链
    UserDefined,
}

impl FirewallChain {
    /// 创建新的防火墙链
    pub fn new(name: String, chain_type: ChainType, policy: RuleTarget) -> Self {
        Self {
            name,
            chain_type,
            rules: Vec::new(),
            policy,
            next_rule_id: AtomicU64::new(1),
        }
    }

    /// 添加规则
    pub fn add_rule(&mut self, match_condition: PacketMatch, target: RuleTarget) -> u64 {
        let id = self.next_rule_id.fetch_add(1, Ordering::SeqCst);
        let rule = FirewallRule {
            id,
            priority: self.rules.len() as u32,
            match_condition,
            target,
            description: String::new(),
            stats: RuleStats::default(),
            enabled: true,
        };
        self.rules.push(rule);
        id
    }

    /// 删除规则
    pub fn remove_rule(&mut self, rule_id: u64) -> Result<(), FirewallError> {
        let pos = self
            .rules
            .iter()
            .position(|r| r.id == rule_id)
            .ok_or(FirewallError::RuleNotFound)?;
        self.rules.remove(pos);
        Ok(())
    }

    /// 插入规则到指定位置
    pub fn insert_rule(
        &mut self,
        index: usize,
        match_condition: PacketMatch,
        target: RuleTarget,
    ) -> u64 {
        let id = self.next_rule_id.fetch_add(1, Ordering::SeqCst);
        let rule = FirewallRule {
            id,
            priority: index as u32,
            match_condition,
            target,
            description: String::new(),
            stats: RuleStats::default(),
            enabled: true,
        };
        self.rules.insert(index, rule);
        // 更新后续规则的优先级
        for (i, r) in self.rules.iter_mut().enumerate().skip(index + 1) {
            r.priority = i as u32;
        }
        id
    }

    /// 替换规则
    pub fn replace_rule(
        &mut self,
        rule_id: u64,
        match_condition: PacketMatch,
        target: RuleTarget,
    ) -> Result<(), FirewallError> {
        let rule = self
            .rules
            .iter_mut()
            .find(|r| r.id == rule_id)
            .ok_or(FirewallError::RuleNotFound)?;
        rule.match_condition = match_condition;
        rule.target = target;
        Ok(())
    }

    /// 获取规则
    pub fn get_rule(&self, rule_id: u64) -> Option<&FirewallRule> {
        self.rules.iter().find(|r| r.id == rule_id)
    }

    /// 获取规则（可变）
    pub fn get_rule_mut(&mut self, rule_id: u64) -> Option<&mut FirewallRule> {
        self.rules.iter_mut().find(|r| r.id == rule_id)
    }

    /// 清空规则
    pub fn flush_rules(&mut self) {
        self.rules.clear();
    }

    /// 统计规则数量
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

/// 连接跟踪条目
#[derive(Debug, Clone)]
pub struct ConntrackEntry {
    /// 连接 ID
    pub id: u64,
    /// 源地址
    pub source_addr: Ipv4Addr,
    /// 源端口
    pub source_port: u16,
    /// 目标地址
    pub dest_addr: Ipv4Addr,
    /// 目标端口
    pub dest_port: u16,
    /// 协议
    pub protocol: Protocol,
    /// 连接状态
    pub state: ConntrackState,
    /// 创建时间
    pub created_at: u64,
    /// 最后活动时间
    pub last_activity: u64,
    /// 包统计
    pub packets_origin: u64,
    pub packets_reply: u64,
    pub bytes_origin: u64,
    pub bytes_reply: u64,
    /// 超时时间（秒）
    pub timeout: u64,
}

impl ConntrackEntry {
    /// 检查连接是否超时
    pub fn is_expired(&self, current_time: u64) -> bool {
        current_time - self.last_activity > self.timeout * 1_000_000_000
    }

    /// 更新活动时间
    pub fn update_activity(&mut self, current_time: u64) {
        self.last_activity = current_time;
    }
}

/// NAT 映射条目
#[derive(Debug, Clone)]
pub struct NatEntry {
    /// 映射 ID
    pub id: u64,
    /// NAT 类型
    pub nat_type: NatType,
    /// 原始地址
    pub original_addr: Ipv4Addr,
    /// 原始端口
    pub original_port: u16,
    /// 转换后地址
    pub translated_addr: Ipv4Addr,
    /// 转换后端口
    pub translated_port: u16,
    /// 创建时间
    pub created_at: u64,
    /// 最后使用时间
    pub last_used: u64,
}

/// NAT 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatType {
    /// SNAT（源地址转换）
    SNAT,
    /// DNAT（目标地址转换）
    DNAT,
    /// MASQUERADE
    Masquerade,
    /// REDIRECT
    Redirect,
}

impl NatEntry {
    /// 检查映射是否过期
    pub fn is_expired(&self, current_time: u64, timeout: u64) -> bool {
        current_time - self.last_used > timeout * 1_000_000_000
    }

    /// 更新使用时间
    pub fn update_usage(&mut self, current_time: u64) {
        self.last_used = current_time;
    }
}

/// 防火墙统计信息
#[derive(Debug, Clone, Default)]
pub struct FirewallStats {
    /// 总处理包数
    pub total_packets: u64,
    /// 接受的包
    pub accepted_packets: u64,
    /// 拒绝的包
    pub dropped_packets: u64,
    /// 拒绝并通知的包
    pub rejected_packets: u64,
    /// 转发的包
    pub forwarded_packets: u64,
    /// NAT 映射数
    pub nat_mappings: u64,
    /// 连接跟踪条目数
    pub conntrack_entries: u64,
}

/// 防火墙引擎
pub struct FirewallEngine {
    /// INPUT 链
    input_chain: Mutex<FirewallChain>,
    /// OUTPUT 链
    output_chain: Mutex<FirewallChain>,
    /// FORWARD 链
    forward_chain: Mutex<FirewallChain>,
    /// 用户自定义链
    custom_chains: Mutex<BTreeMap<String, FirewallChain>>,
    /// 连接跟踪表
    conntrack_table: Mutex<BTreeMap<u64, ConntrackEntry>>,
    /// NAT 映射表
    nat_table: Mutex<BTreeMap<u64, NatEntry>>,
    /// 统计信息
    stats: Mutex<FirewallStats>,
    /// 连接跟踪配置
    conntrack_config: ConntrackConfig,
    /// NAT 配置
    nat_config: NatConfig,
    /// 下一个条目 ID
    next_entry_id: AtomicU64,
}

/// 连接跟踪配置
#[derive(Debug, Clone)]
pub struct ConntrackConfig {
    /// 最大跟踪条目数
    pub max_entries: usize,
    /// TCP 超时（秒）
    pub tcp_timeout: u64,
    /// UDP 超时（秒）
    pub udp_timeout: u64,
    /// ICMP 超时（秒）
    pub icmp_timeout: u64,
    /// 已建立连接超时（秒）
    pub established_timeout: u64,
}

impl Default for ConntrackConfig {
    fn default() -> Self {
        Self {
            max_entries: 100000,
            tcp_timeout: 300,
            udp_timeout: 60,
            icmp_timeout: 30,
            established_timeout: 43200, // 12 hours
        }
    }
}

/// NAT 配置
#[derive(Debug, Clone)]
pub struct NatConfig {
    /// 最大 NAT 条目数
    pub max_entries: usize,
    /// NAT 超时（秒）
    pub timeout: u64,
    /// 端口范围
    pub port_range: (u16, u16),
}

impl Default for NatConfig {
    fn default() -> Self {
        Self {
            max_entries: 50000,
            timeout: 300,
            port_range: (1024, 65535),
        }
    }
}

impl FirewallEngine {
    /// 创建新的防火墙引擎
    pub fn new() -> Self {
        Self {
            input_chain: Mutex::new(FirewallChain::new(
                String::from("INPUT"),
                ChainType::BuiltIn,
                RuleTarget::Accept,
            )),
            output_chain: Mutex::new(FirewallChain::new(
                String::from("OUTPUT"),
                ChainType::BuiltIn,
                RuleTarget::Accept,
            )),
            forward_chain: Mutex::new(FirewallChain::new(
                String::from("FORWARD"),
                ChainType::BuiltIn,
                RuleTarget::Accept,
            )),
            custom_chains: Mutex::new(BTreeMap::new()),
            conntrack_table: Mutex::new(BTreeMap::new()),
            nat_table: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(FirewallStats::default()),
            conntrack_config: ConntrackConfig::default(),
            nat_config: NatConfig::default(),
            next_entry_id: AtomicU64::new(1),
        }
    }

    /// 创建带配置的防火墙引擎
    pub fn with_config(conntrack_config: ConntrackConfig, nat_config: NatConfig) -> Self {
        Self {
            input_chain: Mutex::new(FirewallChain::new(
                String::from("INPUT"),
                ChainType::BuiltIn,
                RuleTarget::Accept,
            )),
            output_chain: Mutex::new(FirewallChain::new(
                String::from("OUTPUT"),
                ChainType::BuiltIn,
                RuleTarget::Accept,
            )),
            forward_chain: Mutex::new(FirewallChain::new(
                String::from("FORWARD"),
                ChainType::BuiltIn,
                RuleTarget::Accept,
            )),
            custom_chains: Mutex::new(BTreeMap::new()),
            conntrack_table: Mutex::new(BTreeMap::new()),
            nat_table: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(FirewallStats::default()),
            conntrack_config,
            nat_config,
            next_entry_id: AtomicU64::new(1),
        }
    }

    /// 处理入站包
    pub fn process_input_packet(
        &self,
        packet: &Packet,
        in_interface: &str,
    ) -> Result<RuleTarget, FirewallError> {
        let mut chain = self.input_chain.lock();
        self.process_chain(&mut chain, packet, Some(in_interface), None)
    }

    /// 处理出站包
    pub fn process_output_packet(
        &self,
        packet: &Packet,
        out_interface: &str,
    ) -> Result<RuleTarget, FirewallError> {
        let mut chain = self.output_chain.lock();
        self.process_chain(&mut chain, packet, None, Some(out_interface))
    }

    /// 处理转发包
    pub fn process_forward_packet(
        &self,
        packet: &Packet,
        in_interface: &str,
        out_interface: &str,
    ) -> Result<RuleTarget, FirewallError> {
        let mut chain = self.forward_chain.lock();
        self.process_chain(&mut chain, packet, Some(in_interface), Some(out_interface))
    }

    /// 处理链
    fn process_chain(
        &self,
        chain: &mut FirewallChain,
        packet: &Packet,
        in_interface: Option<&str>,
        out_interface: Option<&str>,
    ) -> Result<RuleTarget, FirewallError> {
        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.total_packets += 1;
        }

        // 检查每条规则
        for rule in &mut chain.rules {
            if !rule.enabled {
                continue;
            }

            if self.matches_rule(packet, &rule.match_condition, in_interface, out_interface) {
                // 更新规则统计
                rule.stats.matches += 1;
                rule.stats.last_match = Some(crate::subsystems::time::get_timestamp());

                // 执行目标动作
                match rule.target {
                    RuleTarget::Accept => {
                        let mut stats = self.stats.lock();
                        stats.accepted_packets += 1;
                        return Ok(RuleTarget::Accept);
                    }
                    RuleTarget::Drop => {
                        let mut stats = self.stats.lock();
                        stats.dropped_packets += 1;
                        return Ok(RuleTarget::Drop);
                    }
                    RuleTarget::Reject => {
                        let mut stats = self.stats.lock();
                        stats.rejected_packets += 1;
                        return Ok(RuleTarget::Reject);
                    }
                    RuleTarget::Goto(ref chain_name) => {
                        // 跳转到自定义链
                        if let Ok(result) = self.process_custom_chain(chain_name, packet) {
                            return Ok(result);
                        }
                    }
                    RuleTarget::Return => {
                        // 返回到调用链
                        return Ok(RuleTarget::Return);
                    }
                    _ => {
                        // 其他目标动作
                        return Ok(rule.target.clone());
                    }
                }
            }
        }

        // 没有规则匹配，应用默认策略
        Ok(chain.policy.clone())
    }

    /// 处理自定义链
    fn process_custom_chain(
        &self,
        chain_name: &str,
        packet: &Packet,
    ) -> Result<RuleTarget, FirewallError> {
        let mut chains = self.custom_chains.lock();
        let chain = chains
            .get_mut(chain_name)
            .ok_or(FirewallError::ChainNotFound)?;

        self.process_chain(chain, packet, None, None)
    }

    /// 检查包是否匹配规则
    fn matches_rule(
        &self,
        packet: &Packet,
        match_cond: &PacketMatch,
        in_interface: Option<&str>,
        out_interface: Option<&str>,
    ) -> bool {
        // 检查接口
        if let Some(ref expected_in) = match_cond.in_interface {
            if in_interface != Some(expected_in.as_str()) {
                return false;
            }
        }

        if let Some(ref expected_out) = match_cond.out_interface {
            if out_interface != Some(expected_out.as_str()) {
                return false;
            }
        }

        // 检查协议
        if let Some(protocol) = match_cond.protocol {
            if !self.matches_protocol(packet, protocol) {
                return false;
            }
        }

        // 检查地址
        if let Some(ref source_addr) = match_cond.source_addr {
            if let Some(addr) = self.get_source_addr(packet) {
                if !source_addr.matches(addr) {
                    return false;
                }
            }
        }

        if let Some(ref dest_addr) = match_cond.dest_addr {
            if let Some(addr) = self.get_dest_addr(packet) {
                if !dest_addr.matches(addr) {
                    return false;
                }
            }
        }

        // 检查端口
        if let Some(ref source_port) = match_cond.source_port {
            if let Some(port) = self.get_source_port(packet) {
                if !source_port.matches(port) {
                    return false;
                }
            }
        }

        if let Some(ref dest_port) = match_cond.dest_port {
            if let Some(port) = self.get_dest_port(packet) {
                if !dest_port.matches(port) {
                    return false;
                }
            }
        }

        // 检查连接状态
        if let Some(state) = match_cond.conntrack_state {
            if !self.matches_conntrack_state(packet, state) {
                return false;
            }
        }

        // 检查 TCP 标志
        if let Some(ref tcp_flags) = match_cond.tcp_flags {
            if let Some(flags) = self.get_tcp_flags(packet) {
                if !tcp_flags.matches(flags) {
                    return false;
                }
            }
        }

        // 检查包长度
        if let Some((min_len, max_len)) = match_cond.packet_length {
            let len = packet.data().len();
            if len < min_len || len > max_len {
                return false;
            }
        }

        true
    }

    /// 获取源地址
    fn get_source_addr(&self, _packet: &Packet) -> Option<Ipv4Addr> {
        // 简化实现 - 实际需要解析 IP 头
        None
    }

    /// 获取目标地址
    fn get_dest_addr(&self, _packet: &Packet) -> Option<Ipv4Addr> {
        // 简化实现 - 实际需要解析 IP 头
        None
    }

    /// 获取源端口
    fn get_source_port(&self, _packet: &Packet) -> Option<u16> {
        // 简化实现 - 实际需要解析 TCP/UDP 头
        None
    }

    /// 获取目标端口
    fn get_dest_port(&self, _packet: &Packet) -> Option<u16> {
        // 简化实现 - 实际需要解析 TCP/UDP 头
        None
    }

    /// 检查协议
    fn matches_protocol(&self, _packet: &Packet, protocol: Protocol) -> bool {
        match protocol {
            Protocol::All => true,
            _ => {
                // 简化实现 - 实际需要检查 IP 头中的协议字段
                false
            }
        }
    }

    /// 检查连接状态
    fn matches_conntrack_state(&self, _packet: &Packet, _state: ConntrackState) -> bool {
        // 检查连接跟踪表
        let _conntrack = self.conntrack_table.lock();
        // 简化实现 - 实际需要查找连接跟踪条目
        true
    }

    /// 获取 TCP 标志
    fn get_tcp_flags(&self, _packet: &Packet) -> Option<u8> {
        // 简化实现 - 实际需要解析 TCP 头
        None
    }

    /// 添加 INPUT 规则
    pub fn add_input_rule(&self, match_cond: PacketMatch, target: RuleTarget) -> u64 {
        self.input_chain.lock().add_rule(match_cond, target)
    }

    /// 添加 OUTPUT 规则
    pub fn add_output_rule(&self, match_cond: PacketMatch, target: RuleTarget) -> u64 {
        self.output_chain.lock().add_rule(match_cond, target)
    }

    /// 添加 FORWARD 规则
    pub fn add_forward_rule(&self, match_cond: PacketMatch, target: RuleTarget) -> u64 {
        self.forward_chain.lock().add_rule(match_cond, target)
    }

    /// 删除规则
    pub fn remove_rule(&self, chain: &str, rule_id: u64) -> Result<(), FirewallError> {
        match chain {
            "INPUT" => self.input_chain.lock().remove_rule(rule_id),
            "OUTPUT" => self.output_chain.lock().remove_rule(rule_id),
            "FORWARD" => self.forward_chain.lock().remove_rule(rule_id),
            _ => {
                let mut chains = self.custom_chains.lock();
                if let Some(chain) = chains.get_mut(chain) {
                    chain.remove_rule(rule_id)
                } else {
                    Err(FirewallError::ChainNotFound)
                }
            }
        }
    }

    /// 创建自定义链
    pub fn create_chain(&self, name: String, policy: RuleTarget) -> Result<(), FirewallError> {
        let mut chains = self.custom_chains.lock();
        if chains.contains_key(&name) {
            return Err(FirewallError::InternalError("Chain already exists".to_string()));
        }

        chains.insert(
            name.clone(),
            FirewallChain::new(name, ChainType::UserDefined, policy),
        );
        Ok(())
    }

    /// 删除自定义链
    pub fn delete_chain(&self, name: &str) -> Result<(), FirewallError> {
        let mut chains = self.custom_chains.lock();
        chains
            .remove(name)
            .ok_or(FirewallError::ChainNotFound)
            .map(|_| ())
    }

    /// 清空链规则
    pub fn flush_chain(&self, name: &str) -> Result<(), FirewallError> {
        match name {
            "INPUT" => {
                self.input_chain.lock().flush_rules();
                Ok(())
            }
            "OUTPUT" => {
                self.output_chain.lock().flush_rules();
                Ok(())
            }
            "FORWARD" => {
                self.forward_chain.lock().flush_rules();
                Ok(())
            }
            _ => {
                let mut chains = self.custom_chains.lock();
                if let Some(chain) = chains.get_mut(name) {
                    chain.flush_rules();
                    Ok(())
                } else {
                    Err(FirewallError::ChainNotFound)
                }
            }
        }
    }

    /// 设置链策略
    pub fn set_chain_policy(&self, name: &str, policy: RuleTarget) -> Result<(), FirewallError> {
        match name {
            "INPUT" => self.input_chain.lock().policy = policy,
            "OUTPUT" => self.output_chain.lock().policy = policy,
            "FORWARD" => self.forward_chain.lock().policy = policy,
            _ => {
                let mut chains = self.custom_chains.lock();
                if let Some(chain) = chains.get_mut(name) {
                    chain.policy = policy;
                } else {
                    return Err(FirewallError::ChainNotFound);
                }
            }
        }
        Ok(())
    }

    /// 添加或更新连接跟踪条目
    pub fn update_conntrack(&self, entry: ConntrackEntry) -> Result<(), FirewallError> {
        let mut table = self.conntrack_table.lock();

        if table.len() >= self.conntrack_config.max_entries {
            // 清理过期条目
            self.cleanup_expired_conntrack(&mut table);
        }

        if table.len() >= self.conntrack_config.max_entries {
            return Err(FirewallError::ConntrackTableFull);
        }

        table.insert(entry.id, entry);

        let mut stats = self.stats.lock();
        stats.conntrack_entries = table.len() as u64;

        Ok(())
    }

    /// 清理过期的连接跟踪条目
    fn cleanup_expired_conntrack(&self, table: &mut BTreeMap<u64, ConntrackEntry>) {
        let current_time = crate::subsystems::time::get_timestamp();
        table.retain(|_, entry| !entry.is_expired(current_time));
    }

    /// 查找连接跟踪条目
    pub fn lookup_conntrack(
        &self,
        source_addr: Ipv4Addr,
        source_port: u16,
        dest_addr: Ipv4Addr,
        dest_port: u16,
        protocol: Protocol,
    ) -> Option<ConntrackEntry> {
        let table = self.conntrack_table.lock();
        table.values().find(|entry| {
            entry.source_addr == source_addr
                && entry.source_port == source_port
                && entry.dest_addr == dest_addr
                && entry.dest_port == dest_port
                && entry.protocol == protocol
        }).cloned()
    }

    /// 添加 NAT 映射
    pub fn add_nat_mapping(&self, entry: NatEntry) -> Result<(), FirewallError> {
        let mut table = self.nat_table.lock();

        if table.len() >= self.nat_config.max_entries {
            // 清理过期条目
            self.cleanup_expired_nat(&mut table);
        }

        if table.len() >= self.nat_config.max_entries {
            return Err(FirewallError::NatTableFull);
        }

        table.insert(entry.id, entry);

        let mut stats = self.stats.lock();
        stats.nat_mappings = table.len() as u64;

        Ok(())
    }

    /// 清理过期的 NAT 映射
    fn cleanup_expired_nat(&self, table: &mut BTreeMap<u64, NatEntry>) {
        let current_time = crate::subsystems::time::get_timestamp();
        table.retain(|_, entry| !entry.is_expired(current_time, self.nat_config.timeout));
    }

    /// 查找 NAT 映射
    pub fn lookup_nat(&self, id: u64) -> Option<NatEntry> {
        let table = self.nat_table.lock();
        table.get(&id).cloned()
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> FirewallStats {
        let stats = self.stats.lock();
        FirewallStats {
            total_packets: stats.total_packets,
            accepted_packets: stats.accepted_packets,
            dropped_packets: stats.dropped_packets,
            rejected_packets: stats.rejected_packets,
            forwarded_packets: stats.forwarded_packets,
            nat_mappings: stats.nat_mappings,
            conntrack_entries: stats.conntrack_entries,
        }
    }

    /// 重置统计信息
    pub fn reset_stats(&self) {
        *self.stats.lock() = FirewallStats::default();
    }

    /// 清理过期条目
    pub fn cleanup(&self) {
        {
            let mut table = self.conntrack_table.lock();
            self.cleanup_expired_conntrack(&mut table);
        }

        {
            let mut table = self.nat_table.lock();
            self.cleanup_expired_nat(&mut table);
        }
    }
}

impl Default for FirewallEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局防火墙引擎实例
pub static GLOBAL_FIREWALL: Mutex<Option<FirewallEngine>> = Mutex::new(None);

/// 初始化全局防火墙
pub fn init_firewall() -> Result<(), FirewallError> {
    let mut global = GLOBAL_FIREWALL.lock();
    if global.is_some() {
        return Err(FirewallError::InternalError("Firewall already initialized".to_string()));
    }

    *global = Some(FirewallEngine::new());

    crate::println!("[Firewall] Firewall engine initialized successfully");
    Ok(())
}

/// 获取全局防火墙引擎
pub fn get_firewall() -> Option<&'static Mutex<FirewallEngine>> {
    // Note: This is a simplified access pattern
    // In practice, you'd need a more sophisticated locking strategy
    None
}

/// 添加防火墙规则（便捷函数）
pub fn add_rule(
    chain: &str,
    match_cond: PacketMatch,
    target: RuleTarget,
) -> Result<u64, FirewallError> {
    let global = GLOBAL_FIREWALL.lock();
    let _firewall = global
        .as_ref()
        .ok_or(FirewallError::InternalError("Firewall not initialized".to_string()))?;

    // 这里需要一个更好的锁定策略来避免死锁
    // 简化实现中，我们先释放全局锁，然后获取特定链的锁

    drop(global);
    let global = GLOBAL_FIREWALL.lock();

    match chain {
        "INPUT" => Ok(global.as_ref().unwrap().input_chain.lock().add_rule(match_cond, target)),
        "OUTPUT" => Ok(global.as_ref().unwrap().output_chain.lock().add_rule(match_cond, target)),
        "FORWARD" => Ok(global.as_ref().unwrap().forward_chain.lock().add_rule(match_cond, target)),
        _ => Err(FirewallError::ChainNotFound),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_match() {
        let addr = Ipv4Addr::new(192, 168, 1, 0);
        let mask = Ipv4Addr::new(255, 255, 255, 0);

        let match_cond = AddressMatch::new(addr, mask);

        assert!(match_cond.matches(Ipv4Addr::new(192, 168, 1, 100)));
        assert!(!match_cond.matches(Ipv4Addr::new(192, 168, 2, 100)));
    }

    #[test]
    fn test_address_match_inverted() {
        let addr = Ipv4Addr::new(192, 168, 1, 0);
        let mask = Ipv4Addr::new(255, 255, 255, 0);

        let match_cond = AddressMatch::new(addr, mask).inverted();

        assert!(!match_cond.matches(Ipv4Addr::new(192, 168, 1, 100)));
        assert!(match_cond.matches(Ipv4Addr::new(192, 168, 2, 100)));
    }

    #[test]
    fn test_port_match_single() {
        let match_cond = PortMatch::single(80);

        assert!(match_cond.matches(80));
        assert!(!match_cond.matches(443));
    }

    #[test]
    fn test_port_match_range() {
        let match_cond = PortMatch::range(1024, 65535);

        assert!(match_cond.matches(8080));
        assert!(match_cond.matches(1024));
        assert!(match_cond.matches(65535));
        assert!(!match_cond.matches(80));
    }

    #[test]
    fn test_tcp_flags() {
        let flags = TcpFlags::new().syn(true).ack(false);

        assert!(flags.matches(0x02)); // SYN only
        assert!(!flags.matches(0x12)); // SYN + ACK
        assert!(flags.matches(0x01)); // FIN only (mask doesn't include FIN)
    }

    #[test]
    fn test_firewall_chain_creation() {
        let chain = FirewallChain::new(
            String::from("TEST"),
            ChainType::UserDefined,
            RuleTarget::Drop,
        );

        assert_eq!(chain.name, "TEST");
        assert_eq!(chain.chain_type, ChainType::UserDefined);
        assert_eq!(chain.policy, RuleTarget::Drop);
        assert_eq!(chain.rule_count(), 0);
    }

    #[test]
    fn test_firewall_chain_add_rule() {
        let mut chain = FirewallChain::new(
            String::from("TEST"),
            ChainType::UserDefined,
            RuleTarget::Drop,
        );

        let match_cond = PacketMatch::default();
        let rule_id = chain.add_rule(match_cond, RuleTarget::Accept);

        assert_eq!(chain.rule_count(), 1);
        assert_eq!(rule_id, 1);
    }

    #[test]
    fn test_firewall_engine_creation() {
        let engine = FirewallEngine::new();

        let stats = engine.get_stats();
        assert_eq!(stats.total_packets, 0);
    }

    #[test]
    fn test_conntrack_entry_expiration() {
        let entry = ConntrackEntry {
            id: 1,
            source_addr: Ipv4Addr::new(192, 168, 1, 1),
            source_port: 12345,
            dest_addr: Ipv4Addr::new(10, 0, 0, 1),
            dest_port: 80,
            protocol: Protocol::Tcp,
            state: ConntrackState::Established,
            created_at: 0,
            last_activity: 0,
            packets_origin: 0,
            packets_reply: 0,
            bytes_origin: 0,
            bytes_reply: 0,
            timeout: 300,
        };

        // 300 seconds later
        assert!(entry.is_expired(300_000_000_001));

        // Still within timeout
        assert!(!entry.is_expired(299_999_999_999));
    }
}
