//! # Virtual CPU Scheduler
//!
//! This module implements a scheduler for virtual CPUs (vCPUs) that manages
//! their execution, priorities, time slices, and load balancing. The vCPU
//! scheduler ensures fair distribution of host CPU resources among VMs.
//!
//! ## Architecture
//!
//! The vCPU scheduler provides:
//! - **Priority Management**: Different priority levels for vCPUs
//! - **Time Slice Allocation**: Fair CPU time distribution
//! - **Load Balancing**: Distribute vCPUs across physical CPUs
//! - **State Management**: Track vCPU running/blocked states
//! - **Preemption**: Allow higher-priority vCPUs to preempt lower ones
//!
//! ## Scheduling Algorithm
//!
//! The scheduler uses a priority-based round-robin algorithm with:
//! - Multiple priority queues
//! - Time slice allocation based on priority
//! - Dynamic priority adjustment
//! - CPU affinity support
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::vmm::vcpu_sched::{VcpuScheduler, VcpuState, VcpuPriority};
//!
//! let mut scheduler = VcpuScheduler::new(4); // 4 physical CPUs
//! scheduler.add_vcpu(0, VcpuPriority::Normal);
//! scheduler.schedule();
//! ```

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic {AtomicU32, Ordering, Ordering};

/// vCPU state for scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcpuState {
    /// vCPU is not running
    Stopped,
    /// vCPU is ready to run
    Ready,
    /// vCPU is currently running
    Running,
    /// vCPU is blocked (waiting for I/O, etc.)
    Blocked,
    /// vCPU is being destroyed
    Destroying,
}

/// vCPU priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VcpuPriority {
    /// Lowest priority (batch workloads)
    Low = 0,
    /// Normal priority (default)
    Normal = 1,
    /// High priority (interactive workloads)
    High = 2,
    /// Real-time priority (critical workloads)
    Realtime = 3,
}

impl Default for VcpuPriority {
    fn default() -> Self {
        VcpuPriority::Normal
    }
}

/// vCPU descriptor for scheduling
#[derive(Debug)]
pub struct VcpuDescriptor {
    /// vCPU ID
    pub id: u32,
    /// VM ID
    pub vm_id: u32,
    /// Current state
    pub state: VcpuState,
    /// Priority
    pub priority: VcpuPriority,
    /// Time slice remaining (in ticks)
    pub time_slice: u32,
    /// Physical CPU affinity (None = any CPU)
    pub cpu_affinity: Option<u32>,
    /// Total execution time (in ticks)
    pub total_ticks: u64,
    /// Number of times scheduled
    pub schedule_count: u64,
    /// Last time this vCPU was scheduled
    pub last_schedule_time: u64,
}

impl VcpuDescriptor {
    /// Create a new vCPU descriptor
    ///
    /// # Arguments
    ///
    /// * `id` - vCPU ID
    /// * `vm_id` - VM ID
    /// * `priority` - Initial priority
    pub fn new(id: u32, vm_id: u32, priority: VcpuPriority) -> Self {
        VcpuDescriptor {
            id,
            vm_id,
            state: VcpuState::Ready,
            priority,
            time_slice: Self::default_time_slice(priority),
            cpu_affinity: None,
            total_ticks: 0,
            schedule_count: 0,
            last_schedule_time: 0,
        }
    }

    /// Get default time slice for priority
    fn default_time_slice(priority: VcpuPriority) -> u32 {
        match priority {
            VcpuPriority::Low => 10,
            VcpuPriority::Normal => 20,
            VcpuPriority::High => 30,
            VcpuPriority::Realtime => 50,
        }
    }

    /// Reset time slice
    pub fn reset_time_slice(&mut self) {
        self.time_slice = Self::default_time_slice(self.priority);
    }

    /// Consume one tick of time slice
    pub fn consume_tick(&mut self) {
        if self.time_slice > 0 {
            self.time_slice -= 1;
        }
        self.total_ticks += 1;
    }

    /// Check if time slice is exhausted
    pub fn is_time_slice_exhausted(&self) -> bool {
        self.time_slice == 0
    }

    /// Update priority
    pub fn set_priority(&mut self, priority: VcpuPriority) {
        self.priority = priority;
        self.time_slice = Self::default_time_slice(priority);
    }

    /// Set CPU affinity
    pub fn set_cpu_affinity(&mut self, cpu_id: Option<u32>) {
        self.cpu_affinity = cpu_id;
    }

    /// Mark as ready to run
    pub fn mark_ready(&mut self) {
        self.state = VcpuState::Ready;
    }

    /// Mark as running
    pub fn mark_running(&mut self) {
        self.state = VcpuState::Running;
        self.schedule_count += 1;
    }

    /// Mark as blocked
    pub fn mark_blocked(&mut self) {
        self.state = VcpuState::Blocked;
    }

    /// Mark as stopped
    pub fn mark_stopped(&mut self) {
        self.state = VcpuState::Stopped;
    }
}

/// Physical CPU state
#[derive(Debug)]
pub struct PhysicalCpu {
    /// CPU ID
    pub id: u32,
    /// vCPU currently running on this CPU (if any)
    current_vcpu: Option<u32>,
    /// Total idle time (in ticks)
    idle_ticks: u64,
    /// Total busy time (in ticks)
    busy_ticks: u64,
}

impl PhysicalCpu {
    /// Create a new physical CPU descriptor
    pub fn new(id: u32) -> Self {
        PhysicalCpu {
            id,
            current_vcpu: None,
            idle_ticks: 0,
            busy_ticks: 0,
        }
    }

    /// Get current vCPU
    pub fn current_vcpu(&self) -> Option<u32> {
        self.current_vcpu
    }

    /// Set current vCPU
    pub fn set_vcpu(&mut self, vcpu_id: Option<u32>) {
        self.current_vcpu = vcpu_id;
    }

    /// Record idle tick
    pub fn record_idle(&mut self) {
        self.idle_ticks += 1;
    }

    /// Record busy tick
    pub fn record_busy(&mut self) {
        self.busy_ticks += 1;
    }

    /// Get utilization ratio (0.0 to 1.0)
    pub fn utilization(&self) -> f64 {
        let total = self.idle_ticks + self.busy_ticks;
        if total == 0 {
            0.0
        } else {
            self.busy_ticks as f64 / total as f64
        }
    }
}

/// vCPU scheduler error types
#[derive(Debug, Clone, Copy)]
pub enum SchedulerError {
    /// Invalid vCPU ID
    InvalidVcpuId,
    /// Invalid CPU ID
    InvalidCpuId,
    /// No free CPU available
    NoFreeCpu,
    /// Scheduler not initialized
    NotInitialized,
    /// vCPU not found
    VcpuNotFound,
}

/// vCPU scheduler
pub struct VcpuScheduler {
    /// Number of physical CPUs
    num_cpus: u32,
    /// Physical CPUs
    cpus: Vec<PhysicalCpu>,
    /// vCPU descriptors indexed by vCPU ID
    vcpus: Vec<Option<VcpuDescriptor>>,
    /// Ready queues for each priority level
    ready_queues: [VecDeque<u32>; 4],
    /// Current tick counter
    current_tick: AtomicU32,
    /// Next vCPU ID
    next_vcpu_id: AtomicU32,
    /// Load balancing enabled
    load_balancing_enabled: bool,
}

impl VcpuScheduler {
    /// Create a new vCPU scheduler
    ///
    /// # Arguments
    ///
    /// * `num_cpus` - Number of physical CPUs available
    pub fn new(num_cpus: u32) -> Self {
        let mut cpus = Vec::new();
        for i in 0..num_cpus {
            cpus.push(PhysicalCpu::new(i));
        }

        VcpuScheduler {
            num_cpus,
            cpus,
            vcpus: Vec::new(),
            ready_queues: [
                VecDeque::new(), // Low
                VecDeque::new(), // Normal
                VecDeque::new(), // High
                VecDeque::new(), // Realtime
            ],
            current_tick: AtomicU32::new(0),
            next_vcpu_id: AtomicU32::new(0),
            load_balancing_enabled: true,
        }
    }

    /// Add a vCPU to the scheduler
    ///
    /// # Arguments
    ///
    /// * `vm_id` - VM ID that owns this vCPU
    /// * `priority` - Initial priority
    ///
    /// # Returns
    ///
    /// vCPU ID
    pub fn add_vcpu(&mut self, vm_id: u32, priority: VcpuPriority) -> u32 {
        let vcpu_id = self.next_vcpu_id.fetch_add(1, Ordering::SeqCst);
        let descriptor = VcpuDescriptor::new(vcpu_id, vm_id, priority);

        // Ensure vector has enough space
        if vcpu_id as usize >= self.vcpus.len() {
            self.vcpus.push(Some(descriptor));
        } else {
            self.vcpus[vcpu_id as usize] = Some(descriptor);
        }

        // Add to ready queue
        self.ready_queues[priority as usize].push_back(vcpu_id);

        vcpu_id
    }

    /// Remove a vCPU from the scheduler
    ///
    /// # Arguments
    ///
    /// * `vcpu_id` - vCPU to remove
    pub fn remove_vcpu(&mut self, vcpu_id: u32) -> Result<(), SchedulerError> {
        let vcpu = self
            .vcpus
            .get_mut(vcpu_id as usize)
            .and_then(|v| v.as_mut())
            .ok_or(SchedulerError::VcpuNotFound)?;

        vcpu.state = VcpuState::Destroying;

        // Remove from ready queues
        for queue in &mut self.ready_queues {
            queue.retain(|&id| id != vcpu_id);
        }

        // Remove from any running CPU
        for cpu in &mut self.cpus {
            if cpu.current_vcpu() == Some(vcpu_id) {
                cpu.set_vcpu(None);
            }
        }

        self.vcpus[vcpu_id as usize] = None;
        Ok(())
    }

    /// Schedule vCPUs onto physical CPUs
    ///
    /// This is the main scheduling function that decides which vCPUs
    /// should run on which physical CPUs.
    pub fn schedule(&mut self) {
        self.current_tick.fetch_add(1, Ordering::SeqCst);

        // Check all CPUs and reschedule if needed
        for cpu in &mut self.cpus {
            if let Some(vcpu_id) = cpu.current_vcpu() {
                if let Some(vcpu) = self.vcpus.get(vcpu_id as usize).and_then(|v| v.as_ref()) {
                    // Check if vCPU should be preempted
                    if vcpu.is_time_slice_exhausted() || vcpu.state != VcpuState::Running {
                        // Preempt this vCPU
                        cpu.set_vcpu(None);
                        if let Some(v) = self.vcpus.get_mut(vcpu_id as usize).and_then(|v| v.as_mut()) {
                            if v.state == VcpuState::Running {
                                v.state = VcpuState::Ready;
                                v.reset_time_slice();
                                self.ready_queues[v.priority as usize].push_back(vcpu_id);
                            }
                        }
                    }
                }
            }

            // Try to schedule a new vCPU on this CPU
            if cpu.current_vcpu().is_none() {
                if let Some(vcpu_id) = self.pick_next_vcpu(cpu.id) {
                    if let Some(vcpu) = self.vcpus.get_mut(vcpu_id as usize).and_then(|v| v.as_mut()) {
                        vcpu.mark_running();
                        vcpu.last_schedule_time = self.current_tick.load(Ordering::SeqCst) as u64;
                        cpu.set_vcpu(Some(vcpu_id));
                    }
                }
            }

            // Record CPU state
            if cpu.current_vcpu().is_some() {
                cpu.record_busy();
            } else {
                cpu.record_idle();
            }
        }

        // Perform load balancing if enabled
        if self.load_balancing_enabled {
            self.balance_load();
        }
    }

    /// Pick the next vCPU to run on a specific CPU
    ///
    /// # Arguments
    ///
    /// * `cpu_id` - Physical CPU ID
    ///
    /// # Returns
    ///
    /// vCPU ID or None if no vCPU is ready
    fn pick_next_vcpu(&mut self, cpu_id: u32) -> Option<u32> {
        // Check queues from highest to lowest priority
        for priority in (0..4).rev() {
            let queue = &mut self.ready_queues[priority];

            // Find a vCPU with matching affinity or no affinity
            let pos = queue.iter().position(|&vcpu_id| {
                if let Some(vcpu) = self.vcpus.get(vcpu_id as usize).and_then(|v| v.as_ref()) {
                    if let Some(affinity) = vcpu.cpu_affinity {
                        affinity == cpu_id
                    } else {
                        true
                    }
                } else {
                    false
                }
            });

            if let Some(pos) = pos {
                let vcpu_id = queue.remove(pos).unwrap();
                return Some(vcpu_id);
            }
        }

        None
    }

    /// Balance load across physical CPUs
    fn balance_load(&mut self) {
        // Calculate average utilization
        let total_utilization: f64 = self.cpus.iter().map(|cpu| cpu.utilization()).sum();
        let avg_utilization = total_utilization / self.cpus.len() as f64;

        // Find overloaded and underloaded CPUs
        let (overloaded, underloaded): (Vec<_>, Vec<_>) = self
            .cpus
            .iter()
            .enumerate()
            .partition(|(_, cpu)| cpu.utilization() > avg_utilization * 1.2);

        // Try to migrate vCPUs from overloaded to underloaded CPUs
        for (cpu_idx, _) in overloaded {
            if let Some(vcpu_id) = self.cpus[cpu_idx].current_vcpu() {
                // Check if this vCPU can be migrated
                if let Some(vcpu) = self.vcpus.get(vcpu_id as usize).and_then(|v| v.as_ref()) {
                    if vcpu.cpu_affinity.is_none() {
                        // Find an underloaded CPU
                        for (target_cpu_idx, _) in &underloaded {
                            if let Some(target_vcpu_id) = self.pick_next_vcpu(*target_cpu_idx as u32) {
                                if target_vcpu_id == vcpu_id {
                                    // Migrate the vCPU
                                    self.cpus[cpu_idx].set_vcpu(None);
                                    self.cpus[*target_cpu_idx].set_vcpu(Some(vcpu_id));
                                    log::debug!(
                                        "Migrated vCPU {} from CPU {} to CPU {}",
                                        vcpu_id,
                                        cpu_idx,
                                        target_cpu_idx
                                    );
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Block a vCPU (e.g., waiting for I/O)
    ///
    /// # Arguments
    ///
    /// * `vcpu_id` - vCPU to block
    pub fn block_vcpu(&mut self, vcpu_id: u32) -> Result<(), SchedulerError> {
        let vcpu = self
            .vcpus
            .get_mut(vcpu_id as usize)
            .and_then(|v| v.as_mut())
            .ok_or(SchedulerError::VcpuNotFound)?;

        vcpu.mark_blocked();

        // Remove from CPU if running
        for cpu in &mut self.cpus {
            if cpu.current_vcpu() == Some(vcpu_id) {
                cpu.set_vcpu(None);
            }
        }

        // Remove from ready queues
        for queue in &mut self.ready_queues {
            queue.retain(|&id| id != vcpu_id);
        }

        Ok(())
    }

    /// Unblock a vCPU
    ///
    /// # Arguments
    ///
    /// * `vcpu_id` - vCPU to unblock
    pub fn unblock_vcpu(&mut self, vcpu_id: u32) -> Result<(), SchedulerError> {
        let vcpu = self
            .vcpus
            .get_mut(vcpu_id as usize)
            .and_then(|v| v.as_mut())
            .ok_or(SchedulerError::VcpuNotFound)?;

        vcpu.mark_ready();
        vcpu.reset_time_slice();
        self.ready_queues[vcpu.priority as usize].push_back(vcpu_id);

        Ok(())
    }

    /// Change vCPU priority
    ///
    /// # Arguments
    ///
    /// * `vcpu_id` - vCPU to modify
    /// * `new_priority` - New priority level
    pub fn set_priority(
        &mut self,
        vcpu_id: u32,
        new_priority: VcpuPriority,
    ) -> Result<(), SchedulerError> {
        let vcpu = self
            .vcpus
            .get_mut(vcpu_id as usize)
            .and_then(|v| v.as_mut())
            .ok_or(SchedulerError::VcpuNotFound)?;

        let old_priority = vcpu.priority;
        vcpu.set_priority(new_priority);

        // Move to new priority queue if ready
        if vcpu.state == VcpuState::Ready {
            // Remove from old queue
            self.ready_queues[old_priority as usize].retain(|&id| id != vcpu_id);
            // Add to new queue
            self.ready_queues[new_priority as usize].push_back(vcpu_id);
        }

        Ok(())
    }

    /// Set CPU affinity for a vCPU
    ///
    /// # Arguments
    ///
    /// * `vcpu_id` - vCPU to modify
    /// * `cpu_id` - Physical CPU to pin to (None for no affinity)
    pub fn set_cpu_affinity(
        &mut self,
        vcpu_id: u32,
        cpu_id: Option<u32>,
    ) -> Result<(), SchedulerError> {
        if let Some(cpu) = cpu_id {
            if cpu >= self.num_cpus {
                return Err(SchedulerError::InvalidCpuId);
            }
        }

        let vcpu = self
            .vcpus
            .get_mut(vcpu_id as usize)
            .and_then(|v| v.as_mut())
            .ok_or(SchedulerError::VcpuNotFound)?;

        vcpu.set_cpu_affinity(cpu_id);

        Ok(())
    }

    /// Get vCPU descriptor
    ///
    /// # Arguments
    ///
    /// * `vcpu_id` - vCPU to query
    pub fn get_vcpu(&self, vcpu_id: u32) -> Option<&VcpuDescriptor> {
        self.vcpus.get(vcpu_id as usize).and_then(|v| v.as_ref())
    }

    /// Get physical CPU descriptor
    ///
    /// # Arguments
    ///
    /// * `cpu_id` - Physical CPU to query
    pub fn get_cpu(&self, cpu_id: u32) -> Option<&PhysicalCpu> {
        self.cpus.get(cpu_id as usize)
    }

    /// Get current tick
    pub fn current_tick(&self) -> u32 {
        self.current_tick.load(Ordering::SeqCst)
    }

    /// Enable/disable load balancing
    pub fn set_load_balancing(&mut self, enabled: bool) {
        self.load_balancing_enabled = enabled;
    }

    /// Get scheduler statistics
    pub fn get_stats(&self) -> SchedulerStats {
        let total_vcpus = self.vcpus.iter().filter(|v| v.is_some()).count();
        let running_vcpus = self.cpus.iter().filter(|cpu| cpu.current_vcpu().is_some()).count();

        let avg_utilization: f64 = self.cpus.iter().map(|cpu| cpu.utilization()).sum::<f64>()
            / self.cpus.len() as f64;

        SchedulerStats {
            total_vcpus,
            running_vcpus,
            ready_vcpus: self.ready_queues.iter().map(|q| q.len()).sum(),
            blocked_vcpus: self
                .vcpus
                .iter()
                .filter_map(|v| v.as_ref())
                .filter(|v| v.state == VcpuState::Blocked)
                .count(),
            avg_utilization,
            current_tick: self.current_tick.load(Ordering::SeqCst),
        }
    }
}

/// Scheduler statistics
#[derive(Debug, Clone)]
pub struct SchedulerStats {
    /// Total number of vCPUs
    pub total_vcpus: usize,
    /// Number of currently running vCPUs
    pub running_vcpus: usize,
    /// Number of ready vCPUs
    pub ready_vcpus: usize,
    /// Number of blocked vCPUs
    pub blocked_vcpus: usize,
    /// Average CPU utilization (0.0 to 1.0)
    pub avg_utilization: f64,
    /// Current scheduler tick
    pub current_tick: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vcpu_descriptor_creation() {
        let desc = VcpuDescriptor::new(0, 1, VcpuPriority::High);
        assert_eq!(desc.id, 0);
        assert_eq!(desc.vm_id, 1);
        assert_eq!(desc.priority, VcpuPriority::High);
        assert_eq!(desc.state, VcpuState::Ready);
    }

    #[test]
    fn test_time_slice_consumption() {
        let mut desc = VcpuDescriptor::new(0, 1, VcpuPriority::Normal);
        let initial_slice = desc.time_slice;

        desc.consume_tick();
        assert_eq!(desc.time_slice, initial_slice - 1);
        assert_eq!(desc.total_ticks, 1);

        assert!(!desc.is_time_slice_exhausted());
    }

    #[test]
    fn test_scheduler_creation() {
        let scheduler = VcpuScheduler::new(4);
        assert_eq!(scheduler.num_cpus, 4);
        assert_eq!(scheduler.cpus.len(), 4);
    }

    #[test]
    fn test_add_vcpu() {
        let mut scheduler = VcpuScheduler::new(2);
        let vcpu_id = scheduler.add_vcpu(0, VcpuPriority::Normal);

        assert_eq!(vcpu_id, 0);
        assert!(scheduler.get_vcpu(vcpu_id).is_some());
    }

    #[test]
    fn test_remove_vcpu() {
        let mut scheduler = VcpuScheduler::new(2);
        let vcpu_id = scheduler.add_vcpu(0, VcpuPriority::Normal);

        let result = scheduler.remove_vcpu(vcpu_id);
        assert!(result.is_ok());
        assert!(scheduler.get_vcpu(vcpu_id).is_none());
    }

    #[test]
    fn test_priority_ordering() {
        assert!(VcpuPriority::Realtime > VcpuPriority::High);
        assert!(VcpuPriority::High > VcpuPriority::Normal);
        assert!(VcpuPriority::Normal > VcpuPriority::Low);
    }
}
