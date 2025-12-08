# [0028] let effective_gid = current_gid; // TODO: Support effective GID

**File:** `kernel/src/posix/shm.rs`
**Line:** 453
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
450: fn check_permissions(perm: &IpcPerm, required_mode: Mode) -> bool {
451:     let current_uid = crate::process::getuid();
452:     let current_gid = crate::process::getgid();
453:     let effective_gid = current_gid; // TODO: Support effective GID
454: 
455:     // Check owner permissions
456:     if current_uid == perm.uid {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
