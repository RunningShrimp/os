# [0034] // TODO: Implement proper inotify event generation

**File:** `kernel/src/vfs/mod.rs`
**Line:** 495
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Filesystems Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
492:     /// Note: This is a simplified implementation. A full implementation would need
493:     /// to properly track inotify instances and generate events for matching watches.
494:     fn generate_inotify_events(&self, _path: &str, _mask: u32, _cookie: u32, _name: &str) {
495:         // TODO: Implement proper inotify event generation
496:         // This would require:
497:         // 1. Maintaining a global registry of inotify instances
498:         // 2. Checking which watches match the path
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
