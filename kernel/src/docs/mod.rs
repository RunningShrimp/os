#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! Documentation Generation
//!
//! This module implements documentation generation for NOS:
//! - API documentation
//! - Architecture documentation
//! - User manual
//!
//! Features:
//! - Markdown documentation generation
//! - OpenAPI specification generation
//! - Architecture diagrams
//! - Step-by-step tutorials
//! - Searchable FAQ

pub mod api_doc;
pub mod arch_doc;
pub mod user_manual;

// Re-export documentation types
pub use api_doc::*;
pub use arch_doc::*;
pub use user_manual::*;
