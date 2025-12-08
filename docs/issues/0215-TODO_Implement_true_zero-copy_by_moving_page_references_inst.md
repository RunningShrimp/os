# [0215] // TODO: Implement true zero-copy by moving page references instead of copying

**File:** `kernel/src/syscalls/zero_copy.rs`
**Line:** 317
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
314:     // For other types, use chunked transfer
315:     let transferred = match (in_ftype, out_ftype) {
316:         // Pipe to Pipe: Can use zero-copy by moving pipe buffer references
317:         // TODO: Implement true zero-copy by moving page references instead of copying
318:         (FileType::Pipe, FileType::Pipe) => {
319:             // Transfer data between pipes
320:             // Future optimization: Move page references directly without copying
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
