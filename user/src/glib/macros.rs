/// g_new宏 - 分配并初始化数组
#[macro_export]
macro_rules! g_new {
    ($type:ty, $n:expr) => {
        $crate::glib::g_malloc_n(core::mem::size_of::<$type>() * $n) as *mut $type
    };
}

/// g_new0宏 - 分配、初始化为零的数组
#[macro_export]
macro_rules! g_new0 {
    ($type:ty, $n:expr) => {
        $crate::glib::g_malloc0_n(core::mem::size_of::<$type>() * $n) as *mut $type
    };
}
/// g_renew宏 - 重新分配数组
#[macro_export]
macro_rules! g_renew {
    ($ptr:expr, $type:ty, $n:expr) => {
        $crate::glib::g_realloc_n($ptr as *mut core::ffi::c_void, core::mem::size_of::<$type>() * $n) as *mut $type
    };
}

/// g_free宏 - 释放内存
#[macro_export]
macro_rules! g_free {
    ($ptr:expr) => {
        $crate::glib::g_free($ptr as *mut core::ffi::c_void)
    };
}

/// g_println宏 - 打印消息
#[macro_export]
macro_rules! g_println {
    ($($arg:tt)*) => {
        $crate::glib::g_println(format!($($arg)*))
    };
}

/// g_debug宏 - 调试消息
#[macro_export]
macro_rules! g_debug {
    ($($arg:tt)*) => {
        $crate::glib::g_debug(format!($($arg)*))
    };
}

/// g_info宏 - 信息消息
#[macro_export]
macro_rules! g_info {
    ($($arg:tt)*) => {
        $crate::glib::g_info(format!($($arg)*))
    };
}

/// g_warning宏 - 警告消息
#[macro_export]
macro_rules! g_warning {
    ($($arg:tt)*) => {
        $crate::glib::g_warning(format!($($arg)*))
    }    ($($arg:tt)*) => {
        $crate::glib::g_error(format!($($arg)*))
    };
}

/// g_critical宏 - 严重错误消息
#[macro_export]
macro_rules! g_critical {
    ($($arg:tt)*) => {
        $crate::glib::g_critical(format!($($arg)*))
    };
}

/// g_message宏 - 一般消息
#[macro_export]
macro_rules! g_message {
    ($($arg:tt)*) => {
        $crate::glib::g_message(format!($($arg)*))
    };
}

/// g_assert宏 - 断言
#[macro_export]
macro_rules! g_assert {
    ($($arg:tt)*) => {
        $crate::glib::g_assert(format!($($arg)*))
    };
}

/// g_return_val_if_fail宏 - 条件返回值
#[macro_export]
macro_rules! g_return_val_if_fail {
    ($expr:expr, $val:expr) => {
        if $expr {
            $crate::glib::g_warning("Condition failed, returning value");
            return $val;
        }
    };
}

/// g_return_if_fail宏 - 条件返回
#[macro_export]
macro_rules! g_return_if_fail {
    ($expr:expr) => {
        if $expr {
            $crate::glib::g_warning("Condition failed, returning");
    il宏 - 条件警告
#[macro_export]
macro_rules! g_warn_if_fail {
    ($expr:expr) => {
        if $expr {
            $crate::glib::g_warning("Condition failed");
        }
    };
}

/// G_LIKELY宏 - 分支预测优化
#[macro_export]
macro_rules! G_LIKELY {
    ($expr:expr) => {
        $expr
    };
}

/// G_UNLIKELY宏 - 分支预测优化
#[macro_export]
macro_rules! G_UNLIKELY {
    ($expr:expr) => {
        $expr
    };
}

/// G_INLINE_FUNC宏 - 内联函数
#[macro_export]
macro_rules! G_INLINE_FUNC {
    ($name:expr) => {
        #[inline(always)]
        $name
    };
}