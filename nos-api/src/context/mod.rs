//! 上下文管理模块
//!
//! 本模块提供上下文管理功能，用于替代全局状态，实现更好的状态管理。

// 使用重新导出的接口定义
pub use crate::interfaces::{
    InterfaceContext, InterfaceContextType, InterfaceContextManager
};

