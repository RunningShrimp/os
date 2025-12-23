//! File system services
//!
//! This module provides file system related services.

use nos_api::Result;
use crate::core::{Service, ServiceStatus};
use alloc::string::String;
use alloc::string::ToString;
use alloc::boxed::Box;

/// File system service
pub struct FileSystemService {
    name: String,
    mount_point: String,
    fs_type: String,
    status: ServiceStatus,
}

impl FileSystemService {
    /// Create a new file system service
    pub fn new(name: &str, mount_point: &str, fs_type: &str) -> Self {
        Self {
            name: name.to_string(),
            mount_point: mount_point.to_string(),
            fs_type: fs_type.to_string(),
            status: ServiceStatus::Stopped,
        }
    }

    /// Get the mount point
    pub fn mount_point(&self) -> &str {
        &self.mount_point
    }

    /// Get the file system type
    pub fn fs_type(&self) -> &str {
        &self.fs_type
    }
}

impl Service for FileSystemService {
    fn start(&self) -> Result<()> {
        // TODO: Implement actual file system mounting
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        // TODO: Implement actual file system unmounting
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn service_type(&self) -> u32 {
        crate::types::service_type::FILE_SYSTEM
    }

    fn status(&self) -> ServiceStatus {
        self.status
    }
}

/// Register file system services
pub fn register_fs_services() -> Result<()> {
    use crate::registry;
    
    let mut registry = registry::get_registry().lock();
    
    // Register root file system
    let root_fs = FileSystemService::new("root_fs", "/", "ext4");
    registry.register("root_fs", Box::new(root_fs))?;
    
    // Register tmp file system
    let tmp_fs = FileSystemService::new("tmp_fs", "/tmp", "tmpfs");
    registry.register("tmp_fs", Box::new(tmp_fs))?;
    
    // Register proc file system
    let proc_fs = FileSystemService::new("proc_fs", "/proc", "procfs");
    registry.register("proc_fs", Box::new(proc_fs))?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fs_service() {
        let service = FileSystemService::new("test_fs", "/test", "ext4");
        
        assert_eq!(service.name(), "test_fs");
        assert_eq!(service.mount_point(), "/test");
        assert_eq!(service.fs_type(), "ext4");
        assert_eq!(service.service_type(), crate::types::service_type::FILE_SYSTEM);
        assert_eq!(service.status(), ServiceStatus::Stopped);
    }
}