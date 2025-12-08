# [0239] // TODO: Store namespace in process structure

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 401
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
398:         match manager.create_namespace(ns_type, config) {
399:             Ok(ns_id) => {
400:                 // Associate namespace with current process
401:                 // TODO: Store namespace in process structure
402:                 crate::println!("[unshare] Created namespace {:?} (ID: {}) for process {}", ns_type, ns_id, pid);
403:             }
404:             Err(_) => {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
