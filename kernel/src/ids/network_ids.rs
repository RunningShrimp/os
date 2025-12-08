/// Network Intrusion Detection System (NIDS)

extern crate alloc;

/// 网络入侵检测系统模块
/// 负责检测网络流量中的恶意活动和攻击模式

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::format;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use crate::net::Packet as NetworkPacket;
use crate::security::audit::AuditSeverity;
use super::{
    IntrusionDetection, DetectionType, ThreatLevel, DetectionSource, TargetInfo,
    AttackInfo, Evidence, ResponseAction, NetworkIdsConfig
};

/// 网络入侵检测系统
pub struct NetworkIds {
    /// 系统ID
    pub id: u64,
    /// 配置
    config: NetworkIdsConfig,
    /// 检测引擎
    detection_engine: Arc<Mutex<DetectionEngine>>,
    /// 规则管理器
    rule_manager: Arc<Mutex<RuleManager>>,
    /// 流量分析器
    traffic_analyzer: Arc<Mutex<TrafficAnalyzer>>,
    /// 协议分析器
    protocol_analyzers: BTreeMap<String, Box<dyn ProtocolAnalyzer>>,
    /// 状态跟踪器
    state_tracker: Arc<Mutex<StateTracker>>,
    /// 统计信息
    stats: Arc<Mutex<NetworkIdsStats>>,
    /// 下一个规则ID
    next_rule_id: AtomicU64,
}

/// 检测引擎
pub struct DetectionEngine {
    /// 启用的规则
    enabled_rules: Vec<DetectionRule>,
    /// 规则索引
    rule_index: BTreeMap<String, Vec<usize>>,
    /// 规则统计
    rule_stats: BTreeMap<u64, RuleStats>,
}

/// 规则管理器
pub struct RuleManager {
    /// 规则库
    rules: Vec<DetectionRule>,
    /// 规则分类
    rule_categories: BTreeMap<String, Vec<u64>>,
    /// 规则更新历史
    update_history: Vec<RuleUpdate>,
}

/// 流量分析器
pub struct TrafficAnalyzer {
    /// 流量统计
    traffic_stats: BTreeMap<String, TrafficStats>,
    /// 异常检测器
    anomaly_detector: AnomalyDetector,
    /// 基线数据
    baseline: TrafficBaseline,
}

/// 状态跟踪器
pub struct StateTracker {
    /// 连接状态
    connections: BTreeMap<ConnectionKey, ConnectionState>,
    /// 会话状态
    sessions: BTreeMap<SessionKey, SessionState>,
    /// 超时设置
    timeouts: Timeouts,
}

/// 检测规则
#[derive(Debug, Clone)]
pub struct DetectionRule {
    /// 规则ID
    pub id: u64,
    /// 规则名称
    pub name: String,
    /// 规则描述
    pub description: String,
    /// 规则类型
    pub rule_type: RuleType,
    /// 规则分类
    pub category: String,
    /// 匹配条件
    pub conditions: Vec<MatchCondition>,
    /// 动作
    pub actions: Vec<RuleAction>,
    /// 优先级
    pub priority: u8,
    /// 是否启用
    pub enabled: bool,
    /// 创建时间
    pub created_at: u64,
    /// 最后更新时间
    pub updated_at: u64,
}

/// 规则类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleType {
    /// 签名规则
    Signature,
    /// 协议规则
    Protocol,
    /// 异常规则
    Anomaly,
    /// 行为规则
    Behavior,
    /// 流量规则
    Traffic,
}

/// 匹配条件
#[derive(Debug, Clone)]
pub enum MatchCondition {
    /// 协议匹配
    Protocol(ProtocolType),
    /// 端口匹配
    Port(u16),
    /// IP地址匹配
    IpAddress(IpMatcher),
    /// 负载匹配
    Payload(PayloadMatcher),
    /// 标志位匹配
    Flags(Vec<PacketFlag>),
    /// 大小匹配
    Size(SizeMatcher),
    /// 时间匹配
    Time(TimeMatcher),
    /// 组合条件
    And(Vec<MatchCondition>),
    /// 或条件
    Or(Vec<MatchCondition>),
    /// 非条件
    Not(Box<MatchCondition>),
}

/// 协议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProtocolType {
    TCP,
    UDP,
    ICMP,
    HTTP,
    HTTPS,
    FTP,
    SMTP,
    DNS,
    SSH,
    Telnet,
    SNMP,
}

/// IP匹配器
#[derive(Debug, Clone)]
pub struct IpMatcher {
    /// IP地址或网段
    pub address: String,
    /// 掩码
    pub mask: Option<u32>,
    /// 是否反相
    pub negated: bool,
}

/// 负载匹配器
#[derive(Debug, Clone)]
pub struct PayloadMatcher {
    /// 匹配模式
    pub pattern: String,
    /// 匹配类型
    pub match_type: PayloadMatchType,
    /// 偏移量
    pub offset: Option<usize>,
    /// 深度
    pub depth: Option<usize>,
}

/// 负载匹配类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadMatchType {
    /// 精确匹配
    Exact,
    /// 正则表达式
    Regex,
    /// 十六进制
    Hex,
    /// 字符串
    String,
}

/// 包标志位
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketFlag {
    SYN,
    ACK,
    FIN,
    RST,
    URG,
    PSH,
    ECE,
    CWR,
}

/// 大小匹配器
#[derive(Debug, Clone)]
pub struct SizeMatcher {
    /// 操作符
    pub operator: ComparisonOperator,
    /// 值
    pub value: usize,
}

/// 比较操作符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

/// 时间匹配器
#[derive(Debug, Clone)]
pub struct TimeMatcher {
    /// 开始时间
    pub start_time: u64,
    /// 结束时间
    pub end_time: u64,
}

/// 规则动作
#[derive(Debug, Clone)]
pub enum RuleAction {
    /// 告警
    Alert(String),
    /// 记录日志
    Log(String),
    /// 阻止
    Block,
    /// 重置连接
    Reset,
    /// 记录包
    Capture,
    /// 执行脚本
    Execute(String),
}

/// 规则统计
#[derive(Debug, Clone, Default)]
pub struct RuleStats {
    /// 匹配次数
    pub matches: u64,
    /// 告警次数
    pub alerts: u64,
    /// 最后匹配时间
    pub last_match: u64,
    /// 平均处理时间（微秒）
    pub avg_processing_time_us: u64,
}

/// 规则更新
#[derive(Debug, Clone)]
pub struct RuleUpdate {
    /// 更新时间
    pub timestamp: u64,
    /// 更新类型
    pub update_type: UpdateType,
    /// 更新描述
    pub description: String,
}

/// 更新类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateType {
    /// 新增
    Added,
    /// 修改
    Modified,
    /// 删除
    Deleted,
    /// 启用
    Enabled,
    /// 禁用
    Disabled,
}

/// 流量统计
#[derive(Debug, Clone, Default)]
pub struct TrafficStats {
    /// 包数量
    pub packet_count: u64,
    /// 字节数
    pub byte_count: u64,
    /// 连接数
    pub connection_count: u64,
    /// 错误包数
    pub error_count: u64,
    /// 最后更新时间
    pub last_updated: u64,
}

/// 异常检测器
pub struct AnomalyDetector {
    /// 检测模型
    models: Vec<AnomalyModel>,
    /// 阈值设置
    thresholds: AnomalyThresholds,
}

/// 异常模型
#[derive(Debug, Clone)]
pub struct AnomalyModel {
    /// 模型ID
    pub id: u64,
    /// 模型类型
    pub model_type: AnomalyModelType,
    /// 模型参数
    pub parameters: BTreeMap<String, f64>,
    /// 训练数据大小
    pub training_data_size: usize,
}

/// 异常模型类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalyModelType {
    /// 统计模型
    Statistical,
    /// 机器学习模型
    MachineLearning,
    /// 基线模型
    Baseline,
}

/// 异常阈值
#[derive(Debug, Clone)]
pub struct AnomalyThresholds {
    /// 流量异常阈值
    pub traffic_anomaly_threshold: f64,
    /// 连接数异常阈值
    pub connection_anomaly_threshold: f64,
    /// 包大小异常阈值
    pub packet_size_anomaly_threshold: f64,
    /// 端口扫描阈值
    pub port_scan_threshold: u32,
    /// 频率异常阈值
    pub frequency_anomaly_threshold: f64,
}

/// 流量基线
#[derive(Debug, Clone)]
pub struct TrafficBaseline {
    /// 平均流量
    pub avg_traffic: f64,
    /// 峰值流量
    pub peak_traffic: f64,
    /// 正常端口分布
    pub normal_ports: BTreeMap<u16, f64>,
    /// 正常协议分布
    pub normal_protocols: BTreeMap<ProtocolType, f64>,
    /// 更新时间
    pub updated_at: u64,
}

/// 连接键
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConnectionKey {
    /// 源IP
    pub src_ip: u32,
    /// 源端口
    pub src_port: u16,
    /// 目的IP
    pub dst_ip: u32,
    /// 目的端口
    pub dst_port: u16,
    /// 协议
    pub protocol: ProtocolType,
}

/// 连接状态
#[derive(Debug, Clone)]
pub struct ConnectionState {
    /// 状态类型
    pub state: ConnectionStateType,
    /// 创建时间
    pub created_at: u64,
    /// 最后活动时间
    pub last_activity: u64,
    /// 包计数
    pub packet_count: u64,
    /// 字节数
    pub byte_count: u64,
    /// 连接标志
    pub flags: u32,
}

/// 连接状态类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStateType {
    /// 新建
    New,
    /// 已建立
    Established,
    /// 关闭中
    Closing,
    /// 已关闭
    Closed,
    /// 超时
    Timeout,
}

/// 会话键
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SessionKey {
    /// 会话ID
    pub session_id: String,
    /// 协议类型
    pub protocol: ProtocolType,
}

/// 会话状态
#[derive(Debug, Clone)]
pub struct SessionState {
    /// 会话阶段
    pub stage: SessionStage,
    /// 会话数据
    pub data: BTreeMap<String, String>,
    /// 创建时间
    pub created_at: u64,
    /// 最后活动时间
    pub last_activity: u64,
    /// 事务计数
    pub transaction_count: u64,
}

/// 会话阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStage {
    /// 初始阶段
    Initial,
    /// 协商阶段
    Negotiation,
    /// 认证阶段
    Authentication,
    /// 数据传输阶段
    DataTransfer,
    /// 终止阶段
    Termination,
}

/// 超时设置
#[derive(Debug, Clone)]
pub struct Timeouts {
    /// TCP连接超时（秒）
    pub tcp_connection_timeout: u64,
    /// UDP会话超时（秒）
    pub udp_session_timeout: u64,
    /// 空闲连接超时（秒）
    pub idle_connection_timeout: u64,
    /// 碎片重组超时（毫秒）
    pub fragment_reassembly_timeout: u64,
}

/// 协议分析器特征
// Protocol analyzers must be thread-safe so they can be owned by shared
// collections and used from multiple threads. Require Send + Sync here so
// Box<dyn ProtocolAnalyzer> is Send/Sync when stored inside `NetworkIds`.
pub trait ProtocolAnalyzer: Send + Sync {
    /// 分析包
    fn analyze_packet(&mut self, packet: &NetworkPacket) -> Result<Vec<ProtocolEvent>, &'static str>;
    /// 获取协议信息
    fn get_protocol_info(&self) -> ProtocolInfo;
}

/// 协议事件
#[derive(Debug, Clone)]
pub struct ProtocolEvent {
    /// 事件ID
    pub id: u64,
    /// 事件类型
    pub event_type: String,
    /// 事件数据
    pub data: BTreeMap<String, String>,
    /// 时间戳
    pub timestamp: u64,
}

/// 协议信息
#[derive(Debug, Clone)]
pub struct ProtocolInfo {
    /// 协议名称
    pub name: String,
    /// 协议版本
    pub version: String,
    /// 支持的端口
    pub ports: Vec<u16>,
    /// 协议特征
    pub characteristics: Vec<String>,
}

/// 网络入侵检测统计
#[derive(Debug, Default)]
pub struct NetworkIdsStats {
    /// 总处理包数
    pub total_packets_processed: u64,
    /// 总检测次数
    pub total_detections: u64,
    /// 按协议统计
    pub detections_by_protocol: BTreeMap<ProtocolType, u64>,
    /// 按规则类型统计
    pub detections_by_rule_type: BTreeMap<RuleType, u64>,
    /// 平均处理时间（微秒）
    pub avg_processing_time_us: u64,
    /// 丢弃包数
    pub dropped_packets: u64,
    /// 错误包数
    pub error_packets: u64,
    /// 检测引擎统计
    pub engine_stats: DetectionEngineStats,
}

/// 检测引擎统计
#[derive(Debug, Default)]
pub struct DetectionEngineStats {
    /// 规则匹配次数
    pub rule_matches: u64,
    /// 条件评估次数
    pub condition_evaluations: u64,
    /// 最长匹配时间（微秒）
    pub max_match_time_us: u64,
    /// 内存使用量
    pub memory_usage_bytes: usize,
}

impl NetworkIds {
    /// 创建新的网络入侵检测系统
    pub fn new() -> Self {
        let mut protocol_analyzers: BTreeMap<String, Box<dyn ProtocolAnalyzer>> = BTreeMap::new();

        // 注册协议分析器
        protocol_analyzers.insert(String::from("HTTP"), Box::new(HttpAnalyzer::new()));
        protocol_analyzers.insert(String::from("DNS"), Box::new(DnsAnalyzer::new()));
        protocol_analyzers.insert(String::from("SMTP"), Box::new(SmtpAnalyzer::new()));

        Self {
            id: 1,
            config: NetworkIdsConfig::default(),
            detection_engine: Arc::new(Mutex::new(DetectionEngine::new())),
            rule_manager: Arc::new(Mutex::new(RuleManager::new())),
            traffic_analyzer: Arc::new(Mutex::new(TrafficAnalyzer::new())),
            protocol_analyzers,
            state_tracker: Arc::new(Mutex::new(StateTracker::new())),
            stats: Arc::new(Mutex::new(NetworkIdsStats::default())),
            next_rule_id: AtomicU64::new(1),
        }
    }

    /// 初始化网络入侵检测系统
    pub fn init(&mut self, config: &NetworkIdsConfig) -> Result<(), &'static str> {
        self.config = config.clone();

        // 初始化检测引擎
        self.detection_engine.lock().init()?;

        // 初始化规则管理器
        self.rule_manager.lock().load_default_rules()?;

        // 初始化流量分析器
        self.traffic_analyzer.lock().init()?;

        // 初始化状态跟踪器
        self.state_tracker.lock().init()?;

        crate::println!("[NetworkIds] Network intrusion detection system initialized");
        Ok(())
    }

    /// 分析网络包
    pub fn analyze_packet(&mut self, packet: &NetworkPacket) -> Result<Vec<IntrusionDetection>, &'static str> {
        let start_time = crate::time::get_timestamp_nanos();

        // 更新流量统计
        self.update_traffic_stats(packet);

        // 状态跟踪
        self.state_tracker.lock().update_connection_state(packet)?;

        // 协议分析
        let mut protocol_events = Vec::new();
        if let Some(analyzer) = self.protocol_analyzers.get_mut(&packet.protocol) {
            match analyzer.analyze_packet(packet) {
                Ok(events) => protocol_events = events,
                Err(e) => {
                    crate::println!("[NetworkIds] Protocol analysis failed: {}", e);
                }
            }
        }

        // 检测引擎分析
        let detections = self.detection_engine.lock().analyze_packet(packet, &protocol_events)?;

        // 流量异常检测
        let mut anomaly_detections = Vec::new();
        if self.traffic_analyzer.lock().detect_anomalies(packet)? {
            anomaly_detections.push(self.create_anomaly_detection(packet)?);
        }

        // 合并检测结果
        let mut all_detections = detections;
        all_detections.extend(anomaly_detections);

        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.total_packets_processed += 1;
            stats.total_detections += all_detections.len() as u64;

            let elapsed = crate::time::get_timestamp_nanos() - start_time;
            stats.avg_processing_time_us = (stats.avg_processing_time_us + elapsed / 1000) / 2;
        }

        Ok(all_detections)
    }

    /// 更新流量统计
    fn update_traffic_stats(&mut self, packet: &NetworkPacket) {
        let mut stats = self.stats.lock();
        stats.total_packets_processed += 1;
    }

    /// 创建异常检测结果
    fn create_anomaly_detection(&self, packet: &NetworkPacket) -> Result<IntrusionDetection, &'static str> {
        let detection = IntrusionDetection {
            id: 0, // Will be assigned by parent
            detection_type: DetectionType::AnomalyBehavior,
            threat_level: ThreatLevel::Medium,
            confidence: 0.7,
            detected_at: crate::time::get_timestamp_nanos(),
            source: DetectionSource {
                source_type: super::SourceType::NetworkPacket,
                source_id: format!("{}:{}", packet.src_ip, packet.src_port),
                source_address: packet.src_ip.clone(),
                source_port: Some(packet.src_port),
                additional_info: BTreeMap::new(),
            },
            target: TargetInfo {
                target_type: super::TargetType::NetworkService,
                target_id: format!("{}:{}", packet.dst_ip, packet.dst_port),
                target_address: packet.dst_ip.clone(),
                target_port: Some(packet.dst_port),
                affected_components: vec![packet.protocol.clone()],
            },
            attack_info: AttackInfo {
                attack_type: super::AttackType::NetworkAttack,
                attack_stage: super::AttackStage::Exploitation,
                attack_vector: super::AttackVector::Network,
                payload: None,
                technique: String::from("Anomalous network behavior"),
                mitre_id: None,
            },
            evidence: vec![
                Evidence {
                    id: 1,
                    evidence_type: super::EvidenceType::PacketCapture,
                    content: format!("Anomalous packet: {}", packet.protocol),
                    collected_at: crate::time::get_timestamp_nanos(),
                    reliability: 0.8,
                    source: String::from("NetworkIds"),
                }
            ],
            recommended_response: vec![
                ResponseAction::Log,
                ResponseAction::Alert,
            ],
        };

        Ok(detection)
    }

    /// 添加检测规则
    pub fn add_rule(&mut self, rule: DetectionRule) -> Result<u64, &'static str> {
        let rule_id = self.next_rule_id.fetch_add(1, Ordering::SeqCst);
        let mut rule_with_id = rule;
        rule_with_id.id = rule_id;

        self.rule_manager.lock().add_rule(rule_with_id.clone())?;
        self.detection_engine.lock().add_rule(rule_with_id)?;

        Ok(rule_id)
    }

    /// 移除检测规则
    pub fn remove_rule(&mut self, rule_id: u64) -> Result<(), &'static str> {
        self.rule_manager.lock().remove_rule(rule_id)?;
        self.detection_engine.lock().remove_rule(rule_id)?;
        Ok(())
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> NetworkIdsStats {
        let stats = self.stats.lock();
        NetworkIdsStats {
            total_packets_processed: stats.total_packets_processed,
            total_detections: stats.total_detections,
            detections_by_protocol: stats.detections_by_protocol.clone(),
            detections_by_rule_type: stats.detections_by_rule_type.clone(),
            avg_processing_time_us: stats.avg_processing_time_us,
            dropped_packets: stats.dropped_packets,
            error_packets: stats.error_packets,
            engine_stats: DetectionEngineStats {
                rule_matches: stats.engine_stats.rule_matches,
                condition_evaluations: stats.engine_stats.condition_evaluations,
                max_match_time_us: stats.engine_stats.max_match_time_us,
                memory_usage_bytes: stats.engine_stats.memory_usage_bytes,
            },
        }
    }

    /// 重置统计信息
    pub fn reset_stats(&self) {
        *self.stats.lock() = NetworkIdsStats::default();
    }

    /// 停止网络入侵检测系统
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        crate::println!("[NetworkIds] Network intrusion detection system shutdown");
        Ok(())
    }
}

impl DetectionEngine {
    /// 创建新的检测引擎
    pub fn new() -> Self {
        Self {
            enabled_rules: Vec::new(),
            rule_index: BTreeMap::new(),
            rule_stats: BTreeMap::new(),
        }
    }

    /// 初始化检测引擎
    pub fn init(&mut self) -> Result<(), &'static str> {
        // 加载默认检测规则
        self.load_default_rules()?;
        Ok(())
    }

    /// 加载默认规则
    fn load_default_rules(&mut self) -> Result<(), &'static str> {
        // 添加常见攻击检测规则
        let port_scan_rule = DetectionRule {
            id: 1001,
            name: String::from("Port Scan Detection"),
            description: String::from("Detects port scanning activities"),
            rule_type: RuleType::Behavior,
            category: String::from("Scanning"),
            conditions: vec![
                MatchCondition::And(vec![
                    MatchCondition::Protocol(ProtocolType::TCP),
                    MatchCondition::Flags(vec![PacketFlag::SYN]),
                ])
            ],
            actions: vec![
                RuleAction::Alert(String::from("Port scan detected")),
                RuleAction::Log(String::from("Potential reconnaissance activity")),
            ],
            priority: 5,
            enabled: true,
            created_at: crate::time::get_timestamp(),
            updated_at: crate::time::get_timestamp(),
        };

        self.add_rule(port_scan_rule)?;
        Ok(())
    }

    /// 添加规则
    pub fn add_rule(&mut self, rule: DetectionRule) -> Result<(), &'static str> {
        // 添加到规则列表
        self.enabled_rules.push(rule.clone());

        // 更新规则索引
        for condition in &rule.conditions {
            let key = self.get_condition_key(condition);
            self.rule_index.entry(key).or_insert_with(Vec::new).push(self.enabled_rules.len() - 1);
        }

        // 初始化规则统计
        self.rule_stats.insert(rule.id, RuleStats::default());

        Ok(())
    }

    /// 移除规则
    pub fn remove_rule(&mut self, rule_id: u64) -> Result<(), &'static str> {
        let index = self.enabled_rules.iter().position(|r| r.id == rule_id)
            .ok_or("Rule not found")?;

        self.enabled_rules.remove(index);
        self.rule_stats.remove(&rule_id);

        // 重建索引
        self.rebuild_index()?;

        Ok(())
    }

    /// 重建规则索引
    fn rebuild_index(&mut self) -> Result<(), &'static str> {
        self.rule_index.clear();

        for (i, rule) in self.enabled_rules.iter().enumerate() {
            for condition in &rule.conditions {
                let key = self.get_condition_key(condition);
                self.rule_index.entry(key).or_insert_with(Vec::new).push(i);
            }
        }

        Ok(())
    }

    /// 获取条件键
    fn get_condition_key(&self, condition: &MatchCondition) -> String {
        match condition {
            MatchCondition::Protocol(protocol) => format!("protocol:{:?}", protocol),
            MatchCondition::Port(port) => format!("port:{}", port),
            MatchCondition::Flags(flags) => format!("flags:{:?}", flags),
            _ => String::from("other"),
        }
    }

    /// 分析包
    pub fn analyze_packet(&mut self, packet: &NetworkPacket, protocol_events: &[ProtocolEvent]) -> Result<Vec<IntrusionDetection>, &'static str> {
        let mut detections = Vec::new();
        let start_time = crate::time::get_timestamp_nanos();

        for rule in &self.enabled_rules {
            if !rule.enabled {
                continue;
            }

            if self.evaluate_rule(rule, packet, protocol_events)? {
                let detection = self.create_detection_from_rule(rule, packet)?;
                detections.push(detection);

                // 更新规则统计
                if let Some(stats) = self.rule_stats.get_mut(&rule.id) {
                    stats.matches += 1;
                    stats.last_match = crate::time::get_timestamp_nanos();

                    let elapsed = crate::time::get_timestamp_nanos() - start_time;
                    stats.avg_processing_time_us = (stats.avg_processing_time_us + elapsed / 1000) / 2;
                }
            }
        }

        Ok(detections)
    }

    /// 评估规则
    fn evaluate_rule(&self, rule: &DetectionRule, packet: &NetworkPacket, protocol_events: &[ProtocolEvent]) -> Result<bool, &'static str> {
        for condition in &rule.conditions {
            if !self.evaluate_condition(condition, packet, protocol_events)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// 评估条件
    fn evaluate_condition(&self, condition: &MatchCondition, packet: &NetworkPacket, _protocol_events: &[ProtocolEvent]) -> Result<bool, &'static str> {
        match condition {
            MatchCondition::Protocol(protocol) => Ok(packet.protocol == format!("{:?}", protocol)),
            MatchCondition::Port(port) => Ok(packet.dst_port == *port || packet.src_port == *port),
            MatchCondition::Flags(flags) => {
                let packet_flags = self.extract_packet_flags(packet);
                Ok(flags.iter().all(|&flag| packet_flags.contains(&flag)))
            }
            MatchCondition::And(conditions) => {
                for cond in conditions {
                    if !self.evaluate_condition(cond, packet, _protocol_events)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// 提取包标志位
    fn extract_packet_flags(&self, packet: &NetworkPacket) -> Vec<PacketFlag> {
        let mut flags = Vec::new();

        // 简化的标志位提取逻辑
        if packet.tcp_flags.contains("SYN") {
            flags.push(PacketFlag::SYN);
        }
        if packet.tcp_flags.contains("ACK") {
            flags.push(PacketFlag::ACK);
        }
        if packet.tcp_flags.contains("FIN") {
            flags.push(PacketFlag::FIN);
        }
        if packet.tcp_flags.contains("RST") {
            flags.push(PacketFlag::RST);
        }

        flags
    }

    /// 从规则创建检测结果
    fn create_detection_from_rule(&self, rule: &DetectionRule, packet: &NetworkPacket) -> Result<IntrusionDetection, &'static str> {
        let detection = IntrusionDetection {
            id: rule.id,
            detection_type: DetectionType::NetworkIntrusion,
            threat_level: self.determine_threat_level(rule.priority),
            confidence: 0.8,
            detected_at: crate::time::get_timestamp_nanos(),
            source: DetectionSource {
                source_type: super::SourceType::NetworkPacket,
                source_id: format!("{}:{}", packet.src_ip, packet.src_port),
                source_address: packet.src_ip.clone(),
                source_port: Some(packet.src_port),
                additional_info: BTreeMap::new(),
            },
            target: TargetInfo {
                target_type: super::TargetType::NetworkService,
                target_id: format!("{}:{}", packet.dst_ip, packet.dst_port),
                target_address: packet.dst_ip.clone(),
                target_port: Some(packet.dst_port),
                affected_components: vec![packet.protocol.clone()],
            },
            attack_info: AttackInfo {
                attack_type: super::AttackType::NetworkAttack,
                attack_stage: super::AttackStage::Exploitation,
                attack_vector: super::AttackVector::Network,
                payload: Some(packet.payload.clone()),
                technique: rule.name.clone(),
                mitre_id: None,
            },
            evidence: vec![
                Evidence {
                    id: 1,
                    evidence_type: super::EvidenceType::PacketCapture,
                    content: format!("Rule '{}' triggered by packet", rule.name),
                    collected_at: crate::time::get_timestamp_nanos(),
                    reliability: 0.9,
                    source: String::from("NetworkIds"),
                }
            ],
            recommended_response: vec![
                ResponseAction::Alert,
                ResponseAction::Log,
            ],
        };

        Ok(detection)
    }

    /// 确定威胁级别
    fn determine_threat_level(&self, priority: u8) -> ThreatLevel {
        match priority {
            1..=8 => ThreatLevel::Critical,
            6..=7 => ThreatLevel::High,
            4..=5 => ThreatLevel::Medium,
            _ => ThreatLevel::Low,
        }
    }
}

impl RuleManager {
    /// 创建新的规则管理器
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            rule_categories: BTreeMap::new(),
            update_history: Vec::new(),
        }
    }

    /// 加载默认规则
    pub fn load_default_rules(&mut self) -> Result<(), &'static str> {
        // 这里可以加载预定义的规则集
        Ok(())
    }

    /// 添加规则
    pub fn add_rule(&mut self, rule: DetectionRule) -> Result<(), &'static str> {
        // 添加到分类
        self.rule_categories.entry(rule.category.clone()).or_insert_with(Vec::new).push(rule.id);

        // 添加到规则列表
        self.rules.push(rule);

        // 记录更新历史
        self.update_history.push(RuleUpdate {
            timestamp: crate::time::get_timestamp(),
            update_type: UpdateType::Added,
            description: format!("Rule '{}' added", self.rules.last().unwrap().name),
        });

        Ok(())
    }

    /// 移除规则
    pub fn remove_rule(&mut self, rule_id: u64) -> Result<(), &'static str> {
        let index = self.rules.iter().position(|r| r.id == rule_id)
            .ok_or("Rule not found")?;

        let rule = &self.rules[index];
        let category = rule.category.clone();
        let rule_name = rule.name.clone();

        self.rules.remove(index);

        // 从分类中移除
        if let Some(rules) = self.rule_categories.get_mut(&category) {
            rules.retain(|&id| id != rule_id);
            if rules.is_empty() {
                self.rule_categories.remove(&category);
            }
        }

        // 记录更新历史
        self.update_history.push(RuleUpdate {
            timestamp: crate::time::get_timestamp(),
            update_type: UpdateType::Deleted,
            description: format!("Rule '{}' removed", rule_name),
        });

        Ok(())
    }

    /// 获取规则
    pub fn get_rule(&self, rule_id: u64) -> Option<&DetectionRule> {
        self.rules.iter().find(|r| r.id == rule_id)
    }

    /// 获取所有规则
    pub fn get_all_rules(&self) -> &[DetectionRule] {
        &self.rules
    }

    /// 按分类获取规则
    pub fn get_rules_by_category(&self, category: &str) -> Vec<&DetectionRule> {
        self.rules.iter()
            .filter(|r| r.category == category)
            .collect()
    }
}

impl TrafficAnalyzer {
    /// 创建新的流量分析器
    pub fn new() -> Self {
        Self {
            traffic_stats: BTreeMap::new(),
            anomaly_detector: AnomalyDetector::new(),
            baseline: TrafficBaseline {
                avg_traffic: 0.0,
                peak_traffic: 0.0,
                normal_ports: BTreeMap::new(),
                normal_protocols: BTreeMap::new(),
                updated_at: 0,
            },
        }
    }

    /// 初始化流量分析器
    pub fn init(&mut self) -> Result<(), &'static str> {
        // 初始化基线数据
        self.baseline.updated_at = crate::time::get_timestamp();
        Ok(())
    }

    /// 检测异常
    pub fn detect_anomalies(&mut self, _packet: &NetworkPacket) -> Result<bool, &'static str> {
        // 简化的异常检测逻辑
        let current_time = crate::time::get_timestamp();

        // 如果距离上次基线更新超过1小时，更新基线
        if current_time - self.baseline.updated_at > 3600 {
            self.update_baseline();
        }

        // 这里可以实现复杂的异常检测逻辑
        Ok(false)
    }

    /// 更新基线
    fn update_baseline(&mut self) {
        self.baseline.updated_at = crate::time::get_timestamp();
    }
}

impl AnomalyDetector {
    /// 创建新的异常检测器
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            thresholds: AnomalyThresholds {
                traffic_anomaly_threshold: 2.0,
                connection_anomaly_threshold: 3.0,
                packet_size_anomaly_threshold: 5.0,
                port_scan_threshold: 100,
                frequency_anomaly_threshold: 10.0,
            },
        }
    }
}

impl StateTracker {
    /// 创建新的状态跟踪器
    pub fn new() -> Self {
        Self {
            connections: BTreeMap::new(),
            sessions: BTreeMap::new(),
            timeouts: Timeouts {
                tcp_connection_timeout: 300,      // 5 minutes
                udp_session_timeout: 60,         // 1 minute
                idle_connection_timeout: 600,    // 10 minutes
                fragment_reassembly_timeout: 60,  // 60 milliseconds
            },
        }
    }

    /// 初始化状态跟踪器
    pub fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    /// 更新连接状态
    pub fn update_connection_state(&mut self, packet: &NetworkPacket) -> Result<(), &'static str> {
        let key = ConnectionKey {
            src_ip: self.parse_ip(&packet.src_ip),
            src_port: packet.src_port,
            dst_ip: self.parse_ip(&packet.dst_ip),
            dst_port: packet.dst_port,
            protocol: self.parse_protocol(&packet.protocol),
        };

        let now = crate::time::get_timestamp();

        let state = self.connections.entry(key).or_insert_with(|| ConnectionState {
            state: ConnectionStateType::New,
            created_at: now,
            last_activity: now,
            packet_count: 0,
            byte_count: 0,
            flags: 0,
        });

        state.last_activity = now;
        state.packet_count += 1;
        state.byte_count += packet.size as u64;

        Ok(())
    }

    /// 解析IP地址
    fn parse_ip(&self, ip_str: &str) -> u32 {
        // 简化的IP解析，实际应该使用proper IP解析
        0
    }

    /// 解析协议
    fn parse_protocol(&self, protocol_str: &str) -> ProtocolType {
        match protocol_str {
            "TCP" => ProtocolType::TCP,
            "UDP" => ProtocolType::UDP,
            "ICMP" => ProtocolType::ICMP,
            _ => ProtocolType::TCP,
        }
    }

    /// 清理过期连接
    pub fn cleanup_expired_connections(&mut self) {
        let now = crate::time::get_timestamp();
        let timeout = self.timeouts.tcp_connection_timeout;

        self.connections.retain(|_, state| {
            now - state.last_activity < timeout
        });
    }
}

// HTTP协议分析器
pub struct HttpAnalyzer {
    info: ProtocolInfo,
}

impl HttpAnalyzer {
    pub fn new() -> Self {
        Self {
            info: ProtocolInfo {
                name: String::from("HTTP"),
                version: String::from("1.1"),
                ports: vec![80, 8080, 8000, 3000],
                characteristics: vec![String::from("Stateless"), String::from("Text-based")],
            },
        }
    }
}

impl ProtocolAnalyzer for HttpAnalyzer {
    fn analyze_packet(&mut self, _packet: &NetworkPacket) -> Result<Vec<ProtocolEvent>, &'static str> {
        // 简化的HTTP包分析
        Ok(vec![])
    }

    fn get_protocol_info(&self) -> ProtocolInfo {
        self.info.clone()
    }
}

// DNS协议分析器
pub struct DnsAnalyzer {
    info: ProtocolInfo,
}

impl DnsAnalyzer {
    pub fn new() -> Self {
        Self {
            info: ProtocolInfo {
                name: String::from("DNS"),
                version: String::from("1.0"),
                ports: vec![53],
                characteristics: vec![String::from("UDP-based"), String::from("Query-Response")],
            },
        }
    }
}

impl ProtocolAnalyzer for DnsAnalyzer {
    fn analyze_packet(&mut self, _packet: &NetworkPacket) -> Result<Vec<ProtocolEvent>, &'static str> {
        // 简化的DNS包分析
        Ok(vec![])
    }

    fn get_protocol_info(&self) -> ProtocolInfo {
        self.info.clone()
    }
}

// SMTP协议分析器
pub struct SmtpAnalyzer {
    info: ProtocolInfo,
}

impl SmtpAnalyzer {
    pub fn new() -> Self {
        Self {
            info: ProtocolInfo {
                name: String::from("SMTP"),
                version: String::from("1.0"),
                ports: vec![25, 587],
                characteristics: vec![String::from("Text-based"), String::from("Email")],
            },
        }
    }
}

impl ProtocolAnalyzer for SmtpAnalyzer {
    fn analyze_packet(&mut self, _packet: &NetworkPacket) -> Result<Vec<ProtocolEvent>, &'static str> {
        // 简化的SMTP包分析
        Ok(vec![])
    }

    fn get_protocol_info(&self) -> ProtocolInfo {
        self.info.clone()
    }
}

// 为NetworkPacket添加必要的字段（假设的字段）
impl NetworkPacket {
    // 这里假设NetworkPacket已经有必要的字段
    // 如果没有，需要添加相应的字段定义
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_ids_creation() {
        let nids = NetworkIds::new();
        assert_eq!(nids.id, 1);
        assert!(nids.protocol_analyzers.contains_key("HTTP"));
        assert!(nids.protocol_analyzers.contains_key("DNS"));
    }

    #[test]
    fn test_detection_rule_creation() {
        let rule = DetectionRule {
            id: 1,
            name: String::from("Test Rule"),
            description: String::from("Test detection rule"),
            rule_type: RuleType::Signature,
            category: String::from("Test"),
            conditions: vec![],
            actions: vec![],
            priority: 5,
            enabled: true,
            created_at: 0,
            updated_at: 0,
        };

        assert_eq!(rule.id, 1);
        assert_eq!(rule.name, "Test Rule");
        assert_eq!(rule.rule_type, RuleType::Signature);
        assert!(rule.enabled);
    }

    #[test]
    fn test_match_condition() {
        let condition = MatchCondition::Port(80);
        match condition {
            MatchCondition::Port(port) => assert_eq!(port, 80),
            _ => panic!("Expected Port condition"),
        }
    }

    #[test]
    fn test_protocol_analyzer() {
        let analyzer = HttpAnalyzer::new();
        let info = analyzer.get_protocol_info();
        assert_eq!(info.name, "HTTP");
        assert!(info.ports.contains(&80));
        assert!(info.ports.contains(&8080));
    }

    #[test]
    fn test_connection_state() {
        let key = ConnectionKey {
            src_ip: 0,
            src_port: 1234,
            dst_ip: 0,
            dst_port: 80,
            protocol: ProtocolType::TCP,
        };

        let state = ConnectionState {
            state: ConnectionStateType::New,
            created_at: 0,
            last_activity: 0,
            packet_count: 0,
            byte_count: 0,
            flags: 0,
        };

        assert_eq!(state.state, ConnectionStateType::New);
        assert_eq!(state.packet_count, 0);
    }

    #[test]
    fn test_traffic_analyzer() {
        let analyzer = TrafficAnalyzer::new();
        assert!(analyzer.traffic_stats.is_empty());
    }
}