//! Virtual Machine Management - VM instantiation and lifecycle management
//!
//! Provides:
//! - Virtual machine creation and management
//! - vCPU state management
//! - VM execution control (run, pause, stop)
//! - Memory management for guest

/// Virtual machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmState {
    /// Created but not running
    Created,
    /// Currently running
    Running,
    /// Stopped/terminated
    Stopped,
    /// Error state
    Error,
}

/// Virtual CPU (vCPU) states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcpuState {
    /// Ready to run
    Ready,
    /// Currently executing
    Running,
    /// Halted
    Halted,
    /// Error state
    Error,
}

/// vCPU registers structure
#[derive(Debug, Clone, Copy)]
pub struct VcpuRegisters {
    /// RAX register
    pub rax: u64,
    /// RBX register
    pub rbx: u64,
    /// RCX register
    pub rcx: u64,
    /// RDX register
    pub rdx: u64,
    /// RSI register
    pub rsi: u64,
    /// RDI register
    pub rdi: u64,
    /// RBP register
    pub rbp: u64,
    /// RSP register
    pub rsp: u64,
    /// RIP register
    pub rip: u64,
    /// RFLAGS register
    pub rflags: u64,
    /// CR0 register
    pub cr0: u64,
    /// CR3 register
    pub cr3: u64,
}

impl VcpuRegisters {
    /// Create CPU registers
    pub fn new() -> Self {
        VcpuRegisters {
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            rbp: 0,
            rsp: 0,
            rip: 0,
            rflags: 0x0002,
            cr0: 0x80000001,
            cr3: 0,
        }
    }

    /// Set instruction pointer
    pub fn set_rip(&mut self, addr: u64) {
        self.rip = addr;
    }

    /// Set stack pointer
    pub fn set_rsp(&mut self, addr: u64) {
        self.rsp = addr;
    }

    /// Get instruction pointer
    pub fn get_rip(&self) -> u64 {
        self.rip
    }
}

/// Virtual CPU (vCPU) structure
#[derive(Debug, Clone)]
pub struct Vcpu {
    /// vCPU ID
    id: u32,
    /// Current state
    state: VcpuState,
    /// Register state
    registers: VcpuRegisters,
    /// Instruction counter
    instruction_count: u64,
    /// Execution time in cycles
    execution_cycles: u64,
    /// Error counter
    errors: u32,
}

impl Vcpu {
    /// Create a new vCPU
    pub fn new(id: u32) -> Self {
        Vcpu {
            id,
            state: VcpuState::Ready,
            registers: VcpuRegisters::new(),
            instruction_count: 0,
            execution_cycles: 0,
            errors: 0,
        }
    }

    /// Get vCPU ID
    pub fn get_id(&self) -> u32 {
        self.id
    }

    /// Get vCPU state
    pub fn get_state(&self) -> VcpuState {
        self.state
    }

    /// Set vCPU state
    fn set_state(&mut self, state: VcpuState) {
        self.state = state;
    }

    /// Start executing vCPU
    pub fn start(&mut self) -> bool {
        if self.state == VcpuState::Ready || self.state == VcpuState::Halted {
            self.set_state(VcpuState::Running);
            true
        } else {
            false
        }
    }

    /// Pause vCPU execution
    pub fn pause(&mut self) -> bool {
        if self.state == VcpuState::Running {
            self.set_state(VcpuState::Ready);
            true
        } else {
            false
        }
    }

    /// Stop vCPU execution
    pub fn stop(&mut self) -> bool {
        if self.state == VcpuState::Running || self.state == VcpuState::Ready {
            self.set_state(VcpuState::Halted);
            true
        } else {
            false
        }
    }

    /// Get vCPU registers
    pub fn get_registers(&self) -> VcpuRegisters {
        self.registers
    }

    /// Set vCPU registers
    pub fn set_registers(&mut self, regs: VcpuRegisters) {
        self.registers = regs;
    }

    /// Increment instruction counter
    pub fn increment_instruction(&mut self) {
        self.instruction_count += 1;
    }

    /// Get instruction count
    pub fn get_instruction_count(&self) -> u64 {
        self.instruction_count
    }

    /// Add execution cycles
    pub fn add_cycles(&mut self, cycles: u64) {
        self.execution_cycles += cycles;
    }

    /// Get total execution cycles
    pub fn get_execution_cycles(&self) -> u64 {
        self.execution_cycles
    }

    /// Increment error counter
    pub fn increment_error(&mut self) {
        self.errors += 1;
    }

    /// Get error count
    pub fn get_error_count(&self) -> u32 {
        self.errors
    }
}

/// Guest memory structure
#[derive(Debug, Clone, Copy)]
pub struct GuestMemory {
    /// Guest physical address base
    pub guest_phys_addr: u64,
    /// Host virtual address mapping
    pub host_virt_addr: u64,
    /// Memory size in bytes
    pub size: u64,
    /// Valid flag
    pub valid: bool,
}

impl GuestMemory {
    /// Create guest memory
    pub fn new(guest_addr: u64, host_addr: u64, size: u64) -> Self {
        GuestMemory {
            guest_phys_addr: guest_addr,
            host_virt_addr: host_addr,
            size,
            valid: true,
        }
    }

    /// Check if address is within bounds
    pub fn contains_address(&self, addr: u64) -> bool {
        addr >= self.guest_phys_addr && addr < self.guest_phys_addr + self.size
    }
}

/// Virtual machine configuration
#[derive(Debug, Clone, Copy)]
pub struct VmConfig {
    /// VM name
    pub name: [u8; 64],
    /// Number of vCPUs
    pub vcpu_count: u32,
    /// Guest memory size in MB
    pub guest_memory_mb: u32,
    /// Enable nested virtualization
    pub nested_virt: bool,
    /// Enable I/O virtualization
    pub io_virt: bool,
}

impl VmConfig {
    /// Create VM configuration
    pub fn new(vcpu_count: u32, memory_mb: u32) -> Self {
        let mut name = [0u8; 64];
        name[0] = b'V';
        name[1] = b'M';
        
        VmConfig {
            name,
            vcpu_count,
            guest_memory_mb: memory_mb,
            nested_virt: false,
            io_virt: false,
        }
    }

    /// Enable nested virtualization
    pub fn enable_nested(&mut self) {
        self.nested_virt = true;
    }

    /// Enable I/O virtualization
    pub fn enable_io_virt(&mut self) {
        self.io_virt = true;
    }
}

/// Virtual machine structure
pub struct VirtualMachine {
    /// VM ID
    id: u32,
    /// VM state
    state: VmState,
    /// Configuration
    config: VmConfig,
    /// vCPUs
    vcpus: [Option<Vcpu>; 16], // Support up to 16 vCPUs
    /// Actual vCPU count
    vcpu_count: usize,
    /// Guest memory
    guest_memory: [Option<GuestMemory>; 16], // Support up to 16 memory regions
    /// Execution count
    execution_count: u32,
    /// Total runtime cycles
    total_cycles: u64,
    /// VM creation time (simulated)
    created_at: u64,
}

impl VirtualMachine {
    /// Create a new virtual machine
    pub fn new(id: u32, config: VmConfig) -> Self {
        let mut vm = VirtualMachine {
            id,
            state: VmState::Created,
            config,
            vcpus: [None, None, None, None, None, None, None, None,
                    None, None, None, None, None, None, None, None],
            vcpu_count: 0,
            guest_memory: [None; 16],
            execution_count: 0,
            total_cycles: 0,
            created_at: 0,
        };

        // Initialize vCPUs based on config
        let count = (config.vcpu_count as usize).min(16);
        for i in 0..count {
            vm.vcpus[i] = Some(Vcpu::new(i as u32));
        }
        vm.vcpu_count = count;

        vm
    }

    /// Get VM ID
    pub fn get_id(&self) -> u32 {
        self.id
    }

    /// Get VM state
    pub fn get_state(&self) -> VmState {
        self.state
    }

    /// Set VM state
    fn set_state(&mut self, state: VmState) {
        self.state = state;
    }

    /// Add guest memory region
    pub fn add_memory(&mut self, guest_addr: u64, host_addr: u64, size: u64) -> bool {
        for i in 0..self.guest_memory.len() {
            if self.guest_memory[i].is_none() {
                self.guest_memory[i] = Some(GuestMemory::new(guest_addr, host_addr, size));
                return true;
            }
        }
        false
    }

    /// Get guest memory regions
    pub fn get_memory_regions(&self) -> u32 {
        let mut count = 0;
        for mem in &self.guest_memory {
            if mem.is_some() {
                count += 1;
            }
        }
        count
    }

    /// Get total guest memory size
    pub fn get_total_memory(&self) -> u64 {
        let mut total = 0;
        for mem in &self.guest_memory {
            if let Some(m) = mem {
                total += m.size;
            }
        }
        total
    }

    /// Start virtual machine
    pub fn start(&mut self) -> bool {
        if self.state != VmState::Created {
            return false;
        }

        // Start all vCPUs
        for i in 0..self.vcpu_count {
            if let Some(vcpu) = &mut self.vcpus[i] {
                if !vcpu.start() {
                    return false;
                }
            }
        }

        self.set_state(VmState::Running);
        true
    }

    /// Pause virtual machine
    pub fn pause(&mut self) -> bool {
        if self.state != VmState::Running {
            return false;
        }

        // Pause all vCPUs
        for i in 0..self.vcpu_count {
            if let Some(vcpu) = &mut self.vcpus[i] {
                if !vcpu.pause() {
                    return false;
                }
            }
        }

        self.set_state(VmState::Stopped);
        true
    }

    /// Stop virtual machine
    pub fn stop(&mut self) -> bool {
        if self.state == VmState::Stopped {
            return true;
        }

        // Stop all vCPUs
        for i in 0..self.vcpu_count {
            if let Some(vcpu) = &mut self.vcpus[i] {
                vcpu.stop();
            }
        }

        self.set_state(VmState::Stopped);
        true
    }

    /// Run VM for a quantum
    pub fn run_quantum(&mut self, cycles: u64) -> bool {
        if self.state != VmState::Running {
            return false;
        }

        self.execution_count += 1;
        self.total_cycles += cycles;

        // Simulate vCPU execution
        if self.vcpu_count > 0 {
            if let Some(vcpu) = &mut self.vcpus[0] {
                vcpu.add_cycles(cycles);
                vcpu.increment_instruction();
            }
        }

        true
    }

    /// Get vCPU by ID
    pub fn get_vcpu(&self, id: u32) -> Option<&Vcpu> {
        if id < self.config.vcpu_count && (id as usize) < self.vcpus.len() {
            self.vcpus[id as usize].as_ref()
        } else {
            None
        }
    }

    /// Get mutable vCPU by ID
    pub fn get_vcpu_mut(&mut self, id: u32) -> Option<&mut Vcpu> {
        if id < self.config.vcpu_count && (id as usize) < self.vcpus.len() {
            self.vcpus[id as usize].as_mut()
        } else {
            None
        }
    }

    /// Get vCPU count
    pub fn get_vcpu_count(&self) -> u32 {
        self.config.vcpu_count
    }

    /// Get execution count
    pub fn get_execution_count(&self) -> u32 {
        self.execution_count
    }

    /// Get total execution cycles
    pub fn get_total_cycles(&self) -> u64 {
        self.total_cycles
    }

    /// Get VM configuration
    pub fn get_config(&self) -> &VmConfig {
        &self.config
    }

    /// Check if VM is running
    pub fn is_running(&self) -> bool {
        self.state == VmState::Running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_states() {
        assert_ne!(VmState::Running, VmState::Paused);
        assert_ne!(VmState::Created, VmState::Stopped);
    }

    #[test]
    fn test_vcpu_states() {
        assert_ne!(VcpuState::Running, VcpuState::Ready);
    }

    #[test]
    fn test_vcpu_registers_creation() {
        let regs = VcpuRegisters::new();
        assert_eq!(regs.rip, 0);
        assert_eq!(regs.rflags, 0x0002);
    }

    #[test]
    fn test_vcpu_register_manipulation() {
        let mut regs = VcpuRegisters::new();
        regs.set_rip(0x1000);
        assert_eq!(regs.get_rip(), 0x1000);
    }

    #[test]
    fn test_vcpu_creation() {
        let vcpu = Vcpu::new(0);
        assert_eq!(vcpu.get_id(), 0);
        assert_eq!(vcpu.get_state(), VcpuState::Ready);
    }

    #[test]
    fn test_vcpu_start() {
        let mut vcpu = Vcpu::new(0);
        assert!(vcpu.start());
        assert_eq!(vcpu.get_state(), VcpuState::Running);
    }

    #[test]
    fn test_vcpu_pause() {
        let mut vcpu = Vcpu::new(0);
        vcpu.start();
        assert!(vcpu.pause());
        assert_eq!(vcpu.get_state(), VcpuState::Ready);
    }

    #[test]
    fn test_vcpu_stop() {
        let mut vcpu = Vcpu::new(0);
        vcpu.start();
        assert!(vcpu.stop());
        assert_eq!(vcpu.get_state(), VcpuState::Halted);
    }

    #[test]
    fn test_vcpu_instruction_count() {
        let mut vcpu = Vcpu::new(0);
        assert_eq!(vcpu.get_instruction_count(), 0);
        vcpu.increment_instruction();
        assert_eq!(vcpu.get_instruction_count(), 1);
    }

    #[test]
    fn test_vcpu_execution_cycles() {
        let mut vcpu = Vcpu::new(0);
        assert_eq!(vcpu.get_execution_cycles(), 0);
        vcpu.add_cycles(100);
        assert_eq!(vcpu.get_execution_cycles(), 100);
    }

    #[test]
    fn test_vcpu_error_count() {
        let mut vcpu = Vcpu::new(0);
        assert_eq!(vcpu.get_error_count(), 0);
        vcpu.increment_error();
        assert_eq!(vcpu.get_error_count(), 1);
    }

    #[test]
    fn test_guest_memory_creation() {
        let mem = GuestMemory::new(0x1000, 0x2000, 0x1000);
        assert_eq!(mem.guest_phys_addr, 0x1000);
        assert_eq!(mem.size, 0x1000);
    }

    #[test]
    fn test_guest_memory_bounds() {
        let mem = GuestMemory::new(0x1000, 0x2000, 0x1000);
        assert!(mem.contains_address(0x1000));
        assert!(mem.contains_address(0x1500));
        assert!(!mem.contains_address(0x2000));
    }

    #[test]
    fn test_vm_config_creation() {
        let cfg = VmConfig::new(2, 512);
        assert_eq!(cfg.vcpu_count, 2);
        assert_eq!(cfg.guest_memory_mb, 512);
    }

    #[test]
    fn test_vm_config_nested_virt() {
        let mut cfg = VmConfig::new(2, 512);
        assert!(!cfg.nested_virt);
        cfg.enable_nested();
        assert!(cfg.nested_virt);
    }

    #[test]
    fn test_vm_creation() {
        let cfg = VmConfig::new(2, 512);
        let vm = VirtualMachine::new(1, cfg);
        assert_eq!(vm.get_id(), 1);
        assert_eq!(vm.get_state(), VmState::Created);
    }

    #[test]
    fn test_vm_vcpu_initialization() {
        let cfg = VmConfig::new(4, 512);
        let vm = VirtualMachine::new(1, cfg);
        assert_eq!(vm.get_vcpu_count(), 4);
    }

    #[test]
    fn test_vm_get_vcpu() {
        let cfg = VmConfig::new(2, 512);
        let vm = VirtualMachine::new(1, cfg);
        assert!(vm.get_vcpu(0).is_some());
        assert!(vm.get_vcpu(1).is_some());
        assert!(vm.get_vcpu(2).is_none());
    }

    #[test]
    fn test_vm_add_memory() {
        let cfg = VmConfig::new(1, 512);
        let mut vm = VirtualMachine::new(1, cfg);
        assert!(vm.add_memory(0x1000, 0x2000, 0x1000));
        assert_eq!(vm.get_memory_regions(), 1);
    }

    #[test]
    fn test_vm_total_memory() {
        let cfg = VmConfig::new(1, 512);
        let mut vm = VirtualMachine::new(1, cfg);
        vm.add_memory(0x1000, 0x2000, 0x1000);
        vm.add_memory(0x3000, 0x4000, 0x2000);
        assert_eq!(vm.get_total_memory(), 0x3000);
    }

    #[test]
    fn test_vm_start() {
        let cfg = VmConfig::new(1, 512);
        let mut vm = VirtualMachine::new(1, cfg);
        assert!(vm.start());
        assert_eq!(vm.get_state(), VmState::Running);
    }

    #[test]
    fn test_vm_pause() {
        let cfg = VmConfig::new(1, 512);
        let mut vm = VirtualMachine::new(1, cfg);
        vm.start();
        assert!(vm.pause());
        assert_eq!(vm.get_state(), VmState::Stopped);
    }

    #[test]
    fn test_vm_stop() {
        let cfg = VmConfig::new(1, 512);
        let mut vm = VirtualMachine::new(1, cfg);
        vm.start();
        assert!(vm.stop());
        assert_eq!(vm.get_state(), VmState::Stopped);
    }

    #[test]
    fn test_vm_run_quantum() {
        let cfg = VmConfig::new(1, 512);
        let mut vm = VirtualMachine::new(1, cfg);
        vm.start();
        assert!(vm.run_quantum(1000));
        assert_eq!(vm.get_total_cycles(), 1000);
    }

    #[test]
    fn test_vm_execution_count() {
        let cfg = VmConfig::new(1, 512);
        let mut vm = VirtualMachine::new(1, cfg);
        vm.start();
        assert_eq!(vm.get_execution_count(), 0);
        vm.run_quantum(1000);
        assert_eq!(vm.get_execution_count(), 1);
    }

    #[test]
    fn test_vm_is_running() {
        let cfg = VmConfig::new(1, 512);
        let mut vm = VirtualMachine::new(1, cfg);
        assert!(!vm.is_running());
        vm.start();
        assert!(vm.is_running());
    }

    #[test]
    fn test_vm_multiple_quanta() {
        let cfg = VmConfig::new(1, 512);
        let mut vm = VirtualMachine::new(1, cfg);
        vm.start();
        vm.run_quantum(1000);
        vm.run_quantum(1000);
        vm.run_quantum(1000);
        assert_eq!(vm.get_execution_count(), 3);
        assert_eq!(vm.get_total_cycles(), 3000);
    }

    #[test]
    fn test_vm_vcpu_execution() {
        let cfg = VmConfig::new(1, 512);
        let mut vm = VirtualMachine::new(1, cfg);
        vm.start();
        vm.run_quantum(100);
        
        let vcpu = vm.get_vcpu(0).unwrap();
        assert!(vcpu.get_execution_cycles() > 0);
    }

    #[test]
    fn test_vm_get_config() {
        let cfg = VmConfig::new(4, 1024);
        let vm = VirtualMachine::new(1, cfg);
        assert_eq!(vm.get_config().vcpu_count, 4);
        assert_eq!(vm.get_config().guest_memory_mb, 1024);
    }

    #[test]
    fn test_vcpu_multiple_states() {
        let mut vcpu = Vcpu::new(0);
        assert_eq!(vcpu.get_state(), VcpuState::Ready);
        
        vcpu.start();
        assert_eq!(vcpu.get_state(), VcpuState::Running);
        
        vcpu.pause();
        assert_eq!(vcpu.get_state(), VcpuState::Ready);
        
        vcpu.start();
        assert_eq!(vcpu.get_state(), VcpuState::Running);
    }
}
