# [0094] // TODO: Integrate with actual seccomp subsystem

**File:** `kernel/src/security/permission_check.rs`
**Line:** 169
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Security Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
166:         // Convert operation to syscall number (simplified)
167:         let _syscall_num = self.operation_to_syscall(request.operation);
168:         
169:         // TODO: Integrate with actual seccomp subsystem
170:         // For now, return None to skip seccomp check
171:         None
172:     }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
