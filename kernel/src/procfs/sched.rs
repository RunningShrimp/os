//! /proc/sched 统计导出（占位）

use crate::sched::{with_global, StatsSnapshot};
use alloc::string::String;
use alloc::fmt::Write;

pub fn read_sched_stats() -> String {
    let mut out = String::new();
    if let Some(s) = with_global(|sched| {
        let mut s = String::new();
        for cpu in 0..sched.cpu_count() {
            if let Some(stat) = sched.stats(cpu) {
                let snap: StatsSnapshot = stat.snapshot();
                let _ = writeln!(
                    s,
                    "cpu {}: ticks={} preempt={} voluntary={} latency={:?}",
                    cpu,
                    snap.ticks,
                    snap.preemptions,
                    snap.voluntary_switches,
                    snap.latency_hist
                );
            }
        }
        s
    }) {
        out.push_str(&s);
    } else {
        out.push_str("sched stats unavailable\n");
    }
    out
}









