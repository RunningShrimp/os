# Module Nesting Flattening Plan

## Current State Analysis

### Nesting Depth Distribution
- **Maximum Depth**: 5 levels
- **Problem Areas**:
  - `subsystems/syscalls/implementation/handlers/mm/` (5 levels, 10 files)
  - `subsystems/syscalls/implementation/handlers/` (4 levels)
  - `subsystems/syscalls/security/access_control/` (4 levels, 4 files)

### Dependency Analysis
**Most imported modules** (by frequency):
- `common` (17 imports) - Core utilities
- `interface` (15 imports) - Core interfaces
- `services` (7 imports) - Service management
- `implementation` (1 import) - Only used in `example.rs` ✅

**Critical Insight**: The `implementation` module is minimally used, making it safe to refactor.

## Restructuring Plan

### Phase 1: Move MM Handlers (Deepest Nesting)

**Source**: `subsystems/syscalls/implementation/handlers/mm/` (5 levels → 3 levels)
**Target**: `subsystems/syscalls/mm/`

**Files to move** (10 files):
- mmap.rs
- munmap.rs
- mprotect.rs
- madvise.rs
- mlock.rs
- brk.rs
- shm.rs
- numa.rs
- utils.rs
- mod.rs

**Impact Analysis**:
- ✅ Zero direct imports (safe to move)
- Parent module only used in `example.rs`
- Self-contained module with no external dependencies

**Actions**:
1. Create `kernel/src/subsystems/syscalls/mm/` directory
2. Move all 10 files from `implementation/handlers/mm/` to `mm/`
3. Update `subsystems/syscalls/mod.rs` to declare `pub mod mm;`
4. Update `example.rs` to use new path (if needed)
5. Remove old `implementation/handlers/mm/` directory
6. Update `implementation/handlers/mod.rs` to remove mm reference

---

### Phase 2: Flatten Handlers Module

**Source**: `subsystems/syscalls/implementation/handlers/` (4 levels → 3 levels)
**Target**: `subsystems/syscalls/handlers/`

**Files to move** (after mm is removed):
- fs.rs
- net.rs
- types.rs
- mod.rs

**Impact Analysis**:
- ✅ Minimal usage (only in example.rs)
- Self-contained handler implementations
- No complex dependencies

**Actions**:
1. Create `kernel/src/subsystems/syscalls/handlers/` directory
2. Move 4 files from `implementation/handlers/` to `handlers/`
3. Update `subsystems/syscalls/mod.rs` to declare `pub mod handlers;`
4. Update imports in any files that reference this module
5. Remove old `implementation/handlers/` directory
6. Update `implementation/mod.rs` to remove handlers reference

---

### Phase 3: Flatten Security Access Control

**Source**: `subsystems/syscalls/security/access_control/` (4 levels → 3 levels)
**Target**: `subsystems/syscalls/security/` (integrate files)

**Current structure**:
```
security/
├── access_control/
│   ├── types.rs
│   ├── config.rs
│   ├── manager.rs
│   └── mod.rs
├── syscall_validator.rs
└── mod.rs
```

**Target structure**:
```
security/
├── access_types.rs      (renamed from access_control/types.rs)
├── access_config.rs     (renamed from access_control/config.rs)
├── access_manager.rs    (renamed from access_control/manager.rs)
├── syscall_validator.rs
└── mod.rs              (updated to re-export from new files)
```

**Impact Analysis**:
- 1 import in `dispatcher.rs`: `use crate::subsystems::syscalls::security::access_control::{ResourceType, AccessResult};`
- Need to update this import after flattening

**Actions**:
1. Move and rename files from `security/access_control/` to `security/`:
   - `types.rs` → `access_types.rs`
   - `config.rs` → `access_config.rs`
   - `manager.rs` → `access_manager.rs`
2. Update `security/mod.rs` to declare new modules and re-export types
3. Update import in `dispatcher.rs`
4. Remove old `security/access_control/` directory

---

### Phase 4: Remove Empty Implementation Module

**Source**: `subsystems/syscalls/implementation/`
**Target**: Remove entire module (empty after Phases 1-2)

**Files to check**:
- `mod.rs` (may have remaining content)
- Any other subdirectories

**Actions**:
1. Check if `implementation/mod.rs` has any remaining content
2. Update `example.rs` to remove implementation import
3. Remove `implementation/` directory if empty
4. Update `subsystems/syscalls/mod.rs` to remove implementation declaration

---

### Phase 5: Update MM Module Nesting

**Source**: `subsystems/mm/vm/arch/` (4 levels)
**Target**: `subsystems/mm/arch_*.rs` (3 levels)

**Current structure**:
```
mm/vm/arch/
├── mod.rs
├── x86_64.rs
├── aarch64.rs
└── riscv64.rs
```

**Target structure**:
```
mm/
├── vm_x86_64.rs    (moved from vm/arch/x86_64.rs)
├── vm_aarch64.rs   (moved from vm/arch/aarch64.rs)
├── vm_riscv64.rs   (moved from vm/arch/riscv64.rs)
├── vm/
│   ├── mod.rs      (updated to remove arch module)
│   └── ...
└── ...
```

**Actions**:
1. Move architecture-specific files:
   - `vm/arch/x86_64.rs` → `vm_x86_64.rs`
   - `vm/arch/aarch64.rs` → `vm_aarch64.rs`
   - `vm/arch/riscv64.rs` → `vm_riscv64.rs`
2. Update `vm/mod.rs` to remove arch module declaration
3. Update imports in all files that use `vm::arch::*`
4. Remove `vm/arch/` directory
5. Update `mm/mod.rs` to declare new `vm_*` modules

---

## Success Criteria

- [ ] Maximum nesting depth ≤ 3 levels
- [ ] All imports updated and working
- [ ] Zero circular dependencies
- [ ] All tests pass without modification
- [ ] Code readability improved

---

## Risk Management

### Low Risk
- **MM Handlers move**: Zero external dependencies, safe to move
- **Implementation module removal**: Only 1 usage location

### Medium Risk
- **Security access_control flatten**: Requires import updates and renames
- **MM arch flatten**: Multiple import paths to update

### Mitigation
- Perform changes incrementally (one phase at a time)
- Run `cargo check` after each phase
- Keep backup of original structure
- Test compilation at each step

---

## Timeline

- **Phase 1**: MM Handlers (Day 2 morning)
- **Phase 2**: Handlers Module (Day 2 afternoon)
- **Phase 3**: Security Access Control (Day 3 morning)
- **Phase 4**: Remove Implementation (Day 3 afternoon)
- **Phase 5**: MM Arch (Day 4)
- **Verification & Testing**: (Day 4 afternoon)

---

## Verification Commands

After each phase:
```bash
# Check compilation
cargo check --lib

# Check for any remaining deep nesting
find kernel/src -type d | awk -F/ '{print NF-2, $0}' | sort -rn | head -10

# Verify no broken imports
grep -r "use.*implementation::" kernel/src/subsystems/syscalls/
grep -r "use.*access_control::" kernel/src/
```

---

**Generated**: 2025-01-01
**Status**: Ready for Execution
**Estimated Time**: 4 days
**Confidence**: High (minimal dependencies identified)
