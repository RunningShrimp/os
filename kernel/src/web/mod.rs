//! Web engine integration for NOS
//!
//! Provides integration for Rust web engines (Servo/Blitz) with NOS graphics and network stack.

pub mod engine;
pub mod runtime;
pub mod api;

/// Initialize web engine subsystem
pub fn init() {
    // Initialize web engine manager
    if let Err(e) = engine::init_web_engine_manager() {
        crate::println!("[web] Failed to initialize web engine manager: {}", e);
    }
    
    // Initialize web runtime manager
    if let Err(e) = runtime::init_web_runtime_manager() {
        crate::println!("[web] Failed to initialize web runtime manager: {}", e);
    }
    
    // Initialize web system API
    if let Err(e) = api::init_web_system_api() {
        crate::println!("[web] Failed to initialize web system API: {}", e);
    }
    
    crate::println!("[web] Web engine subsystem initialized");
}

