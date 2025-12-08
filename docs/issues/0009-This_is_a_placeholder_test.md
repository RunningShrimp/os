# [0009] // This is a placeholder test

**File:** `kernel/tests/fs_syscall_tests.rs`
**Line:** 61
**Marker:** placeholder
**Suggested Priority:** Critical
**Suggested Owner Role:** Filesystems Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;placeholder`

## Context

```
58: 
59: /// Test lstat system call
60: pub fn test_lstat() -> IntegrationTestResult {
61:     // This is a placeholder test
62:     // In a full implementation, we would:
63:     // 1. Create a symbolic link
64:     // 2. Call sys_lstat (should return link info, not target)
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
