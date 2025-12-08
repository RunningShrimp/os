# [0107] // TODO: Set up trapframe and call userret

**File:** `kernel/src/trap/mod.rs`
**Line:** 434
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
431:         // Set stvec to uservec for user traps
432:         core::arch::asm!("csrw stvec, {}", in(reg) uservec as usize);
433:         
434:         // TODO: Set up trapframe and call userret
435:     }
436: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
