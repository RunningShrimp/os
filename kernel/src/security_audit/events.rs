// Event Processing Module for Security Audit
//
// 事件处理模块，负责处理和管理安全审计事件

extern crate alloc;

use alloc::format;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;
use alloc::string::ToString;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use crate::security::audit::{AuditEvent, AuditEventType, AuditSeverity};
use super::{SecurityAuditConfig, AuditRule, AuditAction, AuditCondition, AuditOperator};

/// 事件处理器
pub struct EventProcessor {
    /// 处理器ID
    pub id: u64,
    /// 事件队列
    event_queue: Arc<Mutex<Vec<AuditEvent>>>,
    /// 处理规则
    rules: Vec<AuditRule>,
    /// 事件统计
    stats: Arc<Mutex<EventProcessorStats>>,
    /// 下一个事件ID
    next_event_id: AtomicU64,
    /// 是否正在运行
    running: bool,
}

/// 事件处理器统计
#[derive(Debug, Default, Clone)]
pub struct EventProcessorStats {
    /// 处理的事件总数
    pub total_events: u64,
    /// 按类型统计
    pub events_by_type: BTreeMap<AuditEventType, u64>,
    /// 按严重级别统计
    pub events_by_severity: BTreeMap<AuditSeverity, u64>,
    /// 规则匹配次数
    pub rule_matches: u64,
    /// 规则执行次数
    pub rule_executions: u64,
    /// 平均处理时间（微秒）
    pub avg_processing_time_us: u64,
    /// 丢弃的事件数
    pub dropped_events: u64,
    /// 事件队列长度
    pub queue_length: usize,
}

impl EventProcessor {
    /// 创建新的事件处理器
    pub fn new() -> Self {
        Self {
            id: 1,
            event_queue: Arc::new(Mutex::new(Vec::new())),
            rules: Vec::new(),
            stats: Arc::new(Mutex::new(EventProcessorStats::default())),
            next_event_id: AtomicU64::new(1),
            running: false,
        }
    }

    /// 初始化事件处理器
    pub fn init(&mut self, config: &SecurityAuditConfig) -> Result<(), &'static str> {
        self.rules = config.audit_rules.clone();
        self.running = true;
        crate::println!("[EventProcessor] Event processor initialized with {} rules", self.rules.len());
        Ok(())
    }

    /// 处理事件
    pub fn process_event(&mut self, event: &AuditEvent) -> Result<(), &'static str> {
        if !self.running {
            return Err("Event processor not running");
        }

        let start_time = crate::time::get_timestamp_nanos();

        // 应用处理规则
        self.apply_rules(event)?;

        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.total_events += 1;
            *stats.events_by_type.entry(event.event_type).or_insert(0) += 1;
            *stats.events_by_severity.entry(event.severity).or_insert(0) += 1;

            let elapsed = crate::time::get_timestamp_nanos() - start_time;
            stats.avg_processing_time_us = (stats.avg_processing_time_us + elapsed / 1000) / 2;
        }

        Ok(())
    }

    /// 应用处理规则
    fn apply_rules(&mut self, event: &AuditEvent) -> Result<(), &'static str> {
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            if self.rule_matches(rule, event) {
                {
                    let mut stats = self.stats.lock();
                    stats.rule_matches += 1;
                }

                // 执行规则动作
                self.execute_rule_actions(rule, event)?;

                {
                    let mut stats = self.stats.lock();
                    stats.rule_executions += 1;
                }
            }
        }
        Ok(())
    }

    /// 检查规则是否匹配
    fn rule_matches(&self, rule: &AuditRule, event: &AuditEvent) -> bool {
        // 根据规则类型进行匹配
        match rule.rule_type {
            super::AuditRuleType::EventMatch => {
                self.event_match_conditions(&rule.conditions, event)
            }
            super::AuditRuleType::BehaviorAnalysis => {
                self.behavior_analysis_conditions(&rule.conditions, event)
            }
            super::AuditRuleType::AnomalyDetection => {
                self.anomaly_detection_conditions(&rule.conditions, event)
            }
            super::AuditRuleType::ComplianceCheck => {
                self.compliance_check_conditions(&rule.conditions, event)
            }
            super::AuditRuleType::SecurityPolicy => {
                self.security_policy_conditions(&rule.conditions, event)
            }
        }
    }

    /// 事件匹配条件
    fn event_match_conditions(&self, conditions: &[AuditCondition], event: &AuditEvent) -> bool {
        for condition in conditions {
            if !self.evaluate_condition(condition, event) {
                return false;
            }
        }
        true
    }

    /// 行为分析条件
    fn behavior_analysis_conditions(&self, conditions: &[AuditCondition], event: &AuditEvent) -> bool {
        // 简化的行为分析逻辑
        // 实际实现会包含更复杂的行为模式识别
        self.event_match_conditions(conditions, event)
    }

    /// 异常检测条件
    fn anomaly_detection_conditions(&self, conditions: &[AuditCondition], event: &AuditEvent) -> bool {
        // 简化的异常检测逻辑
        // 实际实现会使用机器学习算法
        self.event_match_conditions(conditions, event)
    }

    /// 合规检查条件
    fn compliance_check_conditions(&self, conditions: &[AuditCondition], event: &AuditEvent) -> bool {
        // 简化的合规检查逻辑
        self.event_match_conditions(conditions, event)
    }

    /// 安全策略条件
    fn security_policy_conditions(&self, conditions: &[AuditCondition], event: &AuditEvent) -> bool {
        // 简化的安全策略检查逻辑
        self.event_match_conditions(conditions, event)
    }

    /// 评估条件
    fn evaluate_condition(&self, condition: &AuditCondition, event: &AuditEvent) -> bool {
        let field_value = self.get_field_value(&condition.field, event);

        match condition.operator {
            AuditOperator::Equals => field_value == condition.value,
            AuditOperator::NotEquals => field_value != condition.value,
            AuditOperator::Contains => field_value.contains(&condition.value),
            AuditOperator::NotContains => !field_value.contains(&condition.value),
            AuditOperator::GreaterThan => {
                // 简化的数值比较
                match (field_value.parse::<u64>(), condition.value.parse::<u64>()) {
                    (Ok(val1), Ok(val2)) => val1 > val2,
                    _ => field_value > condition.value,
                }
            }
            AuditOperator::GreaterThanOrEqual => {
                match (field_value.parse::<u64>(), condition.value.parse::<u64>()) {
                    (Ok(val1), Ok(val2)) => val1 >= val2,
                    _ => field_value >= condition.value,
                }
            }
            AuditOperator::LessThan => {
                match (field_value.parse::<u64>(), condition.value.parse::<u64>()) {
                    (Ok(val1), Ok(val2)) => val1 < val2,
                    _ => field_value < condition.value,
                }
            }
            AuditOperator::LessThanOrEqual => {
                match (field_value.parse::<u64>(), condition.value.parse::<u64>()) {
                    (Ok(val1), Ok(val2)) => val1 <= val2,
                    _ => field_value <= condition.value,
                }
            }
            AuditOperator::Regex => {
                // 简化的正则表达式匹配
                field_value.contains(&condition.value)
            }
        }
    }

    /// 获取字段值
    fn get_field_value(&self, field: &str, event: &AuditEvent) -> String {
        match field {
            "event_type" => format!("{:?}", event.event_type),
            "severity" => format!("{:?}", event.severity),
            "pid" => event.pid.to_string(),
            "uid" => event.uid.to_string(),
            "gid" => event.gid.to_string(),
            "tid" => event.tid.to_string(),
            "message" => event.message.clone(),
            "timestamp" => event.timestamp.to_string(),
            "id" => event.id.to_string(),
            _ => {
                // 检查数据字段
                event.data.get(field).cloned().unwrap_or_default()
            }
        }
    }

    /// 执行规则动作
    fn execute_rule_actions(&self, rule: &AuditRule, event: &AuditEvent) -> Result<(), &'static str> {
        for action in &rule.actions {
            match action {
                AuditAction::Log => {
                    crate::println!("[EventProcessor] Rule '{}' triggered: {}", rule.name, event.message);
                }
                AuditAction::Alert => {
                    crate::println!("[EventProcessor] ALERT: Rule '{}' triggered by event {}", rule.name, event.id);
                }
                AuditAction::Block => {
                    crate::println!("[EventProcessor] BLOCKING: Rule '{}' blocking event {}", rule.name, event.id);
                }
                AuditAction::Notify => {
                    crate::println!("[EventProcessor] NOTIFY: Rule '{}' notification for event {}", rule.name, event.id);
                }
                AuditAction::ExecuteScript(script) => {
                    crate::println!("[EventProcessor] EXECUTE: Rule '{}' executing script: {}", rule.name, script);
                }
                AuditAction::CallApi(api) => {
                    crate::println!("[EventProcessor] API CALL: Rule '{}' calling API: {}", rule.name, api);
                }
            }
        }
        Ok(())
    }

    /// 添加规则
    pub fn add_rule(&mut self, rule: AuditRule) -> Result<(), &'static str> {
        self.rules.push(rule);
        Ok(())
    }

    /// 移除规则
    pub fn remove_rule(&mut self, rule_id: u64) -> Result<(), &'static str> {
        self.rules.retain(|rule| rule.id != rule_id);
        Ok(())
    }

    /// 获取规则列表
    pub fn get_rules(&self) -> &[AuditRule] {
        &self.rules
    }

    /// 更新规则
    pub fn update_rule(&mut self, rule: AuditRule) -> Result<(), &'static str> {
        if let Some(existing_rule) = self.rules.iter_mut().find(|r| r.id == rule.id) {
            *existing_rule = rule;
            Ok(())
        } else {
            Err("Rule not found")
        }
    }

    /// 启用/禁用规则
    pub fn enable_rule(&mut self, rule_id: u64, enabled: bool) -> Result<(), &'static str> {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.id == rule_id) {
            rule.enabled = enabled;
            Ok(())
        } else {
            Err("Rule not found")
        }
    }

    /// 获取统计数据
    pub fn get_stats(&self) -> EventProcessorStats {
        self.stats.lock().clone()
    }

    /// 重置统计数据
    pub fn reset_stats(&self) {
        *self.stats.lock() = EventProcessorStats::default();
    }

    /// 停止事件处理器
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        self.running = false;
        crate::println!("[EventProcessor] Event processor shutdown");
        Ok(())
    }
}

/// 事件聚合器
pub struct EventAggregator {
    /// 聚合窗口大小（秒）
    pub window_size: u64,
    /// 聚合规则
    pub aggregation_rules: Vec<AggregationRule>,
    /// 聚合结果
    pub aggregated_events: Vec<AggregatedEvent>,
}

/// 聚合规则
#[derive(Debug, Clone)]
pub struct AggregationRule {
    /// 规则ID
    pub id: u64,
    /// 规则名称
    pub name: String,
    /// 聚合条件
    pub conditions: Vec<AuditCondition>,
    /// 聚合类型
    pub aggregation_type: AggregationType,
    /// 聚合字段
    pub aggregation_field: String,
    /// 阈值
    pub threshold: u64,
}

/// 聚合类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationType {
    /// 计数
    Count,
    /// 求和
    Sum,
    /// 平均值
    Average,
    /// 最大值
    Max,
    /// 最小值
    Min,
}

/// 聚合事件
#[derive(Debug, Clone)]
pub struct AggregatedEvent {
    /// 聚合ID
    pub id: u64,
    /// 聚合类型
    pub aggregation_type: AggregationType,
    /// 聚合值
    pub aggregated_value: f64,
    /// 原始事件数量
    pub event_count: u64,
    /// 时间窗口
    pub time_window: (u64, u64),
    /// 生成时间
    pub generated_at: u64,
    /// 触发规则
    pub triggered_rule: u64,
}

impl EventAggregator {
    /// 创建新的事件聚合器
    pub fn new(window_size: u64) -> Self {
        Self {
            window_size,
            aggregation_rules: Vec::new(),
            aggregated_events: Vec::new(),
        }
    }

    /// 添加聚合规则
    pub fn add_rule(&mut self, rule: AggregationRule) {
        self.aggregation_rules.push(rule);
    }

    /// 聚合事件
    pub fn aggregate_events(&mut self, events: &[AuditEvent]) -> Vec<AggregatedEvent> {
        let mut results = Vec::new();

        for rule in &self.aggregation_rules {
            let filtered_events: Vec<&AuditEvent> = events.iter()
                .filter(|event| self.rule_matches(rule, event))
                .collect();

            if filtered_events.len() < rule.threshold as usize {
                continue;
            }

            let aggregated_value = self.calculate_aggregation(&rule.aggregation_type, &filtered_events, &rule.aggregation_field);

            let aggregated_event = AggregatedEvent {
                id: results.len() as u64 + 1,
                aggregation_type: rule.aggregation_type,
                aggregated_value,
                event_count: filtered_events.len() as u64,
                time_window: (
                    filtered_events.iter().map(|e| e.timestamp).min().unwrap_or(0),
                    filtered_events.iter().map(|e| e.timestamp).max().unwrap_or(0),
                ),
                generated_at: crate::time::get_timestamp_nanos(),
                triggered_rule: rule.id,
            };

            results.push(aggregated_event);
        }

        self.aggregated_events.extend(results.clone());
        results
    }

    /// 检查规则是否匹配
    fn rule_matches(&self, rule: &AggregationRule, event: &AuditEvent) -> bool {
        for condition in &rule.conditions {
            let field_value = self.get_field_value(&condition.field, event);

            let matches = match condition.operator {
                AuditOperator::Equals => field_value == condition.value,
                AuditOperator::NotEquals => field_value != condition.value,
                AuditOperator::Contains => field_value.contains(&condition.value),
                AuditOperator::NotContains => !field_value.contains(&condition.value),
                _ => true, // 简化处理
            };

            if !matches {
                return false;
            }
        }
        true
    }

    /// 获取字段值
    fn get_field_value(&self, field: &str, event: &AuditEvent) -> String {
        match field {
            "event_type" => format!("{:?}", event.event_type),
            "severity" => format!("{:?}", event.severity),
            "pid" => event.pid.to_string(),
            "uid" => event.uid.to_string(),
            "message" => event.message.clone(),
            _ => {
                event.data.get(field).cloned().unwrap_or_default()
            }
        }
    }

    /// 计算聚合值
    fn calculate_aggregation(&self, agg_type: &AggregationType, events: &[&AuditEvent], field: &str) -> f64 {
        match agg_type {
            AggregationType::Count => events.len() as f64,
            AggregationType::Sum => {
                events.iter()
                    .filter_map(|event| {
                        let value = self.get_field_value(field, event);
                        value.parse::<f64>().ok()
                    })
                    .sum()
            }
            AggregationType::Average => {
                let values: Vec<f64> = events.iter()
                    .filter_map(|event| {
                        let value = self.get_field_value(field, event);
                        value.parse::<f64>().ok()
                    })
                    .collect();

                if values.is_empty() {
                    0.0
                } else {
                    values.iter().sum::<f64>() / values.len() as f64
                }
            }
            AggregationType::Max => {
                events.iter()
                    .filter_map(|event| {
                        let value = self.get_field_value(field, event);
                        value.parse::<f64>().ok()
                    })
                    .fold(0.0, f64::max)
            }
            AggregationType::Min => {
                events.iter()
                    .filter_map(|event| {
                        let value = self.get_field_value(field, event);
                        value.parse::<f64>().ok()
                    })
                    .fold(f64::INFINITY, f64::min)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_processor_creation() {
        let processor = EventProcessor::new();
        assert_eq!(processor.id, 1);
        assert!(!processor.running);
        assert_eq!(processor.rules.len(), 0);
    }

    #[test]
    fn test_event_processor_stats() {
        let processor = EventProcessor::new();
        let stats = processor.get_stats();
        assert_eq!(stats.total_events, 0);
        assert_eq!(stats.rule_matches, 0);
        assert_eq!(stats.rule_executions, 0);
    }

    #[test]
    fn test_aggregation_rule_creation() {
        let rule = AggregationRule {
            id: 1,
            name: "Test Rule".to_string(),
            conditions: vec![],
            aggregation_type: AggregationType::Count,
            aggregation_field: "event_type".to_string(),
            threshold: 10,
        };

        assert_eq!(rule.id, 1);
        assert_eq!(rule.name, "Test Rule");
        assert_eq!(rule.aggregation_type, AggregationType::Count);
        assert_eq!(rule.threshold, 10);
    }

    #[test]
    fn test_event_aggregator() {
        let aggregator = EventAggregator::new(300); // 5 minutes
        assert_eq!(aggregator.window_size, 300);
        assert_eq!(aggregator.aggregation_rules.len(), 0);
        assert_eq!(aggregator.aggregated_events.len(), 0);
    }
}
