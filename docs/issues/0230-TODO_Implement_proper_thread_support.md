# [0230] // TODO: Implement proper thread support

**File:** `kernel/src/syscalls/signal.rs`
**Line:** 847
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
844:     }
845: 
846:     // For now, treat tgid and tid as the same (single-threaded processes)
847:     // TODO: Implement proper thread support
848:     let pid = if tgid != 0 { tgid } else { tid };
849: 
850:     // Find target process
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
