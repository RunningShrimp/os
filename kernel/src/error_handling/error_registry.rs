//! Error Registry Module
//!
//! 错误注册表模块
//! 管理错误代码和错误类型的注册与查找

extern crate alloc;
extern crate hashbrown;
use crate::sync::{SpinLock, Mutex};
use hashbrown::HashMap;
use crate::time::SystemTime;
use crate::compat::DefaultHasherBuilder;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;
use alloc::{format, vec};
use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, Ordering};

use super::*;

/// 错误注册表
pub struct ErrorRegistry {
    /// 注册表ID
    pub id: u64,
    /// 错误代码映射
    error_codes: HashMap<u32, ErrorDefinition, DefaultHasherBuilder>,
    /// 错误名称映射
    error_names: HashMap<String, u32, DefaultHasherBuilder>,
    /// 错误类别映射
    error_categories: HashMap<ErrorCategory, Vec<u32>, DefaultHasherBuilder>,
    /// 动态错误代码计数器
    dynamic_code_counter: AtomicU64,
    /// 注册统计
    stats: RegistryStats,
}

/// 错误定义
#[derive(Debug, Clone)]
pub struct ErrorDefinition {
    /// 错误代码
    pub code: u32,
    /// 错误名称
    pub name: String,
    /// 错误类型
    pub error_type: ErrorType,
    /// 错误类别
    pub category: ErrorCategory,
    /// 默认严重级别
    pub default_severity: ErrorSeverity,
    /// 错误描述
    pub description: String,
    /// 错误原因
    pub causes: Vec<String>,
    /// 建议解决方案
    pub solutions: Vec<String>,
    /// 相关文档
    pub documentation: Vec<String>,
    /// 创建时间
    pub created_at: u64,
    /// 是否为系统错误
    pub is_system_error: bool,
    /// 是否为用户错误
    pub is_user_error: bool,
    /// 恢复策略
    pub recovery_strategies: Vec<RecoveryStrategy>,
    /// 重复阈值
    pub duplicate_threshold: u32,
    /// 升级阈值
    pub escalation_threshold: u32,
}

/// 注册表统计
#[derive(Debug, Clone, Default)]
pub struct RegistryStats {
    /// 注册的错误总数
    pub total_errors: u64,
    /// 系统错误数
    pub system_errors: u64,
    /// 用户错误数
    pub user_errors: u64,
    /// 动态注册错误数
    pub dynamic_errors: u64,
    /// 按类别统计
    pub errors_by_category: HashMap<ErrorCategory, u64, DefaultHasherBuilder>,
    /// 按严重级别统计
    pub errors_by_severity: HashMap<ErrorSeverity, u64, DefaultHasherBuilder>,
}

impl ErrorRegistry {
    /// 创建新的错误注册表
    pub fn new() -> Self {
        Self {
            id: 1,
            error_codes: HashMap::with_hasher(DefaultHasherBuilder),
            error_names: HashMap::with_hasher(DefaultHasherBuilder),
            error_categories: HashMap::with_hasher(DefaultHasherBuilder),
            dynamic_code_counter: AtomicU64::new(10000), // 动态错误从10000开始
            stats: RegistryStats::default(),
        }
    }

    /// 初始化错误注册表
    pub fn init(&mut self) -> Result<(), &'static str> {
        // 注册系统预定义错误
        self.register_system_errors()?;

        crate::println!("[ErrorRegistry] Error registry initialized successfully");
        Ok(())
    }

    /// 注册错误
    pub fn register_error(&mut self, error_def: ErrorDefinition) -> Result<(), &'static str> {
        // 检查错误代码是否已存在
        if self.error_codes.contains_key(&error_def.code) {
            return Err("Error code already exists");
        }

        // 检查错误名称是否已存在
        if self.error_names.contains_key(&error_def.name) {
            return Err("Error name already exists");
        }

        // 注册错误
        self.error_codes.insert(error_def.code, error_def.clone());
        self.error_names.insert(error_def.name.clone(), error_def.code);

        // 更新类别映射
        let category_errors = self.error_categories
            .entry(error_def.category)
            .or_insert_with(Vec::new);
        category_errors.push(error_def.code);

        // 更新统计信息
        self.stats.total_errors += 1;
        *self.stats.errors_by_category.entry(error_def.category).or_insert(0) += 1;
        *self.stats.errors_by_severity.entry(error_def.default_severity).or_insert(0) += 1;

        if error_def.is_system_error {
            self.stats.system_errors += 1;
        }
        if error_def.is_user_error {
            self.stats.user_errors += 1;
        }

        Ok(())
    }

    /// 注册动态错误
    pub fn register_dynamic_error(&mut self, name: &str, error_type: ErrorType, category: ErrorCategory, severity: ErrorSeverity, description: &str) -> Result<u32, &'static str> {
        let code = self.dynamic_code_counter.fetch_add(1, Ordering::SeqCst) as u32;

        let error_def = ErrorDefinition {
            code,
            name: name.to_string(),
            error_type,
            category,
            default_severity: severity,
            description: description.to_string(),
            causes: Vec::new(),
            solutions: Vec::new(),
            documentation: Vec::new(),
            created_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs(),
            is_system_error: false,
            is_user_error: true,
            recovery_strategies: Vec::new(),
            duplicate_threshold: 3,
            escalation_threshold: 10,
        };

        self.register_error(error_def)?;
        self.stats.dynamic_errors += 1;

        Ok(code)
    }

    /// 根据错误代码查找错误定义
    pub fn find_by_code(&self, code: u32) -> Option<&ErrorDefinition> {
        self.error_codes.get(&code)
    }

    /// 根据错误名称查找错误定义
    pub fn find_by_name(&self, name: &str) -> Option<&ErrorDefinition> {
        if let Some(code) = self.error_names.get(name) {
            self.error_codes.get(code)
        } else {
            None
        }
    }

    /// 根据类别查找错误
    pub fn find_by_category(&self, category: ErrorCategory) -> Vec<&ErrorDefinition> {
        if let Some(codes) = self.error_categories.get(&category) {
            codes.iter()
                .filter_map(|&code| self.error_codes.get(&code))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 搜索错误
    pub fn search_errors(&self, keyword: &str) -> Vec<&ErrorDefinition> {
        self.error_codes.values()
            .filter(|error| {
                error.name.contains(keyword) ||
                error.description.contains(keyword) ||
                error.causes.iter().any(|cause| cause.contains(keyword)) ||
                error.solutions.iter().any(|solution| solution.contains(keyword))
            })
            .collect()
    }

    /// 获取所有错误定义
    pub fn get_all_errors(&self) -> Vec<&ErrorDefinition> {
        self.error_codes.values().collect()
    }

    /// 获取错误类别列表
    pub fn get_categories(&self) -> Vec<ErrorCategory> {
        self.error_categories.keys().copied().collect()
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> RegistryStats {
        self.stats.clone()
    }

    /// 注册系统预定义错误
    fn register_system_errors(&mut self) -> Result<(), &'static str> {
        // 系统错误 (1000-1999)
        let system_errors = vec![
            ErrorDefinition {
                code: 1001,
                name: "ERR_SYSTEM_INIT_FAILED".to_string(),
                error_type: ErrorType::RuntimeError,
                category: ErrorCategory::System,
                default_severity: ErrorSeverity::Critical,
                description: "System initialization failed".to_string(),
                causes: vec![
                    "Hardware malfunction".to_string(),
                    "Memory corruption".to_string(),
                    "Configuration error".to_string(),
                ],
                solutions: vec![
                    "Check hardware components".to_string(),
                    "Verify system configuration".to_string(),
                    "Reboot system".to_string(),
                ],
                documentation: vec![
                    "System initialization manual".to_string(),
                    "Hardware troubleshooting guide".to_string(),
                ],
                created_at: 0,
                is_system_error: true,
                is_user_error: false,
                recovery_strategies: vec![RecoveryStrategy::Restart, RecoveryStrategy::Manual],
                duplicate_threshold: 1,
                escalation_threshold: 1,
            },
            ErrorDefinition {
                code: 1002,
                name: "ERR_MEMORY_ALLOCATION_FAILED".to_string(),
                error_type: ErrorType::MemoryError,
                category: ErrorCategory::Memory,
                default_severity: ErrorSeverity::Critical,
                description: "Memory allocation failed".to_string(),
                causes: vec![
                    "Out of memory".to_string(),
                    "Memory fragmentation".to_string(),
                    "Invalid memory request".to_string(),
                ],
                solutions: vec![
                    "Free unused memory".to_string(),
                    "Increase memory pool".to_string(),
                    "Check for memory leaks".to_string(),
                ],
                documentation: vec!["Memory management guide".to_string()],
                created_at: 0,
                is_system_error: true,
                is_user_error: false,
                recovery_strategies: vec![RecoveryStrategy::Release, RecoveryStrategy::Restart],
                duplicate_threshold: 5,
                escalation_threshold: 20,
            },
            ErrorDefinition {
                code: 1003,
                name: "ERR_FILE_SYSTEM_ERROR".to_string(),
                error_type: ErrorType::IOError,
                category: ErrorCategory::FileSystem,
                default_severity: ErrorSeverity::Error,
                description: "File system operation failed".to_string(),
                causes: vec![
                    "Disk corruption".to_string(),
                    "Permission denied".to_string(),
                    "File not found".to_string(),
                ],
                solutions: vec![
                    "Check file permissions".to_string(),
                    "Run file system check".to_string(),
                    "Verify disk integrity".to_string(),
                ],
                documentation: vec!["File system troubleshooting".to_string()],
                created_at: 0,
                is_system_error: true,
                is_user_error: false,
                recovery_strategies: vec![RecoveryStrategy::Retry, RecoveryStrategy::Manual],
                duplicate_threshold: 3,
                escalation_threshold: 10,
            },
            ErrorDefinition {
                code: 1004,
                name: "ERR_NETWORK_CONNECTION_FAILED".to_string(),
                error_type: ErrorType::NetworkError,
                category: ErrorCategory::Network,
                default_severity: ErrorSeverity::Warning,
                description: "Network connection failed".to_string(),
                causes: vec![
                    "Network unreachable".to_string(),
                    "Connection refused".to_string(),
                    "Timeout occurred".to_string(),
                ],
                solutions: vec![
                    "Check network connectivity".to_string(),
                    "Verify server status".to_string(),
                    "Retry connection".to_string(),
                ],
                documentation: vec!["Network troubleshooting guide".to_string()],
                created_at: 0,
                is_system_error: true,
                is_user_error: false,
                recovery_strategies: vec![RecoveryStrategy::Retry, RecoveryStrategy::Failover],
                duplicate_threshold: 5,
                escalation_threshold: 15,
            },
        ];

        // 文件系统错误 (2000-2999)
        let filesystem_errors = vec![
            ErrorDefinition {
                code: 2001,
                name: "ERR_FILE_NOT_FOUND".to_string(),
                error_type: ErrorType::IOError,
                category: ErrorCategory::FileSystem,
                default_severity: ErrorSeverity::Error,
                description: "File not found".to_string(),
                causes: vec![
                    "Incorrect file path".to_string(),
                    "File was deleted".to_string(),
                    "Permission denied".to_string(),
                ],
                solutions: vec![
                    "Verify file path".to_string(),
                    "Check file permissions".to_string(),
                    "Recreate file if necessary".to_string(),
                ],
                documentation: vec!["File operations guide".to_string()],
                created_at: 0,
                is_system_error: false,
                is_user_error: true,
                recovery_strategies: vec![RecoveryStrategy::Manual],
                duplicate_threshold: 3,
                escalation_threshold: 5,
            },
            ErrorDefinition {
                code: 2002,
                name: "ERR_PERMISSION_DENIED".to_string(),
                error_type: ErrorType::PermissionError,
                category: ErrorCategory::FileSystem,
                default_severity: ErrorSeverity::Error,
                description: "Permission denied".to_string(),
                causes: vec![
                    "Insufficient privileges".to_string(),
                    "File is read-only".to_string(),
                    "Access control restrictions".to_string(),
                ],
                solutions: vec![
                    "Check file permissions".to_string(),
                    "Run with appropriate privileges".to_string(),
                    "Contact administrator".to_string(),
                ],
                documentation: vec!["Permission management guide".to_string()],
                created_at: 0,
                is_system_error: false,
                is_user_error: true,
                recovery_strategies: vec![RecoveryStrategy::Manual],
                duplicate_threshold: 2,
                escalation_threshold: 3,
            },
        ];

        // 进程错误 (3000-3999)
        let process_errors = vec![
            ErrorDefinition {
                code: 3001,
                name: "ERR_PROCESS_CREATION_FAILED".to_string(),
                error_type: ErrorType::RuntimeError,
                category: ErrorCategory::Process,
                default_severity: ErrorSeverity::Error,
                description: "Process creation failed".to_string(),
                causes: vec![
                    "Insufficient resources".to_string(),
                    "Security policy violation".to_string(),
                    "Invalid process parameters".to_string(),
                ],
                solutions: vec![
                    "Check available resources".to_string(),
                    "Verify security policies".to_string(),
                    "Review process parameters".to_string(),
                ],
                documentation: vec!["Process management guide".to_string()],
                created_at: 0,
                is_system_error: true,
                is_user_error: false,
                recovery_strategies: vec![RecoveryStrategy::Retry, RecoveryStrategy::Manual],
                duplicate_threshold: 3,
                escalation_threshold: 10,
            },
        ];

        // 注册所有错误
        for error in system_errors {
            self.register_error(error)?;
        }

        for error in filesystem_errors {
            self.register_error(error)?;
        }

        for error in process_errors {
            self.register_error(error)?;
        }

        Ok(())
    }

    /// 停止错误注册表
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        // 清理所有注册的错误
        self.error_codes.clear();
        self.error_names.clear();
        self.error_categories.clear();
        self.stats = RegistryStats::default();

        crate::println!("[ErrorRegistry] Error registry shutdown successfully");
        Ok(())
    }
}

/// 创建默认的错误注册表
pub fn create_error_registry() -> Arc<Mutex<ErrorRegistry>> {
    Arc::new(Mutex::new(ErrorRegistry::new()))
}

/// 预定义错误代码常量
pub mod error_codes {
    // 系统错误 1000-1999
    pub const SYSTEM_INIT_FAILED: u32 = 1001;
    pub const MEMORY_ALLOCATION_FAILED: u32 = 1002;
    pub const FILE_SYSTEM_ERROR: u32 = 1003;
    pub const NETWORK_CONNECTION_FAILED: u32 = 1004;

    // 文件系统错误 2000-2999
    pub const FILE_NOT_FOUND: u32 = 2001;
    pub const PERMISSION_DENIED: u32 = 2002;
    pub const DISK_FULL: u32 = 2003;
    pub const FILE_CORRUPTED: u32 = 2004;
    pub const INVALID_FILE_PATH: u32 = 2005;

    // 进程错误 3000-3999
    pub const PROCESS_CREATION_FAILED: u32 = 3001;
    pub const PROCESS_TERMINATION_FAILED: u32 = 3002;
    pub const INVALID_PROCESS_ID: u32 = 3003;

    // 网络错误 4000-4999
    pub const NETWORK_TIMEOUT: u32 = 4001;
    pub const CONNECTION_REFUSED: u32 = 4002;
    pub const HOST_UNREACHABLE: u32 = 4003;
    pub const NETWORK_UNREACHABLE: u32 = 4004;

    // 安全错误 5000-5999
    pub const AUTHENTICATION_FAILED: u32 = 5001;
    pub const AUTHORIZATION_FAILED: u32 = 5002;
    pub const SECURITY_VIOLATION: u32 = 5003;
    pub const ENCRYPTION_FAILED: u32 = 5004;

    // 用户错误 6000-6999
    pub const INVALID_INPUT: u32 = 6001;
    pub const INVALID_PARAMETER: u32 = 6002;
    pub const OPERATION_NOT_SUPPORTED: u32 = 6003;
}

/// 获取错误名称
pub fn get_error_name(code: u32) -> Option<String> {
    let registry = get_error_registry().lock();
    registry.find_by_code(code).map(|error| error.name.clone())
}

/// 获取错误描述
pub fn get_error_description(code: u32) -> Option<String> {
    let registry = get_error_registry().lock();
    registry.find_by_code(code).map(|error| error.description.clone())
}

/// 获取默认严重级别
pub fn get_default_severity(code: u32) -> Option<ErrorSeverity> {
    let registry = get_error_registry().lock();
    registry.find_by_code(code).map(|error| error.default_severity)
}

use spin::Once;

/// 全局错误注册表实例
pub static ERROR_REGISTRY: Once<spin::Mutex<ErrorRegistry>> = Once::new();

/// Initialize the error registry
pub fn init_error_registry() {
    ERROR_REGISTRY.call_once(|| spin::Mutex::new(ErrorRegistry::new()));
}

/// Get the error registry instance, initializing it if needed
pub fn get_error_registry() -> &'static spin::Mutex<ErrorRegistry> {
    ERROR_REGISTRY.call_once(|| spin::Mutex::new(ErrorRegistry::new()));
    ERROR_REGISTRY.get().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_registry_initialization() {
        let registry = ErrorRegistry::new();
        assert_eq!(registry.id, 1);
        assert!(registry.error_codes.is_empty());
        assert!(registry.error_names.is_empty());
    }

    #[test]
    fn test_error_registration() {
        let mut registry = ErrorRegistry::new();

        let error_def = ErrorDefinition {
            code: 9999,
            name: "TEST_ERROR".to_string(),
            error_type: ErrorType::RuntimeError,
            category: ErrorCategory::Application,
            default_severity: ErrorSeverity::Warning,
            description: "Test error".to_string(),
            causes: vec!["Test cause".to_string()],
            solutions: vec!["Test solution".to_string()],
            documentation: vec!["Test doc".to_string()],
            created_at: 0,
            is_system_error: false,
            is_user_error: true,
            recovery_strategies: vec![],
            duplicate_threshold: 1,
            escalation_threshold: 1,
        };

        assert!(registry.register_error(error_def).is_ok());
        assert!(registry.find_by_code(9999).is_some());
        assert!(registry.find_by_name("TEST_ERROR").is_some());
    }

    #[test]
    fn test_error_search() {
        let registry = ErrorRegistry::new();

        // 搜索不存在的错误
        let results = registry.search_errors("nonexistent");
        assert!(results.is_empty());
    }
}
