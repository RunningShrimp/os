extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use core::ptr;
use crate::process::thread::Tid;

#[derive(Debug)]
struct Node {
    tid: Tid,
    priority: u8,
    next: AtomicPtr<Node>,
}

impl Node {
    fn new(tid: Tid, priority: u8) -> Self {
        Self {
            tid,
            priority,
            next: AtomicPtr::new(ptr::null_mut()),
        }
    }
}

pub struct LockFreeMpscQueue {
    head: AtomicPtr<Node>,
    tail: AtomicPtr<Node>,
    count: AtomicUsize,
    dropped: AtomicUsize,
}

impl LockFreeMpscQueue {
    pub fn new() -> Self {
        let dummy = Arc::new(Node::new(0, 0));
        let dummy_ptr = Arc::into_raw(dummy) as *mut Node;

        Self {
            head: AtomicPtr::new(dummy_ptr),
            tail: AtomicPtr::new(dummy_ptr),
            count: AtomicUsize::new(0),
            dropped: AtomicUsize::new(0),
        }
    }

    pub fn enqueue(&self, tid: Tid, priority: u8) -> bool {
        let node = Arc::new(Node::new(tid, priority));
        let node_ptr = Arc::into_raw(node) as *mut Node;

        let mut prev_tail = self.tail.load(Ordering::Acquire);
        loop {
            if prev_tail.is_null() {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                unsafe { drop(Arc::from_raw(node_ptr as *const Node)); }
                return false;
            }

            let node_next = unsafe { (*prev_tail).next.load(Ordering::Acquire) };

            if node_next.is_null() {
                match unsafe { (*prev_tail).next.compare_exchange_weak(
                    ptr::null_mut(),
                    node_ptr,
                    Ordering::Release,
                    Ordering::Relaxed,
                ) } {
                    Ok(_) => {
                        match self.tail.compare_exchange_weak(
                            prev_tail,
                            node_ptr,
                            Ordering::Release,
                            Ordering::Relaxed,
                        ) {
                            Ok(_) => {
                                self.count.fetch_add(1, Ordering::Relaxed);
                                return true;
                            }
                            Err(_) => {}
                        }
                    }
                    Err(_) => {
                        prev_tail = unsafe { (*prev_tail).next.load(Ordering::Acquire) };
                        continue;
                    }
                }
            } else {
                prev_tail = node_next;
            }
        }
    }

    pub fn dequeue_batch(&self, batch: &mut [(Tid, u8)], max_count: usize) -> usize {
        let mut dequeued = 0;
        let mut first_ptr: *mut Node = ptr::null_mut();

        while dequeued < max_count {
            let dummy = self.head.load(Ordering::Acquire);
            if dummy.is_null() {
                break;
            }

            first_ptr = unsafe { (*dummy).next.load(Ordering::Acquire) };
            if first_ptr.is_null() {
                break;
            }

            batch[dequeued] = unsafe { ((*first_ptr).tid, (*first_ptr).priority) };
            dequeued += 1;

            match self.head.compare_exchange_weak(
                dummy,
                first_ptr,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    unsafe { (*dummy).next.store(ptr::null_mut(), Ordering::Release); }
                    self.count.fetch_sub(dequeued, Ordering::Relaxed);
                    break;
                }
                Err(_) => {
                    continue;
                }
            }
        }

        if !first_ptr.is_null() && self.tail.load(Ordering::Acquire) == self.head.load(Ordering::Acquire) {
            let dummy = self.head.load(Ordering::Acquire);
            match self.tail.compare_exchange_weak(
                dummy,
                first_ptr,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => {}
                Err(_) => {}
            }
        }

        dequeued
    }

    pub fn dequeue(&self) -> Option<(Tid, u8)> {
        let mut result = (0, 0);
        let count = self.dequeue_batch(&mut [result], 1);
        if count > 0 {
            Some(result)
        } else {
            None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.count.load(Ordering::Relaxed) == 0
    }

    pub fn len(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    pub fn dropped_count(&self) -> usize {
        self.dropped.load(Ordering::Relaxed)
    }
}

impl Default for LockFreeMpscQueue {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for LockFreeMpscQueue {}
unsafe impl Sync for LockFreeMpscQueue {}
