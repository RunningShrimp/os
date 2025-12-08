# [0105] // TODO: Handle external interrupts

**File:** `kernel/src/trap/mod.rs`
**Line:** 87
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
84:             }
85:             cause::SUPERVISOR_EXTERNAL => {
86:                 // External interrupt (e.g., UART)
87:                 // TODO: Handle external interrupts
88:             }
89:             _ => {
90:                 crate::println!("unexpected interrupt: {:#x}", scause);
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
