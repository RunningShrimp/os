/// Platform probing for memory and MMIO via DTB/firmware (stub)
use crate::mm;
use crate::sync::Mutex;
extern crate alloc;
use alloc::vec::Vec;
use core::slice;
use core::str;

const FDT_BEGIN_NODE: u32 = 1;
const FDT_END_NODE: u32 = 2;
const FDT_PROP: u32 = 3;
const FDT_NOP: u32 = 4;
const FDT_END: u32 = 9;

#[derive(Clone, Copy)]
struct Cells { addr: u32, size: u32 }

#[derive(Clone, Copy)]
struct Range { child: u128, parent: u128, size: u128, weight: u32, order: usize }

static STRONG_COMPAT: &[&str] = &[
    "arm,gic-400",
    "arm,gic-v2",
    "arm,gic-v3",
    "arm,arch-timer",
    "arm,sp804",
    "syscon",
    "soc,syscon",
];

struct SoCEntry { id: &'static str, strong: &'static [&'static str] }
static SOC_TABLE: &[SoCEntry] = &[
    SoCEntry { id: "qemu,virt", strong: &["arm,gic-v3", "arm,arch-timer"] },
    SoCEntry { id: "arm,vexpress", strong: &["arm,gic-400", "arm,sp804"] },
];

static GICV2_BASES: Mutex<Option<(usize, usize)>> = Mutex::new(None);
pub fn gicv2_bases() -> Option<(usize, usize)> { *GICV2_BASES.lock() }
static GICV3_BASES: Mutex<Option<(usize, usize)>> = Mutex::new(None);
pub fn gicv3_bases() -> Option<(usize, usize)> { *GICV3_BASES.lock() }
static GICR_REDISTS: Mutex<Vec<(u64, usize)>> = Mutex::new(Vec::new());
pub fn gicr_redists() -> Vec<(u64, usize)> { GICR_REDISTS.lock().clone() }
pub fn gicr_lookup(mpidr: u64) -> Option<usize> { GICR_REDISTS.lock().iter().find(|(m, _)| *m == mpidr).map(|(_, b)| *b) }
pub fn gicr_default() -> Option<usize> { GICR_REDISTS.lock().get(0).map(|(_, b)| *b) }

fn translate_addr(mut addr: usize, ranges: &[Range]) -> usize {
    loop {
        let mut best: Option<&Range> = None;
        for r in ranges {
            let cb = r.child as usize;
            let sz = r.size as usize;
            if addr >= cb && addr < cb.saturating_add(sz) {
                best = match best {
                    None => Some(r),
                    Some(b) => {
                        if r.weight > b.weight { Some(r) }
                        else if r.weight < b.weight { Some(b) }
                        else if (r.size as usize) > (b.size as usize) { Some(r) }
                        else if (r.size as usize) < (b.size as usize) { Some(b) }
                        else if r.order < b.order { Some(r) } else { Some(b) }
                    }
                };
            }
        }
        if let Some(r) = best {
            let cb = r.child as usize;
            let pb = r.parent as usize;
            addr = pb.saturating_add(addr - cb);
        } else {
            break;
        }
    }
    addr
}

pub fn probe_dtb() {
    let dtb_ptr = dtb_pointer();
    if dtb_ptr == 0 { return; }
    
    unsafe {
        let magic = *(dtb_ptr as *const u32);
        if u32::from_be(magic) != 0xd00dfeed { return; }
        
        let off_struct = u32::from_be(*((dtb_ptr + 8) as *const u32)) as usize;
        let off_strings = u32::from_be(*((dtb_ptr + 12) as *const u32)) as usize;
        
        let struct_ptr = (dtb_ptr + off_struct) as *const u32;
        let strings_ptr = (dtb_ptr + off_strings) as *const u8;
        
        let mut p = struct_ptr;
        let mut node_name = "";
        let mut cells_stack: [Cells; 32] = [Cells { addr: 2, size: 1 }; 32];
        let mut ranges_stack: [usize; 32] = [0; 32];
        let mut ranges: Vec<Range> = Vec::new();
        let mut current_weights: Vec<u32> = Vec::new();
        let mut weights_stack: [usize; 32] = [0; 32];
        let mut range_order: usize = 0;
        let mut depth: usize = 0;
        let mut current_cells = Cells { addr: 2, size: 1 };
        let mut max_mem_end: usize = 0;
        let mut node_is_strong = false;
        let mut node_is_wc = false;
        let mut strong_list: Vec<&'static str> = Vec::new();
        strong_list.extend_from_slice(STRONG_COMPAT);
        let mut node_is_gicv2 = false;
        let mut node_is_gicv3 = false;
        let mut node_is_gicr = false;
        let mut current_gicr_mpidr: u64 = 0;
        let mut current_gicr_addr: Option<usize> = None;
        let mut cfg_decay_num: u64 = 0;
        let mut cfg_decay_den: u64 = 0;
        let mut cfg_threshold_hits: u64 = 0;
        let mut cfg_cooldown_ticks: u64 = 0;
        let mut cfg_decay_interval: u64 = 0;
        let mut cfg_sample_div: u64 = 0;
        
        while u32::from_be(*p) != FDT_END {
            let token = u32::from_be(*p);
            p = p.add(1);
            
            match token {
                FDT_BEGIN_NODE => {
                    let name_ptr = p as *const u8;
                    let len = strlen(name_ptr);
                    node_name = str::from_utf8(slice::from_raw_parts(name_ptr, len)).unwrap_or("");
                    let padded = (len + 1 + 3) & !3;
                    p = (name_ptr.add(padded)) as *const u32;
                    if depth < cells_stack.len() { cells_stack[depth] = current_cells; ranges_stack[depth] = ranges.len(); weights_stack[depth] = current_weights.len(); }
                    depth += 1;
                    node_is_strong = false;
                    node_is_wc = false;
                    node_is_gicr = false;
                    current_gicr_mpidr = 0;
                    current_gicr_addr = None;
                }
                FDT_END_NODE => {
                    node_name = "";
                    if depth > 0 { depth -= 1; current_cells = cells_stack[depth.min(cells_stack.len()-1)]; }
                    let keep = ranges_stack[depth.min(ranges_stack.len()-1)];
                    if ranges.len() > keep { ranges.truncate(keep); }
                    let wkeep = weights_stack[depth.min(weights_stack.len()-1)];
                    if current_weights.len() > wkeep { current_weights.truncate(wkeep); }
                    node_is_strong = false;
                    node_is_wc = false;
                    if node_is_gicr {
                        if let Some(addr) = current_gicr_addr {
                            let mut v = GICR_REDISTS.lock();
                            v.push((current_gicr_mpidr, addr));
                        }
                    }
                    node_is_gicr = false;
                }
                FDT_PROP => {
                    let len = u32::from_be(*p) as usize;
                    p = p.add(1);
                    let nameoff = u32::from_be(*p) as usize;
                    p = p.add(1);
                    
                    let prop_name_ptr = strings_ptr.add(nameoff);
                    let prop_len = strlen(prop_name_ptr);
                    let prop_name = str::from_utf8(slice::from_raw_parts(prop_name_ptr, prop_len)).unwrap_or("");
                    
                    let data_ptr = p as *const u8;
                    
                    if prop_name == "#address-cells" && len >= 4 { current_cells.addr = u32::from_be(*(data_ptr as *const u32)); }
                    if prop_name == "#size-cells" && len >= 4 { current_cells.size = u32::from_be(*(data_ptr as *const u32)); }
                    if prop_name == "interrupt-controller" { node_is_strong = true; }
                    if depth == 1 && prop_name == "compatible" {
                        let mut off2 = 0usize;
                        while off2 < len {
                            let s_ptr2 = data_ptr.add(off2);
                            let sl2 = strlen(s_ptr2);
                            if sl2 == 0 { break; }
                            let s2 = str::from_utf8(slice::from_raw_parts(s_ptr2, sl2)).unwrap_or("");
                            for entry in SOC_TABLE {
                                if s2 == entry.id {
                                    strong_list.extend_from_slice(entry.strong);
                                }
                            }
                            off2 += sl2 + 1;
                        }
                    }
                    if depth == 1 && prop_name == "nos,mmio-decay-num" { if len >= 4 { cfg_decay_num = u32::from_be(*(data_ptr as *const u32)) as u64; } }
                    if depth == 1 && prop_name == "nos,mmio-decay-den" { if len >= 4 { cfg_decay_den = u32::from_be(*(data_ptr as *const u32)) as u64; } }
                    if depth == 1 && prop_name == "nos,mmio-threshold" { if len >= 4 { cfg_threshold_hits = u32::from_be(*(data_ptr as *const u32)) as u64; } }
                    if depth == 1 && prop_name == "nos,mmio-cooldown-ticks" { if len >= 4 { cfg_cooldown_ticks = u32::from_be(*(data_ptr as *const u32)) as u64; } }
                    if depth == 1 && prop_name == "nos,mmio-decay-interval" { if len >= 4 { cfg_decay_interval = u32::from_be(*(data_ptr as *const u32)) as u64; } }
                    if depth == 1 && prop_name == "nos,mmio-sample-div" { if len >= 4 { cfg_sample_div = u32::from_be(*(data_ptr as *const u32)) as u64; } }
                    if prop_name == "nos,strong-compat" {
                        let mut off3 = 0usize;
                        while off3 < len {
                            let s_ptr3 = data_ptr.add(off3);
                            let sl3 = strlen(s_ptr3);
                            if sl3 == 0 { break; }
                            let s3 = str::from_utf8(slice::from_raw_parts(s_ptr3, sl3)).unwrap_or("");
                            strong_list.push(s3);
                            off3 += sl3 + 1;
                        }
                    }
                    if prop_name == "nos,strong-device" { node_is_strong = true; }
                    if prop_name == "compatible" {
                        let mut off = 0usize;
                        while off < len {
                            let s_ptr = data_ptr.add(off);
                            let sl = strlen(s_ptr);
                            if sl == 0 { break; }
                            let s = str::from_utf8(slice::from_raw_parts(s_ptr, sl)).unwrap_or("");
                            for c in strong_list.iter() {
                                if s == *c { node_is_strong = true; break; }
                            }
                            if s == "arm,gic-400" || s == "arm,gic-v2" { node_is_gicv2 = true; }
                            if s == "arm,gic-v3" { node_is_gicv3 = true; }
                            if s.contains("redistributor") || s.contains("gicr") { node_is_gicr = true; }
                            if s == "simple-framebuffer" || s.contains("framebuffer") || s == "efi-framebuffer" { node_is_wc = true; }
                            off += sl + 1;
                        }
                    }
                    if prop_name == "arm,mpidr" && len >= 8 { current_gicr_mpidr = u64::from_be(*(data_ptr as *const u64)); }
                    if prop_name == "nos,range-weight" {
                        current_weights.clear();
                        let mut offw = 0usize;
                        while offw + 4 <= len {
                            let w = u32::from_be(*((data_ptr.add(offw)) as *const u32));
                            current_weights.push(w);
                            offw += 4;
                        }
                    }
                    if prop_name == "ranges" {
                        let c_ac = current_cells.addr as usize;
                        let p_ac = cells_stack[depth.saturating_sub(1).min(cells_stack.len()-1)].addr as usize;
                        let sc = current_cells.size as usize;
                        let cell_bytes = 4usize;
                        let entry_cells = c_ac + p_ac + sc;
                        let mut off = 0usize;
                        let mut idxw = 0usize;
                        let mut entries = 0usize;
                        while entry_cells > 0 && off + entry_cells * cell_bytes <= len {
                            let mut child: u128 = 0; let mut parent: u128 = 0; let mut size128: u128 = 0;
                            for i in 0..c_ac { let v = u32::from_be(*((data_ptr.add(off + i * cell_bytes)) as *const u32)) as u128; child = (child << 32) | v; }
                            for i in 0..p_ac { let v = u32::from_be(*((data_ptr.add(off + c_ac * cell_bytes + i * cell_bytes)) as *const u32)) as u128; parent = (parent << 32) | v; }
                            for i in 0..sc { let v = u32::from_be(*((data_ptr.add(off + (c_ac + p_ac) * cell_bytes + i * cell_bytes)) as *const u32)) as u128; size128 = (size128 << 32) | v; }
                            let w = if idxw < current_weights.len() { current_weights[idxw] } else { 0 };
                            ranges.push(Range { child, parent, size: size128, weight: w, order: range_order });
                            range_order += 1;
                            off += entry_cells * cell_bytes;
                            idxw += 1;
                            entries += 1;
                        }
                        if !current_weights.is_empty() && entries != current_weights.len() {
                            crate::println!("[dtb] ranges weight mismatch: entries={} weights={}", entries, current_weights.len());
                        }
                    }
                    if prop_name == "reg" {
                        let ac = current_cells.addr as usize;
                        let sc = current_cells.size as usize;
                        let cell_bytes = 4usize;
                        let entry_cells = ac + sc;
                        if entry_cells > 0 {
                            let mut off = 0usize;
                            let mut gic_entries: [(usize, usize); 2] = [(0,0); 2];
                            let mut gic_count = 0usize;
                            while off + entry_cells * cell_bytes <= len {
                                let mut addr64: u128 = 0;
                                for i in 0..ac {
                                    let v = u32::from_be(*((data_ptr.add(off + i * cell_bytes)) as *const u32)) as u128;
                                    addr64 = (addr64 << 32) | v;
                                }
                                let mut size64: u128 = 0;
                                for i in 0..sc {
                                    let v = u32::from_be(*((data_ptr.add(off + ac * cell_bytes + i * cell_bytes)) as *const u32)) as u128;
                                    size64 = (size64 << 32) | v;
                                }
                                let mut addr = addr64 as usize;
                                let size = size64 as usize;
                                addr = translate_addr(addr, &ranges);
                                if node_name.starts_with("memory") {
                                    let end = addr.saturating_add(size);
                                    if end > max_mem_end { max_mem_end = end; }
                                } else if !node_name.starts_with("cpu") && !node_name.is_empty() {
                                    if size != 0 {
                                        if node_is_wc { mm::add_mmio_region_wc(addr, size); }
                                        else if node_is_strong { mm::add_mmio_region_strong(addr, size); }
                                        else { mm::add_mmio_region(addr, size); }
                                    }
                                }
                                if (node_is_gicv2 || node_is_gicv3) && gic_count < 2 && size != 0 {
                                    gic_entries[gic_count] = (addr, size);
                                    gic_count += 1;
                                    if gic_count == 2 {
                                        if node_is_gicv2 {
                                            let mut s = GICV2_BASES.lock();
                                            *s = Some((gic_entries[0].0, gic_entries[1].0));
                                        }
                                        if node_is_gicv3 {
                                            let mut s3 = GICV3_BASES.lock();
                                            *s3 = Some((gic_entries[0].0, gic_entries[1].0));
                                        }
                                    }
                                }
                                if node_is_gicr && size != 0 { current_gicr_addr = Some(addr); }
                                off += entry_cells * cell_bytes;
                            }
                        }
                    }
                    
                    let padded = (len + 3) & !3;
                    p = (p as *const u8).add(padded) as *const u32;
                }
                FDT_NOP => {}
                _ => break,
            }
        }
        if max_mem_end != 0 { mm::set_phys_end(max_mem_end); }
        if cfg_decay_num != 0 || cfg_decay_den != 0 || cfg_threshold_hits != 0 || cfg_cooldown_ticks != 0 || cfg_decay_interval != 0 || cfg_sample_div != 0 {
            mm::mmio_cfg_update(
                if cfg_decay_interval != 0 { Some(cfg_decay_interval) } else { None },
                if cfg_decay_num != 0 { Some(cfg_decay_num) } else { None },
                if cfg_decay_den != 0 { Some(cfg_decay_den) } else { None },
                if cfg_threshold_hits != 0 { Some(cfg_threshold_hits) } else { None },
                if cfg_cooldown_ticks != 0 { Some(cfg_cooldown_ticks) } else { None },
                if cfg_sample_div != 0 { Some(cfg_sample_div) } else { None },
            );
        }
    }
}

unsafe fn strlen(ptr: *const u8) -> usize {
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    len
}

#[inline]
fn dtb_pointer() -> usize {
    // In a real kernel, this would retrieve the DTB address passed by bootloader
    // e.g. from a global variable set in start.S
    // For now, return 0 to disable unless we have a mechanism
    0
}
