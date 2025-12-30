// GLib object manager trait and implementation

use crate::subsystems::syscalls::interface::{SyscallResult, SyscallError};
use core::ffi::c_void;

/// Convert i32 error code to SyscallError
fn convert_error(result: i32) -> SyscallError {
    match result {
        _ => SyscallError::InvalidArgument,
    }
}

// Import the trait from parent module
use crate::subsystems::syscalls::object::GObjectManager;

// impl Default for GObjectManager {
//     fn default() -> Self {
//         Self
//     }
// }

impl GObjectManager for () {
    fn register_type(
        &mut self,
        name: &str,
        parent_type: u64,
        size: usize,
        flags: u32,
    ) -> SyscallResult<u64> {
        let result = super::type_::sys_glib_object_type_register(
            name.as_ptr() as *const core::ffi::c_char,
            parent_type,
            size,
            flags,
        );
        if result > 0 {
            Ok(result as u64)
        } else {
            Err(SyscallError::InvalidArgument)
        }
    }

    fn create_instance(&mut self, type_id: u64, object_ptr: *mut c_void) -> SyscallResult<u64> {
        let result = unsafe { super::instance::sys_glib_object_instance_create(type_id, object_ptr) };
        if result > 0 {
            Ok(result as u64)
        } else {
            Err(SyscallError::InvalidArgument)
        }
    }

    fn ref_instance(&self, instance_id: u64) -> SyscallResult<usize> {
        let result = unsafe { super::instance::sys_glib_object_ref(instance_id) };
        if result > 0 {
            Ok(result as usize)
        } else {
            Err(SyscallError::InvalidArgument)
        }
    }

    fn unref_instance(&self, instance_id: u64) -> SyscallResult<usize> {
        let result = unsafe { super::instance::sys_glib_object_unref(instance_id) };
        if result >= 0 {
            Ok(result as usize)
        } else {
            Err(SyscallError::InvalidArgument)
        }
    }

    fn register_signal(
        &mut self,
        type_id: u64,
        name: &str,
        param_types: &[u64],
        return_type: u64,
        flags: u32,
    ) -> SyscallResult<u64> {
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
            Err(SyscallError::InvalidArgument)
        }
    }

      fn emit_signal(&self, instance_id: u64, signal_id: u64, args: &[u64]) -> SyscallResult<usize> {
        let result = super::signal::sys_glib_object_signal_emit(
            instance_id,
            signal_id,
            args.as_ptr(),
            args.len(),
        );
        if result >= 0 {
            Ok(result as usize)
        } else {
            Err(SyscallError::InvalidArgument)
        }
    }

    fn set_property(&mut self, instance_id: u64, name: &str, value: u64) -> SyscallResult<()> {
        let result = unsafe {
            super::property::sys_glib_object_set_property(
                instance_id,
                name.as_ptr() as *const core::ffi::c_char,
                value,
            )
        };
        if result == 0 {
            Ok(())
        } else {
            Err(SyscallError::InvalidArgument)
        }
    }

    fn get_property(&self, instance_id: u64, name: &str) -> SyscallResult<u64> {
        let mut value = 0u64;
        let result = unsafe {
            super::property::sys_glib_object_get_property(
                instance_id,
                name.as_ptr() as *const core::ffi::c_char,
                &mut value as *mut u64,
            )
        };
        if result == 0 {
            Ok(value)
        } else {
            Err(SyscallError::InvalidArgument)
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
            0,   // 无父类型
            128, // 128字节大小
            0,   // 无标志
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
        let instance_id =
            super::instance::sys_glib_object_instance_create(type_id as u64, dummy_ptr);
        assert!(instance_id > 0);

        // 测试引用计数
        let ref_count = super::instance::sys_glib_object_ref(instance_id as u64);
        assert!(ref_count > 0);
        assert_eq!(ref_count, 2);

        let ref_count = super::instance::sys_glib_object_unref(instance_id as u64);
        assert!(ref_count >= 0);
        assert_eq!(ref_count, 1);

        let ref_count = super::instance::sys_glib_object_unref(instance_id as u64);
        assert!(ref_count >= 0);
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
        let instance_id =
            super::instance::sys_glib_object_instance_create(type_id as u64, dummy_ptr);
        assert!(instance_id > 0);

        // 设置属性
        let result = super::property::sys_glib_object_set_property(
            instance_id as u64,
            b"test-property\0".as_ptr() as *const core::ffi::c_char,
            42,
        );
        if result != 0 {
            panic!("Expected result 0, got {}", result);
        }

        // 获取属性
        let mut value = 0u64;
        let result = super::property::sys_glib_object_get_property(
            instance_id as u64,
            b"test-property\0".as_ptr() as *const core::ffi::c_char,
            &mut value as *mut u64,
        );
        if result == 0 {
            assert_eq!(value, 42);
        } else {
            panic!("Expected success, got error: {}", result);
        }

        // 清理
        super::property::sys_glib_object_cleanup();
    }
}
