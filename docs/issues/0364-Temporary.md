# [0364] Temporary,

**File:** `kernel/src/debug/breakpoint.rs`
**Line:** 90
**Marker:** Temporary
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;temporary`

## Context

```
87:     /// 条件断点
88:     Conditional,
89:     /// 临时断点
90:     Temporary,
91:     /// 看门断点
92:     Watchpoint,
93: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
