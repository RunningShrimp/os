// 性能监控可视化模块
//
// 提供实时性能指标的可视化显示和交互式分析功能。
//
// 主要功能：
// - 实时性能指标显示
// - 历史数据可视化
// - 图表和图形表示
// - 交互式界面
// - 与现有性能监控框架集成
//
// 设计为no_std环境，支持嵌入式系统和内核级应用。

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{string::String, format};
use core::sync::atomic::{AtomicU64, Ordering};
use core::time::Duration;
use spin::Mutex;

use crate::time;
use super::{metrics, profiling, monitoring};

// Import println macro
#[allow(unused_imports)]
use crate::println;

/// 可视化图表类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartType {
    /// 折线图
    Line,
    /// 柱状图
    Bar,
    /// 饼图
    Pie,
    /// 仪表盘
    Gauge,
    /// 热力图
    Heatmap,
    /// 散点图
    Scatter,
}

/// 图表配置
#[derive(Debug, Clone)]
pub struct ChartConfig {
    /// 图表类型
    pub chart_type: ChartType,
    /// 图表标题
    pub title: String,
    /// X轴标签
    pub x_label: String,
    /// Y轴标签
    pub y_label: String,
    /// 是否显示网格
    pub show_grid: bool,
    /// 是否显示图例
    pub show_legend: bool,
    /// 数据采样间隔
    pub sample_interval: Duration,
    /// 数据保留数量
    pub max_data_points: usize,
    /// 颜色主题
    pub color_theme: ColorTheme,
}

/// 颜色主题
#[derive(Debug, Clone, Copy)]
pub enum ColorTheme {
    /// 亮色主题
    Light,
    /// 暗色主题
    Dark,
    /// 彩色主题
    Colorful,
}

impl Default for ColorTheme {
    fn default() -> Self {
        ColorTheme::Dark
    }
}

/// 可视化数据系列
#[derive(Debug, Clone)]
pub struct DataSeries {
    /// 系列名称
    pub name: String,
    /// 数据点
    pub data: Vec<(u64, f64)>, // (timestamp, value)
    /// 颜色
    pub color: String,
}

/// 可视化面板
#[derive(Debug, Clone)]
pub struct VisualizationPanel {
    /// 面板ID
    pub id: String,
    /// 面板标题
    pub title: String,
    /// 图表配置
    pub chart_config: ChartConfig,
    /// 数据系列
    pub data_series: Vec<DataSeries>,
    /// 面板类型
    pub panel_type: PanelType,
    /// 更新时间
    pub last_updated: u64,
}

/// 面板类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelType {
    /// 系统指标面板
    SystemMetrics,
    /// 性能分析面板
    Profiling,
    /// 监控事件面板
    MonitoringEvents,
    /// 热点分析面板
    HotspotAnalysis,
}

/// 可视化视图
#[derive(Debug, Clone)]
pub struct VisualizationView {
    /// 视图ID
    pub id: String,
    /// 视图名称
    pub name: String,
    /// 视图面板
    pub panels: Vec<VisualizationPanel>,
    /// 视图布局
    pub layout: Layout,
    /// 是否自动更新
    pub auto_update: bool,
}

/// 视图布局
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// 单列
    SingleColumn,
    /// 双列
    DoubleColumn,
    /// 三列
    TripleColumn,
    /// 自定义布局
    Custom,
}

/// 可视化引擎
pub struct VisualizationEngine {
    /// 可视化视图
    views: Arc<Mutex<BTreeMap<String, VisualizationView>>>,
    /// 配置
    config: VisualizationConfig,
    /// 统计信息
    statistics: VisualizationStatistics,
    /// 是否已启动
    running: core::sync::atomic::AtomicBool,
}

/// 可视化配置
#[derive(Debug, Clone)]
pub struct VisualizationConfig {
    /// 默认图表配置
    pub default_chart_config: ChartConfig,
    /// 自动更新间隔
    pub auto_update_interval: Duration,
    /// 最大同时视图数
    pub max_views: usize,
    /// 最大同时面板数
    pub max_panels: usize,
}

impl Default for VisualizationConfig {
    fn default() -> Self {
        Self {
            default_chart_config: ChartConfig {
                chart_type: ChartType::Line,
                title: "Performance Metrics".to_string(),
                x_label: "Time".to_string(),
                y_label: "Value".to_string(),
                show_grid: true,
                show_legend: true,
                sample_interval: Duration::from_secs(1),
                max_data_points: 100,
                color_theme: ColorTheme::default(),
            },
            auto_update_interval: Duration::from_millis(500),
            max_views: 10,
            max_panels: 20,
        }
    }
}

/// 可视化统计信息
#[derive(Debug, Default)]
pub struct VisualizationStatistics {
    /// 视图总数
    pub total_views: AtomicU64,
    /// 面板总数
    pub total_panels: AtomicU64,
    /// 数据更新总数
    pub total_updates: AtomicU64,
    /// 当前活跃视图数
    pub active_views: AtomicU64,
    /// 当前活跃面板数
    pub active_panels: AtomicU64,
}

impl VisualizationEngine {
    /// 创建新的可视化引擎
    pub fn new(config: VisualizationConfig) -> Self {
        Self {
            views: Arc::new(Mutex::new(BTreeMap::new())),
            config,
            statistics: VisualizationStatistics::default(),
            running: core::sync::atomic::AtomicBool::new(false),
        }
    }

    /// 启动可视化引擎
    pub fn start(&self) -> Result<(), VisualizationError> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);
        crate::println!("[visualization] 可视化引擎已启动");

        Ok(())
    }

    /// 停止可视化引擎
    pub fn stop(&self) -> Result<(), VisualizationError> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.running.store(false, Ordering::SeqCst);
        crate::println!("[visualization] 可视化引擎已停止");

        Ok(())
    }

    /// 创建新的可视化视图
    pub fn create_view(&self, view_id: String, name: String, layout: Layout) -> Result<VisualizationView, VisualizationError> {
        let mut views = self.views.lock();

        if views.contains_key(&view_id) {
            return Err(VisualizationError::ViewAlreadyExists(view_id));
        }

        if views.len() >= self.config.max_views {
            return Err(VisualizationError::TooManyViews);
        }

        let view = VisualizationView {
            id: view_id.clone(),
            name,
            panels: Vec::new(),
            layout,
            auto_update: true,
        };

        views.insert(view_id, view.clone());
        self.statistics.total_views.fetch_add(1, Ordering::SeqCst);
        self.statistics.active_views.store(views.len() as u64, Ordering::SeqCst);

        Ok(view)
    }

    /// 添加面板到视图
    pub fn add_panel_to_view(&self, view_id: &str, panel: VisualizationPanel) -> Result<(), VisualizationError> {
        let mut views = self.views.lock();

        if let Some(view) = views.get_mut(view_id) {
            if view.panels.len() >= self.config.max_panels {
                return Err(VisualizationError::TooManyPanels);
            }

            view.panels.push(panel.clone());
            self.statistics.total_panels.fetch_add(1, Ordering::SeqCst);
            self.statistics.active_panels.store(view.panels.len() as u64, Ordering::SeqCst);

            Ok(())
        } else {
            Err(VisualizationError::ViewNotFound(view_id.to_string()))
        }
    }

    /// 更新实时性能指标
    pub fn update_realtime_metrics(&self, metric_name: &str, timestamp: u64, value: f64) -> Result<(), VisualizationError> {
        // 更新所有相关面板
        let mut views = self.views.lock();

        for view in views.values_mut() {
            for panel in &mut view.panels {
                // 仅更新系统指标面板和性能分析面板
                if panel.panel_type == PanelType::SystemMetrics || panel.panel_type == PanelType::Profiling {
                    // 查找或创建数据系列
                    let series_index = panel.data_series.iter().position(|s| s.name == metric_name);
                    
                    match series_index {
                        Some(index) => {
                            // 更新现有数据系列
                            let series = &mut panel.data_series[index];
                            series.data.push((timestamp, value));
                            
                            // 限制数据点数量
                            if series.data.len() > panel.chart_config.max_data_points {
                                series.data.remove(0);
                            }
                        }
                        None => {
                            // 创建新的数据系列
                            let new_series = DataSeries {
                                name: metric_name.to_string(),
                                data: vec![(timestamp, value)],
                                color: self.generate_color(panel.data_series.len()),
                            };
                            panel.data_series.push(new_series);
                        }
                    }
                    
                    panel.last_updated = timestamp;
                }
            }
        }

        self.statistics.total_updates.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }

    /// 辅助函数：生成系列颜色
    fn generate_color(&self, index: usize) -> String {
        let colors = [
            "#FF5733", "#33FF57", "#3357FF", "#F333FF", "#FF33A1",
            "#FFC300", "#C70039", "#900C3F", "#581845", "#1ABC9C"
        ];
        colors[index % colors.len()].to_string()
    }
    
    /// 从指标系统获取实时数据
    pub fn update_from_metrics(&self) -> Result<(), VisualizationError> {
        if let Ok(registry) = metrics::get_metric_registry() {
            if let Ok(metric_instances) = registry.get_all_metrics() {
                for instance in metric_instances {
                    let timestamp = instance.last_updated;
                    
                    match &instance.value {
                        metrics::MetricValue::Counter(value) => {
                            let metric_name = &instance.definition.name;
                            self.update_realtime_metrics(metric_name, timestamp, *value as f64)?;
                        }
                        metrics::MetricValue::Gauge(value) => {
                            let metric_name = &instance.definition.name;
                            self.update_realtime_metrics(metric_name, timestamp, *value)?;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    /// 从分析系统获取历史数据
    pub fn update_from_profiling(&self, profiler_id: &str) -> Result<(), VisualizationError> {
        let mut engine = profiling::get_performance_engine();
        if let Some(perf_engine) = &mut *engine {
            if let Ok(profiler) = perf_engine.get_profiler(profiler_id) {
                if let Ok(result) = profiler.get_analysis_result() {
                    // 处理性能分析结果
                    let timestamp = time::timestamp_millis();
                    
                    // 提取性能指标
                    self.update_realtime_metrics(
                        "cpu_usage_percentage",
                        timestamp,
                        result.performance_metrics.cpu_usage_percentage
                    )?;
                    
                    self.update_realtime_metrics(
                        "memory_peak_bytes",
                        timestamp,
                        result.performance_metrics.memory_peak as f64
                    )?;
                    
                    self.update_realtime_metrics(
                        "context_switches",
                        timestamp,
                        result.performance_metrics.context_switches as f64
                    )?;
                    
                    self.update_realtime_metrics(
                        "cache_misses",
                        timestamp,
                        result.performance_metrics.cache_misses as f64
                    )?;
                    
                    // 处理热点函数
                    for hotspot in &result.hotspots {
                        let hotspot_metric = format!("hotspot_{}_time", hotspot.function_name);
                        self.update_realtime_metrics(
                            &hotspot_metric,
                            timestamp,
                            hotspot.time_percentage
                        )?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// 渲染可视化数据
    pub fn render(&self) -> Result<String, VisualizationError> {
        // 简单的文本渲染实现，实际应该输出图形格式
        let mut output = String::new();

        output.push_str("\n=== 性能监控可视化 ===\n");
        output.push_str(format!("更新时间: {}\n", time::timestamp_millis()).as_str());
        output.push_str(format!("总视图数: {}\n", self.statistics.total_views.load(Ordering::SeqCst)).as_str());
        output.push_str(format!("总面板数: {}\n", self.statistics.total_panels.load(Ordering::SeqCst)).as_str());
        output.push_str("\n");

        let views = self.views.lock();
        for view in views.values() {
            output.push_str(format!("视图: {}\n", view.name).as_str());
            
            for panel in &view.panels {
                output.push_str(format!("  面板: {}\n", panel.title).as_str());
                
                for series in &panel.data_series {
                    output.push_str(format!("    系列: {} ({}个数据点)\n", series.name, series.data.len()).as_str());
                    
                    // 显示最近的5个数据点
                    let start = if series.data.len() > 5 { series.data.len() - 5 } else { 0 };
                    for data in &series.data[start..] {
                        output.push_str(format!("      {}: {:.2}\n", data.0, data.1).as_str());
                    }
                }
            }
        }

        Ok(output)
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> VisualizationStatistics {
        VisualizationStatistics {
            total_views: AtomicU64::new(self.statistics.total_views.load(Ordering::SeqCst)),
            total_panels: AtomicU64::new(self.statistics.total_panels.load(Ordering::SeqCst)),
            total_updates: AtomicU64::new(self.statistics.total_updates.load(Ordering::SeqCst)),
            active_views: AtomicU64::new(self.statistics.active_views.load(Ordering::SeqCst)),
            active_panels: AtomicU64::new(self.statistics.active_panels.load(Ordering::SeqCst)),
        }
    }
}

/// 可视化错误类型
#[derive(Debug, Clone)]
pub enum VisualizationError {
    /// 视图已存在
    ViewAlreadyExists(String),
    /// 视图不存在
    ViewNotFound(String),
    /// 面板不存在
    PanelNotFound(String),
    /// 指标不存在
    MetricNotFound(String),
    /// 视图数量过多
    TooManyViews,
    /// 面板数量过多
    TooManyPanels,
    /// 引擎未启动
    NotStarted,
    /// 配置错误
    ConfigurationError(String),
    /// 系统错误
    SystemError(String),
}

impl core::fmt::Display for VisualizationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VisualizationError::ViewAlreadyExists(id) => write!(f, "视图已存在: {}", id),
            VisualizationError::ViewNotFound(id) => write!(f, "视图不存在: {}", id),
            VisualizationError::PanelNotFound(id) => write!(f, "面板不存在: {}", id),
            VisualizationError::MetricNotFound(name) => write!(f, "指标不存在: {}", name),
            VisualizationError::TooManyViews => write!(f, "视图数量过多"),
            VisualizationError::TooManyPanels => write!(f, "面板数量过多"),
            VisualizationError::NotStarted => write!(f, "可视化引擎未启动"),
            VisualizationError::ConfigurationError(msg) => write!(f, "配置错误: {}", msg),
            VisualizationError::SystemError(msg) => write!(f, "系统错误: {}", msg),
        }
    }
}

/// 全局可视化引擎实例
static VISUALIZATION_ENGINE: spin::Mutex<Option<VisualizationEngine>> = spin::Mutex::new(None);

/// 初始化可视化子系统
pub fn init() -> Result<(), VisualizationError> {
    let config = VisualizationConfig::default();
    let engine = VisualizationEngine::new(config);

    // 启动可视化引擎
    engine.start()?;

    // 创建默认视图
    let default_view = engine.create_view(
        "default".to_string(),
        "Performance Dashboard".to_string(),
        Layout::DoubleColumn
    )?;

    // 创建系统指标面板
    let system_metrics_panel = VisualizationPanel {
        id: "system_metrics".to_string(),
        title: "System Metrics".to_string(),
        chart_config: config.default_chart_config.clone(),
        data_series: Vec::new(),
        panel_type: PanelType::SystemMetrics,
        last_updated: time::timestamp_millis(),
    };

    engine.add_panel_to_view("default", system_metrics_panel)?;

    let mut global_engine = VISUALIZATION_ENGINE.lock();
    *global_engine = Some(engine);

    crate::println!("[visualization] 可视化子系统初始化完成");

    Ok(())
}

/// 获取全局可视化引擎
pub fn get_visualization_engine() -> Result<Arc<VisualizationEngine>, VisualizationError> {
    let engine = VISUALIZATION_ENGINE.lock();
    engine.as_ref()
        .cloned()
        .ok_or(VisualizationError::SystemError("可视化引擎未初始化".to_string()))
}

/// 更新实时指标并渲染
pub fn update_and_render() -> Result<String, VisualizationError> {
    if let Ok(engine) = get_visualization_engine() {
        engine.update_from_metrics()?;
        engine.render()
    } else {
        Err(VisualizationError::SystemError("可视化引擎未初始化".to_string()))
    }
}