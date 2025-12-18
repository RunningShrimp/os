extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
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

pub fn summary() -> String {
    ensure_init();
    let mut s = String::new();
    s.push_str("# Boot Timeline Summary\n");
    unsafe {
        if let Some(ref v) = EVENTS {
            let events = v.lock();
            let get = |label: &str| -> Option<u64> {
                for (ts, l) in events.iter() {
                    if *l == label { return Some(*ts); }
                }
                None
            };
            let dur = |a: Option<u64>, b: Option<u64>| -> Option<u64> {
                match (a, b) { (Some(x), Some(y)) if y >= x => Some(y - x), _ => None }
            };

            let boot_total = dur(get("boot_start"), get("boot_complete"));
            if let Some(d) = boot_total { s.push_str(&format!("boot_total_ms: {}\n", d / 1_000_000)); }
            let stages = [
                ("early_init_ms", "boot_start", "early_init"),
                ("vm_init_ms", "early_init", "vm_init"),
                ("drivers_init_ms", "vm_init", "drivers_init"),
                ("fs_init_ms", "drivers_init", "fs_init"),
                ("services_init_ms", "fs_init", "services_init"),
            ];
            for (name, a, b) in stages.iter() {
                if let Some(d) = dur(get(a), get(b)) { s.push_str(&format!("{}: {}\n", name, d / 1_000_000)); }
            }

            let lazy_total = dur(get("lazy_init_start"), get("lazy_init_complete"));
            if let Some(d) = lazy_total { s.push_str(&format!("lazy_total_ms: {}\n", d / 1_000_000)); }
            let lazy_stages = [
                ("lazy_net_init_ms", "lazy_init_start", "lazy_net_init"),
                ("lazy_graphics_init_ms", "lazy_net_init", "lazy_graphics_init"),
                ("lazy_web_init_ms", "lazy_graphics_init", "lazy_web_init"),
            ];
            for (name, a, b) in lazy_stages.iter() {
                if let Some(d) = dur(get(a), get(b)) { s.push_str(&format!("{}: {}\n", name, d / 1_000_000)); }
            }
        }
    }
    s
}

pub fn summary_json() -> String {
    ensure_init();
    let mut s = String::new();
    s.push_str("{\n");
    unsafe {
        if let Some(ref v) = EVENTS {
            let events = v.lock();
            let get = |label: &str| -> Option<u64> {
                for (ts, l) in events.iter() { if *l == label { return Some(*ts); } }
                None
            };
            let dur = |a: Option<u64>, b: Option<u64>| -> Option<u64> {
                match (a, b) { (Some(x), Some(y)) if y >= x => Some(y - x), _ => None }
            };
            let ms = |nanos: Option<u64>| -> u64 { nanos.map(|d| d / 1_000_000).unwrap_or(0) };

            let boot_total = ms(dur(get("boot_start"), get("boot_complete")));
            s.push_str(&alloc::format!("  \"boot_total_ms\": {},\n", boot_total));
            let stages = [
                ("early_init_ms", "boot_start", "early_init"),
                ("vm_init_ms", "early_init", "vm_init"),
                ("drivers_init_ms", "vm_init", "drivers_init"),
                ("fs_init_ms", "drivers_init", "fs_init"),
                ("services_init_ms", "fs_init", "services_init"),
            ];
            for (i, (name, a, b)) in stages.iter().enumerate() {
                let val = ms(dur(get(a), get(b)));
                s.push_str(&alloc::format!("  \"{}\": {}",
                    name, val));
                s.push_str(",\n");
            }

            let lazy_total = ms(dur(get("lazy_init_start"), get("lazy_init_complete")));
            s.push_str(&alloc::format!("  \"lazy_total_ms\": {},\n", lazy_total));
            let lazy_stages = [
                ("lazy_net_init_ms", "lazy_init_start", "lazy_net_init"),
                ("lazy_graphics_init_ms", "lazy_net_init", "lazy_graphics_init"),
                ("lazy_web_init_ms", "lazy_graphics_init", "lazy_web_init"),
            ];
            for (i, (name, a, b)) in lazy_stages.iter().enumerate() {
                let val = ms(dur(get(a), get(b)));
                s.push_str(&alloc::format!("  \"{}\": {}",
                    name, val));
                if i + 1 < lazy_stages.len() { s.push_str(",\n"); } else { s.push_str("\n"); }
            }
        }
    }
    s.push_str("}\n");
    s
}

pub fn events_json() -> String {
    ensure_init();
    let mut s = String::new();
    s.push_str("{\n  \"events\": [\n");
    unsafe {
        if let Some(ref v) = EVENTS {
            let events = v.lock();
            for (i, (ts, label)) in events.iter().enumerate() {
                s.push_str(&alloc::format!("    {{ \"ts\": {}, \"label\": \"{}\" }}", ts, label));
                if i + 1 < events.len() { s.push_str(",\n"); } else { s.push_str("\n"); }
            }
        }
    }
    s.push_str("  ]\n}\n");
    s
}
