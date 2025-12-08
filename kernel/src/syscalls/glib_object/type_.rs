// Object type management functions

use super::*;
use core::ffi::c_char;

/// 注册新的GLib对象类型
///
/// # 参数
/// * `name` - 类型名称
/// * `parent_type` - 父类型ID
/// * `type_size` - 类型大小
/// * `flags` - 类型标志（位掩码）
///
/// # 返回值
/// * 成功时返回类型ID
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_object_type_register(
    name: *const c_char,
    parent_type: u64,
    type_size: usize,
    flags: u32,
) -> SyscallResult {
    crate::println!("[glib_object] 注册对象类型: parent={}, size={}, flags=0x{:x}",
        parent_type, type_size, flags);

    // 验证参数
    if name.is_null() {
        crate::println!("[glib_object] 类型名称为空");
        return -22; // EINVAL
    }

    if type_size == 0 {
        crate::println!("[glib_object] 类型大小为0");
        return -22; // EINVAL
    }

    // 读取类型名称
    let type_name = unsafe {
        let len = (0..).find(|&i| *name.add(i) == 0).unwrap_or(255);
        core::str::from_utf8(core::slice::from_raw_parts(name as *const u8, len))
            .unwrap_or("invalid")
            .to_string()
    };

    if type_name.is_empty() {
        crate::println!("[glib_object] 类型名称无效");
        return -22; // EINVAL
    }

    // 检查类型名称是否已存在
    {
        let types = OBJECT_TYPES.lock();
        for (_, type_info) in types.iter() {
            if type_info.name == type_name {
                crate::println!("[glib_object] 类型名称已存在: {}", type_name);
                return -17; // EEXIST
            }
        }
    }

    // 检查父类型是否存在（如果不是根类型）
    if parent_type != 0 {
        let types = OBJECT_TYPES.lock();
        if !types.contains_key(&parent_type) {
            crate::println!("[glib_object] 父类型不存在: {}", parent_type);
            return -2; // ENOENT
        }
    }

    // 分配类型ID
    let type_id = NEXT_TYPE_ID.fetch_add(1, Ordering::SeqCst) as u64;
    if type_id == 0 {
        crate::println!("[glib_object] 类型ID溢出");
        return -12; // ENOMEM
    }

    // 解析类型标志
    let is_abstract = (flags & 0x01) != 0;
    let is_final = (flags & 0x02) != 0;

    // 创建类型信息
    let type_info = GObjectTypeInfo {
        name: type_name.clone(),
        parent_type,
        type_size,
        type_id,
        is_abstract,
        is_final,
        created_timestamp: crate::time::get_timestamp() as u64,
        instance_count: AtomicUsize::new(0),
    };

    // 注册类型
    {
        let mut types = OBJECT_TYPES.lock();
        types.insert(type_id, type_info);
    }

    // 初始化信号列表
    {
        let mut signals = OBJECT_SIGNALS.lock();
        signals.insert(type_id, Vec::new());
    }

    crate::println!("[glib_object] 成功注册对象类型: {} (ID={})", type_name, type_id);
    type_id as SyscallResult
}

/// 获取对象类型信息
///
/// # 参数
/// * `type_id` - 类型ID
/// * `info` - 用于存储类型信息的结构体指针
///
/// # 返回值
/// * 成功时返回0
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_object_type_info(
    type_id: u64,
    info: *mut GObjectTypeInfo,
) -> SyscallResult {
    crate::println!("[glib_object] 获取类型信息: {}", type_id);

    // 验证参数
    if type_id == 0 || info.is_null() {
        crate::println!("[glib_object] 无效参数: type={}, info={:p}", type_id, info);
        return -22; // EINVAL
    }

    // 获取类型信息
    let type_info = {
        let types = OBJECT_TYPES.lock();
        match types.get(&type_id) {
            Some(info) => info.clone(),
            None => {
                crate::println!("[glib_object] 对象类型不存在: {}", type_id);
                return -2; // ENOENT
            }
        }
    };

    // 复制信息
    unsafe {
        *info = GObjectTypeInfo {
            name: type_info.name.clone(),
            parent_type: type_info.parent_type,
            type_size: type_info.type_size,
            type_id: type_info.type_id,
            is_abstract: type_info.is_abstract,
            is_final: type_info.is_final,
            created_timestamp: type_info.created_timestamp,
            instance_count: AtomicUsize::new(type_info.instance_count.load(Ordering::SeqCst)),
        };
    }

    crate::println!("[glib_object] 类型信息获取成功: {} ({})", type_info.name, type_id);
    0
}