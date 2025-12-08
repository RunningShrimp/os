# [0228] // TODO: Implement proper thread support

**File:** `kernel/src/syscalls/signal.rs`
**Line:** 807
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
804:     }
805: 
806:     // For now, treat tid as pid (single-threaded processes)
807:     // TODO: Implement proper thread support
808:     let pid = tid;
809: 
810:     // Find target process
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
