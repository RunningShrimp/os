//! 内核核心模块
//!
//! 本模块提供内核核心功能，合并自nos-kernel-core。

use nos_api::Result;

/// 内核版本信息
pub const KERNEL_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const KERNEL_NAME: &str = "NOS";
pub const KERNEL_VERSION_MAJOR: u32 = 0;
pub const KERNEL_VERSION_MINOR: u32 = 1;
pub const KERNEL_VERSION_PATCH: u32 = 0;

/// 内核信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelInfo {
    /// 内核名称
    pub name: String,
    /// 内核版本
    pub version: String,
    /// 主版本号
    pub major: u32,
    /// 次版本号
    pub minor: u32,
    /// 补丁版本号
    pub patch: u32,
    /// 构建日期
    pub build_date: String,
    /// 构建时间
    pub build_time: String,
    /// 提交哈希
    pub commit_hash: String,
    /// 目标三元组
    pub target_triple: String,
    /// 启用的功能
    pub features: Vec<String>,
}

/// 获取内核信息
pub fn get_kernel_info() -> KernelInfo {
    KernelInfo {
        name: KERNEL_NAME.to_string(),
        version: KERNEL_VERSION.to_string(),
        major: KERNEL_VERSION_MAJOR,
        minor: KERNEL_VERSION_MINOR,
        patch: KERNEL_VERSION_PATCH,
        build_date: option_env!("VERGEN_BUILD_DATE").unwrap_or("unknown").to_string(),
        build_time: option_env!("VERGEN_BUILD_TIME").unwrap_or("unknown").to_string(),
        commit_hash: option_env!("VERGEN_GIT_SHA").unwrap_or("unknown").to_string(),
        target_triple: option_env!("VERGEN_TARGET_TRIPLE").unwrap_or("unknown").to_string(),
        features: get_enabled_features(),
    }
}

/// 获取启用的功能
fn get_enabled_features() -> Vec<String> {
    let mut features = Vec::new();
    
    // 移除了无效的 #[cfg(feature = "std")] 条件编译
    
    #[cfg(feature = "log")]
    features.push("log".to_string());
    
    #[cfg(feature = "debug_subsystems")]
    features.push("debug_subsystems".to_string());
    
    #[cfg(feature = "formal_verification")]
    features.push("formal_verification".to_string());
    
    #[cfg(feature = "security_audit")]
    features.push("security_audit".to_string());
    
    features
}

/// 初始化内核核心
pub fn initialize_kernel() -> Result<()> {
    // 初始化架构特定代码
    crate::arch::initialize()?;
    
    // 初始化中断处理
    crate::trap::initialize()?;
    
    // 初始化同步原语
    crate::sync::initialize()?;
    
    Ok(())
}

/// 关闭内核核心
pub fn shutdown_kernel() -> Result<()> {
    // 关闭中断处理
    crate::trap::shutdown()?;
    
    // 关闭架构特定代码
    crate::arch::shutdown()?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_info() {
        let info = get_kernel_info();
        assert_eq!(info.name, KERNEL_NAME);
        assert_eq!(info.version, KERNEL_VERSION);
        assert_eq!(info.major, KERNEL_VERSION_MAJOR);
        assert_eq!(info.minor, KERNEL_VERSION_MINOR);
        assert_eq!(info.patch, KERNEL_VERSION_PATCH);
    }

    #[test]
    fn test_enabled_features() {
        let features = get_enabled_features();
        
        // 至少应该有默认功能
        assert!(!features.is_empty());
    }
}