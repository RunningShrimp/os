# [0256] // TODO: Clean up already allocated pages

**File:** `kernel/src/syscalls/memory.rs`
**Line:** 85
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
82:             unsafe {
83:                 if map_pages(proc.pagetable, va, page as usize, PAGE_SIZE, perm).is_err() {
84:                     kfree(page);
85:                     // TODO: Clean up already allocated pages
86:                     return Err(SyscallError::OutOfMemory);
87:                 }
88:             }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
