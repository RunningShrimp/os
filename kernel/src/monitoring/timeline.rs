extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use crate::sync::Mutex;
use core::sync::atomic::{AtomicBool, Ordering};

static INIT: AtomicBool = AtomicBool::new(false);
static mut EVENTS: Option<Mutex<Vec<(u64, &'static str)>>> = None;

fn ensure_init() {
    if !INIT.load(Ordering::SeqCst) {
        unsafe { EVENTS = Some(Mutex::new(Vec::new())); }
        INIT.store(true, Ordering::SeqCst);
    }
}

pub fn record(label: &'static str) {
    ensure_init();
    let ts = crate::time::get_time_ns();
    unsafe {
        if let Some(ref v) = EVENTS {
            v.lock().push((ts, label));
        }
    }
}

pub fn to_string() -> String {
    ensure_init();
    let mut s = String::new();
    s.push_str("# Boot Timeline\n");
    unsafe {
        if let Some(ref v) = EVENTS {
            for (ts, label) in v.lock().iter() {
                s.push_str(&alloc::format!("{}: {}\n", ts, label));
            }
        }
    }
    s
}
