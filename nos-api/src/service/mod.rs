//! Service interface module

pub mod discovery;
pub mod interface;
pub mod registry;

// Re-export commonly used items
// Use specific imports to avoid ambiguous glob re-exports
pub use discovery::DefaultServiceDiscovery;
pub use interface::{ServiceDiscovery, *};
pub use registry::*;
