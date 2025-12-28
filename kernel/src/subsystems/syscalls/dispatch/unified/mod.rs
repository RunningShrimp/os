//! Unified Dispatcher Module
//!
//! Provides unified system call dispatching

use nos_api::Result;

/// Unified dispatcher configuration
#[derive(Debug, Clone)]
pub struct UnifiedDispatcherConfig {
    pub enable_fast_path: bool,
    pub enable_caching: bool,
}

/// Unified dispatcher
pub struct UnifiedDispatcher {
    config: UnifiedDispatcherConfig,
}

impl UnifiedDispatcher {
    pub fn new(config: UnifiedDispatcherConfig) -> Self {
        Self { config }
    }
}

/// Initialize unified dispatcher
pub fn init_unified_dispatcher(config: UnifiedDispatcherConfig) -> Result<()> {
    let _dispatcher = UnifiedDispatcher::new(config);
    Ok(())
}

/// Get unified dispatcher
pub fn get_unified_dispatcher() -> Option<&'static UnifiedDispatcher> {
    static DISPATCHER: UnifiedDispatcher = UnifiedDispatcher::new(UnifiedDispatcherConfig {
        enable_fast_path: true,
        enable_caching: true,
    });
    Some(&DISPATCHER)
}

/// Fast path handler
pub trait FastPathHandler: Send + Sync {
    fn handle_fast_path(&self, syscall_num: usize, args: &[usize]) -> Option<isize>;
}
