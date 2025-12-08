# [0029] // TODO: Use OnceCell or LazyLock for proper initialization

**File:** `kernel/src/libc/memory_adapter.rs`
**Line:** 76
**Marker:** TODO
**Suggested Priority:** High
**Suggested Owner Role:** Memory/MM Engineer
**Suggested Estimate (hours):** 32
**Suggested Labels:** `high;todo`

## Context

```
73: pub fn get_libc_adapter() -> &'static LibcMemoryAdapter {
74:     // Use a static with OnceCell or similar for thread safety
75:     // For now, create a new instance each time (not ideal but works)
76:     // TODO: Use OnceCell or LazyLock for proper initialization
77:     static mut ADAPTER: Option<LibcMemoryAdapter> = None;
78:     unsafe {
79:         if ADAPTER.is_none() {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
