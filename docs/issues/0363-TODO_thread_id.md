# [0363] // TODO: 使用 thread_id 参数来获取特定线程的堆栈信息

**File:** `kernel/src/debug/manager.rs`
**Line:** 341
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
338:     /// 收集堆栈信息
339:     fn collect_stack_info(&self, _thread_id: u32) -> Result<Vec<StackFrame>, &'static str> {
340:         // 简化实现，实际实现需要调用栈遍历
341:         // TODO: 使用 thread_id 参数来获取特定线程的堆栈信息
342:         Ok(vec![
343:             StackFrame {
344:                 return_address: 0x7FFFFFF0,
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
