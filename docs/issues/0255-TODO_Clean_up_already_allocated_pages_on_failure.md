# [0255] // TODO: Clean up already allocated pages on failure

**File:** `kernel/src/syscalls/memory.rs`
**Line:** 73
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
70:             let va = old_sz + i * PAGE_SIZE;
71:             let page = kalloc();
72:             if page.is_null() {
73:                 // TODO: Clean up already allocated pages on failure
74:                 return Err(SyscallError::OutOfMemory);
75:             }
76: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
