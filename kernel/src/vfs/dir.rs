//! Directory entry for VFS
extern crate alloc;

use super::types::FileType;

/// Directory entry for readdir
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: alloc::string::String,
    pub ino: u64,
    pub file_type: FileType,
}
