# [0324] let _rusage_ptr = args[3] as *mut crate::posix::Rusage; // TODO: Implement rusage support

**File:** `kernel/src/syscalls/process.rs`
**Line:** 332
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
329:     let pid = args[0] as i32;
330:     let status_ptr = args[1] as *mut i32;
331:     let options = args[2] as i32;
332:     let _rusage_ptr = args[3] as *mut crate::posix::Rusage; // TODO: Implement rusage support
333: 
334:     // Call waitpid implementation
335:     match crate::process::manager::waitpid(pid, status_ptr, options) {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
