//! RCU/分片进程表占位实现
//!
//! 提供读多写少的接口骨架，后续可替换为真正的 RCU 读和分段锁写。

use crate::process::manager::{Pid, PROC_TABLE};
use crate::sync::{Mutex, RwLock};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// 每分片的 PID 索引（占位：仅缓存 PID -> Pid）
struct Shard {
    inner: RwLock<BTreeMap<Pid, Pid>>,
}

impl Default for Shard {
    fn default() -> Self {
        Self {
            inner: RwLock::new(BTreeMap::new()),
        }
    }
}

/// 分片表
pub struct RcuProcTable {
    shards: Vec<Shard>,
}

impl RcuProcTable {
    pub fn new(shard_cnt: usize) -> Self {
        let count = core::cmp::max(1, shard_cnt);
        let mut shards = Vec::with_capacity(count);
        for _ in 0..count {
            shards.push(Shard::default());
        }
        Self { shards }
    }

    #[inline]
    fn shard(&self, pid: Pid) -> &Shard {
        let idx = pid % self.shards.len();
        &self.shards[idx]
    }

    /// 读路径（占位）：当前仍从全局 PROC_TABLE 读取，再缓存 PID。
    pub fn find_pid(&self, pid: Pid) -> Option<Pid> {
        let shard = self.shard(pid);
        {
            let r = shard.inner.read();
            if let Some(p) = r.get(&pid) {
                return Some(*p);
            }
        }
        // 回落全局表，避免改变现有逻辑
        let table = PROC_TABLE.lock();
        if table.find_ref(pid).is_some() {
            drop(table);
            let mut w = shard.inner.write();
            w.insert(pid, pid);
            return Some(pid);
        }
        None
    }

    /// 注册新 PID，供读路径使用
    pub fn register(&self, pid: Pid) {
        let shard = self.shard(pid);
        let mut w = shard.inner.write();
        w.insert(pid, pid);
    }

    /// 写路径占位：仅删除缓存
    pub fn remove(&self, pid: Pid) {
        let shard = self.shard(pid);
        let mut w = shard.inner.write();
        w.remove(&pid);
    }
}

/// 全局分片表（延迟初始化）
static GLOBAL_SHARDED: Mutex<Option<RcuProcTable>> = Mutex::new(None);

pub fn init_sharded(count: usize) {
    let mut g = GLOBAL_SHARDED.lock();
    if g.is_none() {
        *g = Some(RcuProcTable::new(count));
    }
}

pub fn with_sharded<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&RcuProcTable) -> R,
{
    let g = GLOBAL_SHARDED.lock();
    g.as_ref().map(f)
}

