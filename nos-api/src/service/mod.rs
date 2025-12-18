//! Service interface module

pub mod interface;
pub mod registry;
pub mod discovery;

// Re-export commonly used items
pub use interface::*;
pub use registry::*;
// Use specific imports to avoid ambiguous glob re-exports
pub use discovery::DefaultServiceDiscovery;
pub use interface::ServiceDiscovery;