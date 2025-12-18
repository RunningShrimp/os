//! Boot Parallelization - Parallel Boot Initialization
//!
//! Enables parallel execution of independent boot tasks:
//! - Task scheduling and execution
//! - Dependency management
//! - Work queue management
//! - Synchronization primitives

use core::fmt;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "Pending"),
            TaskStatus::Running => write!(f, "Running"),
            TaskStatus::Completed => write!(f, "Completed"),
            TaskStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// Task priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskPriority::Low => write!(f, "Low"),
            TaskPriority::Normal => write!(f, "Normal"),
            TaskPriority::High => write!(f, "High"),
            TaskPriority::Critical => write!(f, "Critical"),
        }
    }
}

/// Boot task
#[derive(Debug, Clone)]
pub struct BootTask {
    pub task_id: u32,
    pub name: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub dependencies: Vec<u32>,
    pub execution_time: u64,
    pub start_time: u64,
    pub end_time: u64,
}

impl BootTask {
    /// Create new task
    pub fn new(id: u32, name: &str) -> Self {
        BootTask {
            task_id: id,
            name: String::from(name),
            status: TaskStatus::Pending,
            priority: TaskPriority::Normal,
            dependencies: Vec::new(),
            execution_time: 0,
            start_time: 0,
            end_time: 0,
        }
    }

    /// Set priority
    pub fn set_priority(&mut self, priority: TaskPriority) {
        self.priority = priority;
    }

    /// Add dependency
    pub fn add_dependency(&mut self, dep_id: u32) {
        if !self.dependencies.contains(&dep_id) {
            self.dependencies.push(dep_id);
        }
    }

    /// Set execution time
    pub fn set_timing(&mut self, start: u64, end: u64) {
        self.start_time = start;
        self.end_time = end;
        if end > start {
            self.execution_time = end - start;
        }
    }

    /// Check dependencies met
    pub fn dependencies_met(&self, completed: &[u32]) -> bool {
        self.dependencies.iter().all(|d| completed.contains(d))
    }
}

impl fmt::Display for BootTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Task{}: {} [{}] ({}ms)",
            self.task_id, self.name, self.status, self.execution_time
        )
    }
}

/// Task queue
#[derive(Debug, Clone)]
pub struct TaskQueue {
    pub tasks: Vec<BootTask>,
    pub total_queued: u32,
    pub total_completed: u32,
    pub total_failed: u32,
}

impl TaskQueue {
    /// Create new queue
    pub fn new() -> Self {
        TaskQueue {
            tasks: Vec::new(),
            total_queued: 0,
            total_completed: 0,
            total_failed: 0,
        }
    }

    /// Enqueue task
    pub fn enqueue(&mut self, task: BootTask) -> bool {
        self.tasks.push(task);
        self.total_queued += 1;
        true
    }

    /// Get next runnable task
    pub fn get_next_runnable(&self) -> Option<&BootTask> {
        let completed: Vec<u32> = self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .map(|t| t.task_id)
            .collect();

        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Pending && t.dependencies_met(&completed))
            .max_by_key(|t| t.priority)
    }

    /// Mark task completed
    pub fn mark_completed(&mut self, task_id: u32) -> bool {
        for task in &mut self.tasks {
            if task.task_id == task_id {
                task.status = TaskStatus::Completed;
                self.total_completed += 1;
                return true;
            }
        }
        false
    }

    /// Mark task failed
    pub fn mark_failed(&mut self, task_id: u32) -> bool {
        for task in &mut self.tasks {
            if task.task_id == task_id {
                task.status = TaskStatus::Failed;
                self.total_failed += 1;
                return true;
            }
        }
        false
    }

    /// Get pending count
    pub fn get_pending_count(&self) -> u32 {
        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .count() as u32
    }

    /// Get running count
    pub fn get_running_count(&self) -> u32 {
        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Running)
            .count() as u32
    }

    /// All tasks completed
    pub fn all_completed(&self) -> bool {
        self.get_pending_count() == 0 && self.get_running_count() == 0
    }
}

impl fmt::Display for TaskQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Queue {{ tasks: {}, completed: {}, failed: {} }}",
            self.total_queued, self.total_completed, self.total_failed
        )
    }
}

/// Boot Parallelizer
pub struct BootParallelizer {
    task_queue: TaskQueue,
    worker_count: u32,
    total_boot_time: u64,
    is_enabled: bool,
    max_parallel_tasks: u32,
}

impl BootParallelizer {
    /// Create new parallelizer
    pub fn new(workers: u32) -> Self {
        BootParallelizer {
            task_queue: TaskQueue::new(),
            worker_count: workers,
            total_boot_time: 0,
            is_enabled: false,
            max_parallel_tasks: workers,
        }
    }

    /// Add task
    pub fn add_task(&mut self, task: BootTask) -> bool {
        self.task_queue.enqueue(task)
    }

    /// Enable parallelization
    pub fn enable_parallel(&mut self) -> bool {
        self.is_enabled = true;
        true
    }

    /// Check if enabled
    pub fn is_parallel_enabled(&self) -> bool {
        self.is_enabled && self.worker_count > 1
    }

    /// Execute next task
    pub fn execute_next(&mut self) -> Option<u32> {
        // Check if we have reached max parallel tasks limit
        if self.task_queue.get_running_count() >= self.max_parallel_tasks {
            return None;
        }
        
        if let Some(task) = self.task_queue.get_next_runnable() {
            let task_id = task.task_id;
            if let Some(t) = self.task_queue.tasks.iter_mut().find(|x| x.task_id == task_id) {
                t.status = TaskStatus::Running;
            }
            Some(task_id)
        } else {
            None
        }
    }
    
    /// Get maximum parallel tasks limit
    pub fn max_parallel_tasks(&self) -> u32 {
        self.max_parallel_tasks
    }
    
    /// Set maximum parallel tasks limit
    pub fn set_max_parallel_tasks(&mut self, limit: u32) -> bool {
        self.max_parallel_tasks = limit;
        true
    }

    /// Complete task
    pub fn complete_task(&mut self, task_id: u32) -> bool {
        self.task_queue.mark_completed(task_id)
    }

    /// Get statistics
    pub fn get_stats(&self) -> (u32, u32, u32, u32) {
        (
            self.task_queue.total_queued,
            self.task_queue.total_completed,
            self.task_queue.total_failed,
            self.task_queue.get_pending_count(),
        )
    }

    /// Get speedup estimate
    pub fn estimate_speedup(&self) -> f32 {
        let total_time: u64 = self.task_queue.tasks
            .iter()
            .map(|t| t.execution_time)
            .sum();

        if self.total_boot_time > 0 {
            total_time as f32 / self.total_boot_time as f32
        } else {
            1.0
        }
    }

    /// Get parallelization report
    pub fn parallelization_report(&self) -> String {
        let mut report = String::from("=== Parallelization Report ===\n");

        report.push_str(&format!("Workers: {}\n", self.worker_count));
        report.push_str(&format!("Max Parallel Tasks: {}\n", self.max_parallel_tasks));
        report.push_str(&format!("Parallel Enabled: {}\n", self.is_parallel_enabled()));
        report.push_str(&format!("{}\n\n", self.task_queue));

        report.push_str("--- Task List ---\n");
        for task in &self.task_queue.tasks {
            report.push_str(&format!("{}\n", task));
        }

        report.push_str(&format!("\nEstimated Speedup: {:.2}x\n", self.estimate_speedup()));
        report.push_str(&format!("Total Boot Time: {} ms\n", self.total_boot_time));

        report
    }
}

impl fmt::Display for BootParallelizer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BootParallelizer {{ workers: {}, tasks: {}, completed: {}, speedup: {:.2}x }}",
            self.worker_count,
            self.task_queue.total_queued,
            self.task_queue.total_completed,
            self.estimate_speedup()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = BootTask::new(1, "Test");
        assert_eq!(task.task_id, 1);
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[test]
    fn test_task_priority() {
        let mut task = BootTask::new(1, "Test");
        task.set_priority(TaskPriority::High);
        assert_eq!(task.priority, TaskPriority::High);
    }

    #[test]
    fn test_task_dependency() {
        let mut task = BootTask::new(1, "Test");
        task.add_dependency(0);
        assert_eq!(task.dependencies.len(), 1);
    }

    #[test]
    fn test_task_timing() {
        let mut task = BootTask::new(1, "Test");
        task.set_timing(100, 200);
        assert_eq!(task.execution_time, 100);
    }

    #[test]
    fn test_task_dependencies_met() {
        let mut task = BootTask::new(1, "Test");
        task.add_dependency(0);
        let completed = vec![0];
        assert!(task.dependencies_met(&completed));
    }

    #[test]
    fn test_queue_creation() {
        let queue = TaskQueue::new();
        assert_eq!(queue.total_queued, 0);
    }

    #[test]
    fn test_queue_enqueue() {
        let mut queue = TaskQueue::new();
        let task = BootTask::new(1, "Test");
        assert!(queue.enqueue(task));
        assert_eq!(queue.total_queued, 1);
    }

    #[test]
    fn test_queue_mark_completed() {
        let mut queue = TaskQueue::new();
        let task = BootTask::new(1, "Test");
        queue.enqueue(task);
        assert!(queue.mark_completed(1));
        assert_eq!(queue.total_completed, 1);
    }

    #[test]
    fn test_queue_pending_count() {
        let mut queue = TaskQueue::new();
        let task = BootTask::new(1, "Test");
        queue.enqueue(task);
        assert_eq!(queue.get_pending_count(), 1);
    }

    #[test]
    fn test_parallelizer_creation() {
        let par = BootParallelizer::new(4);
        assert_eq!(par.worker_count, 4);
    }

    #[test]
    fn test_parallelizer_add_task() {
        let mut par = BootParallelizer::new(4);
        let task = BootTask::new(1, "Test");
        assert!(par.add_task(task));
    }

    #[test]
    fn test_parallelizer_enable() {
        let mut par = BootParallelizer::new(4);
        assert!(par.enable_parallel());
        assert!(par.is_parallel_enabled());
    }

    #[test]
    fn test_parallelizer_execute_next() {
        let mut par = BootParallelizer::new(4);
        let task = BootTask::new(1, "Test");
        par.add_task(task);
        let next = par.execute_next();
        assert_eq!(next, Some(1));
    }

    #[test]
    fn test_parallelizer_complete_task() {
        let mut par = BootParallelizer::new(4);
        let task = BootTask::new(1, "Test");
        par.add_task(task);
        par.execute_next();
        assert!(par.complete_task(1));
    }

    #[test]
    fn test_parallelizer_report() {
        let par = BootParallelizer::new(4);
        let report = par.parallelization_report();
        assert!(report.contains("Parallelization Report"));
    }
}
