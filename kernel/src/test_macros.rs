//! Test assertion macros for kernel testing

#[macro_export]
macro_rules! test_assert {
    ($cond:expr) => {
        if !($cond) {
            crate::println!("[kernel_test] ASSERTION FAILED at {}:{}: {}", file!(), line!(), stringify!($cond));
            return false;
        }
    };
    ($cond:expr, $msg:expr) => {
        if !($cond) {
            crate::println!("[kernel_test] ASSERTION FAILED: {}", $msg);
            return false;
        }
    };
}

#[macro_export]
macro_rules! test_assert_eq {
    ($left:expr, $right:expr) => {
        if ($left) != ($right) {
            crate::println!("[kernel_test] ASSERTION FAILED: {} != {} (expected {})",
                           $left, $right, $right);
            return false;
        }
    };
    ($left:expr, $right:expr, $msg:expr) => {
        if ($left) != ($right) {
            crate::println!("[kernel_test] ASSERTION FAILED: {} != {} (expected {}): {}",
                           $left, $right, $right, $msg);
            return false;
        }
    };
}

#[macro_export]
macro_rules! test_assert_ne {
    ($left:expr, $right:expr) => {
        if ($left) == ($right) {
            crate::println!("[kernel_test] ASSERTION FAILED: {} == {} (should be different)",
                           $left, $right);
            return false;
        }
    };
    ($left:expr, $right:expr, $msg:expr) => {
        if ($left) == ($right) {
            crate::println!("[kernel_test] ASSERTION FAILED: {} == {} (should be different): {}",
                           $left, $right, $msg);
            return false;
        }
    };
}