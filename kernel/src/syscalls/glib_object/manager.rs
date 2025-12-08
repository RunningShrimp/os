// GLib object manager trait and implementation

use super::*;

/// GLib对象系统管理器特征
pub trait GObjectManager {
    /// 注册新的对象类型
    fn register_type(&mut self, name: &str, parent_type: u64, size: usize, flags: u32) -> Result<u64, c_int>;

    /// 创建对象实例
    fn create_instance(&mut self, type_id: u64, object_ptr: *mut c_void) -> Result<u64, c_int>;

    /// 增加引用计数
    fn ref_instance(&self, instance_id: u64) -> Result<usize, c_int>;

    /// 减少引用计数
    fn unref_instance(&self, instance_id: u64) -> Result<usize, c_int>;

    /// 注册信号
    fn register_signal(&mut self, type_id: u64, name: &str, param_types: &[u64],
                      return_type: u64, flags: u32) -> Result<u64, c_int>;

    /// 发射信号
    fn emit_signal(&self, instance_id: u64, signal_id: u64, args: &[u64]) -> Result<usize, c_int>;

    /// 设置属性
    fn set_property(&mut self, instance_id: u64, name: &str, value: u64) -> Result<(), c_int>;

    /// 获取属性
    fn get_property(&self, instance_id: u64, name: &str) -> Result<u64, c_int>;
}

impl Default for GObjectManager {
    fn default() -> Self {
        Self
    }
}

impl GObjectManager for () {
    fn register_type(&mut self, name: &str, parent_type: u64, size: usize, flags: u32) -> Result<u64, c_int> {
        let result = super::type_::sys_glib_object_type_register(
            name.as_ptr() as *const core::ffi::c_char,
            parent_type,
            size,
            flags,
        );
        if result > 0 {
            Ok(result as u64)
        } else {
            Err(result)
        }
    }

    fn create_instance(&mut self, type_id: u64, object_ptr: *mut c_void) -> Result<u64, c_int> {
        let result = super::instance::sys_glib_object_instance_create(type_id, object_ptr);
        if result > 0 {
            Ok(result as u64)
        } else {
            Err(result)
        }
    }

    fn ref_instance(&self, instance_id: u64) -> Result<usize, c_int> {
        let result = super::instance::sys_glib_object_ref(instance_id);
        if result > 0 {
            Ok(result as usize)
        } else {
            Err(result)
        }
    }

    fn unref_instance(&self, instance_id: u64) -> Result<usize, c_int> {
        let result = super::instance::sys_glib_object_unref(instance_id);
        if result >= 0 {
            Ok(result as usize)
        } else {
            Err(result)
        }
    }

    fn register_signal(&mut self, type_id: u64, name: &str, param_types: &[u64],
                      return_type: u64, flags: u32) -> Result<u64, c_int> {
        let result = super::signal::sys_glib_object_signal_register(
            type_id,
            name.as_ptr() as *const core::ffi::c_char,
            param_types.as_ptr(),
            param_types.len(),
            return_type,
            flags,
        );
        if result > 0 {
            Ok(result as u64)
        } else {
            Err(result)
        }
    }

    fn emit_signal(&self, instance_id: u64, signal_id: u64, args: &[u64]) -> Result<usize, c_int> {
        let result = super::signal::sys_glib_object_signal_emit(
            instance_id,
            signal_id,
            args.as_ptr(),
            args.len(),
        );
        if result >= 0 {
            Ok(result as usize)
        } else {
            Err(result)
        }
    }

    fn set_property(&mut self, instance_id: u64, name: &str, value: u64) -> Result<(), c_int> {
        let result = super::property::sys_glib_object_set_property(
            instance_id,
            name.as_ptr() as *const core::ffi::c_char,
            value,
        );
        if result == 0 {
            Ok(())
        } else {
            Err(result)
        }
    }

    fn get_property(&self, instance_id: u64, name: &str) -> Result<u64, c_int> {
        let mut value = 0u64;
        let result = super::property::sys_glib_object_get_property(
            instance_id,
            name.as_ptr() as *const core::ffi::c_char,
            &mut value as *mut u64,
        );
        if result == 0 {
            Ok(value)
        } else {
            Err(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_registration() {
        // 测试类型注册
        let type_id = super::type_::sys_glib_object_type_register(
            b"TestObject\0".as_ptr() as *const core::ffi::c_char,
            0, // 无父类型
            128, // 128字节大小
            0,  // 无标志
        );
        assert!(type_id > 0);

        // 清理
        super::property::sys_glib_object_cleanup();
    }

    #[test]
    fn test_instance_creation() {
        // 先注册类型
        let type_id = super::type_::sys_glib_object_type_register(
            b"TestObject\0".as_ptr() as *const core::ffi::c_char,
            0,
            128,
            0,
        );
        assert!(type_id > 0);

        // 创建实例
        let dummy_ptr = 0x1000 as *mut c_void;
        let instance_id = super::instance::sys_glib_object_instance_create(type_id as u64, dummy_ptr);
        assert!(instance_id > 0);

        // 测试引用计数
        let ref_count = super::instance::sys_glib_object_ref(instance_id as u64);
        assert_eq!(ref_count, 2);

        let ref_count = super::instance::sys_glib_object_unref(instance_id as u64);
        assert_eq!(ref_count, 1);

        let ref_count = super::instance::sys_glib_object_unref(instance_id as u64);
        assert_eq!(ref_count, 0); // 对象应该被销毁

        // 清理
        super::property::sys_glib_object_cleanup();
    }

    #[test]
    fn test_signal_registration() {
        // 注册类型
        let type_id = super::type_::sys_glib_object_type_register(
            b"TestObject\0".as_ptr() as *const core::ffi::c_char,
            0,
            128,
            0,
        );
        assert!(type_id > 0);

        // 注册信号
        let signal_id = super::signal::sys_glib_object_signal_register(
            type_id as u64,
            b"test-signal\0".as_ptr() as *const core::ffi::c_char,
            core::ptr::null(),
            0,
            0, // void返回类型
            0, // 无标志
        );
        assert!(signal_id > 0);

        // 清理
        super::property::sys_glib_object_cleanup();
    }

    #[test]
    fn test_properties() {
        // 注册类型
        let type_id = super::type_::sys_glib_object_type_register(
            b"TestObject\0".as_ptr() as *const core::ffi::c_char,
            0,
            128,
            0,
        );
        assert!(type_id > 0);

        // 创建实例
        let dummy_ptr = 0x1000 as *mut c_void;
        let instance_id = super::instance::sys_glib_object_instance_create(type_id as u64, dummy_ptr);
        assert!(instance_id > 0);

        // 设置属性
        let result = super::property::sys_glib_object_set_property(
            instance_id as u64,
            b"test-property\0".as_ptr() as *const core::ffi::c_char,
            42,
        );
        assert_eq!(result, 0);

        // 获取属性
        let mut value = 0u64;
        let result = super::property::sys_glib_object_get_property(
            instance_id as u64,
            b"test-property\0".as_ptr() as *const core::ffi::c_char,
            &mut value as *mut u64,
        );
        assert_eq!(result, 0);
        assert_eq!(value, 42);

        // 清理
        super::property::sys_glib_object_cleanup();
    }
}