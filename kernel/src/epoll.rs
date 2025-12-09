extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use crate::sync::Mutex;
#[cfg(feature = "posix_layer")]
use crate::posix;

#[derive(Clone, Copy)]
pub struct EpollItem {
    pub fd: i32,
    pub events: i32,
}

pub struct EpollInst {
    pub items: Vec<EpollItem>,
}

impl EpollInst { pub fn new() -> Self { Self { items: Vec::new() } } }

static EPOLL_TABLE: Mutex<BTreeMap<i32, EpollInst>> = Mutex::new(BTreeMap::new());
static NEXT_EPFID: Mutex<i32> = Mutex::new(1);

pub fn epoll_create(_size: i32) -> i32 {
    let mut idg = NEXT_EPFID.lock();
    let id = *idg;
    *idg += 1;
    drop(idg);
    let mut t = EPOLL_TABLE.lock();
    t.insert(id, EpollInst::new());
    id
}

pub fn epoll_ctl(epfd: i32, op: i32, fd: i32, events: i32) -> isize {
    let mut t = EPOLL_TABLE.lock();
    let inst = match t.get_mut(&epfd) { Some(i) => i, None => return crate::syscalls::E_BADARG };
    match op {
        1 /* EPOLL_CTL_ADD */ => {
            // prevent duplicates
            inst.items.retain(|it| it.fd != fd);
            inst.items.push(EpollItem { fd, events });
            crate::syscalls::E_OK
        }
        2 /* EPOLL_CTL_DEL */ => {
            inst.items.retain(|it| it.fd != fd);
            crate::syscalls::E_OK
        }
        3 /* EPOLL_CTL_MOD */ => {
            for it in inst.items.iter_mut() { if it.fd == fd { it.events = events; return crate::syscalls::E_OK; } }
            crate::syscalls::E_BADARG
        }
        _ => crate::syscalls::E_INVAL,
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct EpollEvent { events: u32, data: u64 }

pub fn epoll_wait(epfd: i32, events_ptr: usize, maxevents: i32, timeout: i32) -> isize {
    if maxevents <= 0 { return crate::syscalls::E_BADARG; }
    let mut ready = 0;
    let start = crate::time::get_ticks();
    loop {
        ready = 0;
        let mut out: Vec<EpollEvent> = Vec::new();
        {
            let t = EPOLL_TABLE.lock();
            let inst = match t.get(&epfd) { Some(i) => i, None => return crate::syscalls::E_BADARG };
            for it in inst.items.iter() {
                let idx = match crate::process::fdlookup(it.fd) { Some(i) => i, None => continue };
                let ev = crate::file::file_poll(idx) as i32;
                if (ev & it.events) != 0 {
                    out.push(EpollEvent { events: ev as u32, data: it.fd as u64 });
                    ready += 1;
                    if ready >= maxevents { break; }
                }
            }
        }
        if ready > 0 {
            // write events to user buffer
            let usize_sz = core::mem::size_of::<EpollEvent>();
            let mut ptable = crate::process::PROC_TABLE.lock();
            let pt = match crate::process::myproc().and_then(|pid| ptable.find(pid).map(|p| p.pagetable)) { Some(x) => x, None => return crate::syscalls::E_BADARG };
            drop(ptable);
            for (i, ev) in out.into_iter().enumerate() {
                let dst = events_ptr + i * usize_sz;
                let bytes = unsafe { core::slice::from_raw_parts((&ev as *const EpollEvent) as *const u8, usize_sz) };
                if unsafe { crate::vm::copyout(pt, dst, bytes.as_ptr(), bytes.len()) }.is_err() { return crate::syscalls::E_FAULT; }
            }
            return ready as isize;
        }
        if timeout == 0 { return 0; }
        if timeout > 0 {
            let elapsed = (crate::time::get_ticks() - start) as i32;
            if elapsed >= timeout { return 0; }
        }
        let target = crate::time::get_ticks() + 1;
        crate::time::add_sleeper(target, crate::syscalls::POLL_WAKE_CHAN);
        crate::process::sleep(crate::syscalls::POLL_WAKE_CHAN);
    }
}
