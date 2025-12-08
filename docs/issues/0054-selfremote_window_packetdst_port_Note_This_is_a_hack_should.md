# [0054] self.remote_window = packet.dst_port(); // Note: This is a hack, should use header.window_size

**File:** `kernel/src/net/tcp/state.rs`
**Line:** 184
**Marker:** hack
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;hack`

## Context

```
181:         // Update remote sequence and acknowledgment numbers
182:         self.remote_seq = packet.seq_num();
183:         self.remote_ack = packet.ack_num();
184:         self.remote_window = packet.dst_port(); // Note: This is a hack, should use header.window_size
185: 
186:         match self.state {
187:             TcpState::Closed => self.handle_closed(packet),
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
