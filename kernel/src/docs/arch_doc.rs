//! Architecture Documentation
//!
//! This module generates architecture documentation:
//! - Component diagrams
//! - Data flow documentation
//! - Module dependencies
//! - Design patterns
//!
//! Features:
//! - ASCII art diagrams
//! - Markdown architecture docs
//! - Design pattern catalog
//! - Best practices

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

/// Maximum components to document
pub const MAX_COMPONENTS: usize = 1 << 8;

/// Maximum design patterns
pub const MAX_PATTERNS: usize = 1 << 6;

// ============================================================================
// Component Documentation
// ============================================================================

/// Component type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    /// Kernel core
    KernelCore,
    
    /// Scheduler
    Scheduler,
    
    /// Memory manager
    MemoryManager,
    
    /// File system
    FileSystem,
    
    /// Network stack
    NetworkStack,
    
    /// Security module
    SecurityModule,
    
    /// I/O subsystem
    IoSubsystem,
    
    /// User space
    UserSpace,
}

/// Component documentation
#[derive(Debug, Clone)]
pub struct ComponentDoc {
    pub component_id: String,
    pub name: String,
    pub component_type: ComponentType,
    pub description: String,
    pub responsibilities: Vec<String>,
    pub interfaces: Vec<String>,
    pub dependencies: Vec<String>,
    pub subcomponents: Vec<String>,
    pub design_patterns: Vec<String>,
}

impl ComponentDoc {
    pub fn new(component_id: String, name: String, component_type: ComponentType) -> Self {
        Self {
            component_id,
            name,
            component_type,
            description: String::new(),
            responsibilities: Vec::new(),
            interfaces: Vec::new(),
            dependencies: Vec::new(),
            subcomponents: Vec::new(),
            design_patterns: Vec::new(),
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::from("## ");
        md.push_str(&self.name);
        md.push_str("\n\n");

        // Type
        md.push_str("**Type:** `");
        md.push_str(match self.component_type {
            ComponentType::KernelCore => "KernelCore",
            ComponentType::Scheduler => "Scheduler",
            ComponentType::MemoryManager => "MemoryManager",
            ComponentType::FileSystem => "FileSystem",
            ComponentType::NetworkStack => "NetworkStack",
            ComponentType::SecurityModule => "SecurityModule",
            ComponentType::IoSubsystem => "IoSubsystem",
            ComponentType::UserSpace => "UserSpace",
        });
        md.push_str("`\n\n");

        // Description
        md.push_str("**Description:** ");
        md.push_str(&self.description);
        md.push_str("\n\n");

        // Responsibilities
        if !self.responsibilities.is_empty() {
            md.push_str("### Responsibilities\n\n");
            for resp in &self.responsibilities {
                md.push_str("- ");
                md.push_str(resp);
                md.push_str("\n");
            }
            md.push_str("\n");
        }

        // Interfaces
        if !self.interfaces.is_empty() {
            md.push_str("### Interfaces\n\n");
            for iface in &self.interfaces {
                md.push_str("- `");
                md.push_str(iface);
                md.push_str("`\n");
            }
            md.push_str("\n");
        }

        // Dependencies
        if !self.dependencies.is_empty() {
            md.push_str("### Dependencies\n\n");
            for dep in &self.dependencies {
                md.push_str("- ");
                md.push_str(dep);
                md.push_str("\n");
            }
            md.push_str("\n");
        }

        // Subcomponents
        if !self.subcomponents.is_empty() {
            md.push_str("### Subcomponents\n\n");
            for sub in &self.subcomponents {
                md.push_str("- ");
                md.push_str(sub);
                md.push_str("\n");
            }
            md.push_str("\n");
        }

        // Design patterns
        if !self.design_patterns.is_empty() {
            md.push_str("### Design Patterns\n\n");
            for pattern in &self.design_patterns {
                md.push_str("- ");
                md.push_str(pattern);
                md.push_str("\n");
            }
            md.push_str("\n");
        }

        md
    }
}

// ============================================================================
// Data Flow Documentation
// ============================================================================

/// Data flow diagram
#[derive(Debug, Clone)]
pub struct DataFlowDoc {
    pub flow_id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<FlowStep>,
    pub data_structures: Vec<String>,
}

/// Flow step
#[derive(Debug, Clone)]
pub struct FlowStep {
    pub step_id: String,
    pub description: String,
    pub component: String,
    pub input_data: Option<String>,
    pub output_data: Option<String>,
}

impl DataFlowDoc {
    pub fn new(flow_id: String, name: String, description: String) -> Self {
        Self {
            flow_id,
            name,
            description,
            steps: Vec::new(),
            data_structures: Vec::new(),
        }
    }

    pub fn to_diagram(&self) -> String {
        let mut diagram = String::from("```\n");
        diagram.push_str(&self.name);
        diagram.push_str("\n\n");

        for (i, step) in self.steps.iter().enumerate() {
            diagram.push_str(&step.step_id);
            diagram.push_str(": ");
            diagram.push_str(&step.description);
            diagram.push_str(" (");
            diagram.push_str(&step.component);
            diagram.push_str(")\n");

            if i < self.steps.len() - 1 {
                diagram.push_str("  |\n");
                diagram.push_str("  v\n");
            }
        }

        diagram.push_str("```\n");
        diagram
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::from("## ");
        md.push_str(&self.name);
        md.push_str("\n\n");

        // Description
        md.push_str("**Description:** ");
        md.push_str(&self.description);
        md.push_str("\n\n");

        // Diagram
        md.push_str("### Flow Diagram\n\n");
        md.push_str(&self.to_diagram());
        md.push_str("\n");

        // Data structures
        if !self.data_structures.is_empty() {
            md.push_str("### Data Structures\n\n");
            for ds in &self.data_structures {
                md.push_str("- `");
                md.push_str(ds);
                md.push_str("`\n");
            }
            md.push_str("\n");
        }

        md
    }
}

// ============================================================================
// Design Pattern Documentation
// ============================================================================

/// Design pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignPattern {
    /// Singleton pattern
    Singleton,
    
    /// Factory pattern
    Factory,
    
    /// Observer pattern
    Observer,
    
    /// Strategy pattern
    Strategy,
    
    /// Adapter pattern
    Adapter,
    
    /// Decorator pattern
    Decorator,
    
    /// Proxy pattern
    Proxy,
    
    /// Command pattern
    Command,
}

/// Pattern documentation
#[derive(Debug, Clone)]
pub struct PatternDoc {
    pub pattern_id: String,
    pub name: String,
    pub pattern: DesignPattern,
    pub description: String,
    pub use_case: String,
    pub implementation_notes: Vec<String>,
    pub components_using: Vec<String>,
}

impl PatternDoc {
    pub fn new(pattern_id: String, name: String, pattern: DesignPattern) -> Self {
        Self {
            pattern_id,
            name,
            pattern,
            description: String::new(),
            use_case: String::new(),
            implementation_notes: Vec::new(),
            components_using: Vec::new(),
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::from("## ");
        md.push_str(&self.name);
        md.push_str("\n\n");

        // Pattern
        md.push_str("**Pattern:** `");
        md.push_str(match self.pattern {
            DesignPattern::Singleton => "Singleton",
            DesignPattern::Factory => "Factory",
            DesignPattern::Observer => "Observer",
            DesignPattern::Strategy => "Strategy",
            DesignPattern::Adapter => "Adapter",
            DesignPattern::Decorator => "Decorator",
            DesignPattern::Proxy => "Proxy",
            DesignPattern::Command => "Command",
        });
        md.push_str("`\n\n");

        // Description
        md.push_str("**Description:** ");
        md.push_str(&self.description);
        md.push_str("\n\n");

        // Use case
        md.push_str("**Use Case:** ");
        md.push_str(&self.use_case);
        md.push_str("\n\n");

        // Implementation notes
        if !self.implementation_notes.is_empty() {
            md.push_str("### Implementation Notes\n\n");
            for note in &self.implementation_notes {
                md.push_str("- ");
                md.push_str(note);
                md.push_str("\n");
            }
            md.push_str("\n");
        }

        // Components using
        if !self.components_using.is_empty() {
            md.push_str("### Components Using\n\n");
            for comp in &self.components_using {
                md.push_str("- ");
                md.push_str(comp);
                md.push_str("\n");
            }
            md.push_str("\n");
        }

        md
    }
}

// ============================================================================
// Architecture Documentation Manager
// ============================================================================

/// Architecture documentation manager
pub struct ArchitectureDocManager {
    pub components: Mutex<BTreeMap<String, Arc<ComponentDoc>>>>,
    pub data_flows: Mutex<BTreeMap<String, Arc<DataFlowDoc>>>>,
    pub patterns: Mutex<BTreeMap<String, Arc<PatternDoc>>>>,
    pub next_doc_id: AtomicU64,
    pub stats: Mutex<ArchitectureDocStats>,
}

/// Architecture documentation statistics
#[derive(Debug, Clone, Copy)]
pub struct ArchitectureDocStats {
    pub total_components: usize,
    pub total_flows: usize,
    pub total_patterns: usize,
    pub documented_items: usize,
}

impl Default for ArchitectureDocStats {
    fn default() -> Self {
        Self {
            total_components: 0,
            total_flows: 0,
            total_patterns: 0,
            documented_items: 0,
        }
    }
}

impl ArchitectureDocManager {
    pub fn new() -> Self {
        Self {
            components: Mutex::new(BTreeMap::new()),
            data_flows: Mutex::new(BTreeMap::new()),
            patterns: Mutex::new(BTreeMap::new()),
            next_doc_id: AtomicU64::new(1),
            stats: Mutex::new(ArchitectureDocStats::default()),
        }
    }

    pub fn add_component(&self, component: Arc<ComponentDoc>) -> Result<(), String> {
        let mut components = self.components.lock();
        let comp_id = component.component_id.clone();

        if components.contains_key(&comp_id) {
            return Err(alloc::string::String::from("Component ") + &comp_id.to_string() + alloc::string::String::from(" already documented"));
        }

        components.insert(comp_id, component);
        crate::println!("[arch_doc] Added component: {}", comp_id);

        let mut stats = self.stats.lock();
        stats.total_components = components.len();

        Ok(())
    }

    pub fn add_data_flow(&self, flow: Arc<DataFlowDoc>) -> Result<(), String> {
        let mut flows = self.data_flows.lock();
        let flow_id = flow.flow_id.clone();

        if flows.contains_key(&flow_id) {
            return Err(alloc::string::String::from("Data flow ") + &flow_id.to_string() + alloc::string::String::from(" already documented"));
        }

        flows.insert(flow_id, flow);
        crate::println!("[arch_doc] Added data flow: {}", flow_id);

        let mut stats = self.stats.lock();
        stats.total_flows = flows.len();

        Ok(())
    }

    pub fn add_pattern(&self, pattern: Arc<PatternDoc>) -> Result<(), String> {
        let mut patterns = self.patterns.lock();
        let pattern_id = pattern.pattern_id.clone();

        if patterns.contains_key(&pattern_id) {
            return Err(alloc::string::String::from("Pattern ") + &pattern_id.to_string() + alloc::string::String::from(" already documented"));
        }

        patterns.insert(pattern_id, pattern);
        crate::println!("[arch_doc] Added pattern: {}", pattern_id);

        let mut stats = self.stats.lock();
        stats.total_patterns = patterns.len();

        Ok(())
    }

    pub fn generate_architecture_doc(&self) -> String {
        let mut md = String::from("# NOS Kernel Architecture\n\n");
        md.push_str("This document describes the architecture of the NOS operating system.\n\n");

        // Components section
        md.push_str("## Components\n\n");
        let components = self.components.lock();
        for component in components.values() {
            md.push_str(&component.to_markdown());
        }

        // Data flows section
        md.push_str("## Data Flows\n\n");
        let flows = self.data_flows.lock();
        for flow in flows.values() {
            md.push_str(&flow.to_markdown());
        }

        // Design patterns section
        md.push_str("## Design Patterns\n\n");
        let patterns = self.patterns.lock();
        for pattern in patterns.values() {
            md.push_str(&pattern.to_markdown());
        }

        md
    }

    pub fn get_stats(&self) -> ArchitectureDocStats {
        let mut stats = self.stats.lock();
        stats.total_components = self.components.lock().len();
        stats.total_flows = self.data_flows.lock().len();
        stats.total_patterns = self.patterns.lock().len();
        stats.documented_items = stats.total_components + stats.total_flows + stats.total_patterns;
        *stats
    }
}
