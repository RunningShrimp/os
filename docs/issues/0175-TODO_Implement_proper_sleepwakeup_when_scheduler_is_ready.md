# [0175] // TODO: Implement proper sleep/wakeup when scheduler is ready

**File:** `kernel/src/sync/mod.rs`
**Line:** 558
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
555:     /// Acquire the sleeplock
556:     /// In a full implementation, this would sleep instead of spin
557:     pub fn lock(&self) -> SleeplockGuard<'_, T> {
558:         // TODO: Implement proper sleep/wakeup when scheduler is ready
559:         // For now, use a simple spin with yield to reduce CPU usage
560:         let mut spin_count = 0;
561:         while self
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
