//! User Manual Generator
//!
//! This module generates user documentation:
//! - Installation guides
//! - Usage tutorials
//! - Troubleshooting guides
//! - FAQ
//!
//! Features:
//! - Step-by-step tutorials
//! - Video links (placeholder)
//! - Screenshots (placeholder)
//! - Searchable FAQ

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
// Manual Constants
// ============================================================================

/// Maximum sections in manual
pub const MAX_MANUAL_SECTIONS: usize = 1 << 8;

/// Maximum FAQs
pub const MAX_FAQS: usize = 1 << 10;

// ============================================================================
// Manual Section
// ============================================================================

/// Manual section type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionType {
    /// Getting started
    GettingStarted,
    
    /// Installation
    Installation,
    
    /// Configuration
    Configuration,
    
    /// Usage guide
    UsageGuide,
    
    /// API reference
    ApiReference,
    
    /// Troubleshooting
    Troubleshooting,
    
    /// FAQ
    Faq,
    
    /// Release notes
    ReleaseNotes,
    
    /// Custom section
    Custom(String),
}

/// Manual section
#[derive(Debug, Clone)]
pub struct ManualSection {
    pub section_id: String,
    pub title: String,
    pub section_type: SectionType,
    pub content: String,
    pub subsections: Vec<ManualSection>,
    pub code_examples: Vec<CodeExample>,
    pub screenshots: Vec<String>,
    pub video_links: Vec<String>,
}

/// Code example
#[derive(Debug, Clone)]
pub struct CodeExample {
    pub language: String,
    pub description: String,
    pub code: String,
}

impl ManualSection {
    pub fn new(section_id: String, title: String, section_type: SectionType) -> Self {
        Self {
            section_id,
            title,
            section_type,
            content: String::new(),
            subsections: Vec::new(),
            code_examples: Vec::new(),
            screenshots: Vec::new(),
            video_links: Vec::new(),
        }
    }

    pub fn add_subsection(&mut self, subsection: ManualSection) {
        self.subsections.push(subsection);
    }

    pub fn add_code_example(&mut self, example: CodeExample) {
        self.code_examples.push(example);
    }

    pub fn add_screenshot(&mut self, url: String) {
        self.screenshots.push(url);
    }

    pub fn add_video_link(&mut self, url: String) {
        self.video_links.push(url);
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::from("# ");
        md.push_str(&self.title);
        md.push_str("\n\n");

        // Content
        if !self.content.is_empty() {
            md.push_str(&self.content);
            md.push_str("\n\n");
        }

        // Code examples
        if !self.code_examples.is_empty() {
            md.push_str("## Code Examples\n\n");
            
            for example in &self.code_examples {
                md.push_str(&example.description);
                md.push_str("\n\n```");
                md.push_str(&example.language);
                md.push_str("\n");
                md.push_str(&example.code);
                md.push_str("\n```\n\n");
            }
        }

        // Screenshots
        if !self.screenshots.is_empty() {
            md.push_str("## Screenshots\n\n");
            
            for url in &self.screenshots {
                md.push_str("![Screenshot](");
                md.push_str(url);
                md.push_str(")\n\n");
            }
        }

        // Video links
        if !self.video_links.is_empty() {
            md.push_str("## Video Tutorials\n\n");
            
            for (i, url) in self.video_links.iter().enumerate() {
                md.push_str(&alloc::format!("{}. [Video]({})\n\n", i + 1, url));
            }
        }

        // Subsections
        if !self.subsections.is_empty() {
            for subsection in &self.subsections {
                md.push_str(&subsection.to_markdown());
            }
        }

        md
    }
}

// ============================================================================
// FAQ Entry
// ============================================================================

/// FAQ entry
#[derive(Debug, Clone)]
pub struct FaqEntry {
    pub faq_id: String,
    pub question: String,
    pub answer: String,
    pub category: String,
    pub tags: Vec<String>,
    pub related_faqs: Vec<String>,
}

impl FaqEntry {
    pub fn new(faq_id: String, question: String, answer: String, category: String) -> Self {
        Self {
            faq_id,
            question,
            answer,
            category,
            tags: Vec::new(),
            related_faqs: Vec::new(),
        }
    }

    pub fn add_tag(&mut self, tag: String) {
        self.tags.push(tag);
    }

    pub fn add_related_faq(&mut self, faq_id: String) {
        self.related_faqs.push(faq_id);
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::from("### ");
        md.push_str(&self.question);
        md.push_str("\n\n");

        md.push_str(&self.answer);
        md.push_str("\n\n");

        // Tags
        if !self.tags.is_empty() {
            md.push_str("**Tags:** ");
            for (i, tag) in self.tags.iter().enumerate() {
                if i > 0 {
                    md.push_str(", ");
                }
                md.push_str(tag);
            }
            md.push_str("\n\n");
        }

        // Related FAQs
        if !self.related_faqs.is_empty() {
            md.push_str("**Related:** ");
            for (i, faq_id) in self.related_faqs.iter().enumerate() {
                if i > 0 {
                    md.push_str(", ");
                }
                md.push_str(faq_id);
            }
            md.push_str("\n\n");
        }

        md
    }
}

// ============================================================================
// User Manual Manager
// ============================================================================

/// User manual manager
pub struct UserManualManager {
    pub sections: Mutex<BTreeMap<String, Arc<ManualSection>>>>,
    pub faqs: Mutex<Vec<Arc<FaqEntry>>>>,
    pub categories: Mutex<Vec<String>>,
    pub next_section_id: AtomicU64,
    pub next_faq_id: AtomicU64,
    pub stats: Mutex<UserManualStats>,
}

/// User manual statistics
#[derive(Debug, Clone, Copy)]
pub struct UserManualStats {
    pub total_sections: usize,
    pub total_faqs: usize,
    pub total_categories: usize,
    pub total_code_examples: usize,
}

impl Default for UserManualStats {
    fn default() -> Self {
        Self {
            total_sections: 0,
            total_faqs: 0,
            total_categories: 0,
            total_code_examples: 0,
        }
    }
}

impl UserManualManager {
    pub fn new() -> Self {
        Self {
            sections: Mutex::new(BTreeMap::new()),
            faqs: Mutex::new(Vec::new()),
            categories: Mutex::new(Vec::new()),
            next_section_id: AtomicU64::new(1),
            next_faq_id: AtomicU64::new(1),
            stats: Mutex::new(UserManualStats::default()),
        }
    }

    pub fn add_section(&self, section: Arc<ManualSection>) -> Result<(), String> {
        let mut sections = self.sections.lock();
        let section_id = section.section_id.clone();

        if sections.contains_key(&section_id) {
            return Err(alloc::string::String::from("Section ") + &section_id.to_string() + alloc::string::String::from(" already exists"));
        }

        sections.insert(section_id, section);
        crate::println!("[user_manual] Added section: {}", section_id);

        let mut stats = self.stats.lock();
        stats.total_sections = sections.len();

        Ok(())
    }

    pub fn add_faq(&self, faq: Arc<FaqEntry>) -> Result<(), String> {
        let mut faqs = self.faqs.lock();
        let mut categories = self.categories.lock();

        // Add category if new
        if !categories.contains(&faq.category) {
            categories.push(faq.category.clone());
        }

        faqs.push(faq);
        crate::println!("[user_manual] Added FAQ: {}", faq.faq_id);

        let mut stats = self.stats.lock();
        stats.total_faqs = faqs.len();
        stats.total_categories = categories.len();

        Ok(())
    }

    pub fn get_section(&self, section_id: String) -> Option<Arc<ManualSection>> {
        let sections = self.sections.lock();
        sections.get(&section_id).cloned()
    }

    pub fn get_sections_by_type(&self, section_type: SectionType) -> Vec<Arc<ManualSection>> {
        let sections = self.sections.lock();
        sections.values()
            .filter(|s| s.section_type == section_type)
            .cloned()
            .collect()
    }

    pub fn search_faqs(&self, query: String) -> Vec<Arc<FaqEntry>> {
        let faqs = self.faqs.lock();
        let query_lower = query.to_lowercase();

        faqs.iter()
            .filter(|faq| {
                faq.question.to_lowercase().contains(&query_lower) ||
                faq.answer.to_lowercase().contains(&query_lower) ||
                faq.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }

    pub fn get_faqs_by_category(&self, category: String) -> Vec<Arc<FaqEntry>> {
        let faqs = self.faqs.lock();
        faqs.iter()
            .filter(|faq| faq.category == category)
            .cloned()
            .collect()
    }

    pub fn generate_user_manual(&self) -> String {
        let mut md = String::from("# NOS Operating System - User Manual\n\n");
        md.push_str("Welcome to NOS, a modern operating system for cloud-native applications.\n\n");

        // Table of contents
        md.push_str("## Table of Contents\n\n");
        
        let sections = self.sections.lock();
        for section in sections.values() {
            md.push_str("- [");
            md.push_str(&section.title);
            md.push_str("](#");
            md.push_str(&section.section_id);
            md.push_str(")\n");
        }
        md.push_str("\n");

        // Sections
        for section in sections.values() {
            md.push_str(&section.to_markdown());
        }

        // FAQ section
        let faqs = self.faqs.lock();
        if !faqs.is_empty() {
            md.push_str("## Frequently Asked Questions\n\n");

            let categories = self.categories.lock();
            for category in categories {
                md.push_str("### ");
                md.push_str(category);
                md.push_str("\n\n");

                for faq in faqs.iter().filter(|f| f.category == *category) {
                    md.push_str(&faq.to_markdown());
                }
            }
        }

        md
    }

    pub fn get_stats(&self) -> UserManualStats {
        let mut stats = self.stats.lock();
        stats.total_sections = self.sections.lock().len();
        stats.total_faqs = self.faqs.lock().len();
        stats.total_code_examples = self.sections.lock()
            .values()
            .map(|s| s.code_examples.len())
            .sum();
        *stats
    }
}
