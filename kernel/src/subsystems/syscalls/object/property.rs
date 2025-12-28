// Object property management functions

use super::*;
use core::ffi::c_char;

/// 设置对象属性
///
/// # 参数
/// * `instance_id` - 实例ID
/// * `name` - 属性名称
/// * `value` - 属性值
///
/// # 返回值
/// * 成功时返回0
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_object_set_property(
    instance_id: u64,
    name: *const c_char,
    value: u64,
) -> SyscallResult {
    crate::println!("[glib_object] 设置属性: instance={}, name={}, value={}",
        instance_id, unsafe {
            if name.is_null() {
                "null".to_string()
            } else {
                let len = (0..).find(|&i| *name.add(i) == 0).unwrap_or(255);
                core::str::from_utf8(core::slice::from_raw_parts(name as *const u8, len))
                    .unwrap_or("invalid").to_string()
            }
        }, value);

    // 验证参数
    if instance_id == 0 || name.is_null() {
        crate::println!("[glib_object] 无效参数: instance={}, name={:p}", instance_id, name);
        return -22; // EINVAL
    }

    // 读取属性名称
    let property_name = unsafe {
        let len = (0..).find(|&i| *name.add(i) == 0).unwrap_or(255);
        core::str::from_utf8(core::slice::from_raw_parts(name as *const u8, len))
            .unwrap_or("invalid")
            .to_string()
    };

    if property_name.is_empty() {
        crate::println!("[glib_object] 属性名称无效");
        return -22; // EINVAL
    }

    // 设置属性
    {
        let mut instances = OBJECT_INSTANCES.lock();
        match instances.get_mut(&instance_id) {
            Some(instance_info) => {
                instance_info.properties.insert(property_name, value);
            }
            None => {
                crate::println!("[glib_object] 对象实例不存在: {}", instance_id);
                return -2; // ENOENT
            }
        }
    }

    crate::println!("[glib_object] 属性设置成功: instance={}, property={}",
        instance_id, property_name);
    0
}

/// 获取对象属性
///
/// # 参数
/// * `instance_id` - 实例ID
/// * `name` - 属性名称
/// * `value` - 用于存储属性值的指针
///
/// # 返回值
/// * 成功时返回0
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_object_get_property(
    instance_id: u64,
    name: *const c_char,
    value: *mut u64,
) -> SyscallResult {
    crate::println!("[glib_object] 获取属性: instance={}", instance_id);

    // 验证参数
    if instance_id == 0 || name.is_null() || value.is_null() {
        crate::println!("[glib_object] 无效参数: instance={}, name={:p}, value={:p}",
            instance_id, name, value);
        return -22; // EINVAL
    }

    // 读取属性名称
    let property_name = unsafe {
        let len = (0..).find(|&i| *name.add(i) == 0).unwrap_or(255);
        core::str::from_utf8(core::slice::from_raw_parts(name as *const u8, len))
            .unwrap_or("invalid")
            .to_string()
    };

    if property_name.is_empty() {
        crate::println!("[glib_object] 属性名称无效");
        return -22; // EINVAL
    }
/// 清理所有GLib对象（用于调试）
#[no_mangle]
pub extern "C" fn sys_glib_object_cleanup() -> c_int {
    crate::println!("[glib_object] 清理所有GLib对象");

    let mut total_types = 0;
    let mut total_instances = 0;
    let mut total_signals = 0;
    let mut leaked_instances = 0;

    {
        let types = OBJECT_TYPES.lock();
        total_types = types.len();

        for (_, type_info) in types.iter() {
            let instance_count = type_info.instance_count.load(Ordering::SeqCst);
            total_instances += instance_count;
            if instance_count > 0 {
                leaked_instances += instance_count;
                crate::println!("[glib_object] 泄漏警告: 类型 {} 仍有 {} 个实例",
                    type_info.name, instance_count);
            }
        }
    }

    {
        let signals = OBJECT_SIGNALS.lock();
        for (_, signal_list) in signals.iter() {
            total_signals += signal_list.len();
        }
    }

    // 清理注册表
    {
        let mut types = OBJECT_TYPES.lock();
        types.clear();
    }

    {
        let mut instances = OBJECT_INSTANCES.lock();
        instances.clear();
    }

    {
        let mut signals = OBJECT_SIGNALS.lock();
        signals.clear();
    }

    // 重置ID计数器
    NEXT_TYPE_ID.store(1, Ordering::SeqCst);
    NEXT_INSTANCE_ID.store(1, Ordering::SeqCst);
    NEXT_SIGNAL_ID.store(1, Ordering::SeqCst);

    crate::println!("[glib_object] 清理完成: {} 个类型, {} 个信号, {} 个实例 ({} 个泄漏)",
        total_types, total_signals, total_instances, leaked_instances);

    0
}

    // 获取属性
    let property_value = {
        let instances = OBJECT_INSTANCES.lock();
        match instances.get(&instance_id) {
            Some(instance_info) => {
                match instance_info.properties.get(&property_name) {
                    Some(val) => *val,
                    None => {
                        crate::println!("[glib_object] 属性不存在: instance={}, property={}",
                            instance_id, property_name);
                        return -2; // ENOENT
                    }
                }
            }
            None => {
                crate::println!("[glib_object] 对象实例不存在: {}", instance_id);
                return -2; // ENOENT
            }
        }
    };

    unsafe {
        *value = property_value;
    }

    crate::println!("[glib_object] 属性获取成功: instance={}, property={}, value={}",
        instance_id, property_name, property_value);
    0
}