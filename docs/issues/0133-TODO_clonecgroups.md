# [0133] // TODO: 使用clone系统调用并应用命名空间和cgroups

**File:** `kernel/src/cloud_native/oci.rs`
**Line:** 564
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
561:         // 4. 执行容器入口程序
562:         
563:         // 目前使用fork创建新进程
564:         // TODO: 使用clone系统调用并应用命名空间和cgroups
565:         match crate::process::manager::fork() {
566:             Some(pid) => {
567:                 crate::println!("[oci] Created process {} for container {}", pid, container_id);
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
