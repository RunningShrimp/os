//! VFS Tests
//!
//! Tests for Virtual File System

#[cfg(feature = "kernel_tests")]
pub mod vfs_tests {
    use crate::tests::{TestResult, test_assert_eq, test_assert};
    use crate::vfs::{FileMode, vfs};

    /// Test VFS create and write
    pub fn test_vfs_create_write() -> TestResult {
        let path = "/test_create";
        let mut f = match vfs().create(path, FileMode::new(FileMode::S_IFREG | 0o644)) {
            Ok(f) => f,
            Err(e) => return Err(alloc::format!("create failed: {:?}", e)),
        };
        
        let msg = b"test content";
        match f.write(msg.as_ptr() as usize, msg.len()) {
            Ok(n) => test_assert_eq!(n, msg.len()),
            Err(e) => return Err(alloc::format!("write failed: {:?}", e)),
        }
        
        // Cleanup
        let _ = vfs().unlink(path);
        Ok(())
    }

    /// Test VFS read
    pub fn test_vfs_read() -> TestResult {
        let path = "/test_read";
        let msg = b"hello world";
        
        // Create and write
        let mut f = match vfs().create(path, FileMode::new(FileMode::S_IFREG | 0o644)) {
            Ok(f) => f,
            Err(e) => return Err(alloc::format!("create failed: {:?}", e)),
        };
        let _ = f.write(msg.as_ptr() as usize, msg.len());
        
        // Open and read
        let mut f2 = match vfs().open(path, 0) {
            Ok(f) => f,
            Err(e) => return Err(alloc::format!("open failed: {:?}", e)),
        };
        
        let mut buf = [0u8; 32];
        match f2.read(buf.as_mut_ptr() as usize, msg.len()) {
            Ok(n) => {
                test_assert_eq!(n, msg.len());
                test_assert_eq!(&buf[..msg.len()], msg);
            }
            Err(e) => return Err(alloc::format!("read failed: {:?}", e)),
        }
        
        // Cleanup
        let _ = vfs().unlink(path);
        Ok(())
    }

    /// Test VFS mkdir
    pub fn test_vfs_mkdir() -> TestResult {
        let path = "/test_dir";
        
        match vfs().mkdir(path, FileMode::new(FileMode::S_IFDIR | 0o755)) {
            Ok(_) => {}
            Err(e) => return Err(alloc::format!("mkdir failed: {:?}", e)),
        }
        
        // Verify it's a directory
        match vfs().stat(path) {
            Ok(attr) => {
                let mode = FileMode::new(attr.mode.0);
                test_assert!(mode.file_type() == crate::vfs::FileType::Directory);
            }
            Err(e) => return Err(alloc::format!("stat failed: {:?}", e)),
        }
        
        // Cleanup
        let _ = vfs().rmdir(path);
        Ok(())
    }
}

// ============================================================================
// Process tests
// ============================================================================

