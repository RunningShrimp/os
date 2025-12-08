# [0097] resource_id: 0, // TODO: Convert resource_id properly

**File:** `kernel/src/security/permission_check.rs`
**Line:** 223
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Security Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
220:                 ResourceType::SystemCall => AclResourceType::SystemCall,
221:                 ResourceType::Network => AclResourceType::Network,
222:             },
223:             resource_id: 0, // TODO: Convert resource_id properly
224:             requested_permissions: self.operation_to_acl_permissions(request.operation),
225:             context: crate::security::acl::AccessContext {
226:                 operation: format!("{:?}", request.operation),
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
