# TODO/FIXME Cleanup Plan

## Current State

**Total TODOs**: 715 (up from 714)
- Security-related: ~33
- Incomplete implementations: ~238
- Optimizations: ~13
- Features: ~19
- Documentation: Remaining
- Potentially obsolete: ~50+

**Target**: 715 → 0 TODOs

---

## Cleanup Strategy

### Phase 1: Critical Security TODOs (Day 2)
**Priority**: CRITICAL - Fix immediately  
**Count**: ~33 TODOs  
**Time**: 2 hours

**Files to fix**:
1. `kernel/src/drivers/vfio/security.rs`
   - Device authorization checks
   - IOMMU isolation verification

2. `kernel/src/security/permission_check.rs`
   - seccomp integration
   - SELinux integration
   - capabilities integration

3. `kernel/src/security/smap_smep.rs`
   - X86Feature and X86Cpu implementation

4. `kernel/src/helpers.rs`
   - User space validation (4 TODOs)
   - Permission checking

**Action**: Implement proper security checks or add security panics

---

### Phase 2: Incomplete Implementation TODOs (Day 2-3)
**Priority**: HIGH - Complete functionality  
**Count**: ~238 TODOs  
**Time**: 4-6 hours

**Categories**:

#### 2a. Core Functionality (HIGH)
- `epoll.rs` (4 TODOs): Event waiting mechanism
- `posix/rt_mutex.rs`: Cycle detection
- `types/stubs.rs`: Thread-local errno, service registry
- `libc/formatter.rs`: va_list handling

#### 2b. Driver Implementations (MEDIUM)
- `drivers/vfio/*` (8 TODOs): DMA, interrupts, PCI config
- Page pinning, eventfd signaling

#### 2c. Network Syscalls (MEDIUM)
- `syscalls/network/options.rs`: getsockname, getpeername
- `syscalls/network/socket.rs`: Socket table lookup

**Action**: Complete implementations or mark as explicitly not supported

---

### Phase 3: Optimization TODOs (Day 3)
**Priority**: MEDIUM - Performance improvements  
**Count**: ~13 TODOs  
**Time**: 1 hour

**Action**: 
- Remove or implement optimization stubs
- Add feature flags for optional optimizations

---

### Phase 4: Feature TODOs (Day 3)
**Priority**: LOW - Nice-to-have  
**Count**: ~19 TODOs  
**Time**: 1 hour

**Action**: Convert to GitHub Issues with proper labels

---

### Phase 5: Document Remaining TODOs (Day 3)
**Priority**: LOWEST  
**Action**: 
- Create GitHub issues for each valid TODO
- Add issue reference in code comment
- Delete TODO from code

**Format**:
```rust
// GH-#123: Implement feature X
// See: https://github.com/user/repo/issues/123
```

---

## Detailed Cleanup Actions

### Action 1: Fix Security TODOs

#### 1.1 Implement Permission Checking
```rust
// Before:
// TODO: Implement proper permission checking

// After:
fn check_permission(...) -> Result<(), Error> {
    // Proper permission check or panic
    if !has_permission(...) {
        return Err(Error::PermissionDenied);
    }
    Ok(())
}
```

#### 1.2 Add Security Assertions
```rust
// Before:
// TODO: Add proper user space validation

// After:
fn validate_user_ptr(ptr: *mut u8, size: usize) -> Result<(), Error> {
    if ptr.is_null() || ptr as usize > USER_SPACE_MAX {
        panic!("Invalid user pointer: {:p}", ptr);
    }
    Ok(())
}
```

---

### Action 2: Complete Core Implementations

#### 2.1 Complete epoll Implementation
```rust
// Before:
// TODO: Implement actual file descriptor lookup and polling

// After:
fn lookup_fd(fd: i32) -> Option<Arc<File>> {
    FD_TABLE.lock().get(fd)
}

fn poll_events(timeout: Duration) -> Vec<Event> {
    // Actual implementation
}
```

#### 2.2 Complete Stubs
```rust
// Before:
// TODO: Implement thread-local errno storage

// After:
thread_local! {
    static ERRNO: Cell<c_int> = Cell::new(0);
}
```

---

### Action 3: Convert Valid TODOs to Issues

For TODOs that represent future work (not bugs):

```rust
// Before:
// TODO: Implement SRTP authentication tag return

// After:
// GH-#456: Implement SRTP authentication tag return
// See: https://github.com/npos/kernel/issues/456
```

---

### Action 4: Remove Obsolete TODOs

For TODOs where feature is already implemented:

**Check**:
1. Search for implementation in codebase
2. Verify functionality exists
3. Remove TODO comment

**Example**:
```rust
// Before:
// TODO: Implement process table lookup by PID
fn get_process(pid: Pid) -> Option<&Process> {
    PROCESS_TABLE.get(pid)
}

// After:
fn get_process(pid: Pid) -> Option<&Process> {
    PROCESS_TABLE.get(pid)  // Already implemented!
}
```

---

### Action 5: Mark Not-Supported Features

For TODOs representing features we don't plan to support:

```rust
// Before:
// TODO: Implement process-shared semaphores

// After:
// Process-shared semaphores are not supported
fn sem_init_pshared(...) -> Result<(), Error> {
    Err(Error::NotSupported)
}
```

---

## Success Criteria

- [ ] All security TODOs fixed or asserted
- [ ] All critical implementation TODOs completed
- [ ] Remaining TODOs converted to GitHub issues
- [ ] Zero TODO/FIXME comments in code
- [ ] All changes committed

---

## Risk Management

### High Risk
- **Security TODOs**: Must not introduce vulnerabilities
  - Mitigation: Use panics for unimplemented checks

### Medium Risk
- **Incomplete implementations**: May break existing code
  - Mitigation: Add #[cfg(feature = "unimplemented")] guards

### Low Risk
- **Feature TODOs**: Safe to defer to issues
  - Mitigation: Clear documentation in issue tracker

---

## Tools & Automation

### TODO Counter Script
```bash
#!/bin/bash
count_todos() {
    grep -r "TODO\|FIXME\|XXX" kernel/src/ --include="*.rs" | wc -l
}

# Run after each phase
count_todos
```

### TODO to Issue Converter
```bash
#!/bin/bash
# Convert TODO to GitHub issue reference
# Usage: ./convert_todo.sh "file:line" "issue_number"

# This would be run interactively during cleanup
```

---

## Timeline

**Day 1** (Completed): 
- ✅ Analysis and categorization
- ✅ Created cleanup plan

**Day 2** (Today):
- Morning: Fix all 33 security TODOs
- Afternoon: Complete top 50 implementation TODOs

**Day 3**:
- Morning: Complete remaining implementation TODOs
- Afternoon: Convert all remaining to GitHub issues

**Estimated Total Time**: 8-10 hours
**Expected Reduction**: 715 → 0 TODOs

---

## Verification

After cleanup:
```bash
# Verify no TODOs remain
grep -r "TODO\|FIXME\|XXX" kernel/src/ --include="*.rs" | wc -l
# Expected output: 0

# Verify compilation
cargo check --lib
# Expected: No errors

# Count GitHub issues created
gh issue list --label "todo-cleanup"
# Expected: ~50 new issues
```

---

**Generated**: 2025-01-01
**Status**: Ready for execution
**Next Action**: Begin Phase 1 - Security TODOs
