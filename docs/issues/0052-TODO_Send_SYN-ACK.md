# [0052] // TODO: Send SYN-ACK

**File:** `kernel/src/net/processor.rs`
**Line:** 290
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
287:                 if socket.state == TcpState::Listen {
288:                     // Transition to SYN_RECEIVED
289:                     socket.state = TcpState::SynReceived;
290:                     // TODO: Send SYN-ACK
291:                     return Ok(PacketResult::Drop);
292:                 }
293:             }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
