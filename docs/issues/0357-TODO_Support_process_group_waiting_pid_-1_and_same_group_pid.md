# [0357] // TODO: Support process group waiting (pid < -1) and same group (pid == 0)

**File:** `kernel/src/process/manager.rs`
**Line:** 1047
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
1044:             // Wait for specific child
1045:             vec![pid as Pid]
1046:         } else {
1047:             // TODO: Support process group waiting (pid < -1) and same group (pid == 0)
1048:             // For now, treat as any child
1049:             if let Some(children) = table.get_children(parent_pid) {
1050:                 children.clone()
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
