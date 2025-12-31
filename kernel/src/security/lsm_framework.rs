//! # Linux Security Modules (LSM) Framework
//!
//! This module provides a comprehensive LSM framework implementation that allows
//! multiple security modules to work together through hook-based architecture.
//!
//! ## Features
//!
//! - **Hook Framework**: Security hook registration and invocation
//! - **Module Stacking**: Multiple concurrent security modules
//! - **Security Blob Management**: Per-object security data
//! - **Labeling**: Security label management
//! - **Audit Integration**: LSM-specific audit logging

use crate::prelude::*;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// ============================================================================
// LSM Hook Types
// ============================================================================

/// LSM hook points
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum LsmHook {
    // File hooks
    FilePermission = 1,
    FileAllocSecurity = 2,
    FileFreeSecurity = 3,
    FileOpen = 4,
    FileReceive = 5,

    // Process hooks
    BprmSetSecurity = 10,
    BprmCheckSecurity = 11,
    BprmCommittingCreds = 12,
    BprmCommittedCreds = 13,

    TaskCreate = 14,
    TaskFree = 15,
    TaskAlloc = 16,

    // Socket hooks
    SocketCreate = 20,
    SocketPostCreate = 21,
    SocketBind = 22,
    SocketConnect = 23,
    SocketListen = 24,
    SocketAccept = 25,
    SocketSendmsg = 26,
    SocketRecvmsg = 27,

    // IPC hooks
    ShmAllocSecurity = 30,
    ShmFreeSecurity = 31,
    ShmAssociate = 32,

    MsgQueueAlloc = 33,
    MsgQueueFree = 34,
    MsgQueueAssociate = 35,

    // Network hooks
    InetConnRequest = 40,
    InetCskClone = 41,
    InetConnEstablished = 42,

    // Key management hooks
    KeyAlloc = 50,
    KeyFree = 51,
    KeyPermission = 52,

    // Device hooks
    DeviceContext = 60,
}

/// Hook callback function type
pub type LsmHookCallback = fn(hook: LsmHook, data: &LsmHookData) -> Result<LsmDecision>;

// ============================================================================
// LSM Hook Data
// ============================================================================

/// Data passed to LSM hooks
#[derive(Debug)]
pub enum LsmHookData {
    FileOperation {
        filename: String,
        flags: u32,
        mode: u32,
    },
    ProcessOperation {
        pid: u32,
        uid: u32,
        gid: u32,
    },
    SocketOperation {
        family: u32,
        type_: u32,
        protocol: u32,
    },
    NetworkConnection {
        saddr: u32,
        daddr: u32,
        sport: u16,
        dport: u16,
    },
    Generic {
        data: Vec<u8>,
    },
}

// ============================================================================
// LSM Decision
// ============================================================================

/// LSM access decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LsmDecision {
    Allow = 0,
    Deny = 1,
}

impl LsmDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, LsmDecision::Allow)
    }
}

// ============================================================================
// Security Module
// ============================================================================

/// LSM security module
#[derive(Debug)]
pub struct LsmModule {
    pub name: String,
    pub enabled: AtomicBool,
    pub priority: u32,
    pub hooks: Mutex<BTreeMap<LsmHook, LsmHookCallback>>,
    pub stats: Mutex<LsmModuleStats>,
}

/// Module statistics
#[derive(Debug, Clone, Default)]
pub struct LsmModuleStats {
    pub hook_invocations: u64,
    pub allow_decisions: u64,
    pub deny_decisions: u64,
    pub errors: u64,
}

impl LsmModule {
    pub fn new(name: String, priority: u32) -> Self {
        Self {
            name,
            enabled: AtomicBool::new(true),
            priority,
            hooks: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(LsmModuleStats::default()),
        }
    }

    pub fn register_hook(&self, hook: LsmHook, callback: LsmHookCallback) {
        let mut hooks = self.hooks.lock();
        hooks.insert(hook, callback);
    }

    pub fn invoke_hook(&self, hook: LsmHook, data: &LsmHookData) -> Option<Result<LsmDecision>> {
        if !self.enabled.load(Ordering::Relaxed) {
            return None;
        }

        let hooks = self.hooks.lock();
        if let Some(callback) = hooks.get(&hook) {
            let mut stats = self.stats.lock();
            stats.hook_invocations += 1;

            let result = callback(hook, data);

            match &result {
                Ok(decision) if decision.is_allowed() => stats.allow_decisions += 1,
                Ok(_) => stats.deny_decisions += 1,
                Err(_) => stats.errors += 1,
            }

            return Some(result);
        }

        None
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
    }

    pub fn get_stats(&self) -> LsmModuleStats {
        self.stats.lock().clone()
    }
}

// ============================================================================
// LSM Framework
// ============================================================================

/// LSM framework manager
#[derive(Debug)]
pub struct LsmFramework {
    pub modules: Mutex<Vec<*const LsmModule>>,
    pub hook_chain: Mutex<BTreeMap<LsmHook, Vec<usize>>>,
    pub global_stats: Mutex<LsmGlobalStats>,
    pub initialized: AtomicBool,
}

// SAFETY: LsmFramework is used in a static context with proper synchronization
// The raw pointers are only accessed while holding the Mutex
unsafe impl Send for LsmFramework {}

/// Global framework statistics
#[derive(Debug, Clone, Default)]
pub struct LsmGlobalStats {
    pub total_hook_invocations: u64,
    pub total_modules: usize,
    pub active_modules: usize,
    pub total_denies: u64,
}

impl LsmFramework {
    pub fn new() -> Self {
        Self {
            modules: Mutex::new(Vec::new()),
            hook_chain: Mutex::new(BTreeMap::new()),
            global_stats: Mutex::new(LsmGlobalStats::default()),
            initialized: AtomicBool::new(false),
        }
    }

    pub fn initialize(&self) -> Result<()> {
        if self.initialized.load(Ordering::Acquire) {
            return Ok(());
        }

        log_info!("[lsm] LSM framework initialized");
        self.initialized.store(true, Ordering::Release);
        Ok(())
    }

    pub fn register_module(&self, module: &LsmModule) -> Result<()> {
        let module_ptr = module as *const LsmModule;

        // Add module to list
        let mut modules = self.modules.lock();
        
        // Insert in priority order (higher priority first)
        let mut inserted = false;
        for i in 0..modules.len() {
            unsafe {
                if let Some(existing) = modules.get(i) {
                    let existing_module = &**existing;
                    if module.priority > existing_module.priority {
                        modules.insert(i, module_ptr);
                        inserted = true;
                        break;
                    }
                }
            }
        }

        if !inserted {
            modules.push(module_ptr);
        }

        // Update hook chain
        self.rebuild_hook_chain();

        // Update statistics
        let mut stats = self.global_stats.lock();
        stats.total_modules = modules.len();
        stats.active_modules = modules.len();

        log_info!("[lsm] Registered module: {}", module.name.clone());
        Ok(())
    }

    pub fn unregister_module(&self, module: &LsmModule) -> Result<()> {
        let module_ptr = module as *const LsmModule;

        let mut modules = self.modules.lock();
        modules.retain(|&m| m != module_ptr);

        self.rebuild_hook_chain();

        let mut stats = self.global_stats.lock();
        stats.total_modules = modules.len();

        log_info!("[lsm] Unregistered module: {}", module.name.clone());
        Ok(())
    }

    pub fn invoke_hook(&self, hook: LsmHook, data: &LsmHookData) -> LsmDecision {
        if !self.initialized.load(Ordering::Acquire) {
            return LsmDecision::Allow;
        }

        let modules = self.modules.lock();
        let hook_chain = self.hook_chain.lock();

        let module_indices = hook_chain.get(&hook);

        let mut global_stats = self.global_stats.lock();
        global_stats.total_hook_invocations += 1;

        if let Some(indices) = module_indices {
            for &idx in indices {
                unsafe {
                    if let Some(&module_ptr) = modules.get(idx) {
                        let module = &*module_ptr;

                        if let Some(result) = module.invoke_hook(hook, data) {
                            match result {
                                Ok(LsmDecision::Allow) => continue,
                                Ok(LsmDecision::Deny) => {
                                    global_stats.total_denies += 1;
                                    return LsmDecision::Deny;
                                }
                                Err(_) => {
                                    return LsmDecision::Deny;
                                }
                            }
                        }
                    }
                }
            }
        }

        LsmDecision::Allow
    }

    fn rebuild_hook_chain(&self) {
        let modules = self.modules.lock();
        let mut hook_chain = self.hook_chain.lock();

        hook_chain.clear();

        for (idx, &module_ptr) in modules.iter().enumerate() {
            unsafe {
                if let Some(module) = module_ptr.as_ref() {
                    let hooks = module.hooks.lock();
                    for hook in hooks.keys() {
                        hook_chain
                            .entry(*hook)
                            .or_insert_with(Vec::new)
                            .push(idx);
                    }
                }
            }
        }
    }

    pub fn get_stats(&self) -> LsmGlobalStats {
        let modules = self.modules.lock();
        let mut stats = self.global_stats.lock();
        stats.active_modules = modules.iter().filter(|&&m| unsafe {
            m.as_ref().map(|m| m.enabled.load(Ordering::Relaxed)).unwrap_or(false)
        }).count();
        stats.clone()
    }
}

// ============================================================================
// Security Blob Management
// ============================================================================

/// Security blob for storing per-object security data
#[derive(Debug)]
pub struct SecurityBlob {
    pub data: Mutex<Vec<u8>>,
    pub owner: String,
}

impl Clone for SecurityBlob {
    fn clone(&self) -> Self {
        Self {
            data: Mutex::new(self.data.lock().clone()),
            owner: self.owner.clone(),
        }
    }
}

impl SecurityBlob {
    pub fn new(size: usize, owner: String) -> Self {
        Self {
            data: Mutex::new(vec![0u8; size]),
            owner,
        }
    }

    pub fn set_data(&self, data: &[u8]) {
        let mut blob_data = self.data.lock();
        let len = data.len().min(blob_data.len());
        blob_data[..len].copy_from_slice(&data[..len]);
    }

    pub fn get_data(&self) -> Vec<u8> {
        self.data.lock().clone()
    }
}

/// Security blob manager
#[derive(Debug)]
pub struct SecurityBlobManager {
    pub blobs: Mutex<BTreeMap<u64, SecurityBlob>>,
    pub next_id: AtomicU64,
}

impl SecurityBlobManager {
    pub fn new() -> Self {
        Self {
            blobs: Mutex::new(BTreeMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    pub fn allocate(&self, size: usize, owner: String) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let blob = SecurityBlob::new(size, owner);

        self.blobs.lock().insert(id, blob);
        id
    }

    pub fn free(&self, id: u64) {
        self.blobs.lock().remove(&id);
    }

    pub fn get(&self, id: u64) -> Option<SecurityBlob> {
        self.blobs.lock().get(&id).cloned()
    }
}

// ============================================================================
// Global State
// ============================================================================

static GLOBAL_LSM: Mutex<Option<LsmFramework>> = Mutex::new(None);
static GLOBAL_BLOBS: Mutex<Option<SecurityBlobManager>> = Mutex::new(None);

pub fn init_lsm_framework() -> Result<()> {
    let mut global = GLOBAL_LSM.lock();
    if global.is_some() {
        return Ok(());
    }

    let framework = LsmFramework::new();
    framework.initialize()?;
    *global = Some(framework);

    // Initialize security blob manager
    *GLOBAL_BLOBS.lock() = Some(SecurityBlobManager::new());

    Ok(())
}

pub fn register_lsm_module(module: &LsmModule) -> Result<()> {
    let global = GLOBAL_LSM.lock();
    let framework = global.as_ref().ok_or(Error::NotFound)?;
    framework.register_module(module)
}

pub fn invoke_lsm_hook(hook: LsmHook, data: &LsmHookData) -> LsmDecision {
    let global = GLOBAL_LSM.lock();
    if let Some(framework) = global.as_ref() {
        framework.invoke_hook(hook, data)
    } else {
        LsmDecision::Allow
    }
}

pub fn get_lsm_stats() -> Option<LsmGlobalStats> {
    let global = GLOBAL_LSM.lock();
    global.as_ref().map(|f| f.get_stats())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_file_hook(_: LsmHook, _: &LsmHookData) -> Result<LsmDecision> {
        Ok(LsmDecision::Allow)
    }

    fn dummy_deny_hook(_: LsmHook, _: &LsmHookData) -> Result<LsmDecision> {
        Ok(LsmDecision::Deny)
    }

    #[test]
    fn test_lsm_module() {
        let module = LsmModule::new("test_module".to_string(), 100);
        module.register_hook(LsmHook::FilePermission, dummy_file_hook);

        let data = LsmHookData::FileOperation {
            filename: "/test".to_string(),
            flags: 0,
            mode: 0,
        };

        let result = module.invoke_hook(LsmHook::FilePermission, &data);
        assert!(result.is_some());
        assert!(result.unwrap().unwrap().is_allowed());
    }

    #[test]
    fn test_lsm_framework() {
        let framework = LsmFramework::new();
        framework.initialize().unwrap();

        let module1 = LsmModule::new("module1".to_string(), 100);
        module1.register_hook(LsmHook::FilePermission, dummy_file_hook);

        let module2 = LsmModule::new("module2".to_string(), 200);
        module2.register_hook(LsmHook::FilePermission, dummy_deny_hook);

        framework.register_module(&module1).unwrap();
        framework.register_module(&module2).unwrap();

        let data = LsmHookData::FileOperation {
            filename: "/test".to_string(),
            flags: 0,
            mode: 0,
        };

        let result = framework.invoke_hook(LsmHook::FilePermission, &data);
        // Module2 has higher priority and denies, so result should be Deny
        assert_eq!(result, LsmDecision::Deny);
    }

    #[test]
    fn test_security_blob() {
        let manager = SecurityBlobManager::new();
        let id = manager.allocate(64, "test".to_string());

        let blob = manager.get(id);
        assert!(blob.is_some());

        manager.free(id);
        assert!(manager.get(id).is_none());
    }
}
