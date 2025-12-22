//! 简单内存日志缓冲（环形），仅用于记录 VFS 操作占位。

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

pub struct LogBuffer {
    buf: Mutex<Vec<String>>,
    head: AtomicUsize,
    cap: usize,
}

impl LogBuffer {
    pub fn with_capacity(cap: usize) -> Self {
        let cap = cap.max(8);
        Self {
            buf: Mutex::new(Vec::with_capacity(cap)),
            head: AtomicUsize::new(0),
            cap,
        }
    }

    pub fn push(&self, entry: String) {
        let idx = self.head.fetch_add(1, Ordering::Relaxed);
        let pos = idx % self.cap;
        let mut buf = self.buf.lock();
        if buf.len() < self.cap {
            buf.push(entry);
        } else {
            buf[pos] = entry;
        }
    }

    pub fn snapshot(&self) -> Vec<String> {
        let mut out = Vec::new();
        let buf = self.buf.lock();
        let len = buf.len();
        let head = self.head.load(Ordering::Relaxed);
        let start = if len == self.cap { head % self.cap } else { 0 };
        for i in 0..len {
            let idx = (start + i) % len;
            out.push(buf[idx].clone());
        }
        out
    }
}








