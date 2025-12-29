//! Test utilities and types for kernel testing

extern crate alloc;

/// Test result type
pub type TestResult = Result<(), alloc::string::String>;

/// Skip a test with a reason
#[macro_export]
macro_rules! skip_test {
    ($reason:expr) => {
        return Err(alloc::format!("SKIP: {}", $reason));
    };
}

/// Test assertion macro
#[macro_export]
macro_rules! test_assert {
    ($cond:expr) => {
        if !($cond) {
            return Err(alloc::format!("Assertion failed: {}", stringify!($cond)));
        }
    };
    ($cond:expr, $msg:expr) => {
        if !($cond) {
            return Err(alloc::format!("Assertion failed: {} - {}", stringify!($cond), $msg));
        }
    };
}

/// Test equality assertion macro
#[macro_export]
macro_rules! test_assert_eq {
    ($left:expr, $right:expr) => {
        if ($left) != ($right) {
            return Err(alloc::format!(
                "Assertion failed: {} == {} (left: {:?}, right: {:?})",
                stringify!($left),
                stringify!($right),
                $left,
                $right
            ));
        }
    };
    ($left:expr, $right:expr, $msg:expr) => {
        if ($left) != ($right) {
            return Err(alloc::format!(
                "Assertion failed: {} == {} - {} (left: {:?}, right: {:?})",
                stringify!($left),
                stringify!($right),
                $msg,
                $left,
                $right
            ));
        }
    };
}

/// Test suite for organizing tests
pub struct TestSuite {
    pub name: alloc::string::String,
    pub tests: alloc::vec::Vec<(&'static str, fn() -> TestResult)>,
}

impl TestSuite {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tests: alloc::vec::Vec::new(),
        }
    }

    pub fn add_test(&mut self, name: &'static str, test_fn: fn() -> TestResult) {
        self.tests.push((name, test_fn));
    }

    pub fn run(&self) {
        crate::println!("=== Running test suite: {} ===", self.name);
        let mut passed = 0;
        let mut failed = 0;

        for (name, test_fn) in &self.tests {
            crate::println!("Running test: {}...", name);
            match test_fn() {
                Ok(()) => {
                    crate::println!("  PASSED");
                    passed += 1;
                }
                Err(e) => {
                    crate::println!("  FAILED: {}", e);
                    failed += 1;
                }
            }
        }

        crate::println!("=== Results: {} passed, {} failed ===", passed, failed);
    }
}

/// Assert equality macro
#[macro_export]
macro_rules! assert_eq {
    ($left:expr, $right:expr) => {
        if ($left) != ($right) {
            panic!("assertion failed: {} == {} (left: {:?}, right: {:?})",
                stringify!($left), stringify!($right), $left, $right);
        }
    };
}

/// Assert inequality macro
#[macro_export]
macro_rules! assert_ne {
    ($left:expr, $right:expr) => {
        if ($left) == ($right) {
            panic!("assertion failed: {} != {} (both are {:?})",
                stringify!($left), stringify!($right), $left);
        }
    };
}

/// Assert true macro
#[macro_export]
macro_rules! assert_true {
    ($cond:expr) => {
        if !($cond) {
            panic!("assertion failed: {} is not true", stringify!($cond));
        }
    };
}
