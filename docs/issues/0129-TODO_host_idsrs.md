# [0129] // TODO: 逐步拆分到各个子模块，将代码从host_ids.rs移动到对应的子模块

**File:** `kernel/src/ids/host_ids/mod.rs`
**Line:** 19
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
16: pub mod malware;
17: 
18: // 临时：保留原有文件作为过渡
19: // TODO: 逐步拆分到各个子模块，将代码从host_ids.rs移动到对应的子模块
20: mod host_ids;
21: 
22: // 重新导出主要类型
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
