# [0196] use crate::vfs::{Vfs, FileMode}; // TODO: Implement other VFS types

**File:** `kernel/src/services/fs.rs`
**Line:** 16
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Filesystems Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
13: use crate::sync::Mutex;
14: use crate::reliability::errno::{EINVAL, ENOMEM, EFAULT, EPERM, ENOENT, EEXIST, ENOTEMPTY, ENOSPC};
15: use crate::fs::{InodeType, MAXPATH, ROOTINO, SuperBlock};
16: use crate::vfs::{Vfs, FileMode}; // TODO: Implement other VFS types
17: 
18: pub type VfsInode = ();
19: pub type VfsFileType = crate::vfs::types::FileType;
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
