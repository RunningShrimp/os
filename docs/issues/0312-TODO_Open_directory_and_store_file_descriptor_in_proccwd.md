# [0312] // TODO: Open directory and store file descriptor in proc.cwd

**File:** `kernel/src/syscalls/fs.rs`
**Line:** 108
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
105:     let mut proc_table = crate::process::manager::PROC_TABLE.lock();
106:     let proc = proc_table.find(pid).ok_or(SyscallError::NotFound)?;
107:     proc.cwd_path = Some(normalized_path.clone());
108:     // TODO: Open directory and store file descriptor in proc.cwd
109:     // For now, we just store the path
110:     
111:     Ok(0)
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
