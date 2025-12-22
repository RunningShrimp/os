//! Formatting utilities for no-alloc environment

#[cfg(feature = "alloc")]
pub use alloc::format;

#[cfg(not(feature = "alloc"))]
/// Simple format macro for no-alloc environment
/// Only supports basic string concatenation for now
#[macro_export]
macro_rules! format {
    ($($arg:tt)*) => {
        // In a real implementation, this would use a fixed-size buffer
        // or a custom formatting solution
        concat!($($arg)*)
    };
}

#[cfg(not(feature = "alloc"))]
/// Extension trait to add to_string method for &str in no-alloc environment
pub trait ToStringExt {
    fn to_string(&self) -> &'static str;
}

// 移除&str的实现，因为我们无法保证它具有'static生命周期

#[cfg(not(feature = "alloc"))]
impl ToStringExt for &'static str {
    fn to_string(&self) -> &'static str {
        *self
    }
}

#[cfg(not(feature = "alloc"))]
/// Helper trait to convert string literals to String type in no-alloc environment
pub trait IntoString {
    fn into_string(&self) -> &'static str;
}

#[cfg(not(feature = "alloc"))]
impl IntoString for &'static str {
    fn into_string(&self) -> &'static str {
        *self
    }
}

// 移除&str的实现，因为我们无法保证它具有'static生命周期

#[cfg(not(feature = "alloc"))]
/// String type for no-alloc environment
pub struct String {
    // Simple placeholder implementation for no-alloc environment
    _phantom: core::marker::PhantomData<()>,
}

#[cfg(not(feature = "alloc"))]
impl String {
    pub fn new() -> Self {
        Self {
            _phantom: core::marker::PhantomData,
        }
    }
}

#[cfg(not(feature = "alloc"))]
impl Default for String {
    fn default() -> Self {
        Self::new()
    }
}