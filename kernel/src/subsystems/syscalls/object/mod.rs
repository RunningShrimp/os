// GLib对象系统支持系统调用

extern crate alloc;
// 为GLib的GObject系统提供内核级支持，包括：
// - 对象类型注册和管理
// - 信号连接和发射
// - 属性系统支持
// - 继承和接口管理
// - 引用计数管理

use crate::prelude::*;
use crate::subsystems::syscalls::interface::SyscallResult;
use alloc::collections::BTreeMap;
use core::{
    ffi::c_void,
    marker::PhantomData,
    sync::atomic::AtomicUsize,
};

use crate::subsystems::sync::Mutex;

/// GLib Object Manager trait
pub trait GObjectManager {
    /// Register a new object type
    fn register_type(
        &mut self,
        name: &str,
        parent_type: u64,
        size: usize,
        flags: u32,
    ) -> SyscallResult<u64>;

    /// Create a new object instance
    fn create_instance(&mut self, type_id: u64, object_ptr: *mut c_void) -> SyscallResult<u64>;

    /// Increment reference count for an instance
    fn ref_instance(&self, instance_id: u64) -> SyscallResult<usize>;

    /// Decrement reference count for an instance
    fn unref_instance(&self, instance_id: u64) -> SyscallResult<usize>;

    /// Register a signal for a type
    fn register_signal(
        &mut self,
        type_id: u64,
        name: &str,
        param_types: &[u64],
        return_type: u64,
        flags: u32,
    ) -> SyscallResult<u64>;

    /// Emit a signal on an instance
    fn emit_signal(&self, instance_id: u64, signal_id: u64, args: &[u64]) -> SyscallResult<usize>;

    /// Set a property value on an instance
    fn set_property(&mut self, instance_id: u64, name: &str, value: u64) -> SyscallResult<()>;

    /// Get a property value from an instance
    fn get_property(&self, instance_id: u64, name: &str) -> SyscallResult<u64>;
}

/// 对象类型信息
#[derive(Debug)]
pub struct GObjectTypeInfo {
    /// 类型名称
    pub name: String,
    /// 父类型ID（0表示根类型）
    pub parent_type: u64,
    /// 类型大小
    pub type_size: usize,
    /// 类型ID
    pub type_id: u64,
    /// 是否为抽象类型
    pub is_abstract: bool,
    /// 是否为最终类型（不可被继承）
    pub is_final: bool,
    /// 创建时间戳
    pub created_timestamp: u64,
    /// 实例数量
    pub instance_count: AtomicUsize,
}

/// 信号信息
#[derive(Debug)]
pub struct GObjectSignalInfo {
    /// 信号名称
    pub name: String,
    /// 参数类型列表
    pub param_types: Vec<u64>,
    /// 返回值类型
    pub return_type: u64,
    /// 信号ID
    pub signal_id: u64,
    /// 信号标志
    pub flags: u32,
    /// 连接的处理器数量
    pub handler_count: AtomicUsize,
    /// 发射次数统计
    pub emission_count: AtomicUsize,
}

/// 对象实例信息
#[derive(Debug)]
pub struct GObjectInstanceInfo {
    /// 实例ID
    pub instance_id: u64,
    /// 类型ID
    pub type_id: u64,
    /// 引用计数
    pub ref_count: AtomicUsize,
    /// 对象指针
    pub object_ptr: *mut c_void,
    /// 创建时间戳
    pub created_timestamp: u64,
    /// 属性存储
    pub properties: BTreeMap<String, u64>,
    /// 用于确保类型安全的标记数据
    pub _phantom: PhantomData<*mut c_void>,
}

// 手动实现 Send 和 Sync，因为裸指针不是线程安全的
// 但 GObjectInstanceInfo 通过 Mutex 保护访问，所以可以安全地在线程间传递
unsafe impl Send for GObjectInstanceInfo {}
unsafe impl Sync for GObjectInstanceInfo {}

/// 全局对象类型注册表
static OBJECT_TYPES: Mutex<BTreeMap<u64, GObjectTypeInfo>> = Mutex::new(BTreeMap::new());

/// 全局信号注册表
static OBJECT_SIGNALS: Mutex<BTreeMap<u64, Vec<GObjectSignalInfo>>> = Mutex::new(BTreeMap::new());

/// 全局对象实例注册表
static OBJECT_INSTANCES: Mutex<BTreeMap<u64, GObjectInstanceInfo>> = Mutex::new(BTreeMap::new());

/// 下一个可用的类型ID
static NEXT_TYPE_ID: AtomicUsize = AtomicUsize::new(1);

/// 下一个可用的实例ID
static NEXT_INSTANCE_ID: AtomicUsize = AtomicUsize::new(1);

/// 下一个可用的信号ID
static NEXT_SIGNAL_ID: AtomicUsize = AtomicUsize::new(1);

/// GLib对象管理器单例
pub static mut GLIB_OBJECT_MANAGER: () = ();

/// 获取GLib对象管理器引用
pub fn get_glib_object_manager() -> &'static dyn GObjectManager {
    unsafe { &GLIB_OBJECT_MANAGER }
}

pub mod instance;
pub mod manager;
pub mod property;
pub mod signal;
pub mod type_;
