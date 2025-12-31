//! # 入侵检测系统 (IDS/IPS) 引擎
//!
//! 提供全面的入侵检测和防御功能：
//! - 基于签名的检测（Snort/Suricata 风格）
//! - 异常检测算法
//! - 实时流量分析
//! - 规则引擎
//! - 告警和响应
//!
//! ## 检测模式
//!
//! 1. **签名检测**: 基于已知攻击模式的匹配
//! 2. **异常检测**: 基于行为分析的异常识别
//! 3. **混合模式**: 结合签名和异常检测
//! 4. **启发式检测**: 基于专家系统的智能检测

extern crate alloc;

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use crate::net::Packet;

/// IDS 错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdsError {
    /// 规则解析错误
    RuleParseError(String),
    /// 规则不存在
    RuleNotFound,
    /// 特征数据库错误
    SignatureDatabaseError,
    /// 检测引擎错误
    DetectionError(String),
    /// 配置错误
    ConfigurationError(String),
    /// 资源耗尽
    ResourceExhausted,
}

/// 威胁级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreatLevel {
    /// 信息
    Info = 0,
    /// 低威胁
    Low = 1,
    /// 中等威胁
    Medium = 2,
    /// 高威胁
    High = 3,
    /// 严重威胁
    Critical = 4,
}

/// 检测结果
#[derive(Debug, Clone)]
pub struct Detection {
    /// 检测 ID
    pub id: u64,
    /// 规则 ID
    pub rule_id: String,
    /// 威胁级别
    pub threat_level: ThreatLevel,
    /// 检测消息
    pub message: String,
    /// 源 IP
    pub source_ip: Option<String>,
    /// 目标 IP
    pub dest_ip: Option<String>,
    /// 源端口
    pub source_port: Option<u16>,
    /// 目标端口
    pub dest_port: Option<u16>,
    /// 协议
    pub protocol: Option<String>,
    /// 检测时间
    pub timestamp: u64,
    /// 置信度 (0.0 - 1.0)
    pub confidence: f32,
    /// 检测类型
    pub detection_type: DetectionType,
}

/// 检测类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionType {
    /// 签名检测
    Signature,
    /// 异常检测
    Anomaly,
    /// 行为检测
    Behavior,
    /// 启发式检测
    Heuristic,
    /// 协议异常
    ProtocolAnomaly,
    /// 流量异常
    TrafficAnomaly,
}

/// IDS 规则
#[derive(Debug, Clone)]
pub struct IdsRule {
    /// 规则 ID
    pub id: String,
    /// 规则动作
    pub action: RuleAction,
    /// 协议
    pub protocol: Protocol,
    /// 源地址
    pub source_addr: Option<AddressSpec>,
    /// 源端口
    pub source_port: Option<PortSpec>,
    /// 目标地址
    pub dest_addr: Option<AddressSpec>,
    /// 目标端口
    pub dest_port: Option<PortSpec>,
    /// 规则选项
    pub options: Vec<RuleOption>,
    /// 威胁级别
    pub threat_level: ThreatLevel,
    /// 规则描述
    pub description: String,
    /// 规则引用
    pub reference: Vec<String>,
    /// 是否启用
    pub enabled: bool,
}

/// 规则动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleAction {
    /// 告警
    Alert,
    /// 记录日志
    Log,
    /// 通过
    Pass,
    /// 拒绝
    Reject,
    /// 丢弃
    Drop,
}

/// 协议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Tcp,
    Udp,
    Icmp,
    Ip,
    Any,
}

/// 地址规范
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressSpec {
    /// 单个地址
    Single(String),
    /// 地址范围
    Range { start: String, end: String },
    /// 任意地址
    Any,
    /// 取反
    Negated(Box<AddressSpec>),
}

/// 端口规范
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortSpec {
    /// 单个端口
    Single(u16),
    /// 端口范围
    Range { start: u16, end: u16 },
    /// 任意端口
    Any,
    /// 取反
    Negated(Box<PortSpec>),
}

/// 规则选项
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleOption {
    /// 选项名称
    pub name: String,
    /// 选项参数
    pub args: Vec<String>,
}

impl RuleOption {
    /// 创建新的规则选项
    pub fn new(name: String, args: Vec<String>) -> Self {
        Self { name, args }
    }
}

/// 特征签名
#[derive(Debug, Clone)]
pub struct Signature {
    /// 签名 ID
    pub id: String,
    /// 签名名称
    pub name: String,
    /// 签名类型
    pub sig_type: SignatureType,
    /// 模式
    pub pattern: Vec<u8>,
    /// 偏移量
    pub offset: Option<usize>,
    /// 深度
    pub depth: Option<usize>,
    /// 距离
    pub distance: Option<isize>,
    /// 在...之后
    pub within: Option<usize>,
    /// 上下文
    pub context: Option<SignatureContext>,
}

/// 签名类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureType {
    /// 内容匹配
    Content,
    /// URI 匹配
    Uri,
    /// 正则表达式
    Regex,
    /// 字节测试
    ByteTest,
    /// 字节跳转
    ByteJump,
    /// PCRE
    Pcre,
}

/// 签名上下文
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureContext {
    /// 包头
    Header,
    /// 包体
    Body,
    /// 整个包
    Packet,
}

/// 异常检测模型
#[derive(Debug, Clone)]
pub struct AnomalyModel {
    /// 模型 ID
    pub id: String,
    /// 模型名称
    pub name: String,
    /// 检测算法
    pub algorithm: DetectionAlgorithm,
    /// 基线数据
    pub baseline: Vec<f32>,
    /// 阈值
    pub threshold: f32,
    /// 灵敏度
    pub sensitivity: f32,
    /// 最后更新
    pub last_updated: u64,
}

/// 检测算法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionAlgorithm {
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

/// IDS 统计信息
#[derive(Debug, Clone, Default)]
pub struct IdsStatistics {
    /// 总检测数
    pub total_detections: u64,
    /// 签名检测数
    pub signature_detections: u64,
    /// 异常检测数
    pub anomaly_detections: u64,
    /// 处理的包数
    pub packets_processed: u64,
    /// 规则匹配数
    pub rule_matches: u64,
    /// 平均检测延迟（微秒）
    pub avg_detection_latency_us: u64,
    /// 按威胁级别统计
    pub detections_by_level: BTreeMap<ThreatLevel, u64>,
}

/// IDS 引擎
pub struct IdsEngine {
    /// IDS 规则
    rules: Mutex<Vec<IdsRule>>,
    /// 特征签名
    signatures: Mutex<Vec<Signature>>,
    /// 异常模型
    anomaly_models: Mutex<Vec<AnomalyModel>>,
    /// 检测历史
    detection_history: Mutex<Vec<Detection>>,
    /// 统计信息
    stats: Mutex<IdsStatistics>,
    /// 配置
    config: IdsConfig,
    /// 下一个检测 ID
    next_detection_id: AtomicU64,
    /// 是否启用
    enabled: AtomicU64,
}

/// IDS 配置
#[derive(Debug, Clone)]
pub struct IdsConfig {
    /// 启用签名检测
    pub enable_signature_detection: bool,
    /// 启用异常检测
    pub enable_anomaly_detection: bool,
    /// 规则文件路径
    pub rule_files: Vec<String>,
    /// 特征数据库路径
    pub signature_database: String,
    /// 最大历史记录数
    pub max_history_size: usize,
    /// 告警阈值
    pub alert_threshold: ThreatLevel,
    /// 性能模式
    pub performance_mode: PerformanceMode,
}

/// 性能模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceMode {
    /// 低延迟（快速检测，可能遗漏）
    LowLatency,
    /// 平衡模式
    Balanced,
    /// 高精度（深度检测，较慢）
    HighAccuracy,
}

impl Default for IdsConfig {
    fn default() -> Self {
        Self {
            enable_signature_detection: true,
            enable_anomaly_detection: true,
            rule_files: Vec::new(),
            signature_database: String::from("/etc/ids/signatures.db"),
            max_history_size: 10000,
            alert_threshold: ThreatLevel::Medium,
            performance_mode: PerformanceMode::Balanced,
        }
    }
}

impl IdsEngine {
    /// 创建新的 IDS 引擎
    pub fn new(config: IdsConfig) -> Self {
        Self {
            rules: Mutex::new(Vec::new()),
            signatures: Mutex::new(Vec::new()),
            anomaly_models: Mutex::new(Vec::new()),
            detection_history: Mutex::new(Vec::new()),
            stats: Mutex::new(IdsStatistics::default()),
            config,
            next_detection_id: AtomicU64::new(1),
            enabled: AtomicU64::new(1),
        }
    }

    /// 初始化 IDS 引擎
    pub fn init(&self) -> Result<(), IdsError> {
        // 加载规则文件
        for rule_file in &self.config.rule_files.clone() {
            self.load_rules_from_file(rule_file)?;
        }

        // 加载特征数据库
        self.load_signature_database()?;

        crate::println!("[IDS] IDS engine initialized successfully");
        Ok(())
    }

    /// 处理网络包
    pub fn process_packet(&self, packet: &Packet) -> Result<Vec<Detection>, IdsError> {
        if self.enabled.load(Ordering::SeqCst) == 0 {
            return Ok(Vec::new());
        }

        let start_time = crate::subsystems::time::get_timestamp_nanos();
        let mut detections = Vec::new();

        // 签名检测
        if self.config.enable_signature_detection {
            let sig_detections = self.signature_detection(packet)?;
            detections.extend(sig_detections);
        }

        // 异常检测
        if self.config.enable_anomaly_detection {
            let anomaly_detections = self.anomaly_detection(packet)?;
            detections.extend(anomaly_detections);
        }

        // 过滤低于阈值的检测
        detections.retain(|d| d.threat_level >= self.config.alert_threshold);

        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.packets_processed += 1;
            stats.total_detections += detections.len() as u64;

            for detection in &detections {
                *stats.detections_by_level.entry(detection.threat_level).or_insert(0) += 1;
            }

            let elapsed = crate::subsystems::time::get_timestamp_nanos() - start_time;
            stats.avg_detection_latency_us =
                (stats.avg_detection_latency_us + elapsed / 1000) / 2;
        }

        // 保存检测历史
        if !detections.is_empty() {
            let mut history = self.detection_history.lock();
            for detection in &detections {
                history.push(detection.clone());
            }

            // 清理旧记录
            if history.len() > self.config.max_history_size {
                let drain_count = history.len() - self.config.max_history_size;
                history.drain(0..drain_count);
            }
        }

        Ok(detections)
    }

    /// 签名检测
    fn signature_detection(&self, packet: &Packet) -> Result<Vec<Detection>, IdsError> {
        let mut detections = Vec::new();
        let rules = self.rules.lock();

        for rule in rules.iter().filter(|r| r.enabled) {
            if self.matches_rule(packet, rule) {
                let detection = Detection {
                    id: self.next_detection_id.fetch_add(1, Ordering::SeqCst),
                    rule_id: rule.id.clone(),
                    threat_level: rule.threat_level,
                    message: rule.description.clone(),
                    source_ip: None, // 需要从包中提取
                    dest_ip: None,
                    source_port: None,
                    dest_port: None,
                    protocol: Some(format!("{:?}", rule.protocol)),
                    timestamp: crate::subsystems::time::get_timestamp(),
                    confidence: 0.9, // 签名检测置信度较高
                    detection_type: DetectionType::Signature,
                };

                detections.push(detection);

                // 更新统计
                let mut stats = self.stats.lock();
                stats.signature_detections += 1;
                stats.rule_matches += 1;
            }
        }

        Ok(detections)
    }

    /// 异常检测
    fn anomaly_detection(&self, packet: &Packet) -> Result<Vec<Detection>, IdsError> {
        let mut detections = Vec::new();
        let models = self.anomaly_models.lock();

        for model in models.iter() {
            if let Some(anomaly_score) = self.detect_anomaly(packet, model)? {
                if anomaly_score > model.threshold {
                    let detection = Detection {
                        id: self.next_detection_id.fetch_add(1, Ordering::SeqCst),
                        rule_id: model.id.clone(),
                        threat_level: if anomaly_score > model.threshold * 2.0 {
                            ThreatLevel::High
                        } else {
                            ThreatLevel::Medium
                        },
                        message: format!("Anomaly detected: {}", model.name),
                        source_ip: None,
                        dest_ip: None,
                        source_port: None,
                        dest_port: None,
                        protocol: None,
                        timestamp: crate::subsystems::time::get_timestamp(),
                        confidence: anomaly_score,
                        detection_type: DetectionType::Anomaly,
                    };

                    detections.push(detection);

                    // 更新统计
                    let mut stats = self.stats.lock();
                    stats.anomaly_detections += 1;
                }
            }
        }

        Ok(detections)
    }

    /// 检查包是否匹配规则
    fn matches_rule(&self, packet: &Packet, rule: &IdsRule) -> bool {
        // 简化实现 - 实际需要解析包头并进行详细匹配

        // 检查协议
        match rule.protocol {
            Protocol::Any => {}
            _ => {
                // 需要检查包的实际协议
            }
        }

        // 检查规则选项
        for option in &rule.options {
            if !self.matches_rule_option(packet, option) {
                return false;
            }
        }

        true
    }

    /// 检查包是否匹配规则选项
    fn matches_rule_option(&self, packet: &Packet, option: &RuleOption) -> bool {
        match option.name.as_str() {
            "content" => {
                if option.args.is_empty() {
                    return false;
                }
                // 检查包内容
                let pattern = &option.args[0].as_bytes();
                packet.data().windows(pattern.len()).any(|w| w == *pattern)
            }
            "msg" => true, // 消息选项不影响匹配
            "sid" => true, // 规则 ID
            "rev" => true, // 规则版本
            "gid" => true, // 规则组 ID
            _ => true,     // 其他选项暂不实现
        }
    }

    /// 检测异常
    fn detect_anomaly(
        &self,
        packet: &Packet,
        model: &AnomalyModel,
    ) -> Result<Option<f32>, IdsError> {
        match model.algorithm {
            DetectionAlgorithm::Statistical => {
                // 统计方法：计算 z-score
                let packet_size = packet.data().len() as f32;
                if model.baseline.is_empty() {
                    return Ok(None);
                }

                let mean: f32 = model.baseline.iter().sum::<f32>() / model.baseline.len() as f32;
                let variance: f32 = model
                    .baseline
                    .iter()
                    .map(|&x| (x - mean).powi(2))
                    .sum::<f32>()
                    / model.baseline.len() as f32;
                let std_dev = variance.sqrt();

                if std_dev > 0.0 {
                    let z_score = (packet_size - mean).abs() / std_dev;
                    Ok(Some(z_score))
                } else {
                    Ok(None)
                }
            }
            DetectionAlgorithm::MachineLearning => {
                // 机器学习方法（简化实现）
                Ok(None)
            }
            DetectionAlgorithm::IsolationForest => {
                // 孤立森林算法（简化实现）
                Ok(None)
            }
            DetectionAlgorithm::Autoencoder => {
                // 自编码器算法（简化实现）
                Ok(None)
            }
            DetectionAlgorithm::Clustering => {
                // 聚类分析算法（简化实现）
                Ok(None)
            }
        }
    }

    /// 添加规则
    pub fn add_rule(&self, rule: IdsRule) -> Result<(), IdsError> {
        let mut rules = self.rules.lock();
        rules.push(rule);
        Ok(())
    }

    /// 删除规则
    pub fn remove_rule(&self, rule_id: &str) -> Result<(), IdsError> {
        let mut rules = self.rules.lock();
        let pos = rules
            .iter()
            .position(|r| r.id == rule_id)
            .ok_or(IdsError::RuleNotFound)?;
        rules.remove(pos);
        Ok(())
    }

    /// 从文件加载规则
    fn load_rules_from_file(&self, path: &str) -> Result<(), IdsError> {
        // 简化实现 - 实际需要读取和解析规则文件
        crate::println!("[IDS] Loading rules from: {}", path);
        Ok(())
    }

    /// 加载特征数据库
    fn load_signature_database(&self) -> Result<(), IdsError> {
        // 简化实现 - 实际需要加载特征数据库
        crate::println!("[IDS] Loading signature database");
        Ok(())
    }

    /// 添加异常模型
    pub fn add_anomaly_model(&self, model: AnomalyModel) -> Result<(), IdsError> {
        let mut models = self.anomaly_models.lock();
        models.push(model);
        Ok(())
    }

    /// 更新异常模型基线
    pub fn update_model_baseline(&self, model_id: &str, new_baseline: Vec<f32>) -> Result<(), IdsError> {
        let mut models = self.anomaly_models.lock();
        let model = models
            .iter_mut()
            .find(|m| m.id == model_id)
            .ok_or(IdsError::RuleNotFound)?;
        model.baseline = new_baseline;
        model.last_updated = crate::subsystems::time::get_timestamp();
        Ok(())
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> IdsStatistics {
        let stats = self.stats.lock();
        IdsStatistics {
            total_detections: stats.total_detections,
            signature_detections: stats.signature_detections,
            anomaly_detections: stats.anomaly_detections,
            packets_processed: stats.packets_processed,
            rule_matches: stats.rule_matches,
            avg_detection_latency_us: stats.avg_detection_latency_us,
            detections_by_level: stats.detections_by_level.clone(),
        }
    }

    /// 重置统计信息
    pub fn reset_statistics(&self) {
        *self.stats.lock() = IdsStatistics::default();
    }

    /// 获取检测历史
    pub fn get_detection_history(&self, limit: Option<usize>) -> Vec<Detection> {
        let history = self.detection_history.lock();
        match limit {
            Some(limit) => history.iter().rev().take(limit).cloned().collect(),
            None => history.clone(),
        }
    }

    /// 启用 IDS
    pub fn enable(&self) {
        self.enabled.store(1, Ordering::SeqCst);
    }

    /// 禁用 IDS
    pub fn disable(&self) {
        self.enabled.store(0, Ordering::SeqCst);
    }

    /// 是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst) == 1
    }
}

/// 全局 IDS 引擎实例
pub static GLOBAL_IDS: Mutex<Option<IdsEngine>> = Mutex::new(None);

/// 初始化全局 IDS 引擎
pub fn init_ids() -> Result<(), IdsError> {
    let config = IdsConfig::default();
    let engine = IdsEngine::new(config);

    engine.init()?;

    let mut global = GLOBAL_IDS.lock();
    *global = Some(engine);

    Ok(())
}

/// 获取全局 IDS 引擎
pub fn get_ids_engine() -> Option<&'static Mutex<IdsEngine>> {
    // 简化实现 - 实际需要更好的访问模式
    None
}

/// 处理网络包（便捷函数）
pub fn process_packet_ids(packet: &Packet) -> Result<Vec<Detection>, IdsError> {
    let global = GLOBAL_IDS.lock();
    let engine = global.as_ref().ok_or(IdsError::DetectionError("IDS not initialized".to_string()))?;
    engine.process_packet(packet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ids_rule_creation() {
        let rule = IdsRule {
            id: String::from("1"),
            action: RuleAction::Alert,
            protocol: Protocol::Tcp,
            source_addr: Some(AddressSpec::Any),
            source_port: Some(PortSpec::Any),
            dest_addr: Some(AddressSpec::Any),
            dest_port: Some(PortSpec::Single(80)),
            options: vec![RuleOption::new(
                String::from("msg"),
                vec![String::from("HTTP Traffic")],
            )],
            threat_level: ThreatLevel::Low,
            description: String::from("Detect HTTP traffic"),
            reference: Vec::new(),
            enabled: true,
        };

        assert_eq!(rule.id, "1");
        assert_eq!(rule.action, RuleAction::Alert);
        assert_eq!(rule.protocol, Protocol::Tcp);
    }

    #[test]
    fn test_threat_level_ordering() {
        assert!(ThreatLevel::Info < ThreatLevel::Low);
        assert!(ThreatLevel::Low < ThreatLevel::Medium);
        assert!(ThreatLevel::Medium < ThreatLevel::High);
        assert!(ThreatLevel::High < ThreatLevel::Critical);
    }

    #[test]
    fn test_ids_engine_creation() {
        let config = IdsConfig::default();
        let engine = IdsEngine::new(config);

        assert!(engine.is_enabled());
        assert_eq!(engine.get_statistics().total_detections, 0);
    }

    #[test]
    fn test_rule_option_creation() {
        let option = RuleOption::new(
            String::from("content"),
            vec![String::from("GET"), String::from("POST")],
        );

        assert_eq!(option.name, "content");
        assert_eq!(option.args.len(), 2);
    }

    #[test]
    fn test_address_spec() {
        let addr = AddressSpec::Single(String::from("192.168.1.1"));
        assert!(matches!(addr, AddressSpec::Single(_)));

        let negated = AddressSpec::Negated(Box::new(addr));
        assert!(matches!(negated, AddressSpec::Negated(_)));
    }

    #[test]
    fn test_port_spec() {
        let port = PortSpec::Single(80);
        assert!(matches!(port, PortSpec::Single(80)));

        let range = PortSpec::Range {
            start: 1024,
            end: 65535,
        };
        assert!(matches!(range, PortSpec::Range { .. }));
    }

    #[test]
    fn test_signature() {
        let sig = Signature {
            id: String::from("1"),
            name: String::from("Test Signature"),
            sig_type: SignatureType::Content,
            pattern: vec![b'G', b'E', b'T'],
            offset: Some(0),
            depth: Some(100),
            distance: None,
            within: None,
            context: Some(SignatureContext::Packet),
        };

        assert_eq!(sig.id, "1");
        assert_eq!(sig.sig_type, SignatureType::Content);
        assert_eq!(sig.pattern, vec![b'G', b'E', b'T']);
    }

    #[test]
    fn test_anomaly_model() {
        let model = AnomalyModel {
            id: String::from("1"),
            name: String::from("Test Model"),
            algorithm: DetectionAlgorithm::Statistical,
            baseline: vec![100.0, 110.0, 105.0, 95.0],
            threshold: 2.0,
            sensitivity: 0.7,
            last_updated: 0,
        };

        assert_eq!(model.id, "1");
        assert_eq!(model.algorithm, DetectionAlgorithm::Statistical);
        assert_eq!(model.baseline.len(), 4);
    }

    #[test]
    fn test_ids_config_default() {
        let config = IdsConfig::default();
        assert!(config.enable_signature_detection);
        assert!(config.enable_anomaly_detection);
        assert_eq!(config.alert_threshold, ThreatLevel::Medium);
    }

    #[test]
    fn test_detection() {
        let detection = Detection {
            id: 1,
            rule_id: String::from("1"),
            threat_level: ThreatLevel::High,
            message: String::from("Test detection"),
            source_ip: Some(String::from("192.168.1.1")),
            dest_ip: Some(String::from("10.0.0.1")),
            source_port: Some(12345),
            dest_port: Some(80),
            protocol: Some(String::from("TCP")),
            timestamp: 0,
            confidence: 0.9,
            detection_type: DetectionType::Signature,
        };

        assert_eq!(detection.id, 1);
        assert_eq!(detection.threat_level, ThreatLevel::High);
        assert_eq!(detection.detection_type, DetectionType::Signature);
    }

    #[test]
    fn test_ids_statistics_default() {
        let stats = IdsStatistics::default();
        assert_eq!(stats.total_detections, 0);
        assert_eq!(stats.packets_processed, 0);
    }
}
