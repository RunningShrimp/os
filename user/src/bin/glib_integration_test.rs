//! GLib integration test program
//!
//! 测试GLib用户空间库的基本集成和功能

#![no_std]
#![no_main]

extern crate alloc;
use user::glib::*;
use user::println;

#[no_mangle]
pub extern "C" fn main() -> i32 {
    println!("=== GLib Integration Test ===");

    // 测试GLib初始化
    match glib::init() {
        Ok(()) => {
            println!("✓ GLib initialization successful");
        }
        Err(e) => {
            println!("✗ GLib initialization failed: {:?}", e);
            return 1;
        }
    }

    // 测试基本内存分配
    test_basic_memory();

    // 测试字符串操作
    test_string_operations();

    // 测试数据结构
    test_data_structures();

    // 清理GLib
    glib::cleanup();

    println!("=== GLib Integration Test Complete ===");
    0
}

fn test_basic_memory() {
    println!("\n--- Testing Basic Memory Operations ---");

    // 测试g_malloc
    let ptr = g_malloc(100);
    assert!(!ptr.is_null(), "g_malloc failed");
    println!("✓ g_malloc successful");

    // 测试g_malloc0
    let zero_ptr = g_malloc0(50);
    assert!(!zero_ptr.is_null(), "g_malloc0 failed");
    println!("✓ g_malloc0 successful");

    // 测试g_realloc
    let realloc_ptr = g_realloc(ptr, 200);
    assert!(!realloc_ptr.is_null(), "g_realloc failed");
    println!("✓ g_realloc successful");

    // 释放内存
    g_free(realloc_ptr);
    g_free(zero_ptr);
    println!("✓ g_free successful");
}

fn test_string_operations() {
    println!("\n--- Testing String Operations ---");

    // 测试字符串复制
    let test_str = "Hello, NOS GLib!";
    let copied = g_strdup(test_str.as_ptr() as *const i8);
    assert!(!copied.is_null(), "g_strdup failed");

    unsafe {
        let len = strlen(copied as *const u8);
        assert_eq!(len, test_str.len(), "String length mismatch");
    }

    g_free(copied as gpointer);
    println!("✓ String operations successful");
}

fn test_data_structures() {
    println!("\n--- Testing Basic Data Structures ---");

    // 测试指针数组
    let array = GPtrArray::new();
    assert!(!array.is_null(), "GPtrArray::new failed");

    let index1 = GPtrArray::add(array, 1 as gpointer);
    let index2 = GPtrArray::add(array, 2 as gpointer);

    assert_eq!(index1, 0, "First index should be 0");
    assert_eq!(index2, 1, "Second index should be 1");
    assert_eq!(GPtrArray::get_len(array), 2, "Array length should be 2");

    GPtrArray::free(array);
    println!("✓ Data structure operations successful");
}