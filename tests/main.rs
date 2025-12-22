//! NOS Test Suite Runner
//!
//! This binary runs the unified test suite for the NOS operating system.

use nos_tests::{run_all_tests, TestResult};

fn main() {
    println!("NOS Test Suite Runner");
    println!("====================");
    
    let result = run_all_tests();
    
    if result.all_passed() {
        println!("\nAll tests passed! 🎉");
        std::process::exit(0);
    } else {
        println!("\nSome tests failed. ❌");
        std::process::exit(1);
    }
}