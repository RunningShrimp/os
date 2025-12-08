# [0287] // TODO: Check if address is already bound

**File:** `kernel/src/syscalls/network/socket.rs`
**Line:** 190
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
187: 
188:     // Check if address is already in use (for TCP sockets)
189:     if socket_entry.socket_type.is_connection_oriented() && !socket_entry.options.reuse_addr {
190:         // TODO: Check if address is already bound
191:         // For now, just allow it
192:     }
193: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
