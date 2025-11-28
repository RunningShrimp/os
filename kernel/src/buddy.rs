use crate::mm::{PAGE_SIZE, page_round_up, page_round_down};
use core::sync::atomic::{AtomicBool, Ordering};

const MAX_ORDER: usize = 12; // up to 2^12 pages

struct FreeList {
    head: Option<*mut Node>,
}

#[repr(C)]
struct Node {
    next: Option<*mut Node>,
}

pub struct Buddy {
    base: usize,
    end: usize,
    lists: [FreeList; MAX_ORDER+1],
    initialized: AtomicBool,
}

impl Buddy {
    pub const fn new() -> Self {
        const EMPTY: FreeList = FreeList { head: None };
        Self { base: 0, end: 0, lists: [EMPTY; MAX_ORDER+1], initialized: AtomicBool::new(false) }
    }

    pub unsafe fn init(&self, start: usize, end: usize) {
        if self.initialized.swap(true, Ordering::SeqCst) { return; }
        let base = page_round_up(start);
        let end = page_round_down(end);
        // store in static via raw pointer write
        let this = self as *const _ as *mut Buddy;
        (*this).base = base;
        (*this).end = end;
        // coalesce whole range into largest blocks
        let mut cur = base;
        while cur < end {
            let remain_pages = (end - cur) / PAGE_SIZE;
            let mut order = 0usize;
            while (1usize << (order+1)) <= remain_pages && ((cur / PAGE_SIZE) & ((1usize << (order+1)) - 1)) == 0 {
                order += 1;
                if order >= MAX_ORDER { break; }
            }
            self.push(cur, order);
            cur += (1usize << order) * PAGE_SIZE;
        }
    }

    fn push(&self, addr: usize, order: usize) {
        let node = addr as *mut Node;
        unsafe { (*node).next = self.lists[order].head; }
        let fl = &self.lists[order] as *const _ as *mut FreeList;
        unsafe { (*fl).head = Some(node); }
    }

    pub fn alloc(&self, pages: usize) -> Option<usize> {
        if pages == 0 { return None; }
        let mut order = 0usize;
        while (1usize << order) < pages && order < MAX_ORDER { order += 1; }
        let mut cur_order = order;
        while cur_order <= MAX_ORDER {
            if let Some(head) = self.lists[cur_order].head {
                // pop
                let fl = &self.lists[cur_order] as *const _ as *mut FreeList;
                unsafe { (*fl).head = (*head).next; }
                // split down to requested order
                let addr = head as usize;
                while cur_order > order {
                    cur_order -= 1;
                    let buddy_addr = addr + (1usize << cur_order) * PAGE_SIZE;
                    self.push(buddy_addr, cur_order);
                }
                return Some(addr);
            }
            cur_order += 1;
        }
        None
    }

    pub fn free(&self, addr: usize, pages: usize) {
        if pages == 0 { return; }
        let mut order = 0usize;
        while (1usize << order) < pages && order < MAX_ORDER { order += 1; }
        let mut block_addr = addr;
        // try coalesce with buddy blocks
        loop {
            if order >= MAX_ORDER { break; }
            let block_pages = 1usize << order;
            let buddy_addr = ((block_addr - self.base) / PAGE_SIZE ^ block_pages) * PAGE_SIZE + self.base;
            // search and remove buddy from freelist if present
            let mut prev: Option<*mut Node> = None;
            let mut cur = self.lists[order].head;
            let mut found = false;
            while let Some(n) = cur {
                if n as usize == buddy_addr {
                    found = true;
                    // remove
                    let next = unsafe { (*n).next };
                    let fl = &self.lists[order] as *const _ as *mut FreeList;
                    unsafe {
                        match prev {
                            Some(p) => { (*p).next = next; }
                            None => { (*fl).head = next; }
                        }
                    }
                    break;
                }
                prev = cur;
                cur = unsafe { (*n).next };
            }
            if found {
                // merge
                let merged = if buddy_addr < block_addr { buddy_addr } else { block_addr };
                block_addr = merged;
                order += 1;
                continue;
            } else {
                break;
            }
        }
        self.push(block_addr, order);
    }

    pub fn free_pages(&self) -> usize {
        let mut total = 0usize;
        for order in 0..=MAX_ORDER {
            let mut cur = self.lists[order].head;
            while let Some(n) = cur {
                total += 1usize << order;
                unsafe { cur = (*n).next; }
            }
        }
        total
    }

    pub fn total_pages(&self) -> usize {
        (self.end - self.base) / PAGE_SIZE
    }

    pub fn largest_free_order(&self) -> Option<usize> {
        for order in (0..=MAX_ORDER).rev() {
            if self.lists[order].head.is_some() { return Some(order); }
        }
        None
    }
}

pub static BUDDY: Buddy = Buddy::new();
unsafe impl Sync for Buddy {}
