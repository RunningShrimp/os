# [0135] // use crate::mm::vm::{Page, VirtAddr, PhysAddr}; // TODO: Implement vm module

**File:** `kernel/src/microkernel/memory.rs`
**Line:** 13
**Marker:** TODO
**Suggested Priority:** High
**Suggested Owner Role:** Memory/MM Engineer
**Suggested Estimate (hours):** 32
**Suggested Labels:** `high;todo`

## Context

```
10: use core::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
11: use crate::sync::Mutex;
12: use crate::reliability::errno::{ENOMEM, EINVAL, EFAULT};
13: // use crate::mm::vm::{Page, VirtAddr, PhysAddr}; // TODO: Implement vm module
14: 
15: pub type VirtAddr = usize;
16: pub type PhysAddr = usize;
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
