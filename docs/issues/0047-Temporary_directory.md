# [0047] /// Temporary directory

**File:** `kernel/src/compat/sandbox.rs`
**Line:** 129
**Marker:** Temporary
**Suggested Priority:** Critical
**Suggested Owner Role:** Filesystems Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;temporary`

## Context

```
126:     pub read_only_paths: Vec<String>,
127:     /// Read-write paths
128:     pub read_write_paths: Vec<String>,
129:     /// Temporary directory
130:     pub temp_dir: Option<String>,
131:     /// Home directory
132:     pub home_dir: Option<String>,
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
