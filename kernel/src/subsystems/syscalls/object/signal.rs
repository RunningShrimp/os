// Object signal management functions

use super::*;
use core::ptr;

/// 注册对象信号
///
/// # 参数
/// * `type_id` - 类型ID
/// * `name` - 信号名称
/// * `param_types` - 参数类型数组
/// * `param_count` - 参数数量
/// * `return_type` - 返回值类型
/// * `flags` - 信号标志
///
/// # 返回值
/// * 成功时返回信号ID
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_object_signal_register(
    type_id: u64,
    name: *const c_char,
    param_types: *const u64,
    param_count: usize,
    return_type: u64,
    flags: u32,
) -> SyscallResult {
    crate::println!("[glib_object] 注册信号: type={}, params={}, return={}, flags=0x{:x}",
        type_id, param_count, return_type, flags);

    // 验证参数
    if type_id == 0 || name.is_null() {
        crate::println!("[glib_object] 无效参数: type={}, name={:p}", type_id, name);
        return -22; // EINVAL
    }

    // 读取信号名称
    let signal_name = unsafe {
        let len = (0..).find(|&i| *name.add(i) == 0).unwrap_or(255);
        core::str::from_utf8(core::slice::from_raw_parts(name as *const u8, len))
            .unwrap_or("invalid")
            .to_string()
    };

    if signal_name.is_empty() {
        crate::println!("[glib_object] 信号名称无效");
        return -22; // EINVAL
    }

    // 检查类型是否存在
    {
        let types = OBJECT_TYPES.lock();
        if !types.contains_key(&type_id) {
            crate::println!("[glib_object] 对象类型不存在: {}", type_id);
            return -2; // ENOENT
        }
    }

    // 读取参数类型
    let mut params = Vec::new();
    if param_count > 0 && !param_types.is_null() {
        for i in 0..param_count {
            let param_type = unsafe { *param_types.add(i) };
            params.push(param_type);
        }
    }

    // 分配信号ID
    let signal_id = NEXT_SIGNAL_ID.fetch_add(1, Ordering::SeqCst) as u64;
    if signal_id == 0 {
        crate::println!("[glib_object] 信号ID溢出");
        return -12; // ENOMEM
    }

    // 创建信号信息
    let signal_info = GObjectSignalInfo {
        name: signal_name.clone(),
        param_types: params.clone(),
        return_type,
        signal_id,
        flags,
        handler_count: AtomicUsize::new(0),
        emission_count: AtomicUsize::new(0),
    };

    // 注册信号
    {
        let mut signals = OBJECT_SIGNALS.lock();
        if let Some(signal_list) = signals.get_mut(&type_id) {
            // 检查信号名称是否已存在
            for existing_signal in signal_list.iter() {
                if existing_signal.name == signal_name {
                    crate::println!("[glib_object] 信号名称已存在: {}", signal_name);
                    return -17; // EEXIST
                }
            }
            signal_list.push(signal_info);
        } else {
            crate::println!("[glib_object] 信号列表不存在: type={}", type_id);
            return -2; // ENOENT
        }
    }

    crate::println!("[glib_object] 成功注册信号: {} (ID={}, Type={})",
        signal_name, signal_id, type_id);
    signal_id as SyscallResult
}

/// 发射对象信号
///
/// # 参数
/// * `instance_id` - 实例ID
/// * `signal_id` - 信号ID
/// * `args` - 参数数组
/// * `arg_count` - 参数数量
///
/// # 返回值
/// * 成功时返回处理器数量
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_object_signal_emit(
    instance_id: u64,
    signal_id: u64,
    args: *const u64,
    arg_count: usize,
) -> SyscallResult {
    crate::println!("[glib_object] 发射信号: instance={}, signal={}, args={}",
        instance_id, signal_id, arg_count);

    // 验证参数
    if instance_id == 0 || signal_id == 0 {
        crate::println!("[glib_object] 无效参数: instance={}, signal={}", instance_id, signal_id);
        return -22; // EINVAL
    }

    // 获取实例信息
    let instance_info = {
        let instances = OBJECT_INSTANCES.lock();
        match instances.get(&instance_id) {
            Some(info) => info.clone(),
            None => {
                crate::println!("[glib_object] 对象实例不存在: {}", instance_id);
                return -2; // ENOENT
            }
        }
    };

    // 查找信号信息
    let handler_count = {
        let signals = OBJECT_SIGNALS.lock();
        if let Some(signal_list) = signals.get(&instance_info.type_id) {
            let mut found_handlers = 0;
            for signal in signal_list.iter() {
                if signal.signal_id == signal_id {
                    found_handlers = signal.handler_count.load(Ordering::SeqCst);
                    signal.emission_count.fetch_add(1, Ordering::SeqCst);
                    break;
                }
            }
            found_handlers
        } else {
            crate::println!("[glib_object] 类型信号列表不存在: type={}", instance_info.type_id);
            return -2; // ENOENT
        }
    };

    crate::println!("[glib_object] 信号发射完成: instance={}, signal={}, handlers={}",
        instance_id, signal_id, handler_count);

    // 实际的信号调用由用户空间GLib处理，这里只做统计
    handler_count as SyscallResult
}