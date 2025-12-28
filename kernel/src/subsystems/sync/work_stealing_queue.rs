//! Lock-free Work-Stealing Deque (Chase-Lev deque)
//!
//! A concurrent work-stealing deque used in work-stealing schedulers.
//! Supports:
//! - push_bottom: Owner pushes tasks (producer)
//! - pop_bottom: Owner pops tasks (consumer)
//! - steal: Others steal tasks (thieves)
//!
//! Based on: "Dynamic Circular Work-Stealing Deque" by Chase and Lev (2005)

extern crate alloc;
use alloc::vec::Vec;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicIsize, AtomicUsize, AtomicPtr, Ordering};

const DEFAULT_CAPACITY: usize = 256;

/// Lock-free work-stealing deque
pub struct WorkStealingDeque<T> {
    buffer: AtomicPtr<AtomicPtr<T>>,
    bottom: AtomicIsize,
    top: AtomicUsize,
    capacity: AtomicUsize,
}

unsafe impl<T: Send> Send for WorkStealingDeque<T> {}
unsafe impl<T: Send> Sync for WorkStealingDeque<T> {}

impl<T> WorkStealingDeque<T> {
    /// Create a new work-stealing deque with default capacity
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Create a new work-stealing deque with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let actual_capacity = capacity.next_power_of_two();

        let buffer: Vec<AtomicPtr<T>> = (0..actual_capacity)
            .map(|_| AtomicPtr::new(core::ptr::null_mut()))
            .collect();

        let buffer_ptr = Box::leak(buffer.into_boxed_slice()) as *mut AtomicPtr<T>;

        Self {
            buffer: AtomicPtr::new(buffer_ptr),
            bottom: AtomicIsize::new(0),
            top: AtomicUsize::new(0),
            capacity: AtomicUsize::new(actual_capacity),
        }
    }

    /// Get current capacity
    pub fn capacity(&self) -> usize {
        self.capacity.load(Ordering::Acquire)
    }

    /// Get approximate size
    pub fn len(&self) -> isize {
        let bottom = self.bottom.load(Ordering::Acquire);
        let top = self.top.load(Ordering::Acquire) as isize;
        bottom - top
    }

    /// Check if deque is empty
    pub fn is_empty(&self) -> bool {
        self.len() <= 0
    }

    /// Push an item to the bottom (owner only)
    pub fn push_bottom(&self, item: T) {
        let b = self.bottom.load(Ordering::Acquire);
        let cap = self.capacity();
        let buffer = self.buffer.load(Ordering::Acquire);

        unsafe {
            let buffer_slice = core::slice::from_raw_parts_mut(buffer, cap);
            let index = (b as usize) & (cap - 1);

            buffer_slice[index].store(Box::leak(Box::new(item)), Ordering::Release);
        }

        self.bottom.store(b + 1, Ordering::Release);
    }

    /// Pop an item from the bottom (owner only)
    pub fn pop_bottom(&self) -> Option<T> {
        let b = self.bottom.load(Ordering::Acquire) - 1;
        self.bottom.store(b, Ordering::Release);

        let t = self.top.load(Ordering::Acquire);
        let cap = self.capacity();
        let buffer = self.buffer.load(Ordering::Acquire);

        if t as isize <= b {
            unsafe {
                let buffer_slice = core::slice::from_raw_parts_mut(buffer, cap);
                let index = (b as usize) & (cap - 1);
                let item_ptr = buffer_slice[index].load(Ordering::Acquire);

                if t as isize < b {
                    self.bottom.store(b + 1, Ordering::Release);
                    return Some(*Box::from_raw(item_ptr));
                } else {
                    if self.top.compare_exchange_weak(
                        t,
                        t + 1,
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    ).is_ok() {
                        self.bottom.store(b + 1, Ordering::Release);
                        return Some(*Box::from_raw(item_ptr));
                    } else {
                        self.bottom.store(b + 1, Ordering::Release);
                        return None;
                    }
                }
            }
        } else {
            self.bottom.store(b + 1, Ordering::Release);
            return None;
        }
    }

    /// Steal an item from the top (thief)
    pub fn steal(&self) -> Option<T> {
        let t = self.top.load(Ordering::Acquire);
        let b = self.bottom.load(Ordering::Acquire);

        if t as isize < b {
            let cap = self.capacity();
            let buffer = self.buffer.load(Ordering::Acquire);

            unsafe {
                let buffer_slice = core::slice::from_raw_parts_mut(buffer, cap);
                let index = t & (cap - 1);
                let item_ptr = buffer_slice[index].load(Ordering::Acquire);

                if self.top.compare_exchange_weak(
                    t,
                    t + 1,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                ).is_ok() {
                    return Some(*Box::from_raw(item_ptr));
                } else {
                    return None;
                }
            }
        } else {
            return None;
        }
    }

    /// Try to steal multiple items (batch steal)
    pub fn steal_batch(&self, max_items: usize) -> Vec<T> {
        let mut result = Vec::new();

        for _ in 0..max_items {
            if let Some(item) = self.steal() {
                result.push(item);
            } else {
                break;
            }
        }

        result
    }
}

impl<T> Drop for WorkStealingDeque<T> {
    fn drop(&mut self) {
        let cap = self.capacity();
        let buffer = self.buffer.load(Ordering::Acquire);

        if !buffer.is_null() {
            unsafe {
                let buffer_slice = core::slice::from_raw_parts_mut(buffer, cap);

                for i in 0..cap {
                    let item_ptr = buffer_slice[i].load(Ordering::Acquire);
                    if !item_ptr.is_null() {
                        drop(Box::from_raw(item_ptr));
                    }
                }

                let buffer_box = Box::from_raw(core::slice::from_raw_parts_mut(buffer, cap));
                drop(buffer_box);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop() {
        let deque = WorkStealingDeque::new();
        deque.push_bottom(1);
        deque.push_bottom(2);
        deque.push_bottom(3);

        assert_eq!(deque.pop_bottom(), Some(3));
        assert_eq!(deque.pop_bottom(), Some(2));
        assert_eq!(deque.pop_bottom(), Some(1));
        assert_eq!(deque.pop_bottom(), None);
    }

    #[test]
    fn test_steal() {
        let deque = WorkStealingDeque::new();
        deque.push_bottom(1);
        deque.push_bottom(2);
        deque.push_bottom(3);

        assert_eq!(deque.steal(), Some(1));
        assert_eq!(deque.steal(), Some(2));
        assert_eq!(deque.pop_bottom(), Some(3));
        assert_eq!(deque.pop_bottom(), None);
    }

    #[test]
    fn test_concurrent_push_steal() {
        let deque = &WorkStealingDeque::new();

        for i in 0..1000 {
            deque.push_bottom(i);
        }

        let stolen_count = (0..100).filter_map(|_| deque.steal()).count();
        let remaining = (0..1000).filter_map(|_| deque.pop_bottom()).count();

        assert_eq!(stolen_count + remaining, 1000);
    }
}
