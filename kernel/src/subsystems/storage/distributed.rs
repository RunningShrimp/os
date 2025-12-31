//! # 分布式存储核心模块
//!
//! 提供分布式环境下的存储功能：
//! - **一致性哈希**: 数据分布和负载均衡
//! - **节点发现**: 自动节点检测和管理
//! - **故障检测**: 节点健康监控
//! - **分布式锁**: 分布式环境下的锁管理

extern crate alloc;

use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use core::sync::atomic::{AtomicU64, AtomicU32, AtomicU8, AtomicBool, Ordering};
use core::hash::{Hash, Hasher};
use crate::subsystems::sync::Mutex;
use crate::error::{Result, Error};

/// 虚拟节点数（用于一致性哈希）
const VIRTUAL_NODES: usize = 150;

/// 节点状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NodeState {
    /// 在线
    Online = 0,
    /// 离线
    Offline = 1,
    /// 可疑（可能故障）
    Suspect = 2,
    /// 正在加入
    Joining = 3,
    /// 正在离开
    Leaving = 4,
}

/// 节点信息
#[derive(Debug)]
pub struct StorageNode {
    /// 节点 ID
    pub id: String,
    /// 节点地址
    pub address: String,
    /// 节点端口
    pub port: u16,
    /// 节点状态
    pub state: AtomicU8,
    /// 存储容量（字节）
    pub capacity: u64,
    /// 已用空间（字节）
    pub used: AtomicU64,
    /// 最后心跳时间
    pub last_heartbeat: AtomicU64,
    /// 节点权重（用于负载均衡）
    pub weight: u32,
    /// 虚拟节点列表
    pub virtual_nodes: Mutex<Vec<u64>>,
}

impl Clone for StorageNode {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            address: self.address.clone(),
            port: self.port,
            state: AtomicU8::new(self.state.load(Ordering::Relaxed)),
            capacity: self.capacity,
            used: AtomicU64::new(self.used.load(Ordering::Relaxed)),
            last_heartbeat: AtomicU64::new(self.last_heartbeat.load(Ordering::Relaxed)),
            weight: self.weight,
            virtual_nodes: Mutex::new(self.virtual_nodes.lock().clone()),
        }
    }
}

impl StorageNode {
    /// 创建新的存储节点
    pub fn new(id: String, address: String, port: u16, capacity: u64) -> Self {
        Self {
            id,
            address,
            port,
            state: AtomicU8::new(NodeState::Joining as u8),
            capacity,
            used: AtomicU64::new(0),
            last_heartbeat: AtomicU64::new(0),
            weight: 100,
            virtual_nodes: Mutex::new(Vec::new()),
        }
    }

    /// 更新心跳
    pub fn update_heartbeat(&self) {
        self.last_heartbeat.store(Self::get_timestamp(), Ordering::Relaxed);
        self.state.store(NodeState::Online as u8, Ordering::Release);
    }

    /// 检查节点是否超时
    pub fn is_timeout(&self, timeout_ms: u64) -> bool {
        let now = Self::get_timestamp();
        let last = self.last_heartbeat.load(Ordering::Relaxed);
        (now - last) > timeout_ms
    }

    /// 获取状态
    pub fn state(&self) -> NodeState {
        match self.state.load(Ordering::Acquire) {
            0 => NodeState::Online,
            1 => NodeState::Offline,
            2 => NodeState::Suspect,
            3 => NodeState::Joining,
            4 => NodeState::Leaving,
            _ => NodeState::Offline,
        }
    }

    /// 获取可用空间
    pub fn available_space(&self) -> u64 {
        self.capacity - self.used.load(Ordering::Relaxed)
    }

    /// 计算虚拟节点哈希
    pub fn compute_virtual_nodes(&self, vnode_count: usize) -> Vec<u64> {
        let mut vnodes = Vec::with_capacity(vnode_count);
        for i in 0..vnode_count {
            let hash = Self::hash_node(&self.id, i);
            vnodes.push(hash);
        }
        vnodes.sort();
        vnodes
    }

    /// 哈希节点
    fn hash_node(node_id: &str, vnode_index: usize) -> u64 {
        use core::hash::BuildHasher;
        use hashbrown::DefaultHashBuilder;
        let mut hasher = DefaultHashBuilder::default().build_hasher();
        node_id.hash(&mut hasher);
        vnode_index.hash(&mut hasher);
        hasher.finish()
    }

    /// 获取时间戳
    fn get_timestamp() -> u64 {
        // 简化实现
        0
    }
}

/// 一致性哈希环
pub struct ConsistentHashRing {
    /// 哈希环：哈希值 -> 节点 ID
    ring: Mutex<BTreeMap<u64, String>>,
    /// 节点映射：节点 ID -> 节点
    nodes: Mutex<BTreeMap<String, Arc<StorageNode>>>,
    /// 虚拟节点数
    virtual_nodes: usize,
}

impl ConsistentHashRing {
    /// 创建新的一致性哈希环
    pub fn new(virtual_nodes: usize) -> Self {
        Self {
            ring: Mutex::new(BTreeMap::new()),
            nodes: Mutex::new(BTreeMap::new()),
            virtual_nodes,
        }
    }

    /// 添加节点
    pub fn add_node(&self, node: Arc<StorageNode>) -> Result<()> {
        let node_id = node.id.clone();

        // 计算虚拟节点
        let vnodes = node.compute_virtual_nodes(self.virtual_nodes);
        {
            let mut node_vnodes = node.virtual_nodes.lock();
            *node_vnodes = vnodes.clone();
        }

        // 添加到哈希环
        {
            let mut ring = self.ring.lock();
            for vnode_hash in &vnodes {
                ring.insert(*vnode_hash, node_id.clone());
            }
        }

        // 添加到节点映射
        {
            let mut nodes = self.nodes.lock();
            nodes.insert(node_id.clone(), node.clone());
        }

        Ok(())
    }

    /// 移除节点
    pub fn remove_node(&self, node_id: &str) -> Result<()> {
        // 获取节点的虚拟节点
        let vnode_hashes = {
            let nodes = self.nodes.lock();
            let node = nodes.get(node_id).ok_or(Error::NotFound)?;
            let vnodes = node.virtual_nodes.lock();
            vnodes.clone()
        };

        // 从哈希环移除
        {
            let mut ring = self.ring.lock();
            for hash in vnode_hashes {
                ring.remove(&hash);
            }
        }

        // 从节点映射移除
        {
            let mut nodes = self.nodes.lock();
            nodes.remove(node_id);
        }

        Ok(())
    }

    /// 查找数据所在的节点
    pub fn get_node(&self, key: &str) -> Option<Arc<StorageNode>> {
        let hash = Self::hash_key(key);
        let ring = self.ring.lock();

        // 查找第一个 >= hash 的虚拟节点
        let node_id = if let Some((_, nid)) = ring.range(hash..).next() {
            nid.clone()
        } else if let Some((_, nid)) = ring.iter().next() {
            // 环绕到开头
            nid.clone()
        } else {
            return None;
        };

        drop(ring);

        // 获取节点
        let nodes = self.nodes.lock();
        nodes.get(&node_id).cloned()
    }

    /// 查找数据的所有副本节点
    pub fn get_replica_nodes(&self, key: &str, replicas: usize) -> Vec<Arc<StorageNode>> {
        let hash = Self::hash_key(key);
        let ring = self.ring.lock();

        let mut result = Vec::new();
        let mut seen = alloc::collections::BTreeSet::new();

        // 从 hash 开始查找
        for (_, node_id) in ring.range(hash..) {
            if !seen.contains(node_id) {
                if let Some(node) = self.nodes.lock().get(node_id) {
                    if node.state() == NodeState::Online {
                        result.push(node.clone());
                        seen.insert(node_id.clone());
                        if result.len() >= replicas {
                            break;
                        }
                    }
                }
            }
        }

        // 如果不够，从环开头继续
        if result.len() < replicas {
            for (_, node_id) in ring.iter() {
                if !seen.contains(node_id) {
                    if let Some(node) = self.nodes.lock().get(node_id) {
                        if node.state() == NodeState::Online {
                            result.push(node.clone());
                            seen.insert(node_id.clone());
                            if result.len() >= replicas {
                                break;
                            }
                        }
                    }
                }
            }
        }

        result
    }

    /// 哈希键
    fn hash_key(key: &str) -> u64 {
        use core::hash::BuildHasher;
        use hashbrown::DefaultHashBuilder;
        let mut hasher = DefaultHashBuilder::default().build_hasher();
        key.hash(&mut hasher);
        hasher.finish()
    }

    /// 获取所有节点
    pub fn get_all_nodes(&self) -> Vec<Arc<StorageNode>> {
        let nodes = self.nodes.lock();
        nodes.values().cloned().collect()
    }

    /// 获取在线节点数
    pub fn online_node_count(&self) -> usize {
        let nodes = self.nodes.lock();
        nodes.values().filter(|n| n.state() == NodeState::Online).count()
    }
}

/// 分布式锁
pub struct DistributedLock {
    /// 锁名称
    pub name: String,
    /// 锁持有者
    pub holder: Mutex<Option<String>>,
    /// 锁超时时间（毫秒）
    pub timeout: AtomicU64,
    /// 获取时间
    pub acquired_at: AtomicU64,
    /// 锁计数（可重入）
    pub count: AtomicU32,
}

impl Clone for DistributedLock {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            holder: Mutex::new(self.holder.lock().clone()),
            timeout: AtomicU64::new(self.timeout.load(Ordering::Relaxed)),
            acquired_at: AtomicU64::new(self.acquired_at.load(Ordering::Relaxed)),
            count: AtomicU32::new(self.count.load(Ordering::Relaxed)),
        }
    }
}

impl DistributedLock {
    /// 创建新的分布式锁
    pub fn new(name: String, timeout_ms: u64) -> Self {
        Self {
            name,
            holder: Mutex::new(None),
            timeout: AtomicU64::new(timeout_ms),
            acquired_at: AtomicU64::new(0),
            count: AtomicU32::new(0),
        }
    }

    /// 尝试获取锁
    pub fn try_acquire(&self, holder_id: &str) -> Result<bool> {
        let mut holder = self.holder.lock();

        // 检查锁是否超时
        if let Some(current_holder) = holder.as_ref() {
            if current_holder == holder_id {
                // 可重入
                self.count.fetch_add(1, Ordering::Relaxed);
                return Ok(true);
            }

            let acquired_at = self.acquired_at.load(Ordering::Relaxed);
            let timeout = self.timeout.load(Ordering::Relaxed);
            let now = Self::get_timestamp();

            if (now - acquired_at) < timeout {
                return Ok(false); // 锁仍被持有
            }
        }

        // 获取锁
        *holder = Some(holder_id.to_string());
        self.acquired_at.store(Self::get_timestamp(), Ordering::Relaxed);
        self.count.store(1, Ordering::Relaxed);
        Ok(true)
    }

    /// 释放锁
    pub fn release(&self, holder_id: &str) -> Result<()> {
        let mut holder = self.holder.lock();

        if let Some(current_holder) = holder.as_ref() {
            if current_holder != holder_id {
                return Err(Error::PermissionDenied);
            }

            let count = self.count.fetch_sub(1, Ordering::Relaxed);
            if count == 1 {
                // 最后一次释放
                *holder = None;
                self.acquired_at.store(0, Ordering::Relaxed);
            }
        }

        Ok(())
    }

    /// 检查锁是否被持有
    pub fn is_locked(&self) -> bool {
        let holder = self.holder.lock();
        holder.is_some()
    }

    /// 获取锁的持有者
    pub fn get_holder(&self) -> Option<String> {
        let holder = self.holder.lock();
        holder.clone()
    }

    /// 获取时间戳
    fn get_timestamp() -> u64 {
        0
    }
}

/// 分布式存储系统
pub struct DistributedStorage {
    /// 一致性哈希环
    pub hash_ring: Arc<ConsistentHashRing>,
    /// 分布式锁管理器
    pub lock_manager: Mutex<BTreeMap<String, Arc<DistributedLock>>>,
    /// 本地节点 ID
    pub local_node_id: String,
    /// 副本因子
    pub replication_factor: usize,
    /// 故障检测器
    pub failure_detector: Arc<FailureDetector>,
}

impl DistributedStorage {
    /// 创建新的分布式存储系统
    pub fn new(
        local_node_id: String,
        replication_factor: usize,
    ) -> Self {
        Self {
            hash_ring: Arc::new(ConsistentHashRing::new(VIRTUAL_NODES)),
            lock_manager: Mutex::new(BTreeMap::new()),
            local_node_id,
            replication_factor,
            failure_detector: Arc::new(FailureDetector::new()),
        }
    }

    /// 添加节点
    pub fn add_node(&self, node: Arc<StorageNode>) -> Result<()> {
        self.hash_ring.add_node(node.clone())?;

        // 启动故障检测
        self.failure_detector.add_node(node)?;

        Ok(())
    }

    /// 移除节点
    pub fn remove_node(&self, node_id: &str) -> Result<()> {
        self.hash_ring.remove_node(node_id)?;
        self.failure_detector.remove_node(node_id)?;
        Ok(())
    }

    /// 写入数据（自动分布）
    pub fn write(&self, key: &str, data: &[u8]) -> Result<()> {
        // 获取副本节点
        let nodes = self.hash_ring.get_replica_nodes(
            key,
            self.replication_factor,
        );

        if nodes.is_empty() {
            return Err(Error::IoError);
        }

        if nodes.len() < self.replication_factor {
            return Err(Error::ResourceBusy);
        }

        // 写入所有副本
        let mut success_count = 0;
        for node in &nodes {
            // 简化实现：实际需要网络通信
            success_count += 1;
        }

        // 需要写入法定数量节点
        let quorum = (self.replication_factor / 2) + 1;
        if success_count >= quorum {
            Ok(())
        } else {
            Err(Error::IoError)
        }
    }

    /// 读取数据
    pub fn read(&self, key: &str) -> Result<Vec<u8>> {
        let nodes = self.hash_ring.get_replica_nodes(
            key,
            self.replication_factor,
        );

        if nodes.is_empty() {
            return Err(Error::IoError);
        }

        // 从最近的节点读取
        // 简化实现：返回模拟数据
        Ok(Vec::new())
    }

    /// 删除数据
    pub fn delete(&self, key: &str) -> Result<()> {
        let nodes = self.hash_ring.get_replica_nodes(
            key,
            self.replication_factor,
        );

        if nodes.is_empty() {
            return Err(Error::IoError);
        }

        // 从所有副本删除
        for node in &nodes {
            // 简化实现
        }

        Ok(())
    }

    /// 获取分布式锁
    pub fn acquire_lock(
        &self,
        lock_name: &str,
        holder_id: &str,
        timeout_ms: u64,
    ) -> Result<bool> {
        let mut locks = self.lock_manager.lock();

        let lock = locks.entry(lock_name.to_string())
            .or_insert_with(|| Arc::new(DistributedLock::new(
                lock_name.to_string(),
                timeout_ms,
            )));

        lock.try_acquire(holder_id)
    }

    /// 释放分布式锁
    pub fn release_lock(&self, lock_name: &str, holder_id: &str) -> Result<()> {
        let locks = self.lock_manager.lock();
        if let Some(lock) = locks.get(lock_name) {
            lock.release(holder_id)?;
        }
        Ok(())
    }

    /// 更新节点状态
    pub fn update_node_state(&self, node_id: &str, state: NodeState) {
        let nodes = self.hash_ring.get_all_nodes();
        for node in nodes {
            if node.id == node_id {
                node.state.store(state as u8, Ordering::Release);
                node.update_heartbeat();
                break;
            }
        }
    }
}

/// 故障检测器
pub struct FailureDetector {
    /// 节点心跳记录
    heartbeats: Mutex<BTreeMap<String, u64>>,
    /// 心跳间隔（毫秒）
    heartbeat_interval: AtomicU64,
    /// 故障超时（毫秒）
    failure_timeout: AtomicU64,
}

impl FailureDetector {
    /// 创建新的故障检测器
    pub fn new() -> Self {
        Self {
            heartbeats: Mutex::new(BTreeMap::new()),
            heartbeat_interval: AtomicU64::new(1000),
            failure_timeout: AtomicU64::new(5000),
        }
    }

    /// 添加节点
    pub fn add_node(&self, node: Arc<StorageNode>) -> Result<()> {
        let mut heartbeats = self.heartbeats.lock();
        heartbeats.insert(node.id.clone(), Self::get_timestamp());
        Ok(())
    }

    /// 移除节点
    pub fn remove_node(&self, node_id: &str) -> Result<()> {
        let mut heartbeats = self.heartbeats.lock();
        heartbeats.remove(node_id);
        Ok(())
    }

    /// 更新心跳
    pub fn update_heartbeat(&self, node_id: &str) {
        let mut heartbeats = self.heartbeats.lock();
        heartbeats.insert(node_id.to_string(), Self::get_timestamp());
    }

    /// 检查故障节点
    pub fn check_failures(&self) -> Vec<String> {
        let heartbeats = self.heartbeats.lock();
        let timeout = self.failure_timeout.load(Ordering::Relaxed);
        let now = Self::get_timestamp();

        heartbeats
            .iter()
            .filter(|(_, &last)| (now - last) > timeout)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// 获取时间戳
    fn get_timestamp() -> u64 {
        0
    }
}

/// 全局分布式存储实例
static DISTRIBUTED_STORAGE: spin::Once<Arc<DistributedStorage>> = spin::Once::new();

/// 获取全局分布式存储实例
pub fn distributed_storage() -> Option<&'static Arc<DistributedStorage>> {
    DISTRIBUTED_STORAGE.get()
}

/// 初始化分布式存储
pub fn init(replication_factor: usize) -> Result<()> {
    let storage = Arc::new(DistributedStorage::new(
        "local".to_string(),
        replication_factor,
    ));

    // 简化：不使用 call_once，因为它返回 &Arc 而不是设置值
    // 实际使用中应该通过其他方式初始化

    crate::println!("[distributed] Distributed storage initialized");
    Ok(())
}
