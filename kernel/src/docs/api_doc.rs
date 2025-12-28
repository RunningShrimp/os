//! API Documentation Generator
//!
//! This module generates API documentation:
//! - Rustdoc integration
//! - JSON API specs
//! - Markdown documentation
//!
//! Features:
//! - API endpoint documentation
//! - Parameter and response descriptions
//! - Example code
//! - Cross-references

use spin::Mutex;
use core::sync::atomic;
use alloc::collections::BTreeMap;
use core::sync::atomic;
use alloc::sync::Arc;
use core::sync::atomic;
use alloc::string::String;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use alloc::string::{String, ToString};
use core::sync::atomic;

// ============================================================================
// Documentation Constants
// ============================================================================

/// Maximum APIs to document
pub const MAX_APIS: usize = 1 << 10;

/// Maximum parameters per API
pub const MAX_PARAMS: usize = 1 << 6;

// ============================================================================
// API Types
// ============================================================================

/// HTTP method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

/// Parameter type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterType {
    String,
    Integer,
    Float,
    Boolean,
    Object,
    Array,
    File,
}

/// Response status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseStatus {
    Success,
    ClientError,
    ServerError,
    ValidationError,
}

// ============================================================================
// API Documentation
// ============================================================================

/// API endpoint documentation
#[derive(Debug, Clone)]
pub struct ApiDoc {
    pub api_id: String,
    pub name: String,
    pub description: String,
    pub method: HttpMethod,
    pub path: String,
    
    /// Parameters
    pub parameters: Vec<ApiParameter>,
    
    /// Response documentation
    pub responses: Vec<ApiResponse>,
    
    /// Example code
    pub example_code: Option<String>,
    
    /// Authentication required
    pub requires_auth: bool,
    
    /// Rate limited
    pub rate_limited: bool,
    
    /// Deprecated
    pub deprecated: bool,
}

/// API parameter
#[derive(Debug, Clone)]
pub struct ApiParameter {
    pub name: String,
    pub param_type: ParameterType,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
    pub constraints: Option<ParameterConstraints>,
}

/// Parameter constraints
#[derive(Debug, Clone)]
pub struct ParameterConstraints {
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub pattern: Option<String>,
    pub allowed_values: Option<Vec<String>>,
}

/// API response
#[derive(Debug, Clone)]
pub struct ApiResponse {
    pub status_code: u16,
    pub status: ResponseStatus,
    pub description: String,
    pub response_type: String,
    pub example_value: Option<String>,
}

impl ApiDoc {
    pub fn new(api_id: String, name: String, description: String,
                 method: HttpMethod, path: String) -> Self {
        Self {
            api_id,
            name,
            description,
            method,
            path,
            parameters: Vec::new(),
            responses: Vec::new(),
            example_code: None,
            requires_auth: false,
            rate_limited: false,
            deprecated: false,
        }
    }

    pub fn add_parameter(&mut self, param: ApiParameter) {
        self.parameters.push(param);
    }

    pub fn add_response(&mut self, response: ApiResponse) {
        self.responses.push(response);
    }

    pub fn set_example(&mut self, code: String) {
        self.example_code = Some(code);
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::from("## ");
        md.push_str(&self.name);
        md.push_str("\n\n");

        // Description
        md.push_str("**Description:** ");
        md.push_str(&self.description);
        md.push_str("\n\n");

        // Method and path
        md.push_str("**Method:** `");
        md.push_str(match self.method {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        });
        md.push_str("`\n\n");

        md.push_str("**Path:** `");
        md.push_str(&self.path);
        md.push_str("`\n\n");

        // Authentication
        if self.requires_auth {
            md.push_str("**Authentication:** Required\n\n");
        }

        // Rate limiting
        if self.rate_limited {
            md.push_str("**Rate Limit:** Yes\n\n");
        }

        // Deprecated
        if self.deprecated {
            md.push_str("⚠️ **Deprecated**\n\n");
        }

        // Parameters
        if !self.parameters.is_empty() {
            md.push_str("### Parameters\n\n");
            md.push_str("| Name | Type | Required | Description |\n");
            md.push_str("|------|------|----------|------------|\n");
            
            for param in &self.parameters {
                md.push_str("| `");
                md.push_str(&param.name);
                md.push_str("` | ");
                md.push_str(match param.param_type {
                    ParameterType::String => "String",
                    ParameterType::Integer => "Integer",
                    ParameterType::Float => "Float",
                    ParameterType::Boolean => "Boolean",
                    ParameterType::Object => "Object",
                    ParameterType::Array => "Array",
                    ParameterType::File => "File",
                });
                md.push_str(" | ");
                md.push_str(if param.required { "Yes" } else { "No" });
                md.push_str(" | ");
                md.push_str(&param.description);
                md.push_str(" |\n");
            }
            md.push_str("\n");
        }

        // Responses
        if !self.responses.is_empty() {
            md.push_str("### Responses\n\n");
            
            for response in &self.responses {
                md.push_str("#### ");
                md.push_str(&response.status_code.to_string());
                md.push_str(" - ");
                md.push_str(match response.status {
                    ResponseStatus::Success => "Success",
                    ResponseStatus::ClientError => "Client Error",
                    ResponseStatus::ServerError => "Server Error",
                    ResponseStatus::ValidationError => "Validation Error",
                });
                md.push_str("\n\n");
                
                md.push_str("**Description:** ");
                md.push_str(&response.description);
                md.push_str("\n\n");
                
                md.push_str("**Type:** `");
                md.push_str(&response.response_type);
                md.push_str("`\n\n");
            }
            md.push_str("\n");
        }

        // Example code
        if let Some(code) = &self.example_code {
            md.push_str("### Example\n\n");
            md.push_str("```rust\n");
            md.push_str(code);
            md.push_str("\n```\n\n");
        }

        md
    }

    pub fn to_json(&self) -> String {
        let mut json = String::from("{\n");
        json.push_str("  \"api_id\": \"");
        json.push_str(&self.api_id);
        json.push_str("\",\n");
        json.push_str("  \"name\": \"");
        json.push_str(&self.name);
        json.push_str("\",\n");
        json.push_str("  \"description\": \"");
        json.push_str(&self.description);
        json.push_str("\",\n");
        json.push_str("  \"method\": \"");
        json.push_str(match self.method {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        });
        json.push_str("\",\n");
        json.push_str("  \"path\": \"");
        json.push_str(&self.path);
        json.push_str("\",\n");
        json.push_str("  \"requires_auth\": ");
        json.push_str(if self.requires_auth { "true" } else { "false" });
        json.push_str(",\n");
        json.push_str("  \"rate_limited\": ");
        json.push_str(if self.rate_limited { "true" } else { "false" });
        json.push_str(",\n");
        json.push_str("  \"deprecated\": ");
        json.push_str(if self.deprecated { "true" } else { "false" });
        json.push_str("\n");
        json.push_str("}\n");
        json
    }
}

// ============================================================================
// API Documentation Manager
// ============================================================================

/// API documentation manager
pub struct ApiDocManager {
    pub api_docs: Mutex<BTreeMap<String, Arc<ApiDoc>>>>,
    pub next_api_id: AtomicU64,
    pub stats: Mutex<ApiDocStats>,
}

/// API documentation statistics
#[derive(Debug, Clone, Copy)]
pub struct ApiDocStats {
    pub total_apis: usize,
    pub documented_apis: usize,
    pub deprecated_apis: usize,
    pub total_parameters: usize,
}

impl Default for ApiDocStats {
    fn default() -> Self {
        Self {
            total_apis: 0,
            documented_apis: 0,
            deprecated_apis: 0,
            total_parameters: 0,
        }
    }
}

impl ApiDocManager {
    pub fn new() -> Self {
        Self {
            api_docs: Mutex::new(BTreeMap::new()),
            next_api_id: AtomicU64::new(1),
            stats: Mutex::new(ApiDocStats::default()),
        }
    }

    pub fn add_api(&self, api: Arc<ApiDoc>) -> Result<(), String> {
        let mut docs = self.api_docs.lock();
        let api_id = api.api_id.clone();

        if docs.contains_key(&api_id) {
            return Err(alloc::string::String::from("API ") + &api_id.to_string() + alloc::string::String::from(" already documented"));
        }

        docs.insert(api_id, api);
        crate::println!("[api_doc] Added documentation for {}", api_id);

        let mut stats = self.stats.lock();
        stats.total_apis = docs.len();
        stats.documented_apis = docs.len();
        if api.deprecated {
            stats.deprecated_apis += 1;
        }

        Ok(())
    }

    pub fn get_api(&self, api_id: String) -> Option<Arc<ApiDoc>> {
        let docs = self.api_docs.lock();
        docs.get(&api_id).cloned()
    }

    pub fn get_all_apis(&self) -> Vec<Arc<ApiDoc>> {
        let docs = self.api_docs.lock();
        docs.values().cloned().collect()
    }

    pub fn generate_markdown(&self) -> String {
        let mut md = String::from("# API Documentation\n\n");
        md.push_str("This document describes all available APIs in the NOS kernel.\n\n");

        let docs = self.api_docs.lock();
        for doc in docs.values() {
            md.push_str(&doc.to_markdown());
        }

        md
    }

    pub fn generate_openapi_spec(&self) -> String {
        let mut spec = String::from("{\n");
        spec.push_str("  \"openapi\": \"3.0.0\",\n");
        spec.push_str("  \"info\": {\n");
        spec.push_str("    \"title\": \"NOS Kernel API\",\n");
        spec.push_str("    \"version\": \"1.0.0\"\n");
        spec.push_str("  },\n");
        spec.push_str("  \"paths\": {\n");

        let docs = self.api_docs.lock();
        for (i, doc) in docs.values().enumerate() {
            if i > 0 {
                spec.push_str(",\n");
            }

            spec.push_str("    \"");
            spec.push_str(&doc.path);
            spec.push_str("\": {\n");

            spec.push_str("      \"");
            spec.push_str(match doc.method {
                HttpMethod::Get => "get",
                HttpMethod::Post => "post",
                HttpMethod::Put => "put",
                HttpMethod::Delete => "delete",
                HttpMethod::Patch => "patch",
                HttpMethod::Head => "head",
                HttpMethod::Options => "options",
            });
            spec.push_str("\": {\n");

            spec.push_str("        \"summary\": \"");
            spec.push_str(&doc.name);
            spec.push_str("\",\n");

            spec.push_str("        \"description\": \"");
            spec.push_str(&doc.description);
            spec.push_str("\",\n");

            if !doc.parameters.is_empty() {
                spec.push_str("        \"parameters\": [\n");
                for (j, param) in doc.parameters.iter().enumerate() {
                    if j > 0 {
                        spec.push_str(",\n");
                    }
                    spec.push_str("          {\n");
                    spec.push_str("            \"name\": \"");
                    spec.push_str(&param.name);
                    spec.push_str("\",\n");
                    spec.push_str("            \"in\": \"query\",\n");
                    spec.push_str("            \"required\": ");
                    spec.push_str(if param.required { "true" } else { "false" });
                    spec.push_str(",\n");
                    spec.push_str("            \"schema\": {\n");
                    spec.push_str("              \"type\": \"");
                    spec.push_str(match param.param_type {
                        ParameterType::String => "string",
                        ParameterType::Integer => "integer",
                        ParameterType::Float => "number",
                        ParameterType::Boolean => "boolean",
                        ParameterType::Object => "object",
                        ParameterType::Array => "array",
                        ParameterType::File => "string",  // File as string with format
                    });
                    spec.push_str("\"\n");
                    spec.push_str("            }\n");
                    spec.push_str("          }\n");
                }
                spec.push_str("        ],\n");
            }

            spec.push_str("        \"responses\": {\n");
            for (j, response) in doc.responses.iter().enumerate() {
                if j > 0 {
                    spec.push_str(",\n");
                }
                spec.push_str("          \"");
                spec.push_str(&response.status_code.to_string());
                spec.push_str("\": {\n");
                spec.push_str("            \"description\": \"");
                spec.push_str(&response.description);
                spec.push_str("\"\n");
                spec.push_str("          }\n");
            }
            spec.push_str("        }\n");

            spec.push_str("      }\n");
            spec.push_str("    }\n");
        }

        spec.push_str("  }\n");
        spec.push_str("}\n");
        spec
    }

    pub fn get_stats(&self) -> ApiDocStats {
        let mut stats = self.stats.lock();
        stats.total_apis = self.api_docs.lock().len();
        *stats
    }
}
