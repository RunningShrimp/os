# [0355] // TODO: Actually copy strings and set up pointers

**File:** `kernel/src/process/exec.rs`
**Line:** 408
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
405:     let sp = USER_STACK_TOP - aligned_size;
406:     let argv_ptr = sp;
407:     
408:     // TODO: Actually copy strings and set up pointers
409:     
410:     Ok((sp, argc, argv_ptr))
411: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
