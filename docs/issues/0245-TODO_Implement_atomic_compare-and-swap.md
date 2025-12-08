# [0245] // TODO: Implement atomic compare-and-swap

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 708
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
705:     // If futex is uncontended, try to acquire it
706:     if current_val == 0 {
707:         let new_val = 1i32; // Set to locked state
708:         // TODO: Implement atomic compare-and-swap
709:         // For now, just write the new value
710:         unsafe {
711:             let page_ptr = uaddr as *mut i32;
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
