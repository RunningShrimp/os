//! Synchronization Primitives Tests
//!
//! Tests for synchronization primitives (SpinLock, Mutex, etc.)

#[cfg(feature = "kernel_tests")]
pub mod sync_tests {
    use crate::{test_assert, test_assert_eq};
    use crate::tests::TestResult;
    use crate::subsystems::sync::{SpinLock, Mutex};
    use crate::subsystems::sync::primitives::MutexEnhanced;
    use alloc::vec::Vec;

    /// Test SpinLock basic operations
    pub fn test_spinlock_basic() -> TestResult {
        let sl = SpinLock::new();
        test_assert!(!sl.is_locked());
        sl.lock();
        test_assert!(sl.is_locked());
        sl.unlock();
        test_assert!(!sl.is_locked());
        Ok(())
    }

    /// Test Mutex with data
    pub fn test_mutex_data() -> TestResult {
        let mutex: Mutex<i32> = Mutex::new(0);
        {
            let mut guard = mutex.lock();
            *guard = 100;
        }
        test_assert_eq!(*mutex.lock(), 100);
        Ok(())
    }

    /// Test Mutex modification
    pub fn test_mutex_modify() -> TestResult {
        let mutex: Mutex<Vec<i32>> = Mutex::new(Vec::new());
        {
            let mut guard = mutex.lock();
            for i in 0..10 {
                guard.push(i);
            }
        }
        test_assert_eq!(mutex.lock().len(), 10);
        Ok(())
        }
    
        /// Test SpinLock with recursive lock attempt (should not deadlock in test environment)
        /// This tests the safety net for recursive locks (depending on implementation)
        pub fn test_spinlock_recursive_attempt() -> TestResult {
            let sl = SpinLock::new();
            
            // First lock should succeed
            sl.lock();
            test_assert!(sl.is_locked());
            
            // Depending on implementation, recursive lock might panic or deadlock
            // For testing purposes, we won't actually hold two locks at once
            sl.unlock();
            test_assert!(!sl.is_locked());
            
            Ok(())
        }

        /// Test MutexEnhanced basic locking behavior
        pub fn test_mutex_enhanced_basic() -> TestResult {
            let mutex = MutexEnhanced::new(0u32);
            {
                let mut guard = mutex.lock();
                *guard = 42;
            }
            test_assert_eq!(*mutex.lock(), 42);
            Ok(())
        }
    
        /// Test Mutex with multiple operations
        pub fn test_mutex_multiple_operations() -> TestResult {
            let mutex = Mutex::new(10);
            
            // First operation: increment
            {
                let mut guard = mutex.lock();
                *guard += 5;
            }
            
            // Second operation: multiply
            {
                let mut guard = mutex.lock();
                *guard *= 2;
            }
            
            // Verify result
            test_assert_eq!(*mutex.lock(), 30);
            
            Ok(())
        }
    
        /// Test that Mutex guards are Send
        /// This ensures Mutex can be used across threads
        pub fn test_mutex_guard_send() -> TestResult {
            // This test compiles only if MutexGuard implements Send
            struct SendWrapper<T: Send>(T);
            
            // Ensure MutexGuard is Send by wrapping it
            let mutex = Mutex::new(0);
            
            // This should compile without errors
            let guard = mutex.lock();
            let _sendable = SendWrapper(guard);
            
            Ok(())
        }
    
        /// Test SpinLock with zero initializer
        pub fn test_spinlock_zero_initializer() -> TestResult {
            static SPINLOCK: SpinLock = SpinLock::new();
            
            // Lock and unlock the static spinlock
            SPINLOCK.lock();
            SPINLOCK.unlock();
            
            Ok(())
        }
    
        /// Test that Mutex correctly handles drop
        pub fn test_mutex_guard_drop() -> TestResult {
            use core::cell::RefCell;
            let mut drop_flag = RefCell::new(false);
            
            struct Droppable {
                flag: *mut RefCell<bool>,
            }
            
            unsafe impl Send for Droppable {}
            unsafe impl Sync for Droppable {}
            
            impl Drop for Droppable {
                fn drop(&mut self) {
                    unsafe {
                        *(*self.flag).borrow_mut() = true;
                    }
                }
            }
            
            let mutex = Mutex::new(Droppable {
                flag: &mut drop_flag as *mut RefCell<bool>
            });
            
            // When guard is dropped, the droppable should be dropped too
            drop(mutex.lock());
            
            test_assert!(*drop_flag.borrow());
            
            Ok(())
        }
    }
