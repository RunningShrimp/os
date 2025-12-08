# [0280] // TODO: Implement set_interface_mtu

**File:** `kernel/src/syscalls/network/interface.rs`
**Line:** 44
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
41: 
42: /// Set interface MTU
43: pub fn set_interface_mtu(_interface_name: &str, _mtu: u32) -> Result<(), i32> {
44:     // TODO: Implement set_interface_mtu
45:     Err(crate::reliability::errno::ENOSYS)
46: }
47: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
