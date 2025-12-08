# [0096] // TODO: Integrate with actual capabilities subsystem

**File:** `kernel/src/security/permission_check.rs`
**Line:** 192
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Security Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
189:     fn check_capabilities(&self, request: &PermissionRequest) -> Option<PermissionResult> {
190:         // Check if operation requires specific capability
191:         if let Some(required_cap) = self.operation_to_capability(request.operation, request.resource_type) {
192:             // TODO: Integrate with actual capabilities subsystem
193:             // For now, check if process is privileged
194:             if request.context.privileged {
195:                 Some(PermissionResult::Granted)
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
