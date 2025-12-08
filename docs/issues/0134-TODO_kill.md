# [0134] // TODO: 实现真正的kill系统调用

**File:** `kernel/src/cloud_native/oci.rs`
**Line:** 579
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
576:         crate::println!("[oci] Sending signal {} to PID {}", signal, pid);
577:         
578:         // 使用kill系统调用发送信号
579:         // TODO: 实现真正的kill系统调用
580:         // 目前返回成功，实际实现需要调用sys_kill
581:         Ok(())
582:     }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
