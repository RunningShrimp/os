// Object instance management functions

use super::*;

/// 创建对象实例
///
/// # 参数
/// * `type_id` - 类型ID
/// * `object_ptr` - 对象指针
///
/// # 返回值
/// * 成功时返回实例ID
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_object_instance_create(
    type_id: u64,
    object_ptr: *mut c_void,
) -> SyscallResult {
    crate::println!("[glib_object] 创建对象实例: type={}, ptr={:p}", type_id, object_ptr);

    // 验证参数
    if type_id == 0 || object_ptr.is_null() {
        crate::println!("[glib_object] 无效参数: type={}, ptr={:p}", type_id, object_ptr);
        return -22; // EINVAL
    }

    // 检查类型是否存在
    let type_info = {
        let types = OBJECT_TYPES.lock();
        match types.get(&type_id) {
            Some(info) => {
                if info.is_abstract {
                    crate::println!("[glib_object] 不能创建抽象类型的实例: {}", type_id);
                    return -22; // EINVAL
                }
                info.clone()
            }
            None => {
                crate::println!("[glib_object] 对象类型不存在: {}", type_id);
                return -2; // ENOENT
            }
        }
    };

    // 分配实例ID
    let instance_id = NEXT_INSTANCE_ID.fetch_add(1, Ordering::SeqCst) as u64;
    if instance_id == 0 {
        crate::println!("[glib_object] 实例ID溢出");
        return -12; // ENOMEM
    }

    // 创建实例信息
    let instance_info = GObjectInstanceInfo {
        instance_id,
        type_id,
        ref_count: AtomicUsize::new(1), // 初始引用计数为1
        object_ptr,
        created_timestamp: crate::subsystems::time::get_timestamp() as u64,
        properties: BTreeMap::new(),
    };

    // 注册实例
    {
        let mut instances = OBJECT_INSTANCES.lock();
        instances.insert(instance_id, instance_info);
    }

    // 更新类型实例计数
    {
        let types = OBJECT_TYPES.lock();
        if let Some(type_info) = types.get(&type_id) {
            type_info.instance_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    crate::println!("[glib_object] 成功创建对象实例: ID={}, Type={}",
        instance_id, type_info.name);
    instance_id as SyscallResult
}

/// 增加对象引用计数
///
/// # 参数
/// * `instance_id` - 实例ID
///
/// # 返回值
/// * 成功时返回新的引用计数
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_object_ref(instance_id: u64) -> SyscallResult {
    crate::println!("[glib_object] 增加引用: instance={}", instance_id);

    // 验证参数
    if instance_id == 0 {
        crate::println!("[glib_object] 无效的实例ID: {}", instance_id);
        return -22; // EINVAL
    }

    // 获取实例并增加引用计数
    let new_ref_count = {
        let instances = OBJECT_INSTANCES.lock();
        match instances.get(&instance_id) {
            Some(instance_info) => {
                instance_info.ref_count.fetch_add(1, Ordering::SeqCst) + 1
            }
            None => {
                crate::println!("[glib_object] 对象实例不存在: {}", instance_id);
                return -2; // ENOENT
            }
        }
    };

    crate::println!("[glib_object] 引用计数增加: instance={}, new_count={}",
        instance_id, new_ref_count);
    new_ref_count as SyscallResult
}

/// 减少对象引用计数
///
/// # 参数
/// * `instance_id` - 实例ID
///
/// # 返回值
/// * 成功时返回新的引用计数
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_object_unref(instance_id: u64) -> SyscallResult {
    crate::println!("[glib_object] 减少引用: instance={}", instance_id);

    // 验证参数
    if instance_id == 0 {
        crate::println!("[glib_object] 无效的实例ID: {}", instance_id);
        return -22; // EINVAL
    }

    // 获取实例并减少引用计数
    let (new_ref_count, should_destroy) = {
        let instances = OBJECT_INSTANCES.lock();
        match instances.get(&instance_id) {
            Some(instance_info) => {
                let old_count = instance_info.ref_count.load(Ordering::SeqCst);
                if old_count == 0 {
                    crate::println!("[glib_object] 引用计数已经为0: instance={}", instance_id);
                    return -22; // EINVAL
                }
                let new_count = old_count - 1;
                instance_info.ref_count.store(new_count, Ordering::SeqCst);
                (new_count, new_count == 0)
            }
            None => {
                crate::println!("[glib_object] 对象实例不存在: {}", instance_id);
                return -2; // ENOENT
            }
        }
    };

    // 如果引用计数为0，销毁对象
    if should_destroy {
        // 更新类型实例计数
        let type_id = {
            let instances = OBJECT_INSTANCES.lock();
            if let Some(instance) = instances.get(&instance_id) {
                instance.type_id
            } else {
                0
            }
        };

        if type_id != 0 {
            let types = OBJECT_TYPES.lock();
            if let Some(type_info) = types.get(&type_id) {
                type_info.instance_count.fetch_sub(1, Ordering::SeqCst);
            }
        }

        // 移除实例
        {
            let mut instances = OBJECT_INSTANCES.lock();
            instances.remove(&instance_id);
        }

        crate::println!("[glib_object] 对象已销毁: instance={}", instance_id);
    } else {
        crate::println!("[glib_object] 引用计数减少: instance={}, new_count={}",
            instance_id, new_ref_count);
    }

    new_ref_count as SyscallResult
}