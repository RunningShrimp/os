//! POSIX 会话/进程组/TTY 语义骨架
//!
//! 仅提供占位实现与数据结构，后续需与进程表、TTY 层正式对接。

use crate::process::manager::Pid;
use crate::sync::Mutex;
use alloc::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionInfo {
    pub sid: Pid,
    pub pgid: Pid,
    pub tty: Option<u32>, // 占位：TTY 编号
}

impl SessionInfo {
    pub fn new(pid: Pid) -> Self {
        Self { sid: pid, pgid: pid, tty: None }
    }
}

static SESSION_TABLE: Mutex<BTreeMap<Pid, SessionInfo>> = Mutex::new(BTreeMap::new());

/// 创建新会话：sid = pgid = pid
pub fn setsid(pid: Pid) -> Result<SessionInfo, ()> {
    let mut tbl = SESSION_TABLE.lock();
    let info = SessionInfo::new(pid);
    tbl.insert(pid, info);
    Ok(info)
}

/// 设置进程组
pub fn setpgid(pid: Pid, pgid: Pid) -> Result<SessionInfo, ()> {
    let mut tbl = SESSION_TABLE.lock();
    let entry = tbl.entry(pid).or_insert(SessionInfo::new(pid));
    entry.pgid = pgid;
    Ok(*entry)
}

/// 获取当前记录的会话信息
pub fn getsession(pid: Pid) -> Option<SessionInfo> {
    let tbl = SESSION_TABLE.lock();
    tbl.get(&pid).copied()
}

/// 绑定控制终端（占位）
pub fn set_tty(pid: Pid, tty: u32) -> Result<(), ()> {
    let mut tbl = SESSION_TABLE.lock();
    let entry = tbl.entry(pid).or_insert(SessionInfo::new(pid));
    entry.tty = Some(tty);
    Ok(())
}

