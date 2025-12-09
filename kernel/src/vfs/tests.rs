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

    /// Test ProcFS servicestats node
    pub fn test_procfs_servicestats() -> TestResult {
        let mut f = match vfs().open("/proc/servicestats", 0) {
            Ok(f) => f,
            Err(e) => return Err(alloc::format!("open servicestats failed: {:?}", e)),
        };
        let mut buf = [0u8; 512];
        let n = match f.read(buf.as_mut_ptr() as usize, buf.len()) {
            Ok(n) => n,
            Err(e) => return Err(alloc::format!("read servicestats failed: {:?}", e)),
        };
        test_assert!(n > 0);
        let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
        test_assert!(s.contains("Service Registry Stats"));
        Ok(())
    }

    /// Test ProcFS features node
    pub fn test_procfs_features() -> TestResult {
        let mut f = match vfs().open("/proc/features", 0) {
            Ok(f) => f,
            Err(e) => return Err(alloc::format!("open features failed: {:?}", e)),
        };
        let mut buf = [0u8; 256];
        let n = match f.read(buf.as_mut_ptr() as usize, buf.len()) {
            Ok(n) => n,
            Err(e) => return Err(alloc::format!("read features failed: {:?}", e)),
        };
        test_assert!(n > 0);
        let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
        test_assert!(s.contains("Kernel Feature Flags"));
        Ok(())
    }

    pub fn test_procfs_servicehealth() -> TestResult {
        let mut f = match vfs().open("/proc/servicehealth", 0) {
            Ok(f) => f,
            Err(e) => return Err(alloc::format!("open servicehealth failed: {:?}", e)),
        };
        let mut buf = [0u8; 256];
        let n = match f.read(buf.as_mut_ptr() as usize, buf.len()) {
            Ok(n) => n,
            Err(e) => return Err(alloc::format!("read servicehealth failed: {:?}", e)),
        };
        test_assert!(n > 0);
        let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
        test_assert!(s.contains("Service Health"));
        Ok(())
    }

    pub fn test_procfs_initlazy() -> TestResult {
        let mut f = match vfs().open("/proc/initlazy", 0) {
            Ok(f) => f,
            Err(e) => return Err(alloc::format!("open initlazy failed: {:?}", e)),
        };
        let mut buf = [0u8; 256];
        let n = match f.read(buf.as_mut_ptr() as usize, buf.len()) {
            Ok(n) => n,
            Err(e) => return Err(alloc::format!("read initlazy failed: {:?}", e)),
        };
        test_assert!(n > 0);
        let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
        // Accept either executed or disabled based on feature
        test_assert!(s.contains("Lazy Init") || s.contains("lazy_init"));
        Ok(())
    }

    pub fn test_procfs_processstats() -> TestResult {
        let mut f = match vfs().open("/proc/processstats", 0) {
            Ok(f) => f,
            Err(e) => return Err(alloc::format!("open processstats failed: {:?}", e)),
        };
        let mut buf = [0u8; 256];
        let n = match f.read(buf.as_mut_ptr() as usize, buf.len()) {
            Ok(n) => n,
            Err(e) => return Err(alloc::format!("read processstats failed: {:?}", e)),
        };
        test_assert!(n > 0);
        let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
        test_assert!(s.contains("Process Stats"));
        Ok(())
    }

    pub fn test_procfs_perfsummary() -> TestResult {
        let mut f = match vfs().open("/proc/perfsummary", 0) {
            Ok(f) => f,
            Err(e) => return Err(alloc::format!("open perfsummary failed: {:?}", e)),
        };
        let mut buf = [0u8; 512];
        let n = match f.read(buf.as_mut_ptr() as usize, buf.len()) {
            Ok(n) => n,
            Err(e) => return Err(alloc::format!("read perfsummary failed: {:?}", e)),
        };
        test_assert!(n > 0);
        let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
        test_assert!(s.contains("Performance Summary") || s.contains("Service Registry Stats"));
        Ok(())
    }

    pub fn test_procfs_perfmonitor() -> TestResult {
        let mut f = match vfs().open("/proc/perfmonitor", 0) {
            Ok(f) => f,
            Err(e) => return Err(alloc::format!("open perfmonitor failed: {:?}", e)),
        };
        let mut buf = [0u8; 512];
        let n = match f.read(buf.as_mut_ptr() as usize, buf.len()) {
            Ok(n) => n,
            Err(e) => return Err(alloc::format!("read perfmonitor failed: {:?}", e)),
        };
        test_assert!(n > 0);
        let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
        test_assert!(s.contains("NOS系统性能报告") || s.contains("Performance"));
        Ok(())
    }

    pub fn test_procfs_heapstats() -> TestResult {
        let mut f = match vfs().open("/proc/heapstats", 0) {
            Ok(f) => f,
            Err(e) => return Err(alloc::format!("open heapstats failed: {:?}", e)),
        };
        let mut buf = [0u8; 512];
        let n = match f.read(buf.as_mut_ptr() as usize, buf.len()) {
            Ok(n) => n,
            Err(e) => return Err(alloc::format!("read heapstats failed: {:?}", e)),
        };
        test_assert!(n > 0);
        let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
        test_assert!(s.contains("Heap Stats"));
        Ok(())
    }

    pub fn test_procfs_timesummary() -> TestResult {
        let mut f = match vfs().open("/proc/timesummary", 0) {
            Ok(f) => f,
            Err(e) => return Err(alloc::format!("open timesummary failed: {:?}", e)),
        };
        let mut buf = [0u8; 512];
        let n = match f.read(buf.as_mut_ptr() as usize, buf.len()) {
            Ok(n) => n,
            Err(e) => return Err(alloc::format!("read timesummary failed: {:?}", e)),
        };
        test_assert!(n >= 0);
        let s = core::str::from_utf8(&buf[..core::cmp::min(n, buf.len())]).unwrap_or("");
        test_assert!(s.contains("Boot Timeline Summary") || s.contains("Timeline"));
        Ok(())
    }

    pub fn test_procfs_timeline() -> TestResult {
        let mut f = match vfs().open("/proc/timeline", 0) {
            Ok(f) => f,
            Err(e) => return Err(alloc::format!("open timeline failed: {:?}", e)),
        };
        let mut buf = [0u8; 512];
        let n = match f.read(buf.as_mut_ptr() as usize, buf.len()) {
            Ok(n) => n,
            Err(e) => return Err(alloc::format!("read timeline failed: {:?}", e)),
        };
        test_assert!(n >= 0);
        let s = core::str::from_utf8(&buf[..core::cmp::min(n, buf.len())]).unwrap_or("");
        test_assert!(s.contains("Timeline") || s.contains("Boot"));
        Ok(())
    }
}

// ============================================================================
// Process tests
// ============================================================================
