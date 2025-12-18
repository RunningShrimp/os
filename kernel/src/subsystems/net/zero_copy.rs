//! 网络零拷贝与环形缓冲骨架
//! 未接入驱动，仅提供数据结构与接口占位。

use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
pub struct RingBuf {
    buf: Vec<u8>,
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl RingBuf {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            buf: vec![0u8; cap],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    pub fn push_slice(&mut self, data: &[u8]) -> usize {
        let cap = self.capacity();
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        let free = cap.wrapping_add(tail).wrapping_sub(head) % cap;
        let to_write = core::cmp::min(free, data.len());
        for i in 0..to_write {
            self.buf[(head + i) % cap] = data[i];
        }
        self.head.store((head + to_write) % cap, Ordering::Release);
        to_write
    }

    pub fn pop_slice(&self, out: &mut [u8]) -> usize {
        let cap = self.capacity();
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        let used = cap.wrapping_add(head).wrapping_sub(tail) % cap;
        let to_read = core::cmp::min(used, out.len());
        for i in 0..to_read {
            out[i] = self.buf[(tail + i) % cap];
        }
        self.tail.store((tail + to_read) % cap, Ordering::Release);
        to_read
    }
}

/// 零拷贝路径占位：记录用户页帧，不真正送网卡
#[derive(Debug, Default)]
pub struct ZeroCopyCtx {
    pub pinned_pages: Vec<usize>,
}

impl ZeroCopyCtx {
    pub fn new() -> Self {
        Self { pinned_pages: Vec::new() }
    }

    pub fn attach_pages(&mut self, pages: Vec<usize>) {
        self.pinned_pages = pages;
    }

    pub fn clear(&mut self) {
        self.pinned_pages.clear();
    }
}

