# [0249] // TODO: Implement atomic compare-and-swap

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 791
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
788:     // If futex is uncontended, try to acquire it
789:     if current_val == 0 {
790:         let new_val = 1i32; // Set to locked state
791:         // TODO: Implement atomic compare-and-swap
792:         // For now, just write the new value
793:         unsafe {
794:             let page_ptr = uaddr as *mut i32;
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
