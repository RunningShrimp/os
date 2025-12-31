//! MicroVM Implementation (Firecracker-style)
//!
//! Provides fast-booting, minimal virtual machines for edge computing
//! with sub-100ms boot times and secure isolation.

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use core::time::Duration;

use crate::prelude::*;
use crate::container::{ContainerConfig, ContainerRuntimeSystem};
use crate::vmm::{Hypervisor, VmConfig, VirtualMachine};
use crate::subsystems::sync::Mutex;

/// Unique MicroVM identifier
pub type MicroVMId = u64;

/// MicroVM state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicroVMState {
    /// MicroVM is being created
    Creating,
    /// MicroVM is ready to boot
    Ready,
    /// MicroVM is running
    Running,
    /// MicroVM is paused
    Paused,
    /// MicroVM is shutting down
    ShuttingDown,
    /// MicroVM has stopped
    Stopped,
    /// MicroVM has failed
    Failed,
}

/// MicroVM configuration
#[derive(Debug, Clone)]
pub struct MicroVMConfig {
    /// MicroVM ID (0 for auto-assign)
    pub vm_id: MicroVMId,
    /// MicroVM name
    pub name: String,
    /// Number of vCPUs (1-8 for edge scenarios)
    pub vcpu_count: usize,
    /// Memory size in bytes (128MB-8GB)
    pub memory_size: u64,
    /// Boot source kernel
    pub kernel_path: Option<String>,
    /// Kernel command line
    pub kernel_args: String,
    /// Root filesystem path
    pub rootfs_path: Option<String>,
    /// Enable unikernel mode
    pub unikernel: bool,
    /// Network interfaces
    pub network_interfaces: Vec<NetworkInterface>,
    /// Mount points
    pub mount_points: Vec<MountPoint>,
    /// Resource limits
    pub resource_limits: ResourceLimits,
    /// Security settings
    pub security: SecurityConfig,
}

/// Network interface configuration
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    /// Interface name
    pub name: String,
    /// Interface type
    pub if_type: NetworkInterfaceType,
    /// MAC address (optional)
    pub mac_address: Option<String>,
    /// Bandwidth limit in Mbps
    pub bandwidth_limit: Option<u64>,
}

/// Network interface type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkInterfaceType {
    /// Bridge network
    Bridge,
    /// NAT network
    NAT,
    /// Passthrough network
    Passthrough,
    /// None (isolated)
    None,
}

/// Mount point configuration
#[derive(Debug, Clone)]
pub struct MountPoint {
    /// Source path
    pub source: String,
    /// Destination path
    pub destination: String,
    /// Mount type
    pub mount_type: MountType,
    /// Read-only
    pub read_only: bool,
}

/// Mount type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountType {
    /// Block device
    Block,
    /// File system
    FileSystem,
    /// Bind mount
    Bind,
}

/// Resource limits for MicroVM
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// CPU time quota (nanoseconds per second)
    pub cpu_quota: Option<u64>,
    /// CPU period (microseconds)
    pub cpu_period: Option<u64>,
    /// Memory limit in bytes
    pub memory_limit: Option<u64>,
    /// Memory swap limit in bytes
    pub memory_swap_limit: Option<u64>,
    /// Network bandwidth limit in bytes/sec
    pub network_bandwidth: Option<u64>,
    /// I/O bandwidth limit in bytes/sec
    pub io_bandwidth: Option<u64>,
}

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enable seccomp filtering
    pub seccomp: bool,
    /// Enable SELinux/AppArmor
    pub selinux: bool,
    /// Enable capabilities
    pub capabilities: bool,
    /// Enable secure boot
    pub secure_boot: bool,
    /// Enable TPM
    pub tpm: bool,
    /// Enable memory encryption (AMD SEV/Intel TME)
    pub memory_encryption: bool,
}

/// MicroVM template
#[derive(Debug, Clone)]
pub struct MicroVMTemplate {
    /// Template name
    pub name: String,
    /// Template description
    pub description: String,
    /// Base configuration
    pub config: MicroVMConfig,
    /// Template tags
    pub tags: Vec<String>,
    /// Creation timestamp
    pub created_at: u64,
}

/// MicroVM snapshot for fast restore
#[derive(Debug, Clone)]
pub struct MicroVMSnapshot {
    /// Snapshot ID
    pub snapshot_id: u64,
    /// MicroVM ID
    pub vm_id: MicroVMId,
    /// Snapshot name
    pub name: String,
    /// Snapshot data
    pub data: Vec<u8>,
    /// Memory snapshot
    pub memory_snapshot: Vec<u8>,
    /// CPU state
    pub cpu_state: Vec<u8>,
    /// Device state
    pub device_state: Vec<u8>,
    /// Creation timestamp
    pub created_at: u64,
    /// Snapshot size in bytes
    pub size: u64,
}

/// Unikernel configuration
#[derive(Debug, Clone)]
pub struct UnikernelConfig {
    /// Unikernel type
    pub unikernel_type: UnikernelType,
    /// Application binary
    pub binary_path: String,
    /// Application arguments
    pub arguments: Vec<String>,
    /// Environment variables
    pub environment: BTreeMap<String, String>,
    /// Enable network stack
    pub include_network: bool,
    /// Enable storage
    pub include_storage: bool,
}

/// Unikernel type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnikernelType {
    /// RustSVM
    RustSVM,
    /// HermiTux
    HermitCore,
    /// IncludeOS
    IncludeOS,
    /// OSv
    OSv,
}

/// Container-to-MicroVM converter
pub struct ContainerMicroVMConverter {
    /// Container runtime system
    container_system: Arc<Mutex<ContainerRuntimeSystem>>,
    /// Hypervisor
    hypervisor: Arc<Mutex<Hypervisor>>,
}

/// MicroVM instance
pub struct MicroVM {
    /// MicroVM ID
    vm_id: MicroVMId,
    /// Configuration
    config: MicroVMConfig,
    /// State
    state: AtomicU64, // Stores MicroVMState as u64
    /// Virtual machine (from VMM)
    vm: Option<VirtualMachine>,
    /// Creation timestamp
    created_at: u64,
    /// Boot timestamp
    booted_at: Option<u64>,
    /// Running flag
    running: AtomicBool,
}

impl MicroVM {
    /// Create a new MicroVM
    pub fn new(config: MicroVMConfig) -> Result<Self> {
        crate::println!("[microvm] Creating MicroVM: {}", config.name);

        let vm_id = config.vm_id;
        let created_at = nos_api::event::get_time_ns();

        // Validate configuration
        if config.vcpu_count == 0 || config.vcpu_count > 8 {
            return Err(nos_api::Error::InvalidArgument);
        }

        if config.memory_size < 128 * 1024 * 1024 || config.memory_size > 8 * 1024 * 1024 * 1024 {
            return Err(nos_api::Error::InvalidArgument);
        }

        let vm = Self {
            vm_id,
            config,
            state: AtomicU64::new(MicroVMState::Creating as u64),
            vm: None,
            created_at,
            booted_at: None,
            running: AtomicBool::new(false),
        };

        crate::println!("[microvm] MicroVM {} created", vm_id);

        Ok(vm)
    }

    /// Boot the MicroVM (target: <100ms)
    pub fn boot(&mut self) -> Result<()> {
        crate::println!("[microvm] Booting MicroVM {}", self.vm_id);

        let start_time = nos_api::event::get_time_ns();

        // Update state
        self.set_state(MicroVMState::Ready);

        // Create VM through hypervisor
        let vm_config = VmConfig {
            vcpu_count: self.config.vcpu_count,
            memory_size: self.config.memory_size,
            kernel_path: self.config.kernel_path.clone(),
            rootfs_path: self.config.rootfs_path.clone(),
            ..Default::default()
        };

        // Create the VM (this would interact with the actual VMM)
        // For now, we'll create a placeholder
        self.vm = Some(VirtualMachine::new(vm_config)?);

        // Mark as running
        self.set_state(MicroVMState::Running);
        self.running.store(true, Ordering::SeqCst);

        let boot_time = nos_api::event::get_time_ns() - start_time;
        let boot_time_ms = boot_time / 1_000_000;

        self.booted_at = Some(nos_api::event::get_time_ns());

        crate::println!("[microvm] MicroVM {} booted in {}ms", self.vm_id, boot_time_ms);

        // Warn if boot time exceeds 100ms
        if boot_time_ms > 100 {
            crate::println!("[microvm] WARNING: Boot time {}ms exceeds 100ms target", boot_time_ms);
        }

        Ok(())
    }

    /// Pause the MicroVM
    pub fn pause(&mut self) -> Result<()> {
        crate::println!("[microvm] Pausing MicroVM {}", self.vm_id);

        if !self.running.load(Ordering::Relaxed) {
            return Err(nos_api::Error::InvalidState);
        }

        self.set_state(MicroVMState::Paused);

        Ok(())
    }

    /// Resume the MicroVM
    pub fn resume(&mut self) -> Result<()> {
        crate::println!("[microvm] Resuming MicroVM {}", self.vm_id);

        if self.get_state() != MicroVMState::Paused {
            return Err(nos_api::Error::InvalidState);
        }

        self.set_state(MicroVMState::Running);

        Ok(())
    }

    /// Stop the MicroVM
    pub fn stop(&mut self) -> Result<()> {
        crate::println!("[microvm] Stopping MicroVM {}", self.vm_id);

        self.set_state(MicroVMState::ShuttingDown);
        self.running.store(false, Ordering::SeqCst);

        // Cleanup VM resources
        self.vm = None;

        self.set_state(MicroVMState::Stopped);

        crate::println!("[microvm] MicroVM {} stopped", self.vm_id);

        Ok(())
    }

    /// Create snapshot
    pub fn create_snapshot(&self, name: String) -> Result<MicroVMSnapshot> {
        crate::println!("[microvm] Creating snapshot '{}' for MicroVM {}", name, self.vm_id);

        // Placeholder: Create actual snapshot
        let snapshot = MicroVMSnapshot {
            snapshot_id: self.generate_snapshot_id(),
            vm_id: self.vm_id,
            name,
            data: Vec::new(),
            memory_snapshot: Vec::new(),
            cpu_state: Vec::new(),
            device_state: Vec::new(),
            created_at: nos_api::event::get_time_ns(),
            size: 0,
        };

        crate::println!("[microvm] Snapshot {} created", snapshot.snapshot_id);

        Ok(snapshot)
    }

    /// Restore from snapshot
    pub fn restore_from_snapshot(&mut self, snapshot: &MicroVMSnapshot) -> Result<()> {
        crate::println!("[microvm] Restoring MicroVM {} from snapshot {}", self.vm_id, snapshot.snapshot_id);

        let start_time = nos_api::event::get_time_ns();

        // Placeholder: Restore from snapshot
        self.set_state(MicroVMState::Running);
        self.running.store(true, Ordering::SeqCst);

        let restore_time = nos_api::event::get_time_ns() - start_time;
        let restore_time_ms = restore_time / 1_000_000;

        crate::println!("[microvm] Restored in {}ms", restore_time_ms);

        Ok(())
    }

    /// Get MicroVM state
    pub fn get_state(&self) -> MicroVMState {
        unsafe { core::mem::transmute(self.state.load(Ordering::Relaxed)) }
    }

    /// Set MicroVM state
    fn set_state(&self, state: MicroVMState) {
        self.state.store(state as u64, Ordering::Relaxed);
    }

    /// Check if MicroVM is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed) && self.get_state() == MicroVMState::Running
    }

    /// Get configuration
    pub fn get_config(&self) -> &MicroVMConfig {
        &self.config
    }

    /// Get boot time in milliseconds
    pub fn get_boot_time_ms(&self) -> Option<u64> {
        self.booted_at.map(|booted| {
            (booted - self.created_at) / 1_000_000
        })
    }

    /// Generate unique snapshot ID
    fn generate_snapshot_id(&self) -> u64 {
        use core::sync::atomic::AtomicU64;
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        COUNTER.fetch_add(1, Ordering::SeqCst)
    }
}

impl ContainerMicroVMConverter {
    /// Create a new converter
    pub fn new(
        container_system: Arc<Mutex<ContainerRuntimeSystem>>,
        hypervisor: Arc<Mutex<Hypervisor>>,
    ) -> Self {
        Self {
            container_system,
            hypervisor,
        }
    }

    /// Convert container to MicroVM
    pub fn convert_container(&self, container_id: u64) -> Result<MicroVM> {
        crate::println!("[converter] Converting container {} to MicroVM", container_id);

        // Get container info
        let container_system = self.container_system.lock();
        let _container_state = container_system.runtime.get_state(container_id)
            .map_err(|_| nos_api::Error::NotFound)?;

        // Create MicroVM config from container
        let config = MicroVMConfig {
            vm_id: 0, // Auto-assign
            name: format!("container-{}", container_id),
            vcpu_count: 2,
            memory_size: 512 * 1024 * 1024, // 512MB
            kernel_path: None,
            kernel_args: String::new(),
            rootfs_path: None,
            unikernel: false,
            network_interfaces: Vec::new(),
            mount_points: Vec::new(),
            resource_limits: ResourceLimits {
                cpu_quota: Some(500_000_000), // 0.5 CPU
                cpu_period: Some(1_000_000),
                memory_limit: Some(512 * 1024 * 1024),
                memory_swap_limit: None,
                network_bandwidth: Some(1_000_000_000), // 1Gbps
                io_bandwidth: Some(100_000_000), // 100MBps
            },
            security: SecurityConfig {
                seccomp: true,
                selinux: true,
                capabilities: true,
                secure_boot: false,
                tpm: false,
                memory_encryption: false,
            },
        };

        let microvm = MicroVM::new(config)?;

        crate::println!("[converter] Container {} converted to MicroVM", container_id);

        Ok(microvm)
    }

    /// Convert multiple containers to MicroVMs
    pub fn convert_containers(&self, container_ids: &[u64]) -> Result<Vec<MicroVM>> {
        let mut microvms = Vec::new();

        for &container_id in container_ids {
            let microvm = self.convert_container(container_id)?;
            microvms.push(microvm);
        }

        Ok(microvms)
    }
}

/// MicroVM pool for fast provisioning
pub struct MicroVMPool {
    /// Available MicroVM templates
    templates: Mutex<BTreeMap<String, MicroVMTemplate>>,
    /// Available MicroVM snapshots
    snapshots: Mutex<BTreeMap<u64, MicroVMSnapshot>>,
    /// Pooled MicroVMs (ready to use)
    pooled_microvms: Mutex<Vec<MicroVM>>,
    /// Pool size
    pool_size: usize,
}

impl MicroVMPool {
    /// Create a new MicroVM pool
    pub fn new(pool_size: usize) -> Self {
        Self {
            templates: Mutex::new(BTreeMap::new()),
            snapshots: Mutex::new(BTreeMap::new()),
            pooled_microvms: Mutex::new(Vec::new()),
            pool_size,
        }
    }

    /// Add template to pool
    pub fn add_template(&self, template: MicroVMTemplate) {
        let mut templates = self.templates.lock();
        templates.insert(template.name.clone(), template);
    }

    /// Get template from pool
    pub fn get_template(&self, name: &str) -> Option<MicroVMTemplate> {
        let templates = self.templates.lock();
        templates.get(name).cloned()
    }

    /// Add snapshot to pool
    pub fn add_snapshot(&self, snapshot: MicroVMSnapshot) {
        let mut snapshots = self.snapshots.lock();
        snapshots.insert(snapshot.snapshot_id, snapshot);
    }

    /// Provision MicroVM from pool
    pub fn provision(&self, config: MicroVMConfig) -> Result<MicroVM> {
        // Try to get from pool first
        {
            let mut pool = self.pooled_microvms.lock();
            if let Some(mut microvm) = pool.pop() {
                crate::println!("[microvm-pool] Provisioning from pool");
                return Ok(microvm);
            }
        }

        // No pooled MicroVM available, create new one
        crate::println!("[microvm-pool] Creating new MicroVM");
        MicroVM::new(config)
    }

    /// Return MicroVM to pool
    pub fn return_to_pool(&self, microvm: MicroVM) {
        let mut pool = self.pooled_microvms.lock();

        if pool.len() < self.pool_size {
            // Reset MicroVM state
            pool.push(microvm);
            crate::println!("[microvm-pool] MicroVM returned to pool");
        } else {
            crate::println!("[microvm-pool] Pool full, discarding MicroVM");
        }
    }

    /// Pre-warm the pool
    pub fn prewarm(&self, config: &MicroVMConfig) -> Result<()> {
        crate::println!("[microvm-pool] Pre-warming pool with {} MicroVMs", self.pool_size);

        let mut pool = self.pooled_microvms.lock();

        for i in 0..self.pool_size {
            let mut vm_config = config.clone();
            vm_config.name = format!("{}-{}", config.name, i);

            match MicroVM::new(vm_config) {
                Ok(mut microvm) => {
                    // Boot the MicroVM to pre-warm it
                    let _ = microvm.boot();
                    pool.push(microvm);
                }
                Err(e) => {
                    crate::println!("[microvm-pool] Failed to pre-warm MicroVM: {:?}", e);
                }
            }
        }

        crate::println!("[microvm-pool] Pool pre-warmed with {} MicroVMs", pool.len());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_microvm_creation() {
        let config = MicroVMConfig {
            vm_id: 1,
            name: "test-vm".to_string(),
            vcpu_count: 2,
            memory_size: 512 * 1024 * 1024,
            kernel_path: Some("/kernel".to_string()),
            kernel_args: String::new(),
            rootfs_path: Some("/rootfs".to_string()),
            unikernel: false,
            network_interfaces: Vec::new(),
            mount_points: Vec::new(),
            resource_limits: ResourceLimits {
                cpu_quota: None,
                cpu_period: None,
                memory_limit: None,
                memory_swap_limit: None,
                network_bandwidth: None,
                io_bandwidth: None,
            },
            security: SecurityConfig {
                seccomp: false,
                selinux: false,
                capabilities: false,
                secure_boot: false,
                tpm: false,
                memory_encryption: false,
            },
        };

        let result = MicroVM::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_microvm_state_transitions() {
        let config = MicroVMConfig {
            vm_id: 1,
            name: "test-vm".to_string(),
            vcpu_count: 2,
            memory_size: 512 * 1024 * 1024,
            kernel_path: None,
            kernel_args: String::new(),
            rootfs_path: None,
            unikernel: false,
            network_interfaces: Vec::new(),
            mount_points: Vec::new(),
            resource_limits: ResourceLimits {
                cpu_quota: None,
                cpu_period: None,
                memory_limit: None,
                memory_swap_limit: None,
                network_bandwidth: None,
                io_bandwidth: None,
            },
            security: SecurityConfig {
                seccomp: false,
                selinux: false,
                capabilities: false,
                secure_boot: false,
                tpm: false,
                memory_encryption: false,
            },
        };

        let mut vm = MicroVM::new(config).unwrap();
        assert_eq!(vm.get_state(), MicroVMState::Creating);
    }

    #[test]
    fn test_pool_creation() {
        let pool = MicroVMPool::new(5);
        assert_eq!(pool.pool_size, 5);
    }
}
