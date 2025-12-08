# [0013] // TODO: Calculate remaining time based on current time

**File:** `kernel/src/posix/timer.rs`
**Line:** 87
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
84:             return Timespec::zero();
85:         }
86: 
87:         // TODO: Calculate remaining time based on current time
88:         // For now, return the expiry time
89:         self.expiry_time
90:     }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
