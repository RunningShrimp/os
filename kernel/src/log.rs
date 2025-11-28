use core::sync::atomic::{AtomicUsize, Ordering};

pub const LOG_ERROR: usize = 1;
pub const LOG_WARN: usize = 2;
pub const LOG_INFO: usize = 3;
pub const LOG_DEBUG: usize = 4;

static LOG_LEVEL: AtomicUsize = AtomicUsize::new(LOG_INFO);

pub fn set_level(level: usize) { LOG_LEVEL.store(level, Ordering::SeqCst); }
pub fn level() -> usize { LOG_LEVEL.load(Ordering::SeqCst) }

#[macro_export]
macro_rules! log_error { ($($arg:tt)*) => { if $crate::log::level() >= $crate::log::LOG_ERROR { $crate::println!("[E] {}", core::format_args!($($arg)*)) } } }
#[macro_export]
macro_rules! log_warn  { ($($arg:tt)*) => { if $crate::log::level() >= $crate::log::LOG_WARN  { $crate::println!("[W] {}", core::format_args!($($arg)*)) } } }
#[macro_export]
macro_rules! log_info  { ($($arg:tt)*) => { if $crate::log::level() >= $crate::log::LOG_INFO  { $crate::println!("[I] {}", core::format_args!($($arg)*)) } } }
#[macro_export]
macro_rules! log_debug { ($($arg:tt)*) => { if $crate::log::level() >= $crate::log::LOG_DEBUG { $crate::println!("[D] {}", core::format_args!($($arg)*)) } } }

