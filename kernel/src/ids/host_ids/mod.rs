//! Host Intrusion Detection System (HIDS)
//!
//! 主机入侵检测系统模块
//! 负责检测主机系统中的恶意活动和攻击模式

extern crate alloc;

pub mod types;
pub mod syscall;
pub mod file;
pub mod process;
pub mod registry;
pub mod network;
pub mod user;
pub mod malware;

// 临时：保留原有文件作为过渡
// TODO: 逐步拆分到各个子模块，将代码从host_ids.rs移动到对应的子模块
mod host_ids;

// 重新导出主要类型
pub use host_ids::{
    HostIds,
};

// 注意：暂时注释掉未使用的重新导出以避免警告
// TODO: 在实现完整的子模块后重新启用这些导出
// pub use host_ids::{
//     SyscallMonitor, FileMonitor, ProcessMonitor, RegistryMonitor,
//     NetworkMonitor, UserMonitor, IntegrityChecker, MalwareScanner,
// };

