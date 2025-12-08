# [0233] // TODO: Store namespace in process structure

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 260
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
257:                         match manager.create_namespace(ns_type, config) {
258:                             Ok(ns_id) => {
259:                                 // Associate namespace with child process
260:                                 // TODO: Store namespace in process structure
261:                                 crate::println!("[clone] Created namespace {:?} (ID: {}) for child process {}", ns_type, ns_id, child_pid);
262:                             }
263:                             Err(_) => {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
