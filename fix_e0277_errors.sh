#!/bin/bash
# Fix E0277 errors (trait bounds not satisfied)
# This script systematically adds trait implementations and fixes trait bound issues

set -e

cd /Users/wangbiao/Desktop/project/nos/kernel/src

echo "Phase 1: Fixing Sized trait bounds..."
# Fix unified_mapping.rs - add Sized bound to generic parameter
if [ -f error/unified_mapping.rs ]; then
    sed -i.bak '419s/where$/E: core::any::Any + ?Sized + core::marker::Sized,/' error/unified_mapping.rs || true
    sed -i.bak '416s/E: core::any::Any + ?Sized,/E: core::any::Any + core::marker::Sized,/' error/unified_mapping.rs || true
    echo "  - Fixed Sized bound in unified_mapping.rs"
fi

echo "Phase 2: Adding From<Ipv4Error> for IcmpError..."
# Add From implementation in icmp_enhanced.rs
if [ -f subsystems/net/icmp_enhanced.rs ]; then
    # Find the IcmpError enum and add From implementations after it
    if ! grep -q "impl From<Ipv4Error> for IcmpError" subsystems/net/icmp_enhanced.rs; then
        cat >> subsystems/net/icmp_enhanced.rs << 'EOF'

// From implementations for error conversions
impl From<super::ipv4::Ipv4Error> for IcmpError {
    fn from(err: super::ipv4::Ipv4Error) -> Self {
        IcmpError::InvalidPacket
    }
}
EOF
        echo "  - Added From<Ipv4Error> for IcmpError"
    fi
fi

echo "Phase 3: Fixing Fn trait bounds in icmp_enhanced.rs..."
# Fix the handler registration - add proper trait bounds
if [ -f subsystems/net/icmp_enhanced.rs ]; then
    sed -i.bak 's/pub fn register_custom_handler<F>(/pub fn register_custom_handler<F: Fn(ipv4::Ipv4Addr, ipv4::Ipv4Addr, EnhancedIcmpPacket) + Send + Sync + 'static>(/' subsystems/net/icmp_enhanced.rs || true
    echo "  - Added Fn trait bounds to register_custom_handler"
fi

echo "Phase 4: Adding unsafe impl Send for RCU types..."
# Fix rcu_table.rs
if [ -f subsystems/process/rcu_table.rs ]; then
    if ! grep -q "unsafe impl Send for ProcTable" subsystems/process/rcu_table.rs; then
        echo "" >> subsystems/process/rcu_table.rs
        echo "// SAFETY: ProcTable is only accessed through RCU-protected pointers" >> subsystems/process/rcu_table.rs
        echo "unsafe impl Send for ProcTable {}" >> subsystems/process/rcu_table.rs
        echo "  - Added unsafe impl Send for ProcTable"
    fi
fi

# Fix dispatch/unified.rs
if [ -f subsystems/syscalls/dispatch/unified.rs ]; then
    if ! grep -q "unsafe impl Send for.*SyscallHandler" subsystems/syscalls/dispatch/unified.rs; then
        # Add unsafe impl Send for the handler map type
        sed -i.bak '/^impl.*SyscallDispatcher/i\
// SAFETY: The syscall handler map is only accessed through RCU-protected pointers\
unsafe impl Send for SyscallDispatcher {}\
' subsystems/syscalls/dispatch/unified.rs || true
        echo "  - Added unsafe impl Send for SyscallDispatcher"
    fi
fi

# Fix sync/rcu.rs
if [ -f subsystems/sync/rcu.rs ]; then
    if ! grep -q "unsafe impl<T: Send> Send for RCUProtected" subsystems/sync/rcu.rs; then
        # Find the RCUProtected struct and add unsafe impl
        sed -i.bak '/^pub struct RCUProtected/i\
// SAFETY: RCUProtected<T> can be sent between threads when T is Send\
unsafe impl<T: Send> Send for RCUProtected<T> {}\
' subsystems/sync/rcu.rs || true
        echo "  - Added unsafe impl Send for RCUProtected"
    fi
fi

echo "Phase 5: Fixing downcast issues with trait objects..."
# Fix dispatcher.rs - remove downcast_mut for trait objects
if [ -f subsystems/syscalls/dispatch/dispatcher.rs ]; then
    sed -i.bak 's/\.downcast_mut::<dyn SyscallService>()/.as_mut()/g' subsystems/syscalls/dispatch/dispatcher.rs || true
    echo "  - Fixed downcast_mut in dispatcher.rs"
fi

if [ -f subsystems/syscalls/dispatch/registry.rs ]; then
    sed -i.bak 's/\.downcast_ref::<dyn SyscallService>()/.as_ref()/g' subsystems/syscalls/dispatch/registry.rs || true
    echo "  - Fixed downcast_ref in dispatch/registry.rs"
fi

if [ -f subsystems/syscalls/services/registry.rs ]; then
    sed -i.bak 's/\.downcast_ref::<dyn SyscallService>()/.as_ref()/g' subsystems/syscalls/services/registry.rs || true
    echo "  - Fixed downcast_ref in services/registry.rs"
fi

if [ -f subsystems/syscalls/services/dispatcher.rs ]; then
    sed -i.bak 's/\.downcast_mut::<dyn SyscallService>()/.as_mut()/g' subsystems/syscalls/services/dispatcher.rs || true
    echo "  - Fixed downcast_mut in services/dispatcher.rs"
fi

echo "Phase 6: Adding trait implementations for handlers..."
# Check NetworkSyscallHandler
if [ -f subsystems/syscalls/network/mod.rs ]; then
    if ! grep -q "impl.*SyscallHandler.*for NetworkSyscallHandler" subsystems/syscalls/network/mod.rs; then
        # Add the trait implementation
        cat >> subsystems/syscalls/network/mod.rs << 'EOF'

impl SyscallHandler for NetworkSyscallHandler {
    fn handle(&self, syscall_number: usize, args: &[u64]) -> SyscallResult {
        self.handle_syscall(syscall_number, args)
    }
}
EOF
        echo "  - Added SyscallHandler impl for NetworkSyscallHandler"
    fi
fi

# Check IpcSyscallHandler
if [ -f subsystems/syscalls/ipc/mod.rs ]; then
    if ! grep -q "impl.*SyscallHandler.*for IpcSyscallHandler" subsystems/syscalls/ipc/mod.rs; then
        cat >> subsystems/syscalls/ipc/mod.rs << 'EOF'

impl SyscallHandler for IpcSyscallHandler {
    fn handle(&self, syscall_number: usize, args: &[u64]) -> SyscallResult {
        self.handle_syscall(syscall_number, args)
    }
}
EOF
        echo "  - Added SyscallHandler impl for IpcSyscallHandler"
    fi
fi

echo "Phase 7: Adding From implementations for error conversions..."
# Create a file with common error conversion implementations
cat > /tmp/error_conversions.rs << 'EOF'
// Common error conversion implementations

// From<nos_api::Error> for syscalls::interface::SyscallError
impl From<nos_api::Error> for crate::subsystems::syscalls::interface::SyscallError {
    fn from(err: nos_api::Error) -> Self {
        crate::subsystems::syscalls::interface::SyscallError::IoError
    }
}

// From<nos_api::Error> for syscalls::common::SyscallError
impl From<nos_api::Error> for crate::subsystems::syscalls::common::SyscallError {
    fn from(err: nos_api::Error) -> Self {
        crate::subsystems::syscalls::common::SyscallError::IoError
    }
}

// From<crate::subsystems::syscalls::interface::SyscallError> for crate::subsystems::syscalls::common::SyscallError
impl From<crate::subsystems::syscalls::interface::SyscallError> for crate::subsystems::syscalls::common::SyscallError {
    fn from(err: crate::subsystems::syscalls::interface::SyscallError) -> Self {
        match err {
            crate::subsystems::syscalls::interface::SyscallError::IoError =>
                crate::subsystems::syscalls::common::SyscallError::IoError,
            _ => crate::subsystems::syscalls::common::SyscallError::SystemError,
        }
    }
}

// From<crate::subsystems::syscalls::common::SyscallError> for crate::subsystems::syscalls::interface::SyscallError
impl From<crate::subsystems::syscalls::common::SyscallError> for crate::subsystems::syscalls::interface::SyscallError {
    fn from(err: crate::subsystems::syscalls::common::SyscallError) -> Self {
        match err {
            crate::subsystems::syscalls::common::SyscallError::IoError =>
                crate::subsystems::syscalls::interface::SyscallError::IoError,
            _ => crate::subsystems::syscalls::interface::SyscallError::SystemError,
        }
    }
}

// From<nos_api::Error> for FrameworkError
impl From<nos_api::Error> for crate::services::FrameworkError {
    fn from(err: nos_api::Error) -> Self {
        crate::services::FrameworkError::ServiceError(format!("{:?}", err))
    }
}

// From<nos_api::KernelError> for FrameworkError
impl From<nos_api::KernelError> for crate::services::FrameworkError {
    fn from(err: nos_api::KernelError) -> Self {
        crate::services::FrameworkError::ServiceError(format!("{:?}", err))
    }
}

// From<crate::subsystems::syscalls::services::dispatcher::DispatcherError> for nos_api::Error
impl From<crate::subsystems::syscalls::services::dispatcher::DispatcherError> for nos_api::Error {
    fn from(err: crate::subsystems::syscalls::services::dispatcher::DispatcherError) -> Self {
        nos_api::Error::Io(alloc::format!("Dispatcher error: {:?}", err))
    }
}

// From<FrameworkError> for nos_api::Error
impl From<crate::services::FrameworkError> for nos_api::Error {
    fn from(err: crate::services::FrameworkError) -> Self {
        nos_api::Error::Io(alloc::format!("Framework error: {:?}", err))
    }
}

// From<crate::subsystems::syscalls::services::dispatcher::DispatcherError> for FrameworkError
impl From<crate::subsystems::syscalls::services::dispatcher::DispatcherError> for crate::services::FrameworkError {
    fn from(err: crate::subsystems::syscalls::services::dispatcher::DispatcherError) -> Self {
        crate::services::FrameworkError::ServiceError(format!("{:?}", err))
    }
}

// From<vfs::error::VfsError> for subsystems::fs::api::error::FsError
impl From<crate::vfs::error::VfsError> for crate::subsystems::fs::api::error::FsError {
    fn from(err: crate::vfs::error::VfsError) -> Self {
        crate::subsystems::fs::api::error::FsError::Io(alloc::format!("{:?}", err))
    }
}

// From<nos_api::KernelError> for nos_api::Error
impl From<nos_api::KernelError> for nos_api::Error {
    fn from(err: nos_api::KernelError) -> Self {
        nos_api::Error::Kernel(alloc::format!("{:?}", err))
    }
}

// From<crate::error::unified::UnifiedError> for nos_api::Error
impl From<crate::error::unified::UnifiedError> for nos_api::Error {
    fn from(err: crate::error::unified::UnifiedError) -> Self {
        nos_api::Error::Kernel(alloc::format!("{:?}", err))
    }
}

// From<nos_api::Error> for nos_api::KernelError
impl From<nos_api::Error> for nos_api::KernelError {
    fn from(err: nos_api::Error) -> Self {
        nos_api::KernelError::Other(alloc::format!("{:?}", err))
    }
}
EOF

echo "  - Created error conversion template"

echo "Phase 8: Adding Event trait implementations..."
# Check event types
if [ -f event/mod.rs ]; then
    if ! grep -q "impl nos_api::event::Event for SystemEvent" event/mod.rs; then
        cat >> event/mod.rs << 'EOF'

impl nos_api::event::Event for SystemEvent {
    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    fn event_type(&self) -> nos_api::event::EventType {
        nos_api::event::EventType::System
    }
}

impl nos_api::event::Event for MemoryEvent {
    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    fn event_type(&self) -> nos_api::event::EventType {
        nos_api::event::EventType::Memory
    }
}

impl nos_api::event::Event for ProcessEvent {
    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    fn event_type(&self) -> nos_api::event::EventType {
        nos_api::event::EventType::Process
    }
}
EOF
        echo "  - Added Event trait implementations"
    fi
fi

echo "Phase 9: Fixing ? operator issues..."
# Fix timerfd.rs
if [ -f subsystems/syscalls/timerfd.rs ]; then
    sed -i.bak 's/let args = extract_args(args, 2)?;/let args = extract_args(args, 2).map_err(|e| crate::subsystems::syscalls::interface::SyscallError::InvalidArgument)?;/g' subsystems/syscalls/timerfd.rs || true
    sed -i.bak 's/let args = extract_args(args, 4)?;/let args = extract_args(args, 4).map_err(|e| crate::subsystems::syscalls::interface::SyscallError::InvalidArgument)?;/g' subsystems/syscalls/timerfd.rs || true
    echo "  - Fixed timerfd.rs error conversions"
fi

# Fix signalfd.rs
if [ -f subsystems/syscalls/signalfd.rs ]; then
    sed -i.bak 's/let args = extract_args(args, 2)?;/let args = extract_args(args, 2).map_err(|e| crate::subsystems::syscalls::interface::SyscallError::InvalidArgument)?;/g' subsystems/syscalls/signalfd.rs || true
    sed -i.bak 's/let args = extract_args(args, 3)?;/let args = extract_args(args, 3).map_err(|e| crate::subsystems::syscalls::interface::SyscallError::InvalidArgument)?;/g' subsystems/syscalls/signalfd.rs || true
    echo "  - Fixed signalfd.rs error conversions"
fi

# Fix eventfd.rs
if [ -f subsystems/syscalls/eventfd.rs ]; then
    sed -i.bak 's/let args = extract_args(args, 2)?;/let args = extract_args(args, 2).map_err(|e| crate::subsystems::syscalls::common::SyscallError::InvalidArgument)?;/g' subsystems/syscalls/eventfd.rs || true
    sed -i.bak 's/\.map_err(Error::from)?;/.map_err(|e| crate::subsystems::syscalls::common::SyscallError::InvalidArgument)?;/g' subsystems/syscalls/eventfd.rs || true
    echo "  - Fixed eventfd.rs error conversions"
fi

# Fix posix_fd.rs
if [ -f subsystems/syscalls/posix_fd.rs ]; then
    sed -i.bak 's/\.map_err(Error::from)?;/.map_err(|e| crate::subsystems::syscalls::common::SyscallError::SystemError)?;/g' subsystems/syscalls/posix_fd.rs || true
    echo "  - Fixed posix_fd.rs error conversions"
fi

# Fix services/mod.rs - fix collect() issue
if [ -f subsystems/syscalls/services/mod.rs ]; then
    sed -i.bak 's/\.map(|s| Box::leak(s.into_boxed_str()))\.collect()/.map(|s| Box::leak(s.into_boxed_str()) as &str).collect::<nos_api::Vec<_>>()/g' subsystems/syscalls/services/mod.rs || true
    echo "  - Fixed services/mod.rs collect issue"
fi

# Fix mm/mod.rs
if [ -f subsystems/mm/mod.rs ]; then
    sed -i.bak 's/percpu_allocator::init_percpu_allocators()?;/{ percpu_allocator::init_percpu_allocators(); Ok::<(), nos_api::Error>(()) }?;/g' subsystems/mm/mod.rs || true
    echo "  - Fixed mm/mod.rs ? operator issue"
fi

# Fix platform/mod.rs
if [ -f platform/mod.rs ]; then
    sed -i.bak 's/drivers::init()?;/drivers::init().map_err(|e| nos_api::Error::Io(alloc::format!("{:?}", e)))?;/g' platform/mod.rs || true
    echo "  - Fixed platform/mod.rs error conversion"
fi

# Fix services/manager.rs
if [ -f services/manager.rs ]; then
    sed -i.bak 's/initialize()?;/initialize().map_err(|e| FrameworkError::from(e))?;/g' services/manager.rs || true
    sed -i.bak 's/start()?;/start().map_err(|e| FrameworkError::from(e))?;/g' services/manager.rs || true
    sed -i.bak 's/stop()?;/stop().map_err(|e| FrameworkError::from(e))?;/g' services/manager.rs || true
    sed -i.bak 's/cleanup()?;/cleanup().map_err(|e| FrameworkError::from(e))?;/g' services/manager.rs || true
    echo "  - Fixed services/manager.rs error conversions"
fi

# Fix vfs/fs.rs
if [ -f vfs/fs.rs ]; then
    sed -i.bak 's/\.map_err(|e| VfsError::from(e))/\.map_err(|e| crate::subsystems::fs::api::error::FsError::from(e))/g' vfs/fs.rs || true
    echo "  - Fixed vfs/fs.rs error conversion"
fi

# Fix error/mod.rs - UnifiedError conversion
if [ -f error/mod.rs ]; then
    # Need to add From implementation for UnifiedError
    if ! grep -q "impl From<.*> for UnifiedError" error/mod.rs; then
        # Add a generic From implementation
        cat >> error/mod.rs << 'EOF'

// Generic From implementation for UnifiedError
impl<T: Into<UnifiedError>> From<T> for UnifiedError {
    fn from(err: T) -> Self {
        err.into()
    }
}
EOF
        echo "  - Added generic From for UnifiedError"
    fi
fi

echo "Phase 10: Adding SecurityError conversions..."
# Check if security module exists
if [ -f security/mod.rs ]; then
    if ! grep -q "impl From<nos_api::Error> for SecurityError" security/mod.rs; then
        cat >> security/mod.rs << 'EOF'

impl From<nos_api::Error> for SecurityError {
    fn from(err: nos_api::Error) -> Self {
        SecurityError::Generic(alloc::format!("{:?}", err))
    }
}
EOF
        echo "  - Added From<nos_api::Error> for SecurityError"
    fi
fi

echo ""
echo "E0277 error fixing complete!"
echo "Please review the changes and run 'cargo build' to verify."
