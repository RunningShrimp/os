# [0314] // TODO: Update cwd_path from file path

**File:** `kernel/src/syscalls/fs.rs`
**Line:** 158
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
155:     let mut proc_table = crate::process::manager::PROC_TABLE.lock();
156:     let proc = proc_table.find(pid).ok_or(SyscallError::NotFound)?;
157:     proc.cwd = Some(file_idx);
158:     // TODO: Update cwd_path from file path
159:     
160:     Ok(0)
161: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
