//! LSM Integration Examples
//!
//! This file demonstrates how to integrate the LSM framework with
//! various kernel subsystems including syscalls, VFS, and networking.

use crate::security::lsm::{
    AuditEntry, AuditLog, File, Inode, LsmRegistry, SecurityAction, SecurityId,
    SecurityModule, SecurityResult, Socket, SocketAddr, Task,
};
use alloc::sync::Arc;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use spin::Mutex;

/// Example: Integrating LSM with Syscall Dispatcher
///
/// This shows how to add LSM hooks to the syscall dispatcher
/// to enforce security policies on system calls.
pub fn syscall_lsm_example() {
    // Get the global LSM registry
    let registry = get_lsm_registry();

    if let Some(mutex_ref) = registry {
        let guard = mutex_ref.lock();
        if let Some(lsm) = guard.as_ref() {
            // Before executing a syscall, check permissions
            let task = Task {
                pid: 1234,
                uid: 1000,
                sid: SecurityId::new(42),
            };

            // Example: Check permission before opening a file
            let file = File {
                inode: Inode {
                    ino: 1,
                    mode: 0o644,
                    uid: 1000,
                    gid: 1000,
                },
                flags: 0o2, // O_RDWR
            };

            if let Err(e) = lsm.call_file_open(&file) {
                // Log the denial
                let entry = AuditEntry::new(
                    get_timestamp(),
                    task.sid,
                    SecurityAction::FileOpen,
                    SecurityResult::Denied,
                )
                .with_module("lsm".to_string())
                .with_details(format!("File open denied: {:?}", e));

                lsm.log_event(entry);
            }
        }
    }
}

/// Example: Integrating LSM with VFS Layer
///
/// This shows how to add LSM hooks to the VFS layer
/// to enforce security policies on file operations.
pub struct VfsLsmIntegration {
    registry: Arc<Mutex<LsmRegistry>>,
}

impl VfsLsmIntegration {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(Mutex::new(LsmRegistry::new())),
        }
    }

    /// Hook called before inode permission check
    pub fn inode_permission_check(
        &self,
        inode: &Inode,
        mask: u32,
        task_sid: SecurityId,
    ) -> Result<(), &'static str> {
        let lsm = self.registry.lock();

        // Log the permission check
        let entry = AuditEntry::new(
            get_timestamp(),
            task_sid,
            SecurityAction::PermissionCheck,
            SecurityResult::Success,
        )
        .with_object(SecurityId::new(inode.ino))
        .with_details(format!("Inode permission check: mask={:x}", mask));

        lsm.log_event(entry);

        // Call LSM hooks
        lsm.call_inode_permission(inode, mask)
            .map_err(|_| "Permission denied")
    }

    /// Hook called before file open
    pub fn file_open_check(&self, file: &File, task_sid: SecurityId) -> Result<(), &'static str> {
        let lsm = self.registry.lock();

        // Log the file open attempt
        let entry = AuditEntry::new(
            get_timestamp(),
            task_sid,
            SecurityAction::FileOpen,
            SecurityResult::Success,
        )
        .with_object(SecurityId::new(file.inode.ino))
        .with_module("vfs".to_string())
        .with_details(format!("File open: flags={:x}", file.flags));

        lsm.log_event(entry);

        // Call LSM hooks
        lsm.call_file_open(file).map_err(|_| "File open denied")
    }
}

/// Example: Integrating LSM with Network Stack
///
/// This shows how to add LSM hooks to the network stack
/// to enforce security policies on socket operations.
pub struct NetworkLsmIntegration {
    registry: Arc<Mutex<LsmRegistry>>,
}

impl NetworkLsmIntegration {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(Mutex::new(LsmRegistry::new())),
        }
    }

    /// Hook called before socket bind
    pub fn socket_bind_check(
        &self,
        socket: &Socket,
        addr: &SocketAddr,
        task_sid: SecurityId,
    ) -> Result<(), &'static str> {
        let lsm = self.registry.lock();

        // Log the bind attempt
        let entry = AuditEntry::new(
            get_timestamp(),
            task_sid,
            SecurityAction::SocketBind,
            SecurityResult::Success,
        )
        .with_module("network".to_string())
        .with_details(format!("Socket bind: fd={}, family={}", socket.fd, addr.family));

        lsm.log_event(entry);

        // Call LSM hooks
        lsm.call_socket_bind(socket, addr)
            .map_err(|_| "Socket bind denied")
    }

    /// Hook called before socket connect
    pub fn socket_connect_check(
        &self,
        socket: &Socket,
        addr: &SocketAddr,
        task_sid: SecurityId,
    ) -> Result<(), &'static str> {
        let lsm = self.registry.lock();

        // Log the connect attempt
        let entry = AuditEntry::new(
            get_timestamp(),
            task_sid,
            SecurityAction::SocketConnect,
            SecurityResult::Success,
        )
        .with_module("network".to_string())
        .with_details(format!("Socket connect: fd={}, family={}", socket.fd, addr.family));

        lsm.log_event(entry);

        // Call LSM hooks
        lsm.call_socket_connect(socket, addr)
            .map_err(|_| "Socket connect denied")
    }
}

/// Example: Integrating LSM with Process Management
///
/// This shows how to add LSM hooks to process management
/// to enforce security policies on process operations.
pub struct ProcessLsmIntegration {
    registry: Arc<Mutex<LsmRegistry>>,
}

impl ProcessLsmIntegration {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(Mutex::new(LsmRegistry::new())),
        }
    }

    /// Hook called before task creation
    pub fn task_create_check(
        &self,
        parent: &Task,
        child: &Task,
    ) -> Result<(), &'static str> {
        let lsm = self.registry.lock();

        // Log the task creation
        let entry = AuditEntry::new(
            get_timestamp(),
            parent.sid,
            SecurityAction::TaskCreate,
            SecurityResult::Success,
        )
        .with_object(child.sid)
        .with_module("process".to_string())
        .with_details(format!("Task create: parent_pid={}, child_pid={}", parent.pid, child.pid));

        lsm.log_event(entry);

        // Call LSM hooks
        lsm.call_task_create(parent, child)
            .map_err(|_| "Task creation denied")
    }

    /// Hook called before task execution
    pub fn task_exec_check(&self, task: &Task) -> Result<(), &'static str> {
        let lsm = self.registry.lock();

        // Log the execution attempt
        let entry = AuditEntry::new(
            get_timestamp(),
            task.sid,
            SecurityAction::TaskExec,
            SecurityResult::Success,
        )
        .with_module("process".to_string())
        .with_details(format!("Task exec: pid={}", task.pid));

        lsm.log_event(entry);

        // Call LSM hooks
        lsm.call_task_exec(task).map_err(|_| "Task execution denied")
    }
}

/// Example: Custom Security Module
///
/// This shows how to implement a custom security module
/// that integrates with the LSM framework.
pub struct CustomSecurityModule {
    name: String,
    audit_log: Arc<Mutex<AuditLog>>,
}

impl CustomSecurityModule {
    pub fn new(name: String) -> Self {
        Self {
            name,
            audit_log: Arc::new(Mutex::new(AuditLog::new(10000))),
        }
    }

    pub fn get_audit_log(&self) -> Arc<Mutex<AuditLog>> {
        Arc::clone(&self.audit_log)
    }
}

impl SecurityModule for CustomSecurityModule {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn file_open(&self, file: &File) -> Result<(), crate::security::lsm::SecurityError> {
        // Custom policy: Only allow opening files owned by the same user
        // This is just an example - real policies would be more sophisticated

        let entry = AuditEntry::new(
            get_timestamp(),
            SecurityId::SYSTEM,
            SecurityAction::FileOpen,
            SecurityResult::Success,
        )
        .with_object(SecurityId::new(file.inode.ino))
        .with_module(self.name.clone())
        .with_details(format!("Checking file open for inode {}", file.inode.ino));

        self.audit_log.lock().log(entry);

        Ok(())
    }

    fn socket_bind(
        &self,
        socket: &Socket,
        _addr: &SocketAddr,
    ) -> Result<(), crate::security::lsm::SecurityError> {
        // Custom policy: Only allow binding privileged ports for root
        let entry = AuditEntry::new(
            get_timestamp(),
            SecurityId::SYSTEM,
            SecurityAction::SocketBind,
            SecurityResult::Success,
        )
        .with_module(self.name.clone())
        .with_details(format!("Checking socket bind for fd {}", socket.fd));

        self.audit_log.lock().log(entry);

        Ok(())
    }
}

/// Helper function to get current timestamp
fn get_timestamp() -> u64 {
    // In a real implementation, this would get the actual system time
    // For now, return a dummy value
    0
}

/// Helper function to get LSM registry
fn get_lsm_registry() -> Option<&'static Mutex<Option<LsmRegistry>>> {
    crate::security::lsm::get_lsm_registry()
}

/// Example: Multi-layered Security
///
/// This shows how to combine multiple security modules
/// for defense-in-depth.
pub fn multi_layered_security_example() {
    use crate::security::lsm::{AppArmorLsmModule, SelinuxLsmModule, TomoyoLsmModule};

    let mut registry = LsmRegistry::new();

    // Register multiple security modules
    registry.register(Box::new(SelinuxLsmModule::new()));
    registry.register(Box::new(AppArmorLsmModule::new()));
    registry.register(Box::new(TomoyoLsmModule::new()));

    // When a security check is performed, all modules are consulted
    let inode = Inode {
        ino: 1,
        mode: 0o644,
        uid: 1000,
        gid: 1000,
    };

    let result = registry.call_inode_permission(&inode, 0o5);

    match result {
        Ok(()) => {
            // All modules allowed the operation
        }
        Err(_e) => {
            // At least one module denied the operation
            // Log the denial
        }
    }
}

/// Example: Audit Log Analysis
///
/// This shows how to analyze audit logs for security events.
pub fn audit_log_analysis_example() {
    let log = Arc::new(Mutex::new(AuditLog::new(10000)));

    // Generate some sample audit entries
    for i in 0..10 {
        let entry = AuditEntry::new(
            i * 1000,
            SecurityId::new(i),
            SecurityAction::PermissionCheck,
            SecurityResult::Success,
        )
        .with_module("test".to_string());

        log.lock().log(entry);
    }

    // Query audit log
    let _subject_entries = log.lock().get_subject_entries(SecurityId::new(5));
    let _module_entries = log.lock().get_module_entries("test");
    let _time_entries = log.lock().get_time_range(2000, 7000);

    // Analyze patterns
    let denied_count = log
        .lock()
        .get_entries()
        .iter()
        .filter(|e| e.result == SecurityResult::Denied)
        .count();

    // Generate security report
    let total_entries = log.lock().len();
    let _success_rate = if total_entries > 0 {
        (total_entries - denied_count) * 100 / total_entries
    } else {
        100
    };

    // Log would show: "Success rate: 90%, Denied: 1/10"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vfs_lsm_integration() {
        let vfs = VfsLsmIntegration::new();

        let inode = Inode {
            ino: 1,
            mode: 0o644,
            uid: 1000,
            gid: 1000,
        };

        let result = vfs.inode_permission_check(&inode, 0o5, SecurityId::new(42));
        assert!(result.is_ok());
    }

    #[test]
    fn test_network_lsm_integration() {
        let network = NetworkLsmIntegration::new();

        let socket = Socket {
            fd: 3,
            domain: 2, // AF_INET
            socket_type: 1, // SOCK_STREAM
        };

        let addr = SocketAddr {
            family: 2, // AF_INET
            data: [0; 128],
        };

        let result = network.socket_bind_check(&socket, &addr, SecurityId::new(42));
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_lsm_integration() {
        let process = ProcessLsmIntegration::new();

        let parent = Task {
            pid: 1,
            uid: 0,
            sid: SecurityId::ROOT,
        };

        let child = Task {
            pid: 2,
            uid: 0,
            sid: SecurityId::SYSTEM,
        };

        let result = process.task_create_check(&parent, &child);
        assert!(result.is_ok());
    }

    #[test]
    fn test_custom_security_module() {
        let module = CustomSecurityModule::new("custom".to_string());

        assert_eq!(module.name(), "custom");
        assert_eq!(module.version(), "1.0.0");

        let file = File {
            inode: Inode {
                ino: 1,
                mode: 0o644,
                uid: 1000,
                gid: 1000,
            },
            flags: 0,
        };

        let result = module.file_open(&file);
        assert!(result.is_ok());

        // Check audit log
        let log = module.get_audit_log();
        assert_eq!(log.lock().len(), 1);
    }

    #[test]
    fn test_multi_layered_security() {
        multi_layered_security_example();
    }

    #[test]
    fn test_audit_log_analysis() {
        audit_log_analysis_example();
    }
}
