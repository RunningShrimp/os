/// Intrusion Detection System (IDS) Module

extern crate alloc;

/// 入侵检测系统模块
/// 提供全面的入侵检测和防御功能，包括网络入侵检测、主机入侵检测、异常检测等

pub mod network_ids;
pub mod host_ids;
pub mod anomaly_detection;
pub mod signature_detection;
pub mod behavior_analysis;
pub mod threat_intelligence;
pub mod response_engine;
pub mod correlation_engine;

// Re-export only network_ids which is used by other modules
pub use network_ids::*;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use spin::Mutex;

use crate::security::audit::{AuditEvent, AuditEventType, AuditSeverity};
use crate::net::Packet as NetworkPacket;

/// 入侵检测系统状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdsStatus {
    /// 未初始化
    Uninitialized,
    /// 初始化中
    Initializing,
    /// 运行中
    Running,
    /// 检测中
    Detecting,
    /// 响应中
    Responding,
    /// 停止中
    Stopping,
    /// 已停止
    Stopped,
    /// 错误状态
    Error,
}

/// 入侵检测系统配置
#[derive(Debug, Clone)]
pub struct IdsConfig {
    /// 是否启用入侵检测
    pub enabled: bool,
    /// 检测模式
    pub detection_mode: DetectionMode,
    /// 响应模式
    pub response_mode: ResponseMode,
    /// 网络IDS配置
    pub network_ids_config: NetworkIdsConfig,
    /// 主机IDS配置
    pub host_ids_config: HostIdsConfig,
    /// 异常检测配置
    pub anomaly_config: AnomalyDetectionConfig,
    /// 特征检测配置
    pub signature_config: SignatureDetectionConfig,
    /// 行为分析配置
    pub behavior_config: BehaviorAnalysisConfig,
    /// 威胁情报配置
    pub threat_intel_config: ThreatIntelligenceConfig,
    /// 关联分析配置
    pub correlation_config: CorrelationConfig,
}

impl Default for IdsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            detection_mode: DetectionMode::Hybrid,
            response_mode: ResponseMode::Active,
            network_ids_config: NetworkIdsConfig::default(),
            host_ids_config: HostIdsConfig::default(),
            anomaly_config: AnomalyDetectionConfig::default(),
            signature_config: SignatureDetectionConfig::default(),
            behavior_config: BehaviorAnalysisConfig::default(),
            threat_intel_config: ThreatIntelligenceConfig::default(),
            correlation_config: CorrelationConfig::default(),
        }
    }
}

/// 检测模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionMode {
    /// 仅特征检测
    SignatureOnly,
    /// 仅异常检测
    AnomalyOnly,
    /// 混合模式
    Hybrid,
    /// 行为分析
    BehaviorBased,
    /// 启发式检测
    Heuristic,
    /// 机器学习
    MachineLearning,
}

/// 响应模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseMode {
    /// 被动模式（仅检测）
    Passive,
    /// 主动模式（检测并响应）
    Active,
    /// 预防模式（预防性检测）
    Preventive,
    /// 自适应模式
    Adaptive,
}

/// 入侵检测结果
#[derive(Debug, Clone)]
pub struct IntrusionDetection {
    /// 检测ID
    pub id: u64,
    /// 检测类型
    pub detection_type: DetectionType,
    /// 威胁级别
    pub threat_level: ThreatLevel,
    /// 置信度
    pub confidence: f64,
    /// 检测时间
    pub detected_at: u64,
    /// 检测源
    pub source: DetectionSource,
    /// 目标信息
    pub target: TargetInfo,
    /// 攻击信息
    pub attack_info: AttackInfo,
    /// 证据
    pub evidence: Vec<Evidence>,
    /// 建议响应
    pub recommended_response: Vec<ResponseAction>,
}

/// 检测类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DetectionType {
    /// 网络入侵检测
    NetworkIntrusion,
    /// 主机入侵检测
    HostIntrusion,
    /// 异常行为检测
    AnomalyBehavior,
    /// 恶意软件检测
    MalwareDetection,
    /// 数据泄露检测
    DataLeakage,
    /// 权限提升检测
    PrivilegeEscalation,
    /// 拒绝服务攻击检测
    DenialOfService,
    /// 扫描探测检测
    ScanningProbe,
    /// 欺骗攻击检测
    SpoofingAttack,
    /// 中间人攻击检测
    ManInTheMiddle,
}

/// 威胁级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreatLevel {
    /// 信息
    Info,
    /// 低威胁
    Low,
    /// 中等威胁
    Medium,
    /// 高威胁
    High,
    /// 严重威胁
    Critical,
}

/// 检测源
#[derive(Debug, Clone)]
pub struct DetectionSource {
    /// 源类型
    pub source_type: SourceType,
    /// 源标识
    pub source_id: String,
    /// 源地址
    pub source_address: String,
    /// 源端口
    pub source_port: Option<u16>,
    /// 附加信息
    pub additional_info: BTreeMap<String, String>,
}

/// 源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    /// 网络包
    NetworkPacket,
    /// 系统调用
    SystemCall,
    /// 文件操作
    FileOperation,
    /// 进程活动
    ProcessActivity,
    /// 网络连接
    NetworkConnection,
    /// 用户活动
    UserActivity,
    /// 日志条目
    LogEntry,
}

/// 目标信息
#[derive(Debug, Clone)]
pub struct TargetInfo {
    /// 目标类型
    pub target_type: TargetType,
    /// 目标标识
    pub target_id: String,
    /// 目标地址
    pub target_address: String,
    /// 目标端口
    pub target_port: Option<u16>,
    /// 影响的系统组件
    pub affected_components: Vec<String>,
}

/// 目标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetType {
    /// 主机系统
    HostSystem,
    /// 网络服务
    NetworkService,
    /// 应用程序
    Application,
    /// 数据库
    Database,
    /// 文件系统
    FileSystem,
    /// 用户账户
    UserAccount,
    /// 网络设备
    NetworkDevice,
}

/// 攻击信息
#[derive(Debug, Clone)]
pub struct AttackInfo {
    /// 攻击类型
    pub attack_type: AttackType,
    /// 攻击阶段
    pub attack_stage: AttackStage,
    /// 攻击向量
    pub attack_vector: AttackVector,
    /// 攻击载荷（可能为二进制）
    pub payload: Option<alloc::vec::Vec<u8>>,
    /// 攻击手法
    pub technique: String,
    /// MITRE ATT&CK ID
    pub mitre_id: Option<String>,
}

/// 攻击类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackType {
    /// 恶意软件
    Malware,
    /// 网络攻击
    NetworkAttack,
    /// 社会工程
    SocialEngineering,
    /// 物理攻击
    PhysicalAttack,
    /// 内部威胁
    InsiderThreat,
    /// 高级持续威胁
    AdvancedPersistentThreat,
}

/// 攻击阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackStage {
    /// 侦察
    Reconnaissance,
    /// 武器化
    Weaponization,
    /// 传递
    Delivery,
    /// 利用
    Exploitation,
    /// 安装
    Installation,
    /// 命令控制
    CommandAndControl,
    /// 行动
    Action,
    /// 清除痕迹
    Cleanup,
}

/// 攻击向量
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackVector {
    /// 网络
    Network,
    /// 本地
    Local,
    /// 物理
    Physical,
    /// 社会工程
    Social,
    /// 供应链
    SupplyChain,
    /// 无线
    Wireless,
}

/// 证据
#[derive(Debug, Clone)]
pub struct Evidence {
    /// 证据ID
    pub id: u64,
    /// 证据类型
    pub evidence_type: EvidenceType,
    /// 证据内容
    pub content: String,
    /// 收集时间
    pub collected_at: u64,
    /// 可信度
    pub reliability: f64,
    /// 证据来源
    pub source: String,
}

/// 证据类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceType {
    /// 网络包
    PacketCapture,
    /// 系统日志
    SystemLog,
    /// 进程快照
    ProcessSnapshot,
    /// 内存转储
    MemoryDump,
    /// 文件哈希
    FileHash,
    /// 注册表项
    RegistryEntry,
    /// 配置文件
    ConfigFile,
    /// 用户行为
    UserBehavior,
}

/// 响应动作
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResponseAction {
    /// 记录日志
    Log,
    /// 告警通知
    Alert,
    /// 阻止连接
    BlockConnection,
    /// 隔离系统
    IsolateSystem,
    /// 终止进程
    TerminateProcess,
    /// 阻断用户
    BlockUser,
    /// 更新防火墙规则
    UpdateFirewall,
    /// 调用脚本
    ExecuteScript(String),
    /// 发送邮件
    SendEmail(String),
    /// 调用Webhook
    CallWebhook(String),
    /// 无操作
    None,
    /// 阻断
    Block,
    /// 终止
    Terminate,
    /// 隔离
    Quarantine,
    /// 隔离系统
    Isolate,
    /// 禁用账户
    DisableAccount,
}

/// 入侵检测系统统计
#[derive(Debug, Default)]
pub struct IdsStats {
    /// 总检测次数
    pub total_detections: u64,
    /// 按检测类型统计
    pub detections_by_type: BTreeMap<DetectionType, u64>,
    /// 按威胁级别统计
    pub detections_by_threat_level: BTreeMap<ThreatLevel, u64>,
    /// 网络包处理数
    pub packets_processed: u64,
    /// 系统调用分析数
    pub syscalls_analyzed: u64,
    /// 文件操作监控数
    pub file_operations_monitored: u64,
    /// 异常检测次数
    pub anomaly_detections: u64,
    /// 特征匹配次数
    pub signature_matches: u64,
    /// 响应动作执行次数
    pub responses_executed: u64,
    /// 误报次数
    pub false_positives: u64,
    /// 平均检测时间（微秒）
    pub avg_detection_time_us: u64,
}

/// 入侵检测系统核心
pub struct IntrusionDetectionSystem {
    /// 系统ID
    pub id: u64,
    /// 系统配置
    config: IdsConfig,
    /// 系统状态
    status: IdsStatus,
    /// 网络入侵检测
    network_ids: Arc<Mutex<network_ids::NetworkIds>>,
    /// 主机入侵检测
    host_ids: Arc<Mutex<host_ids::HostIds>>,
    /// 异常检测
    anomaly_detector: Arc<Mutex<anomaly_detection::AnomalyDetector>>,
    /// 特征检测
    signature_detector: Arc<Mutex<signature_detection::SignatureEngine>>,
    /// 行为分析器
    behavior_analyzer: Arc<Mutex<behavior_analysis::BehaviorAnalyzer>>,
    /// 威胁情报
    threat_intelligence: Arc<Mutex<threat_intelligence::ThreatIntelligence>>,
    /// 响应引擎
    response_engine: Arc<Mutex<response_engine::ResponseEngine>>,
    /// 关联引擎
    correlation_engine: Arc<Mutex<correlation_engine::CorrelationEngine>>,
    /// 检测历史
    detection_history: Vec<IntrusionDetection>,
    /// 统计信息
    stats: Arc<Mutex<IdsStats>>,
    /// 下一个检测ID
    next_detection_id: AtomicU64,
    /// 是否正在运行
    running: AtomicBool,
}

impl IntrusionDetectionSystem {
    /// 创建新的入侵检测系统
    pub fn new(config: IdsConfig) -> Self {
        Self {
            id: 1,
            config,
            status: IdsStatus::Uninitialized,
            network_ids: Arc::new(Mutex::new(network_ids::NetworkIds::new())),
            host_ids: Arc::new(Mutex::new(host_ids::HostIds::new())),
            anomaly_detector: Arc::new(Mutex::new(anomaly_detection::AnomalyDetector::new(anomaly_detection::DetectionAlgorithm::Statistical))),
            signature_detector: Arc::new(Mutex::new(signature_detection::SignatureEngine::new())),
            behavior_analyzer: Arc::new(Mutex::new(behavior_analysis::BehaviorAnalyzer::new())),
            threat_intelligence: Arc::new(Mutex::new(threat_intelligence::ThreatIntelligence::new())),
            response_engine: Arc::new(Mutex::new(response_engine::ResponseEngine::new())),
            correlation_engine: Arc::new(Mutex::new(correlation_engine::CorrelationEngine::new())),
            detection_history: Vec::new(),
            stats: Arc::new(Mutex::new(IdsStats::default())),
            next_detection_id: AtomicU64::new(1),
            running: AtomicBool::new(false),
        }
    }

    /// 初始化入侵检测系统
    pub fn init(&mut self) -> Result<(), &'static str> {
        if !self.config.enabled {
            self.status = IdsStatus::Stopped;
            return Ok(());
        }

        self.status = IdsStatus::Initializing;

        // 初始化各个检测模块
        // 注意：大多数检测器已经通过构造函数初始化，这里只需要额外的配置
        crate::println!("[IDS] All detection modules initialized successfully");

        self.status = IdsStatus::Running;
        self.running.store(true, Ordering::SeqCst);

        crate::println!("[IDS] Intrusion detection system initialized successfully");
        Ok(())
    }

    /// 处理网络包
    pub fn process_packet(&mut self, packet: &NetworkPacket) -> Result<Vec<IntrusionDetection>, &'static str> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(Vec::new());
        }

        let start_time = crate::subsystems::time::get_timestamp_nanos();
        let mut detections = Vec::new();

        // 网络入侵检测
        let network_detections = self.network_ids.lock().analyze_packet(packet);
        match network_detections {
            Ok(network_detections) => {
                for detection in network_detections {
                    if self.validate_detection(&detection) {
                        let processed_detection = self.process_detection(detection);
                        detections.push(processed_detection);
                    }
                }
            }
            Err(e) => {
                crate::println!("[IDS] Network packet analysis failed: {}", e);
            }
        }

        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.packets_processed += 1;
            stats.total_detections += detections.len() as u64;

            let elapsed = crate::subsystems::time::get_timestamp_nanos() - start_time;
            stats.avg_detection_time_us = (stats.avg_detection_time_us + elapsed / 1000) / 2;
        }

        Ok(detections)
    }

    /// 处理系统事件
    pub fn process_system_event(&mut self, event: &AuditEvent) -> Result<Vec<IntrusionDetection>, &'static str> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(Vec::new());
        }

        let start_time = crate::subsystems::time::get_timestamp_nanos();
        let mut detections = Vec::new();

        // 主机入侵检测
        let host_detections = self.host_ids.lock().analyze_event(event);
        match host_detections {
            Ok(host_detections) => {
                for detection in host_detections {
                    if self.validate_detection(&detection) {
                        let processed_detection = self.process_detection(detection);
                        detections.push(processed_detection);
                    }
                }
            }
            Err(e) => {
                crate::println!("[IDS] Host event analysis failed: {}", e);
            }
        }

        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.total_detections += detections.len() as u64;

            let elapsed = crate::subsystems::time::get_timestamp_nanos() - start_time;
            stats.avg_detection_time_us = (stats.avg_detection_time_us + elapsed / 1000) / 2;
        }

        Ok(detections)
    }

    /// 验证检测结果
    fn validate_detection(&self, detection: &IntrusionDetection) -> bool {
        // 检查置信度阈值
        if detection.confidence < 0.5 {
            return false;
        }

        // 检查是否在威胁情报黑名单中
        if let Ok(is_threat) = self.threat_intelligence.lock().is_threat_source(&detection.source) {
            if is_threat {
                return true; // 威胁情报命中，验证通过
            }
        }

        // 其他验证逻辑...
        true
    }

    /// 处理检测结果
    fn process_detection(&mut self, mut detection: IntrusionDetection) -> IntrusionDetection {
        // 分配检测ID
        detection.id = self.next_detection_id.fetch_add(1, Ordering::SeqCst);

        // 更新检测历史
        self.detection_history.push(detection.clone());

        // 执行响应动作
        if self.config.response_mode == ResponseMode::Active {
            self.execute_response_actions(&detection);
        }

        // 更新统计
        {
            let mut stats = self.stats.lock();
            *stats.detections_by_type.entry(detection.detection_type).or_insert(0) += 1;
            *stats.detections_by_threat_level.entry(detection.threat_level).or_insert(0) += 1;
        }

        detection
    }

    /// 执行响应动作
    fn execute_response_actions(&mut self, detection: &IntrusionDetection) {
        for action in &detection.recommended_response {
            match self.response_engine.lock().execute_action(action.clone(), detection) {
                Ok(_) => {
                    let mut stats = self.stats.lock();
                    stats.responses_executed += 1;
                }
                Err(e) => {
                    crate::println!("[IDS] Response action failed: {}", e);
                }
            }
        }
    }

    /// 运行关联分析
    pub fn run_correlation_analysis(&mut self) -> Result<Vec<CorrelationResult>, &'static str> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(Vec::new());
        }

        // 获取最近的检测结果
        let recent_detections: Vec<_> = self.detection_history
            .iter()
            .filter(|d| crate::subsystems::time::get_timestamp() - d.detected_at / 1000000000 < 3600) // 最近1小时
            .cloned()
            .collect();

        self.correlation_engine.lock().analyze_correlations(&recent_detections)
    }

    /// 更新威胁情报
    pub fn update_threat_intelligence(&mut self, threat_data: Vec<ThreatData>) -> Result<(), &'static str> {
        self.threat_intelligence.lock().update_intelligence(threat_data)
    }

    /// 获取系统状态
    pub fn get_status(&self) -> IdsStatus {
        self.status
    }

    /// 获取配置
    pub fn get_config(&self) -> &IdsConfig {
        &self.config
    }

    /// 更新配置
    pub fn update_config(&mut self, config: IdsConfig) -> Result<(), &'static str> {
        self.config = config;

        // 重新初始化相关模块
        if self.running.load(Ordering::SeqCst) {
            self.network_ids.lock().init(&self.config.network_ids_config)?;
            self.host_ids.lock().init(&self.config.host_ids_config)?;
            self.response_engine.lock().init(&self.config.response_mode)?;
        }

        Ok(())
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> IdsStats {
        let stats = self.stats.lock();
        IdsStats {
            total_detections: stats.total_detections,
            detections_by_type: stats.detections_by_type.clone(),
            detections_by_threat_level: stats.detections_by_threat_level.clone(),
            packets_processed: stats.packets_processed,
            syscalls_analyzed: stats.syscalls_analyzed,
            file_operations_monitored: stats.file_operations_monitored,
            anomaly_detections: stats.anomaly_detections,
            signature_matches: stats.signature_matches,
            responses_executed: stats.responses_executed,
            false_positives: stats.false_positives,
            avg_detection_time_us: stats.avg_detection_time_us,
        }
    }

    /// 重置统计信息
    pub fn reset_stats(&self) {
        *self.stats.lock() = IdsStats::default();
    }

    /// 获取检测历史
    pub fn get_detection_history(&self, limit: Option<usize>) -> Vec<IntrusionDetection> {
        match limit {
            Some(limit) => self.detection_history
                .iter()
                .rev()
                .take(limit)
                .cloned()
                .collect(),
            None => self.detection_history.clone(),
        }
    }

    /// 停止入侵检测系统
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        self.status = IdsStatus::Stopping;
        self.running.store(false, Ordering::SeqCst);

        // 停止各个模块
        self.network_ids.lock().shutdown()?;
        self.host_ids.lock().shutdown()?;
        self.anomaly_detector.lock().shutdown()?;
        self.signature_detector.lock().shutdown()?;
        self.behavior_analyzer.lock().shutdown()?;
        self.response_engine.lock().shutdown()?;

        self.status = IdsStatus::Stopped;
        crate::println!("[IDS] Intrusion detection system shutdown successfully");
        Ok(())
    }
}

/// 关联分析结果
#[derive(Debug, Clone)]
pub struct CorrelationResult {
    /// 关联ID
    pub id: u64,
    /// 关联的检测
    pub related_detections: Vec<u64>,
    /// 关联类型
    pub correlation_type: CorrelationType,
    /// 关联强度
    pub correlation_strength: f64,
    /// 关联时间窗口
    pub time_window: (u64, u64),
    /// 攻击模式
    pub attack_pattern: String,
    /// 攻击者画像
    pub attacker_profile: AttackerProfile,
}

/// 关联类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrelationType {
    /// 同源攻击
    SameSource,
    /// 同目标攻击
    SameTarget,
    /// 时间序列
    TimeSeries,
    /// 攻击链
    AttackChain,
    /// 命令控制
    CommandControl,
    /// 数据泄露
    DataExfiltration,
}

/// 攻击者画像
#[derive(Debug, Clone)]
pub struct AttackerProfile {
    /// 攻击者ID
    pub attacker_id: String,
    /// 攻击技巧
    pub techniques: Vec<String>,
    /// 偏好目标
    pub preferred_targets: Vec<String>,
    /// 活动模式
    pub activity_pattern: ActivityPattern,
    /// 威胁等级
    pub threat_level: ThreatLevel,
    /// 地理位置
    pub location: Option<String>,
}

/// 活动模式
#[derive(Debug, Clone)]
pub struct ActivityPattern {
    /// 活动时间
    pub active_hours: Vec<u8>,
    /// 攻击频率
    pub attack_frequency: f64,
    /// 持续时间
    pub attack_duration: u64,
    /// 攻击间隔
    pub attack_interval: u64,
}

/// 威胁数据
#[derive(Debug, Clone)]
pub struct ThreatData {
    /// 威胁ID
    pub threat_id: String,
    /// 威胁类型
    pub threat_type: ThreatType,
    /// 指示器
    pub indicators: Vec<Indicator>,
    /// 置信度
    pub confidence: f64,
    /// 严重程度
    pub severity: ThreatLevel,
    /// 有效期
    pub valid_until: u64,
    /// 来源
    pub source: String,
}

/// 威胁类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreatType {
    /// IP地址
    IpAddress,
    /// 域名
    Domain,
    /// 文件哈希
    FileHash,
    /// URL
    Url,
    /// 邮件地址
    Email,
    /// 恶意软件特征
    MalwareSignature,
}

/// 指示器
#[derive(Debug, Clone)]
pub struct Indicator {
    /// 指示器类型
    pub indicator_type: IndicatorType,
    /// 指示器值
    pub value: String,
    /// 描述
    pub description: String,
}

/// 指示器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndicatorType {
    /// IPv4地址
    IPv4,
    /// IPv6地址
    IPv6,
    /// 域名
    Domain,
    /// URL
    Url,
    /// 文件哈希MD5
    FileHashMD5,
    /// 文件哈希SHA1
    FileHashSHA1,
    /// 文件哈希SHA256
    FileHashSHA256,
    /// 邮件地址
    Email,
}

/// 配置类型定义

/// 网络入侵检测配置
#[derive(Debug, Clone)]
pub struct NetworkIdsConfig {
    /// 启用网络检测
    pub enabled: bool,
    /// 监听的接口
    pub interfaces: Vec<String>,
    /// 检测规则文件
    pub rule_files: Vec<String>,
    /// 流量缓存大小
    pub flow_cache_size: usize,
    /// 最大包大小
    pub max_packet_size: usize,
}

impl Default for NetworkIdsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interfaces: vec![String::from("eth0")],
            rule_files: Vec::new(),
            flow_cache_size: 10000,
            max_packet_size: 65535,
        }
    }
}

/// 主机入侵检测配置
#[derive(Debug, Clone)]
pub struct HostIdsConfig {
    /// 启用主机检测
    pub enabled: bool,
    /// 监控的系统调用
    pub monitored_syscalls: Vec<u32>,
    /// 监控的文件路径
    pub monitored_paths: Vec<String>,
    /// 监控的进程
    pub monitored_processes: Vec<String>,
    /// 监控的网络连接
    pub monitor_network: bool,
}

impl Default for HostIdsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            monitored_syscalls: vec![],
            monitored_paths: vec![String::from("/etc"), String::from("/var/log")],
            monitored_processes: vec![],
            monitor_network: true,
        }
    }
}

/// 异常检测配置
#[derive(Debug, Clone)]
pub struct AnomalyDetectionConfig {
    /// 启用异常检测
    pub enabled: bool,
    /// 检测算法
    pub algorithms: Vec<AnomalyAlgorithm>,
    /// 灵敏度
    pub sensitivity: f32,
    /// 训练数据大小
    pub training_data_size: usize,
    /// 更新间隔（小时）
    pub update_interval_hours: u32,
}

impl Default for AnomalyDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithms: vec![AnomalyAlgorithm::Statistical, AnomalyAlgorithm::MachineLearning],
            sensitivity: 0.7,
            training_data_size: 10000,
            update_interval_hours: 24,
        }
    }
}

/// 异常检测算法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalyAlgorithm {
    /// 统计方法
    Statistical,
    /// 机器学习
    MachineLearning,
    /// 孤立森林
    IsolationForest,
    /// 自编码器
    Autoencoder,
    /// 聚类分析
    Clustering,
}

/// 特征检测配置
#[derive(Debug, Clone)]
pub struct SignatureDetectionConfig {
    /// 启用特征检测
    pub enabled: bool,
    /// 特征数据库
    pub signature_database: String,
    /// 更新频率（小时）
    pub update_frequency_hours: u32,
    /// 启用启发式检测
    pub enable_heuristics: bool,
}

impl Default for SignatureDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            signature_database: String::from("signatures.db"),
            update_frequency_hours: 6,
            enable_heuristics: true,
        }
    }
}

/// 行为分析配置
#[derive(Debug, Clone)]
pub struct BehaviorAnalysisConfig {
    /// 启用行为分析
    pub enabled: bool,
    /// 行为模型数量
    pub model_count: usize,
    /// 分析窗口大小（小时）
    pub analysis_window_hours: u32,
    /// 基线更新频率（天）
    pub baseline_update_frequency_days: u32,
}

impl Default for BehaviorAnalysisConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model_count: 100,
            analysis_window_hours: 24,
            baseline_update_frequency_days: 7,
        }
    }
}

/// 威胁情报配置
#[derive(Debug, Clone)]
pub struct ThreatIntelligenceConfig {
    /// 启用威胁情报
    pub enabled: bool,
    /// 威胁情报源
    pub sources: Vec<String>,
    /// 更新间隔（小时）
    pub update_interval_hours: u32,
    /// 缓存大小
    pub cache_size: usize,
}

impl Default for ThreatIntelligenceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sources: Vec::new(),
            update_interval_hours: 1,
            cache_size: 10000,
        }
    }
}

/// 关联分析配置
#[derive(Debug, Clone)]
pub struct CorrelationConfig {
    /// 启用关联分析
    pub enabled: bool,
    /// 关联窗口大小（小时）
    pub correlation_window_hours: u32,
    /// 最小关联强度
    pub min_correlation_strength: f64,
    /// 启用攻击链分析
    pub enable_attack_chain_analysis: bool,
}

impl Default for CorrelationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            correlation_window_hours: 24,
            min_correlation_strength: 0.7,
            enable_attack_chain_analysis: true,
        }
    }
}

/// 全局入侵检测系统实例
pub static INTRUSION_DETECTION_SYSTEM: spin::Mutex<Option<IntrusionDetectionSystem>> =
    spin::Mutex::new(None);

/// 初始化入侵检测系统
pub fn init_ids() -> Result<(), &'static str> {
    let config = IdsConfig::default();
    let mut guard = INTRUSION_DETECTION_SYSTEM.lock();
    if guard.is_none() {
        *guard = Some(IntrusionDetectionSystem::new(config.clone()));
    }
    let ids = guard.as_mut().unwrap();
    ids.update_config(config)?;
    ids.init()
}

/// 处理网络包
pub fn process_packet(packet: &NetworkPacket) -> Result<Vec<IntrusionDetection>, &'static str> {
    let mut guard = INTRUSION_DETECTION_SYSTEM.lock();
    match guard.as_mut() {
        Some(ids) => ids.process_packet(packet),
        None => Err("IDS not initialized"),
    }
}

/// 处理系统事件
pub fn process_system_event(event: &AuditEvent) -> Result<Vec<IntrusionDetection>, &'static str> {
    let mut guard = INTRUSION_DETECTION_SYSTEM.lock();
    match guard.as_mut() {
        Some(ids) => ids.process_system_event(event),
        None => Err("IDS not initialized"),
    }
}

/// 运行关联分析
pub fn run_correlation_analysis() -> Result<Vec<CorrelationResult>, &'static str> {
    let mut guard = INTRUSION_DETECTION_SYSTEM.lock();
    match guard.as_mut() {
        Some(ids) => ids.run_correlation_analysis(),
        None => Err("IDS not initialized"),
    }
}

/// 更新威胁情报
pub fn update_threat_intelligence(threat_data: Vec<ThreatData>) -> Result<(), &'static str> {
    let mut guard = INTRUSION_DETECTION_SYSTEM.lock();
    match guard.as_mut() {
        Some(ids) => ids.update_threat_intelligence(threat_data),
        None => Err("IDS not initialized"),
    }
}

/// 获取IDS统计信息
pub fn get_ids_statistics() -> IdsStats {
    let guard = INTRUSION_DETECTION_SYSTEM.lock();
    match guard.as_ref() {
        Some(ids) => ids.get_stats(),
        None => IdsStats::default(),
    }
}

/// 停止入侵检测系统
pub fn shutdown_ids() -> Result<(), &'static str> {
    let mut guard = INTRUSION_DETECTION_SYSTEM.lock();
    match guard.as_mut() {
        Some(ids) => ids.shutdown(),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ids_config_default() {
        let config = IdsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.detection_mode, DetectionMode::Hybrid);
        assert_eq!(config.response_mode, ResponseMode::Active);
    }

    #[test]
    fn test_intrusion_detection_creation() {
        let config = IdsConfig::default();
        let ids = IntrusionDetectionSystem::new(config);
        assert_eq!(ids.id, 1);
        assert_eq!(ids.status, IdsStatus::Uninitialized);
        assert!(!ids.running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_threat_level_ordering() {
        assert!(ThreatLevel::Info < ThreatLevel::Low);
        assert!(ThreatLevel::Low < ThreatLevel::Medium);
        assert!(ThreatLevel::Medium < ThreatLevel::High);
        assert!(ThreatLevel::High < ThreatLevel::Critical);
    }

    #[test]
    fn test_detection_type() {
        assert_ne!(DetectionType::NetworkIntrusion, DetectionType::HostIntrusion);
        assert_eq!(DetectionType::MalwareDetection, DetectionType::MalwareDetection);
    }

    #[test]
    fn test_attack_stage() {
        assert_ne!(AttackStage::Reconnaissance, AttackStage::Exploitation);
        assert_eq!(AttackStage::CommandAndControl, AttackStage::CommandAndControl);
    }

    #[test]
    fn test_response_action() {
        match ResponseAction::Log {
            // ResponseAction variants can be tested
        }
    }

    #[test]
    fn test_ids_stats_default() {
        let stats = IdsStats::default();
        assert_eq!(stats.total_detections, 0);
        assert_eq!(stats.packets_processed, 0);
        assert_eq!(stats.syscalls_analyzed, 0);
    }
}