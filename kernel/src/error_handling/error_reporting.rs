// Error Reporting Module

extern crate alloc;
//
// 错误报告模块
// 提供错误报告生成、通知和导出功能

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;
use alloc::{format, vec};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use super::*;

/// 错误报告器
#[derive(Debug)]
pub struct ErrorReporter {
    /// 报告器ID
    pub id: u64,
    /// 报告模板
    report_templates: Vec<ReportTemplate>,
    /// 通知渠道
    notification_channels: Vec<NotificationChannel>,
    /// 报告历史
    report_history: Vec<ReportRecord>,
    /// 统计信息
    stats: ReportingStats,
    /// 配置
    config: ReportingConfig,
    /// 报告计数器
    report_counter: AtomicU64,
}

/// 报告模板
#[derive(Debug, Clone)]
pub struct ReportTemplate {
    /// 模板ID
    pub id: u64,
    /// 模板名称
    pub name: String,
    /// 模板描述
    pub description: String,
    /// 模板类型
    pub template_type: ReportType,
    /// 模板格式
    pub format: ReportFormat,
    /// 模板内容
    pub content: String,
    /// 变量定义
    pub variables: Vec<TemplateVariable>,
    /// 样式配置
    pub styling: StylingConfig,
    /// 是否启用
    pub enabled: bool,
    /// 创建时间
    pub created_at: u64,
    /// 使用次数
    pub usage_count: u64,
}

/// 报告类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReportType {
    /// 错误摘要报告
    ErrorSummary,
    /// 详细错误报告
    DetailedError,
    /// 趋势分析报告
    TrendAnalysis,
    /// 系统健康报告
    SystemHealth,
    /// 性能影响报告
    PerformanceImpact,
    /// 根因分析报告
    RootCauseAnalysis,
    /// 合规性报告
    Compliance,
    /// 自定义报告
    Custom,
}

/// 报告格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReportFormat {
    /// 文本格式
    Text,
    /// HTML格式
    HTML,
    /// JSON格式
    JSON,
    /// CSV格式
    CSV,
    /// XML格式
    XML,
    /// Markdown格式
    Markdown,
    /// PDF格式（占位符）
    PDF,
}

/// 模板变量
#[derive(Debug, Clone)]
pub struct TemplateVariable {
    /// 变量名
    pub name: String,
    /// 变量类型
    pub var_type: VariableType,
    /// 变量描述
    pub description: String,
    /// 默认值
    pub default_value: Option<String>,
    /// 是否必需
    pub required: bool,
}

/// 变量类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableType {
    /// 字符串
    String,
    /// 数字
    Number,
    /// 布尔值
    Boolean,
    /// 日期时间
    DateTime,
    /// 列表
    List,
    /// 对象
    Object,
}

/// 样式配置
#[derive(Debug, Clone)]
pub struct StylingConfig {
    /// 主题
    pub theme: String,
    /// 字体
    pub font: String,
    /// 颜色方案
    pub color_scheme: BTreeMap<String, String>,
    /// 图表配置
    pub chart_config: ChartConfig,
    /// 表格样式
    pub table_style: TableStyle,
}

/// 图表配置
#[derive(Debug, Clone)]
pub struct ChartConfig {
    /// 图表类型
    pub chart_type: ChartType,
    /// 颜色
    pub colors: Vec<String>,
    /// 标题
    pub title: Option<String>,
    /// X轴标签
    pub x_axis_label: Option<String>,
    /// Y轴标签
    pub y_axis_label: Option<String>,
}

/// 图表类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartType {
    /// 柱状图
    Bar,
    /// 折线图
    Line,
    /// 饼图
    Pie,
    /// 散点图
    Scatter,
    /// 热力图
    Heatmap,
}

/// 表格样式
#[derive(Debug, Clone)]
pub struct TableStyle {
    /// 边框样式
    pub border_style: String,
    /// 头部样式
    pub header_style: String,
    /// 行样式
    pub row_style: String,
    /// 斑马纹
    pub zebra_striping: bool,
}

/// 通知渠道
#[derive(Debug, Clone)]
pub struct NotificationChannel {
    /// 渠道ID
    pub id: u64,
    /// 渠道名称
    pub name: String,
    /// 渠道类型
    pub channel_type: ChannelType,
    /// 渠道配置
    pub config: ChannelConfig,
    /// 过滤规则
    pub filters: Vec<NotificationFilter>,
    /// 是否启用
    pub enabled: bool,
    /// 通知统计
    pub stats: ChannelStats,
}

/// 渠道类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelType {
    /// 邮件通知
    Email,
    /// 短信通知
    SMS,
    /// 即时消息
    InstantMessage,
    /// Webhook
    Webhook,
    /// 日志文件
    LogFile,
    /// 系统事件
    SystemEvent,
    /// 数据库记录
    Database,
}

/// 渠道配置
#[derive(Debug, Clone)]
pub struct ChannelConfig {
    /// 端点地址
    pub endpoint: String,
    /// 认证信息
    pub authentication: Option<AuthenticationConfig>,
    /// 连接参数
    pub connection_params: BTreeMap<String, String>,
    /// 重试配置
    pub retry_config: RetryConfig,
    /// 超时配置
    pub timeout_config: TimeoutConfig,
}

/// 认证配置
#[derive(Debug, Clone)]
pub struct AuthenticationConfig {
    /// 认证类型
    pub auth_type: AuthType,
    /// 用户名
    pub username: Option<String>,
    /// 密码或令牌
    pub secret: Option<String>,
    /// API密钥
    pub api_key: Option<String>,
    /// 证书路径
    pub certificate_path: Option<String>,
}

/// 认证类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthType {
    /// 无认证
    None,
    /// 基本认证
    Basic,
    /// API密钥
    APIKey,
    /// OAuth
    OAuth,
    /// 证书认证
    Certificate,
}

/// 重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔（毫秒）
    pub retry_interval_ms: u64,
    /// 指数退避
    pub exponential_backoff: bool,
}

/// 超时配置
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// 连接超时（毫秒）
    pub connect_timeout_ms: u64,
    /// 读取超时（毫秒）
    pub read_timeout_ms: u64,
    /// 总超时（毫秒）
    pub total_timeout_ms: u64,
}

/// 通知过滤器
#[derive(Debug, Clone)]
pub struct NotificationFilter {
    /// 过滤器ID
    pub id: u64,
    /// 过滤器名称
    pub name: String,
    /// 过滤条件
    pub conditions: Vec<FilterCondition>,
    /// 过滤动作
    pub action: FilterAction,
    /// 优先级
    pub priority: u32,
}

/// 过滤条件
#[derive(Debug, Clone)]
pub enum FilterCondition {
    /// 严重级别过滤
    SeverityFilter(ErrorSeverity),
    /// 错误类别过滤
    CategoryFilter(ErrorCategory),
    /// 时间窗口过滤
    TimeWindowFilter(u64, u64),
    /// 频率过滤
    FrequencyFilter(u32, u64),
    /// 自定义过滤
    CustomFilter(String),
}

/// 过滤动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterAction {
    /// 允许通过
    Allow,
    /// 阻止通知
    Block,
    /// 降级处理
    Downgrade,
    /// 延迟发送
    Delay,
}

/// 渠道统计
#[derive(Debug, Clone, Default)]
pub struct ChannelStats {
    /// 发送总数
    pub total_sent: u64,
    /// 成功数
    pub success_count: u64,
    /// 失败数
    pub failure_count: u64,
    /// 平均发送时间（毫秒）
    pub avg_send_time_ms: u64,
    /// 最后发送时间
    pub last_sent: u64,
}

/// 报告记录
#[derive(Debug, Clone)]
pub struct ReportRecord {
    /// 报告ID
    pub id: u64,
    /// 报告类型
    pub report_type: ReportType,
    /// 报告格式
    pub format: ReportFormat,
    /// 报告标题
    pub title: String,
    /// 报告内容
    pub content: String,
    /// 生成时间
    pub generated_at: u64,
    /// 生成耗时（毫秒）
    pub generation_time_ms: u64,
    /// 文件大小（字节）
    pub file_size: u64,
    /// 包含错误数
    pub error_count: u64,
    /// 发送状态
    pub delivery_status: Vec<DeliveryStatus>,
}

/// 发送状态
#[derive(Debug, Clone)]
pub struct DeliveryStatus {
    /// 渠道ID
    pub channel_id: u64,
    /// 渠道名称
    pub channel_name: String,
    /// 发送时间
    pub sent_at: u64,
    /// 发送状态
    pub status: DeliveryStatusType,
    /// 状态消息
    pub message: String,
    /// 重试次数
    pub retry_count: u32,
}

/// 发送状态类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryStatusType {
    /// 等待发送
    Pending,
    /// 发送中
    Sending,
    /// 发送成功
    Success,
    /// 发送失败
    Failed,
    /// 已取消
    Cancelled,
}

/// 报告统计
#[derive(Debug, Clone, Default)]
pub struct ReportingStats {
    /// 总报告数
    pub total_reports: u64,
    /// 按类型统计
    pub reports_by_type: BTreeMap<ReportType, u64>,
    /// 按格式统计
    pub reports_by_format: BTreeMap<ReportFormat, u64>,
    /// 总通知数
    pub total_notifications: u64,
    /// 通知成功率
    pub notification_success_rate: f64,
    /// 平均生成时间（毫秒）
    pub avg_generation_time_ms: u64,
    /// 总数据量（字节）
    pub total_data_volume: u64,
}

/// 报告配置
#[derive(Debug, Clone)]
pub struct ReportingConfig {
    /// 启用自动报告
    pub enable_auto_reporting: bool,
    /// 报告间隔（秒）
    pub reporting_interval_seconds: u64,
    /// 最大报告历史
    pub max_report_history: usize,
    /// 启用通知
    pub enable_notifications: bool,
    /// 通知重试次数
    pub notification_retry_count: u32,
    /// 压缩报告
    pub compress_reports: bool,
    /// 最大报告大小（字节）
    pub max_report_size: u64,
    /// 异步发送
    pub async_delivery: bool,
}

impl Default for ReportingConfig {
    fn default() -> Self {
        Self {
            enable_auto_reporting: true,
            reporting_interval_seconds: 3600, // 1小时
            max_report_history: 100,
            enable_notifications: true,
            notification_retry_count: 3,
            compress_reports: false,
            max_report_size: 10 * 1024 * 1024, // 10MB
            async_delivery: true,
        }
    }
}

impl Clone for ErrorReporter {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            report_templates: self.report_templates.clone(),
            notification_channels: self.notification_channels.clone(),
            report_history: self.report_history.clone(),
            stats: self.stats.clone(),
            config: self.config.clone(),
            report_counter: AtomicU64::new(self.report_counter.load(core::sync::atomic::Ordering::Relaxed)),
        }
    }
}

impl ErrorReporter {
    /// 创建新的错误报告器
    pub fn new() -> Self {
        Self {
            id: 1,
            report_templates: Vec::new(),
            notification_channels: Vec::new(),
            report_history: Vec::new(),
            stats: ReportingStats::default(),
            config: ReportingConfig::default(),
            report_counter: AtomicU64::new(1),
        }
    }

    /// 初始化错误报告器
    pub fn init(&mut self) -> Result<(), &'static str> {
        // 加载默认报告模板
        self.load_default_templates()?;

        // 初始化默认通知渠道
        self.initialize_notification_channels()?;

        crate::println!("[ErrorReporter] Error reporter initialized successfully");
        Ok(())
    }

    /// 报告错误
    pub fn report_error(&mut self, error_record: &ErrorRecord) -> Result<(), &'static str> {
        // 生成即时错误报告
        self.generate_immediate_report(error_record)?;

        // 发送通知
        if self.config.enable_notifications {
            self.send_notifications(error_record)?;
        }

        Ok(())
    }

    /// 生成即时报告
    fn generate_immediate_report(&mut self, error_record: &ErrorRecord) -> Result<(), &'static str> {
        let report_id = self.report_counter.fetch_add(1, Ordering::SeqCst);
        let start_time = crate::time::get_timestamp();

        // 使用默认模板生成报告
        let default_template = self.create_default_error_template();
        let template = self.get_template_by_type(ReportType::DetailedError)
            .unwrap_or(&default_template);

        let content = self.render_template(&template, error_record)?;
        let generation_time = crate::time::get_timestamp() - start_time;

        let record = ReportRecord {
            id: report_id,
            report_type: ReportType::DetailedError,
            format: ReportFormat::Text,
            title: format!("Error Report: {}", error_record.message),
            content,
            generated_at: crate::time::get_timestamp(),
            generation_time_ms: generation_time * 1000,
            file_size: 0, // 将在渲染后计算
            error_count: 1,
            delivery_status: Vec::new(),
        };

        // 保存报告记录
        self.save_report_record(record)?;

        // 更新统计信息
        self.update_reporting_stats(ReportType::DetailedError, ReportFormat::Text, generation_time);

        Ok(())
    }

    /// 生成报告
    pub fn generate_report(&mut self, error_records: &[ErrorRecord], time_range: Option<(u64, u64)>) -> Result<String, &'static str> {
        let default_template = self.create_default_summary_template();
        let template = self.get_template_by_type(ReportType::ErrorSummary)
            .unwrap_or(&default_template);

        // 过滤错误记录
        let filtered_records = if let Some((start, end)) = time_range {
            error_records
                .iter()
                .filter(|record| record.timestamp >= start && record.timestamp <= end)
                .cloned()
                .collect::<Vec<_>>()
        } else {
            error_records.to_vec()
        };

        self.render_summary_template(&template, &filtered_records)
    }

    /// 渲染模板
    fn render_template(&self, template: &ReportTemplate, error_record: &ErrorRecord) -> Result<String, &'static str> {
        let mut content = template.content.clone();

        // 替换模板变量
        content = content.replace("${{error.id}}", &error_record.id.to_string());
        content = content.replace("${{error.code}}", &error_record.code.to_string());
        content = content.replace("${{error.message}}", &error_record.message);
        content = content.replace("${{error.description}}", &error_record.description);
        content = content.replace("${{error.severity}}", &format!("{:?}", error_record.severity));
        content = content.replace("${{error.category}}", &format!("{:?}", error_record.category));
        content = content.replace("${{error.timestamp}}", &error_record.timestamp.to_string());
        content = content.replace("${{error.source.module}}", &error_record.source.module);
        content = content.replace("${{error.source.function}}", &error_record.source.function);
        content = content.replace("${{error.source.file}}", &error_record.source.file);
        content = content.replace("${{error.source.line}}", &error_record.source.line.to_string());

        Ok(content)
    }

    /// 渲染摘要模板
    fn render_summary_template(&self, template: &ReportTemplate, error_records: &[ErrorRecord]) -> Result<String, &'static str> {
        let mut content = template.content.clone();

        // 统计信息
        let total_errors = error_records.len();
        let errors_by_severity = self.count_errors_by_severity(error_records);
        let errors_by_category = self.count_errors_by_category(error_records);

        content = content.replace("${{summary.total_errors}}", &total_errors.to_string());
        content = content.replace("${{summary.critical_count}}", &errors_by_severity.get(&ErrorSeverity::Critical).unwrap_or(&0).to_string());
        content = content.replace("${{summary.error_count}}", &errors_by_severity.get(&ErrorSeverity::Error).unwrap_or(&0).to_string());
        content = content.replace("${{summary.warning_count}}", &errors_by_severity.get(&ErrorSeverity::Warning).unwrap_or(&0).to_string());

        // 添加错误详情
        if !error_records.is_empty() {
            let mut error_details = String::new();
            for record in error_records.iter().take(10) { // 限制显示前10个错误
                error_details.push_str(&format!(
                    "[{}] {:?} - {}\n",
                    record.timestamp,
                    record.severity,
                    record.message
                ));
            }
            content = content.replace("${{summary.error_details}}", &error_details);
        }

        Ok(content)
    }

    /// 按严重级别统计错误
    fn count_errors_by_severity(&self, error_records: &[ErrorRecord]) -> BTreeMap<ErrorSeverity, u64> {
        let mut counts = BTreeMap::new();
        for record in error_records {
            *counts.entry(record.severity).or_insert(0) += 1;
        }
        counts
    }

    /// 按类别统计错误
    fn count_errors_by_category(&self, error_records: &[ErrorRecord]) -> BTreeMap<ErrorCategory, u64> {
        let mut counts = BTreeMap::new();
        for record in error_records {
            *counts.entry(record.category).or_insert(0) += 1;
        }
        counts
    }

    /// 发送通知
    fn send_notifications(&mut self, error_record: &ErrorRecord) -> Result<(), &'static str> {
        let error_record_clone = error_record.clone();

        for channel in &mut self.notification_channels {
            if !channel.enabled {
                continue;
            }

            // 检查过滤条件 - 使用独立函数避免借用冲突
            if !Self::should_notify_static(channel, &error_record_clone) {
                continue;
            }

            // 发送通知
            let result = Self::send_notification_static(channel, &error_record_clone);
            match result {
                Ok(_) => {
                    channel.stats.success_count += 1;
                    channel.stats.last_sent = crate::time::get_timestamp();
                }
                Err(e) => {
                    channel.stats.failure_count += 1;
                    crate::println!("[ErrorReporter] Failed to send notification via channel {}: {}", channel.name, e);
                }
            }

            channel.stats.total_sent += 1;
        }

        Ok(())
    }

    /// 判断是否应该发送通知
    fn should_notify(&self, channel: &NotificationChannel, error_record: &ErrorRecord) -> bool {
        for filter in &channel.filters {
            match &filter.action {
                FilterAction::Block => {
                    if self.matches_filter_condition(&filter.conditions, error_record) {
                        return false;
                    }
                }
                FilterAction::Allow => {
                    if !self.matches_filter_condition(&filter.conditions, error_record) {
                        return false;
                    }
                }
                _ => {}
            }
        }
        true
    }

    /// 匹配过滤条件
    fn matches_filter_condition(&self, conditions: &[FilterCondition], error_record: &ErrorRecord) -> bool {
        for condition in conditions {
            match condition {
                FilterCondition::SeverityFilter(severity) => {
                    if error_record.severity != *severity {
                        return false;
                    }
                }
                FilterCondition::CategoryFilter(category) => {
                    if error_record.category != *category {
                        return false;
                    }
                }
                FilterCondition::TimeWindowFilter(start, end) => {
                    if error_record.timestamp < *start || error_record.timestamp > *end {
                        return false;
                    }
                }
                FilterCondition::FrequencyFilter(min_freq, window) => {
                    if error_record.occurrence_count < *min_freq {
                        return false;
                    }
                }
                FilterCondition::CustomFilter(_condition) => {
                    // 实现自定义过滤逻辑
                }
            }
        }
        true
    }

    /// 发送通知
    fn send_notification(&self, channel: &NotificationChannel, error_record: &ErrorRecord) -> Result<(), &'static str> {
        let message = format!(
            "Error Alert: [{:?}] {} - {} in {} at {}:{}",
            error_record.severity,
            error_record.code,
            error_record.message,
            error_record.source.module,
            error_record.source.file,
            error_record.source.line
        );

        match channel.channel_type {
            ChannelType::LogFile => {
                // 写入日志文件
                crate::println!("[LogFile] {}", message);
            }
            ChannelType::SystemEvent => {
                // 发送系统事件
                crate::println!("[SystemEvent] {}", message);
            }
            ChannelType::Email => {
                // 发送邮件（模拟）
                crate::println!("[Email] To: {}: {}", channel.config.endpoint, message);
            }
            ChannelType::SMS => {
                // 发送短信（模拟）
                crate::println!("[SMS] To: {}: {}", channel.config.endpoint, message);
            }
            ChannelType::Webhook => {
                // 发送Webhook（模拟）
                crate::println!("[Webhook] {}: {}", channel.config.endpoint, message);
            }
            ChannelType::InstantMessage => {
                // 发送即时消息（模拟）
                crate::println!("[IM] {}: {}", channel.config.endpoint, message);
            }
            ChannelType::Database => {
                // 记录到数据库（模拟）
                crate::println!("[Database] Record: {}", message);
            }
        }

        Ok(())
    }

    /// 获取模板
    fn get_template_by_type(&self, report_type: ReportType) -> Option<&ReportTemplate> {
        self.report_templates.iter().find(|t| t.template_type == report_type && t.enabled)
    }

    /// 创建默认错误模板
    fn create_default_error_template(&self) -> ReportTemplate {
        ReportTemplate {
            id: 0,
            name: "Default Error Template".to_string(),
            description: "Default template for error reports".to_string(),
            template_type: ReportType::DetailedError,
            format: ReportFormat::Text,
            content: r#"
Error Report
============

Error ID: ${{error.id}}
Error Code: ${{error.code}}
Timestamp: ${{error.timestamp}}
Severity: ${{error.severity}}
Category: $${error.category}

Message: ${{error.message}}
Description: ${{error.description}}

Source Information:
- Module: ${{error.source.module}}
- Function: ${{error.source.function}}
- File: ${{error.source.file}}
- Line: ${{error.source.line}}
"#.to_string(),
            variables: vec![
                TemplateVariable {
                    name: "error.id".to_string(),
                    var_type: VariableType::Number,
                    description: "Error ID".to_string(),
                    default_value: None,
                    required: true,
                },
            ],
            styling: StylingConfig {
                theme: "default".to_string(),
                font: "monospace".to_string(),
                color_scheme: BTreeMap::new(),
                chart_config: ChartConfig {
                    chart_type: ChartType::Bar,
                    colors: vec!["#007bff".to_string()],
                    title: None,
                    x_axis_label: None,
                    y_axis_label: None,
                },
                table_style: TableStyle {
                    border_style: "solid".to_string(),
                    header_style: "bold".to_string(),
                    row_style: "normal".to_string(),
                    zebra_striping: true,
                },
            },
            enabled: true,
            created_at: crate::time::get_timestamp(),
            usage_count: 0,
        }
    }

    /// 创建默认摘要模板
    fn create_default_summary_template(&self) -> ReportTemplate {
        ReportTemplate {
            id: 0,
            name: "Default Summary Template".to_string(),
            description: "Default template for summary reports".to_string(),
            template_type: ReportType::ErrorSummary,
            format: ReportFormat::Text,
            content: r#"
Error Summary Report
===================

Report Period: [Time Range]
Total Errors: ${{summary.total_errors}}

Error Distribution by Severity:
- Critical: ${{summary.critical_count}}
- Error: ${{summary.error_count}}
- Warning: ${{summary.warning_count}}
- Info: [Info Count]

Recent Errors:
${{summary.error_details}}
"#.to_string(),
            variables: vec![
                TemplateVariable {
                    name: "summary.total_errors".to_string(),
                    var_type: VariableType::Number,
                    description: "Total number of errors".to_string(),
                    default_value: Some("0".to_string()),
                    required: true,
                },
            ],
            styling: StylingConfig {
                theme: "default".to_string(),
                font: "monospace".to_string(),
                color_scheme: BTreeMap::new(),
                chart_config: ChartConfig {
                    chart_type: ChartType::Bar,
                    colors: vec!["#dc3545".to_string()],
                    title: Some("Error Distribution".to_string()),
                    x_axis_label: Some("Severity".to_string()),
                    y_axis_label: Some("Count".to_string()),
                },
                table_style: TableStyle {
                    border_style: "solid".to_string(),
                    header_style: "bold".to_string(),
                    row_style: "normal".to_string(),
                    zebra_striping: true,
                },
            },
            enabled: true,
            created_at: crate::time::get_timestamp(),
            usage_count: 0,
        }
    }

    /// 保存报告记录
    fn save_report_record(&mut self, record: ReportRecord) -> Result<(), &'static str> {
        self.report_history.push(record);

        // 限制历史记录数量
        if self.report_history.len() > self.config.max_report_history {
            self.report_history.remove(0);
        }

        Ok(())
    }

    /// 更新报告统计
    fn update_reporting_stats(&mut self, report_type: ReportType, format: ReportFormat, generation_time: u64) {
        self.stats.total_reports += 1;
        *self.stats.reports_by_type.entry(report_type).or_insert(0) += 1;
        *self.stats.reports_by_format.entry(format).or_insert(0) += 1;

        // 更新平均生成时间
        let total_time = self.stats.avg_generation_time_ms * (self.stats.total_reports - 1) + generation_time;
        self.stats.avg_generation_time_ms = total_time / self.stats.total_reports;
    }

    /// 加载默认模板
    fn load_default_templates(&mut self) -> Result<(), &'static str> {
        let templates = vec![
            self.create_default_error_template(),
            self.create_default_summary_template(),
        ];

        self.report_templates = templates;
        Ok(())
    }

    /// 初始化通知渠道
    fn initialize_notification_channels(&mut self) -> Result<(), &'static str> {
        let channels = vec![
            NotificationChannel {
                id: 1,
                name: "System Log".to_string(),
                channel_type: ChannelType::LogFile,
                config: ChannelConfig {
                    endpoint: "/var/log/errors.log".to_string(),
                    authentication: None,
                    connection_params: BTreeMap::new(),
                    retry_config: RetryConfig {
                        max_retries: 3,
                        retry_interval_ms: 1000,
                        exponential_backoff: true,
                    },
                    timeout_config: TimeoutConfig {
                        connect_timeout_ms: 5000,
                        read_timeout_ms: 10000,
                        total_timeout_ms: 30000,
                    },
                },
                filters: vec![
                    NotificationFilter {
                        id: 1,
                        name: "Critical Errors Only".to_string(),
                        conditions: vec![
                            FilterCondition::SeverityFilter(ErrorSeverity::Critical),
                        ],
                        action: FilterAction::Allow,
                        priority: 1,
                    },
                ],
                enabled: true,
                stats: ChannelStats::default(),
            },
            NotificationChannel {
                id: 2,
                name: "System Events".to_string(),
                channel_type: ChannelType::SystemEvent,
                config: ChannelConfig {
                    endpoint: "system://events".to_string(),
                    authentication: None,
                    connection_params: BTreeMap::new(),
                    retry_config: RetryConfig {
                        max_retries: 1,
                        retry_interval_ms: 500,
                        exponential_backoff: false,
                    },
                    timeout_config: TimeoutConfig {
                        connect_timeout_ms: 1000,
                        read_timeout_ms: 2000,
                        total_timeout_ms: 5000,
                    },
                },
                filters: vec![
                    NotificationFilter {
                        id: 2,
                        name: "Error and Critical".to_string(),
                        conditions: vec![
                            FilterCondition::SeverityFilter(ErrorSeverity::Error),
                            FilterCondition::SeverityFilter(ErrorSeverity::Critical),
                        ],
                        action: FilterAction::Allow,
                        priority: 2,
                    },
                ],
                enabled: true,
                stats: ChannelStats::default(),
            },
        ];

        self.notification_channels = channels;
        Ok(())
    }

    /// 获取报告历史
    pub fn get_report_history(&self, limit: Option<usize>) -> Vec<ReportRecord> {
        let mut history = self.report_history.clone();
        history.sort_by(|a, b| b.generated_at.cmp(&a.generated_at));

        if let Some(limit) = limit {
            history.truncate(limit);
        }

        history
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> ReportingStats {
        self.stats.clone()
    }

    /// 更新配置
    pub fn update_config(&mut self, config: ReportingConfig) -> Result<(), &'static str> {
        self.config = config;
        Ok(())
    }

    /// 添加报告模板
    pub fn add_template(&mut self, template: ReportTemplate) -> Result<(), &'static str> {
        self.report_templates.push(template);
        Ok(())
    }

    /// 添加通知渠道
    pub fn add_notification_channel(&mut self, channel: NotificationChannel) -> Result<(), &'static str> {
        self.notification_channels.push(channel);
        Ok(())
    }

    /// 停止错误报告器
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        self.report_templates.clear();
        self.notification_channels.clear();
        self.report_history.clear();

        crate::println!("[ErrorReporter] Error reporter shutdown successfully");
        Ok(())
    }

    /// 静态方法检查是否应该通知
    fn should_notify_static(channel: &NotificationChannel, error_record: &ErrorRecord) -> bool {
        // 简化的通知逻辑
        match error_record.severity {
            ErrorSeverity::Critical | ErrorSeverity::Fatal => true,
            ErrorSeverity::High => error_record.occurrence_count % 10 == 0,
            ErrorSeverity::Medium => error_record.occurrence_count % 50 == 0,
            ErrorSeverity::Low => error_record.occurrence_count % 100 == 0,
            ErrorSeverity::Info => false,
            ErrorSeverity::Warning => error_record.occurrence_count % 200 == 0,
            ErrorSeverity::Error => error_record.occurrence_count % 25 == 0,
        }
    }

    /// 静态方法发送通知
    fn send_notification_static(channel: &mut NotificationChannel, error_record: &ErrorRecord) -> Result<(), &'static str> {
        // 简化的通知发送逻辑
        channel.stats.total_sent += 1;

        // 模拟发送通知
        crate::println!("[Notification] Sending error report via {:?}: {:?}",
                   channel.channel_type, error_record.error_type);

        Ok(())
    }
}

/// 创建默认的错误报告器
pub fn create_error_reporter() -> Arc<Mutex<ErrorReporter>> {
    Arc::new(Mutex::new(ErrorReporter::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_reporter_creation() {
        let reporter = ErrorReporter::new();
        assert_eq!(reporter.id, 1);
        assert!(reporter.report_templates.is_empty());
        assert!(reporter.notification_channels.is_empty());
    }

    #[test]
    fn test_template_rendering() {
        let reporter = ErrorReporter::new();
        let template = reporter.create_default_error_template();

        let error_record = ErrorRecord {
            id: 1,
            code: 1001,
            error_type: ErrorType::RuntimeError,
            category: ErrorCategory::System,
            severity: ErrorSeverity::Error,
            status: ErrorStatus::New,
            message: "Test error".to_string(),
            description: "Test description".to_string(),
            source: ErrorSource {
                module: "test".to_string(),
                function: "test_func".to_string(),
                file: "test.rs".to_string(),
                line: 10,
                column: 5,
                process_id: 1,
                thread_id: 1,
                cpu_id: 0,
            },
            timestamp: 1234567890,
            context: ErrorContext {
                environment_variables: BTreeMap::new(),
                system_config: BTreeMap::new(),
                user_input: None,
                related_data: Vec::new(),
                operation_sequence: Vec::new(),
                preconditions: Vec::new(),
                postconditions: Vec::new(),
            },
            stack_trace: Vec::new(),
            system_state: SystemStateSnapshot {
                memory_usage: MemoryUsage {
                    total_memory: 0,
                    used_memory: 0,
                    available_memory: 0,
                    cached_memory: 0,
                    swap_used: 0,
                    kernel_memory: 0,
                },
                cpu_usage: CpuUsage {
                    usage_percent: 0.0,
                    user_percent: 0.0,
                    system_percent: 0.0,
                    idle_percent: 0.0,
                    wait_percent: 0.0,
                    interrupt_percent: 0.0,
                },
                process_states: Vec::new(),
                network_state: NetworkState {
                    active_connections: 0,
                    listening_ports: 0,
                    interfaces: Vec::new(),
                    packet_stats: PacketStats {
                        total_rx: 0,
                        total_tx: 0,
                        dropped: 0,
                        errors: 0,
                    },
                },
                filesystem_state: FileSystemState {
                    mount_points: Vec::new(),
                    disk_usage: Vec::new(),
                    io_stats: IoStats {
                        read_operations: 0,
                        write_operations: 0,
                        read_bytes: 0,
                        write_bytes: 0,
                        io_wait_time: 0,
                    },
                },
                device_states: Vec::new(),
                system_load: SystemLoad {
                    load_1min: 0.0,
                    load_5min: 0.0,
                    load_15min: 0.0,
                    run_queue_length: 0,
                    blocked_processes: 0,
                },
                timestamp: 0,
            },
            recovery_actions: Vec::new(),
            occurrence_count: 1,
            last_occurrence: 1234567890,
            resolved: false,
            resolution_time: None,
            resolution_method: None,
            metadata: BTreeMap::new(),
        };

        let result = reporter.render_template(&template, &error_record).unwrap();
        assert!(result.contains("1"));
        assert!(result.contains("1001"));
        assert!(result.contains("Test error"));
        assert!(result.contains("test"));
    }

    #[test]
    fn test_reporting_config_default() {
        let config = ReportingConfig::default();
        assert!(config.enable_auto_reporting);
        assert_eq!(config.reporting_interval_seconds, 3600);
        assert!(config.enable_notifications);
    }
}
