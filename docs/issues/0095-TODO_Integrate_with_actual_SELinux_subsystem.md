# [0095] // TODO: Integrate with actual SELinux subsystem

**File:** `kernel/src/security/permission_check.rs`
**Line:** 183
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Security Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
180:         let _permission = self.operation_to_selinux_permission(request.operation);
181:         let _object_class = self.resource_type_to_selinux_class(request.resource_type);
182:         
183:         // TODO: Integrate with actual SELinux subsystem
184:         // For now, return None to skip SELinux check if not configured
185:         None
186:     }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
