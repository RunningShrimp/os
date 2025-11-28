//! Signal-related system calls
//!
//! Implements sigaction, sigprocmask, sigsuspend, sigpending

use crate::errno;
use super::{E_OK, E_BADARG, E_INVAL, E_FAULT};

/// Set signal action
pub fn sys_sigaction(sig: i32, act: *const crate::signal::SigAction, old: *mut crate::signal::SigAction) -> isize {
    if sig <= 0 || sig as u32 >= crate::signal::NSIG as u32 { return E_BADARG; }
    let mut ptable = crate::process::PROC_TABLE.lock();
    let pid = match crate::process::myproc() { Some(p) => p, None => return E_BADARG };
    let proc = match ptable.find(pid) { Some(p) => p, None => return E_BADARG };
    let pagetable = proc.pagetable;
    if proc.signals.is_none() { proc.signals = Some(crate::signal::SignalState::new()); }
    let state = proc.signals.as_ref().unwrap();
    if !old.is_null() {
        let cur = state.get_action(sig as u32);
        let res = unsafe { crate::vm::copyout(pagetable, old as usize, (&cur as *const crate::signal::SigAction) as *const u8, core::mem::size_of::<crate::signal::SigAction>()) };
        if res.is_err() { return E_FAULT; }
    }
    if !act.is_null() {
        let mut new = crate::signal::SigAction::default();
        let res = unsafe { crate::vm::copyin(pagetable, (&mut new as *mut crate::signal::SigAction) as *mut u8, act as usize, core::mem::size_of::<crate::signal::SigAction>()) };
        if res.is_err() { return E_FAULT; }
        match state.set_action(sig as u32, new) { Ok(_) => {}, Err(_) => return E_INVAL }
    }
    E_OK
}

/// Set signal mask
pub fn sys_sigprocmask(how: i32, set: *const crate::signal::SigSet, old: *mut crate::signal::SigSet) -> isize {
    let mut ptable = crate::process::PROC_TABLE.lock();
    let pid = match crate::process::myproc() { Some(p) => p, None => return E_BADARG };
    let proc = match ptable.find(pid) { Some(p) => p, None => return E_BADARG };
    let pagetable = proc.pagetable;
    if proc.signals.is_none() { proc.signals = Some(crate::signal::SignalState::new()); }
    let state = proc.signals.as_ref().unwrap();
    if !old.is_null() {
        let cur = state.get_mask();
        let res = unsafe { crate::vm::copyout(pagetable, old as usize, (&cur as *const crate::signal::SigSet) as *const u8, core::mem::size_of::<crate::signal::SigSet>()) };
        if res.is_err() { return E_FAULT; }
    }
    if !set.is_null() {
        let mut new = crate::signal::SigSet::empty();
        let res = unsafe { crate::vm::copyin(pagetable, (&mut new as *mut crate::signal::SigSet) as *mut u8, set as usize, core::mem::size_of::<crate::signal::SigSet>()) };
        if res.is_err() { return E_FAULT; }
        match how {
            0 => { state.block(new); }
            1 => { state.unblock(new); }
            2 => { state.set_mask(new); }
            _ => return E_INVAL,
        }
    }
    E_OK
}

/// Suspend process until signal is received
pub fn sys_sigsuspend(mask: *const crate::signal::SigSet) -> isize {
    if mask.is_null() { return E_BADARG; }
    let mut ptable = crate::process::PROC_TABLE.lock();
    let pid = match crate::process::myproc() { Some(p) => p, None => return E_BADARG };
    let proc = match ptable.find(pid) { Some(p) => p, None => return E_BADARG };
    let pagetable = proc.pagetable;
    if proc.signals.is_none() { proc.signals = Some(crate::signal::SignalState::new()); }
    let mut new = crate::signal::SigSet::empty();
    let res = unsafe { crate::vm::copyin(pagetable, (&mut new as *mut crate::signal::SigSet) as *mut u8, mask as usize, core::mem::size_of::<crate::signal::SigSet>()) };
    if res.is_err() { return E_FAULT; }
    if let Some(ref state) = proc.signals { state.suspend(new); }
    drop(ptable);
    let chan = pid | 0x5000_0000;
    loop {
        let mut tbl = crate::process::PROC_TABLE.lock();
        let pr = match tbl.find(pid) { Some(p) => p, None => return E_BADARG };
        let pending = match &pr.signals { Some(s) => s.has_pending(), None => false };
        drop(tbl);
        if pending { break; }
        crate::process::sleep(chan);
    }
    let mut ptable2 = crate::process::PROC_TABLE.lock();
    let proc2 = match ptable2.find(pid) { Some(p) => p, None => return E_BADARG };
    if let Some(ref sigs) = proc2.signals { sigs.restore_mask(); }
    errno::errno_neg(errno::EINTR)
}

/// Get pending signals
pub fn sys_sigpending(set: *mut crate::signal::SigSet) -> isize {
    if set.is_null() { return E_BADARG; }
    let mut ptable = crate::process::PROC_TABLE.lock();
    let pid = match crate::process::myproc() { Some(p) => p, None => return E_BADARG };
    let proc = match ptable.find(pid) { Some(p) => p, None => return E_BADARG };
    let pagetable = proc.pagetable;
    if proc.signals.is_none() { proc.signals = Some(crate::signal::SignalState::new()); }
    let cur = crate::signal::sys_sigpending(proc.signals.as_ref().unwrap());
    let res = unsafe { crate::vm::copyout(pagetable, set as usize, (&cur as *const crate::signal::SigSet) as *const u8, core::mem::size_of::<crate::signal::SigSet>()) };
    if res.is_err() { return E_FAULT; }
    E_OK
}
