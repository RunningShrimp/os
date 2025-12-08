# [0098] path: None, // TODO: Add path information

**File:** `kernel/src/security/permission_check.rs`
**Line:** 227
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Security Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
224:             requested_permissions: self.operation_to_acl_permissions(request.operation),
225:             context: crate::security::acl::AccessContext {
226:                 operation: format!("{:?}", request.operation),
227:                 path: None, // TODO: Add path information
228:                 flags: 0, // TODO: Add flags
229:                 privileged: request.context.privileged,
230:             },
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
