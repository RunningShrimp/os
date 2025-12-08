# [0277] // TODO: Implement add_route

**File:** `kernel/src/syscalls/network/interface.rs`
**Line:** 26
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
23: 
24: /// Add a route to the routing table
25: pub fn add_route(_dest: &str, _gateway: &str, _netmask: &str) -> Result<(), i32> {
26:     // TODO: Implement add_route
27:     Err(crate::reliability::errno::ENOSYS)
28: }
29: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
