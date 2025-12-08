# [0198] // TODO: Implement socket creation

**File:** `kernel/src/services/network.rs`
**Line:** 529
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
526: /// 兼容性接口 - 保持与现有代码的兼容性
527: /// Open a network socket
528: pub fn net_socket(domain: u32, socket_type: u32, protocol: u32) -> Option<usize> {
529:     // TODO: Implement socket creation
530:     None
531: }
532: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
