// Debug Plugin System Module
//
// 调试插件系统模块
// 提供调试插件管理功能

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// 调试插件
#[derive(Debug, Clone)]
pub struct DebugPlugin {
    /// 插件ID
    pub id: String,
    /// 插件名称
    pub name: String,
    /// 插件版本
    pub version: String,
    /// 插件类型
    pub plugin_type: PluginType,
    /// 插件描述
    pub description: String,
    /// 插件作者
    pub author: String,
    /// 插件配置
    pub config: PluginConfig,
    /// 启用状态
    pub enabled: bool,
    /// 插件接口
    pub interface: PluginInterface,
}

/// 插件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginType {
    /// 断点插件
    BreakpointPlugin,
    /// 监控插件
    MonitorPlugin,
    /// 分析插件
    AnalyzerPlugin,
    /// 输出插件
    OutputPlugin,
    /// 过滤插件
    FilterPlugin,
    /// 自定义插件
    CustomPlugin,
}

/// 插件配置
#[derive(Debug, Clone)]
pub struct PluginConfig {
    /// 配置参数
    pub parameters: BTreeMap<String, String>,
    /// 自动启动
    pub auto_start: bool,
    /// 优先级
    pub priority: u32,
    /// 依赖项
    pub dependencies: Vec<String>,
}

/// 插件接口
#[derive(Debug, Clone)]
pub enum PluginInterface {
    /// 基础接口
    Basic,
    /// 高级接口
    Advanced,
    /// 专用接口
    Specialized,
}

