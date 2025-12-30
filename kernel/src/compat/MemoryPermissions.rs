//! Memory permissions for cross-platform compatibility

use core::fmt;
use alloc::string::String;

/// Re-export the struct at module level for easier access
pub use self::MemoryPermissionsStruct as MemoryPermissions;

/// Memory protection permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryPermissionsStruct {
    /// Read permission
    pub read: bool,
    /// Write permission
    pub write: bool,
    /// Execute permission
    pub execute: bool,
}

impl MemoryPermissionsStruct {
    /// Create new memory permissions
    pub fn new(read: bool, write: bool, execute: bool) -> Self {
        Self {
            read,
            write,
            execute,
        }
    }

    /// Read-only permissions
    pub fn readonly() -> Self {
        Self::new(true, false, false)
    }

    /// Read-write permissions
    pub fn readwrite() -> Self {
        Self::new(true, true, false)
    }

    /// Read-execute permissions (for code)
    pub fn read_exec() -> Self {
        Self::new(true, false, true)
    }

    /// No access permissions
    pub fn none() -> Self {
        Self::new(false, false, false)
    }

    /// Check if this permission allows reading
    pub fn allows_read(&self) -> bool {
        self.read
    }

    /// Check if this permission allows writing
    pub fn allows_write(&self) -> bool {
        self.write
    }

    /// Check if this permission allows execution
    pub fn allows_execute(&self) -> bool {
        self.execute
    }
}

impl fmt::Display for MemoryPermissionsStruct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let perms = [
            if self.read { 'r' } else { '-' },
            if self.write { 'w' } else { '-' },
            if self.execute { 'x' } else { '-' },
        ];
        write!(f, "{}", perms.iter().collect::<String>())
    }
}

impl Default for MemoryPermissionsStruct {
    fn default() -> Self {
        Self::readwrite()
    }
}
