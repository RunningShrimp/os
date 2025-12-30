//! Host Intrusion Detection System (HIDS)
//!
//! 主机入侵检测系统模块
//! 负责检测主机系统中的恶意活动和攻击模式
//!
//! # 架构
//!
//! 此模块提供了完整的主机入侵检测功能，包括：
//! - 系统调用监控
//! - 文件系统监控
//! - 进程监控
//! - 注册表监控
//! - 网络连接监控
//! - 用户活动监控
//! - 完整性检查
//! - 恶意软件扫描
//!
//! # 模块组织
//!
//! - [`types`] - 所有类型定义
//! - [`detector`] - 各种监控器和检测器
//! - [`stats`] - 统计信息
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use kernel::ids::host_ids::HostIds;
//!
//! let mut hids = HostIds::new();
//! hids.init(&config)?;
//!
//! // 分析事件
//! let detections = hids.analyze_event(&audit_event)?;
//! ```

extern crate alloc;

// 导入所有子模块
mod detector;
mod host_ids;
mod stats;
mod types;

// 重新导出公共API
pub use detector::*;
pub use host_ids::HostIds;
pub use stats::*;
pub use types::*;
