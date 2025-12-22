//! Application Layer - Use case orchestration
//!
//! Coordinates domain and infrastructure to implement bootloader use cases.
//! Implements the main boot system choreography without containing domain logic.

pub mod boot_orchestrator;

pub use boot_orchestrator::BootApplicationService;
