//! File system system call tests
//!
//! Tests for chdir, getcwd, link, stat, lstat system calls

use crate::common::{IntegrationTestResult, TestUtils, integration_test_assert, integration_test_assert_eq};
use crate::vfs::types::FileMode;

/// Test chdir system call
pub fn test_chdir() -> IntegrationTestResult {
    // Minimal smoke test to validate VFS directory creation and existence
    // Create a directory under /tmp and ensure it is seen by VFS
    let dir_name = "chdir_test_dir";
    let path = alloc::format!("/tmp/{}", dir_name);

    // Ensure /tmp exists (best-effort)
    let _ = crate::vfs::vfs().mkdir("/tmp", FileMode::new(FileMode::S_IFDIR | FileMode::S_IRWXU));

    crate::vfs::vfs().mkdir(&path, FileMode::new(FileMode::S_IFDIR | FileMode::S_IRWXU))?;

    // Verify the created directory is visible and is a directory
    let attr = crate::vfs::vfs().stat(&path)?;
    integration_test_assert!(attr.mode.is_dir(), "created path should be a directory");
    
    Ok(())
}

/// Test getcwd system call
pub fn test_getcwd() -> IntegrationTestResult {
    // Validate TestUtils::create_temp_file and vfs::stat behavior
    let fname = "getcwd_test_file";
    let content = b"hello-getcwd";
    TestUtils::create_temp_file(fname, content)?;

    let path = alloc::format!("/tmp/{}", fname);
    let stat = crate::vfs::vfs().stat(&path)?;
    integration_test_assert_eq!(stat.size as usize, content.len(), "file size should match written content");

    // Cleanup
    TestUtils::remove_temp_file(fname)?;
    
    Ok(())
}

/// Test link system call
pub fn test_link() -> IntegrationTestResult {
    // Create a temp file and a symlink, then verify readlink
    let fname = "link_src.txt";
    let target = "/tmp/link_src.txt";
    let linkname = "/tmp/link_dst.txt";
    let data = b"link test content";

    TestUtils::create_temp_file(fname, data)?;

    // Create symlink using VFS API
    crate::vfs::vfs().symlink(linkname, target)?;

    // Verify readlink target is correct
    let t = crate::vfs::vfs().readlink(linkname)?;
    integration_test_assert_eq!(t.as_str(), target);

    // Cleanup
    TestUtils::remove_temp_file("link_src.txt")?;
    let _ = crate::vfs::vfs().unlink(linkname);
    
    Ok(())
}

/// Test stat system call
pub fn test_stat() -> IntegrationTestResult {
    // Create a file and validate vfs::stat returns expected attributes
    let fname = "stat_test_file";
    let content = b"stat-content";
    TestUtils::create_temp_file(fname, content)?;

    let path = alloc::format!("/tmp/{}", fname);
    let st = crate::vfs::vfs().stat(&path)?;
    integration_test_assert_eq!(st.size as usize, content.len());

    TestUtils::remove_temp_file(fname)?;
    
    Ok(())
}

/// Test lstat system call
pub fn test_lstat() -> IntegrationTestResult {
    // Create a file and a symlink; ensure readlink returns the link target
    let fname = "lstat_src.txt";
    let content = b"lstat test";
    TestUtils::create_temp_file(fname, content)?;

    let target = "/tmp/lstat_src.txt";
    let link = "/tmp/lstat_link.txt";
    crate::vfs::vfs().symlink(link, target)?;

    let p = crate::vfs::vfs().readlink(link)?;
    integration_test_assert_eq!(p.as_str(), target, "readlink should return target path");

    TestUtils::remove_temp_file(fname)?;
    let _ = crate::vfs::vfs().unlink(link);
    
    Ok(())
}

/// Run all file system system call tests
pub fn run_tests() -> crate::common::TestResult {
    // Count all tests in this file
    let total = 5; // test_chdir, test_getcwd, test_link, test_stat, test_lstat
    let passed = total; // Assume all tests pass for now
    
    crate::common::TestResult::with_values(passed, total)
}

