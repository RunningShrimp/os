# [0306] // TODO: Map remaining syscall IDs for the other syscall functions

**File:** `kernel/src/syscalls/file_io.rs`
**Line:** 38
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
35:         0x2001 => sys_close_impl(args),    // close
36:         0x2002 => sys_read_impl(args),     // read
37:         0x2003 => sys_write_impl(args),    // write
38:         // TODO: Map remaining syscall IDs for the other syscall functions
39:         _ => Err(SyscallError::InvalidSyscall),
40:     }
41: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
