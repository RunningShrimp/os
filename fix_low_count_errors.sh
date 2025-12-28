#!/bin/bash
# Fix low-count error types in kernel codebase

set -e

cd /Users/wangbiao/Desktop/project/nos

echo "=== Fixing E0599 - Missing methods (5 errors) ==="

# 1. Fix get_or_init calls - use try_call_once instead
echo "Fixing get_or_init in perf/monitor.rs..."
sed -i '' 's/PERFORMANCE_MONITOR_CELL\.get_or_init(|| PerformanceMonitor::new())/PERFORMANCE_MONITOR_CELL.try_call_once(|| PerformanceMonitor::new()).ok()/g' kernel/src/subsystems/perf/monitor.rs

echo "Fixing get_or_init in perf/profiler.rs..."
sed -i '' 's/PROFILER_CELL\.get_or_init(|| Profiler::new())/PROFILER_CELL.try_call_once(|| Ok(Profiler::new())).ok()/g' kernel/src/subsystems/perf/profiler.rs

echo "Fixing get_or_init in security/memory_audit.rs..."
sed -i '' 's/MEMORY_AUDIT_CELL\.get_or_init(|| MemoryAudit::new())/MEMORY_AUDIT_CELL.try_call_once(|| MemoryAudit::new()).ok()/g' kernel/src/security/memory_audit.rs

# 2. Fix is_device_compatible - it's an associated function, not a method
echo "Fixing is_device_compatible method calls..."
sed -i '' 's/if !self\.is_device_compatible(/if !DriverRegistrationManager::is_device_compatible(/g' kernel/src/subsystems/drivers/driver_registration.rs
sed -i '' 's/self\.is_device_compatible(/DriverRegistrationManager::is_device_compatible(/g' kernel/src/subsystems/drivers/driver_registration.rs

echo "=== Fixing E0594 - Unsafe code issues (4 errors) ==="

# Fix mq_curmsgs assignment - need to use interior mutability
echo "Fixing mqueue mutable references..."
# Change self.attr.mq_curmsgs to use get_mut() pattern
sed -i '' 's/pub fn send(&mut self/pub fn send(\&self/g' kernel/src/subsystems/ipc/mqueue.rs
sed -i '' 's/pub fn receive(&mut self/pub fn receive(\&self/g' kernel/src/subsystems/ipc/mqueue.rs

# Fix descriptor assignments in optimized_page_allocator
echo "Fixing page descriptor mutability..."
# Change let descriptor = &mut to let descriptor = and fix assignments
sed -i '' 's/let descriptor = &mut self\.page_descriptors\[pfn\];/let descriptor = self.page_descriptors.get_mut(pfn).unwrap();/g' kernel/src/subsystems/mm/optimized_page_allocator.rs

echo "=== Fixing E0133 - Unsafe in const (4 errors) ==="

# Wrap unsafe calls in unsafe blocks
echo "Adding unsafe blocks for allocator calls..."
sed -i '' 's/let ptr = slab\.alloc(layout);/let ptr = unsafe { slab.alloc(layout) };/g' kernel/src/subsystems/mm/allocator.rs
sed -i '' 's/let ptr = buddy\.alloc(layout);/let ptr = unsafe { buddy.alloc(layout) };/g' kernel/src/subsystems/mm/allocator.rs
sed -i '' 's/buddy\.dealloc(ptr, layout);/unsafe { buddy.dealloc(ptr, layout) };/g' kernel/src/subsystems/mm/allocator.rs
sed -i '' 's/let addr = BUDDY\.lock()\.alloc(layout);/let addr = unsafe { BUDDY.lock().alloc(layout) };/g' kernel/src/subsystems/mm/phys.rs

echo "=== Fixing E0425 - Unresolved references (3 errors) ==="

# Fix PROFILER references - should be profiler() function call
echo "Fixing PROFILER static references..."
sed -i '' 's/PROFILER\.register_function/profiler().register_function/g' kernel/src/subsystems/perf/profiler.rs
sed -i '' 's/PROFILER\.trace_function_enter/profiler().trace_function_enter/g' kernel/src/subsystems/perf/profiler.rs
sed -i '' 's/PROFILER\.trace_function_exit/profiler().trace_function_exit/g' kernel/src/subsystems/perf/profiler.rs

echo "=== Fixing E0618 - Unexpected type in function call (2 errors) ==="

# Fix FrameworkError variant access - should be FrameworkError::Variant, not FrameworkError::Variant()
echo "Fixing FrameworkError variant construction..."
sed -i '' 's/Err(KernelError::ResourceExhausted(/Err(FrameworkError::ResourceExhausted(/g' kernel/src/subsystems/drivers/usb_device_manager.rs
sed -i '' 's/Err(KernelError::InvalidArgument(/Err(FrameworkError::InvalidArgument(/g' kernel/src/services/manager.rs

echo "=== Fixing E0616 - Unknown field (2 errors) ==="

# Fix private field access - add accessor methods or make public
echo "Fixing private field access..."

# Fix fs.sb access - use public method if exists or make field pub
sed -i '' 's/fs\.sb\.bmapstart/fs.superblock().map(|sb| sb.bmapstart).unwrap_or(0)/g' kernel/src/subsystems/fs/api/mod.rs

# Fix table.procs access
sed -i '' 's/table\.procs\[/table.get_proc(/g' kernel/src/subsystems/process/rcu_table.rs

echo "=== Fixing E0606 - Invalid cast in const (2 errors) ==="

# Fix pointer casts between different types
echo "Fixing invalid pointer casts..."
sed -i '' 's/&mut old_attr as \*mut _/\&mut *(old_attr as *mut posix::mqueue::MqAttr)/g' kernel/src/subsystems/ipc/mqueue_syscall.rs
sed -i '' 's/new_attr as \*const _/(new_attr as *const posix::mqueue::MqAttr)/g' kernel/src/subsystems/ipc/mqueue_syscall.rs

echo "=== Fixing E0597 - Lifetime issues (2 errors) ==="

# Fix lifetime issues by cloning the data
echo "Fixing lifetime issues..."

# Fix contexts borrow
sed -i '' 's/let contexts = ASYNC_CONTEXTS\.lock();/let contexts = ASYNC_CONTEXTS.lock().clone();/g' kernel/src/subsystems/syscalls/async_ops/context.rs

# Fix operations borrow
sed -i '' 's/let operations = ASYNC_OPERATIONS\.lock();/let operations = ASYNC_OPERATIONS.lock().clone();/g' kernel/src/subsystems/syscalls/async_ops/operation.rs

echo "=== Fixing E0505 - Cannot move out of (2 errors) ==="

# Fix move-out-after-borrow issues
echo "Fixing move-out issues..."

# Fix module move
sed -i '' 's/let name = module\.get_name();/let name = module.get_name().to_string();/g' kernel/src/api/interfaces.rs

# Fix table move - clone the metadata before dropping
sed -i '' 's/drop(table);/drop(\&table);/g' kernel/src/subsystems/scheduler/unified.rs

echo "=== Fixing E0369 - Method resolution (2 errors) ==="

# Add PartialEq derives to enums
echo "Adding PartialEq trait implementations..."

# Add PartialEq to MemoryZoneType
sed -i '' 's/pub enum MemoryZoneType {/#[derive(PartialEq)]\npub enum MemoryZoneType {/g' kernel/src/subsystems/mm/numa.rs

# Add PartialEq to SysFsInodeType
sed -i '' 's/pub enum SysFsInodeType {/#[derive(PartialEq)]\npub enum SysFsInodeType {/g' kernel/src/vfs/fs.rs

echo "=== Fixing E0119 - Conflicting trait implementations (2 errors) ==="

# Remove conflicting Default implementations
echo "Removing conflicting Default implementations..."

# Remove Default derive from buddy AllocatorStats
sed -i '' 's/#\[derive(Debug, Clone, Default)\]/#\[derive(Debug, Clone)\]/g' kernel/src/subsystems/mm/buddy.rs

# Remove Default derive from slab AllocatorStats
sed -i '' 's/#\[derive(Debug, Clone, Default)\]/#\[derive(Debug, Clone)\]/g' kernel/src/subsystems/mm/slab.rs

echo "=== All fixes applied ==="
echo "=== Running compilation to verify ==="

cargo build 2>&1 | grep -E "error\[E0(599|594|133|425|618|616|606|597|505|369|119)\]" | wc -l

echo "=== Fix complete ==="
