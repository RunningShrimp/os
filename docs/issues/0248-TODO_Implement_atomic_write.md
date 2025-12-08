# [0248] // TODO: Implement atomic write

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 763
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
760:     
761:     // Unlock the futex
762:     let new_val = 0i32; // Set to unlocked state
763:     // TODO: Implement atomic write
764:     // For now, just write the new value
765:     unsafe {
766:         let page_ptr = uaddr as *mut i32;
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
