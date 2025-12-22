//! Boot Stage - Orchestration, flow control, manager, executor (P0+)

pub mod boot_manager;
pub mod boot_executor;
pub mod boot_preparation;
pub mod boot_validation;
pub mod boot_recovery;
pub mod boot_flow;
pub mod boot_finalization;
pub mod boot_handoff;
pub mod boot_config;
pub mod boot_control;
pub mod boot_diagnostic;
pub mod boot_diagnostics;
pub mod boot_menu;
pub mod boot_params;
pub mod boot_result;
pub mod boot_security;
pub mod boot_stack;
pub mod boot_summary;
pub mod boot_checklist;
pub mod advanced_boot_protocol;
pub mod boot_loader_integration;
pub mod boot_loader;
pub mod boot_integration_tests;
pub mod fallback_boot;
pub mod boot_coordinator;

// Re-export commonly used types
pub use boot_config::BootConfig;
pub use boot_coordinator::BootCoordinator;
