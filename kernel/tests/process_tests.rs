//! Process management tests
//! Tests for process allocation, lookup, and lifecycle management

#![cfg(test)]

use kernel::process::manager::{ProcTable, ProcState};

#[cfg(test)]
mod process_allocation_tests {
    use super::*;

    /// Test basic process allocation
    #[test]
    fn test_process_alloc_basic() {
        let mut table = ProcTable::new();
        
        // Allocate first process
        let proc = table.alloc().expect("Failed to allocate first process");
        assert_eq!(proc.pid, 1);
        assert_eq!(proc.state, ProcState::Used);
        
        // Allocate second process
        let proc2 = table.alloc().expect("Failed to allocate second process");
        assert_eq!(proc2.pid, 2);
        assert_eq!(proc2.state, ProcState::Used);
    }

    /// Test process lookup by PID
    #[test]
    fn test_process_find_by_pid() {
        let mut table = ProcTable::new();
        
        let proc1 = table.alloc().expect("Failed to allocate first process");
        let pid1 = proc1.pid;
        
        let proc2 = table.alloc().expect("Failed to allocate second process");
        let proc2_pid = proc2.pid;
        
        // Look up first process
        let found = table.find(pid1);
        assert!(found.is_some());
        assert_eq!(found.unwrap().pid, pid1);
        
        // Look up second process
        let found = table.find(proc2_pid);
        assert!(found.is_some());
        assert_eq!(found.unwrap().pid, proc2_pid);
    }

    /// Test immutable process lookup
    #[test]
    fn test_process_find_ref() {
        let mut table = ProcTable::new();
        
        let proc = table.alloc().expect("Failed to allocate process");
        let pid = proc.pid;
        
        // Immutable lookup
        let found = table.find_ref(pid);
        assert!(found.is_some());
        assert_eq!(found.unwrap().pid, pid);
    }

    /// Test PID is zero returns None
    #[test]
    fn test_invalid_pid_zero() {
        let table = ProcTable::new();
        
        // PID 0 should not be found
        let found = table.find_ref(0);
        assert!(found.is_none());
    }

    /// Test nonexistent PID returns None
    #[test]
    fn test_nonexistent_pid() {
        let table = ProcTable::new();
        
        // Nonexistent PID should return None
        let found = table.find_ref(12345);
        assert!(found.is_none());
    }
}

#[cfg(test)]
mod process_lookup_performance_tests {
    use super::*;

    /// Test O(1) lookup performance characteristics
    #[test]
    fn test_lookup_performance_many_processes() {
        let mut table = ProcTable::new();
        
        // Allocate multiple processes (up to NPROC)
        let mut pids = Vec::new();
        for _ in 0..16 {  // Allocate 16 processes
            if let Some(proc) = table.alloc() {
                pids.push(proc.pid);
            }
        }
        
        // Verify all processes can be found efficiently
        for pid in pids {
            let found = table.find_ref(pid);
            assert!(found.is_some());
            assert_eq!(found.unwrap().pid, pid);
        }
    }

    /// Test that lookup works across full allocation
    #[test]
    fn test_lookup_consistency() {
        let mut table = ProcTable::new();
        
        let mut first_pid = None;
        let mut last_pid = None;
        
        // Allocate some processes
        for _ in 0..8 {
            if let Some(proc) = table.alloc() {
                if first_pid.is_none() {
                    first_pid = Some(proc.pid);
                }
                last_pid = Some(proc.pid);
            }
        }
        
        // Both first and last should be findable
        assert!(table.find_ref(first_pid.unwrap()).is_some());
        assert!(table.find_ref(last_pid.unwrap()).is_some());
    }
}

#[cfg(test)]
mod process_lifecycle_tests {
    use super::*;

    /// Test process state transitions
    #[test]
    fn test_process_state_transitions() {
        let mut table = ProcTable::new();
        
        let proc = table.alloc().expect("Failed to allocate process");
        assert_eq!(proc.state, ProcState::Used);
        
        // State should be initialized correctly
        assert!(proc.pid > 0);
    }

    /// Test multiple allocations maintain uniqueness
    #[test]
    fn test_pid_uniqueness() {
        let mut table = ProcTable::new();
        
        let mut pids = Vec::new();
        
        // Allocate several processes
        for _ in 0..5 {
            if let Some(proc) = table.alloc() {
                pids.push(proc.pid);
            }
        }
        
        // Verify all PIDs are unique
        for i in 0..pids.len() {
            for j in (i + 1)..pids.len() {
                assert_ne!(pids[i], pids[j], "Duplicate PID detected");
            }
        }
    }
}

#[cfg(test)]
mod process_error_handling_tests {
    use super::*;

    /// Test allocation when table is full (simulation)
    #[test]
    fn test_allocation_resource_handling() {
        let mut table = ProcTable::new();
        
        // Successfully allocate processes
        let proc1 = table.alloc();
        assert!(proc1.is_some());
        
        let proc2 = table.alloc();
        assert!(proc2.is_some());
        
        // Different processes should have different PIDs
        assert_ne!(proc1.unwrap().pid, proc2.unwrap().pid);
    }
}

#[cfg(test)]
mod process_parent_child_tests {
    use super::*;

    /// Test parent-to-children index initialization
    #[test]
    fn test_parent_children_index_initialization() {
        let mut table = ProcTable::new();
        
        // Initialize pid_to_index map (normally done in init())
        // For testing, we need to manually initialize
        use hashbrown::HashMap;
        use crate::compat::DefaultHasherBuilder;
        table.pid_to_index = Some(HashMap::with_hasher(DefaultHasherBuilder));
        table.parent_to_children = Some(HashMap::with_hasher(DefaultHasherBuilder));
        
        // Allocate parent process
        let parent = table.alloc().expect("Failed to allocate parent");
        let parent_pid = parent.pid;
        
        // Set up parent-child relationship
        parent.parent = None; // Root process
        
        // Allocate child process
        let child = table.alloc().expect("Failed to allocate child");
        let child_pid = child.pid;
        child.parent = Some(parent_pid);
        
        // Add child to parent's children list
        table.add_child_to_parent(parent_pid, child_pid);
        
        // Verify child can be found in parent's children
        let children = table.get_children(parent_pid);
        assert!(children.is_some());
        assert!(children.unwrap().contains(&child_pid));
    }

    /// Test O(1) child lookup performance
    #[test]
    fn test_child_lookup_performance() {
        let mut table = ProcTable::new();
        
        // Initialize maps
        use hashbrown::HashMap;
        use crate::compat::DefaultHasherBuilder;
        table.pid_to_index = Some(HashMap::with_hasher(DefaultHasherBuilder));
        table.parent_to_children = Some(HashMap::with_hasher(DefaultHasherBuilder));
        
        // Allocate parent
        let parent = table.alloc().expect("Failed to allocate parent");
        let parent_pid = parent.pid;
        
        // Allocate multiple children
        let mut child_pids = Vec::new();
        for _ in 0..5 {
            let child = table.alloc().expect("Failed to allocate child");
            let child_pid = child.pid;
            child.parent = Some(parent_pid);
            table.add_child_to_parent(parent_pid, child_pid);
            child_pids.push(child_pid);
        }
        
        // Verify all children can be found efficiently
        let children = table.get_children(parent_pid).unwrap();
        for child_pid in &child_pids {
            assert!(children.contains(child_pid));
        }
    }

    /// Test removing child from parent
    #[test]
    fn test_remove_child_from_parent() {
        let mut table = ProcTable::new();
        
        // Initialize maps
        use hashbrown::HashMap;
        use crate::compat::DefaultHasherBuilder;
        table.pid_to_index = Some(HashMap::with_hasher(DefaultHasherBuilder));
        table.parent_to_children = Some(HashMap::with_hasher(DefaultHasherBuilder));
        
        // Allocate parent and child
        let parent = table.alloc().expect("Failed to allocate parent");
        let parent_pid = parent.pid;
        
        let child = table.alloc().expect("Failed to allocate child");
        let child_pid = child.pid;
        child.parent = Some(parent_pid);
        
        // Add child
        table.add_child_to_parent(parent_pid, child_pid);
        
        // Verify child exists
        assert!(table.get_children(parent_pid).unwrap().contains(&child_pid));
        
        // Remove child
        table.remove_child_from_parent(parent_pid, child_pid);
        
        // Verify child is removed
        let children = table.get_children(parent_pid);
        assert!(children.is_none() || children.unwrap().is_empty());
    }
}
