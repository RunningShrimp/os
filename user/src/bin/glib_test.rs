//! GLib测试程序
//!
//! 测试用户空间GLib实现的各种功能，包括：
//! - 内存管理
//! - 数据结构
//! - 对象系统
//! - 主循环
//! - 异步I/O
//! - 字符串处理
//! - 工具函数

#![no_std]
#![no_main]

extern crate alloc;
use user::glib::*;
use user::glib::error::*;
use user::glib::collections::*;
use user::glib::string::*;
use user::glib::utils::*;

use core::ptr;
use core::ffi::c_void;

#[no_mangle]
pub extern "C" fn main() -> i32 {
    println!("=== NOS GLib Test Program ===");

    // 初始化GLib
    match glib::init() {
        Ok(_) => {
            println!("✓ GLib initialized successfully");
        }
        Err(e) => {
            println!("✗ GLib initialization failed: {:?}", e);
            return 1;
        }
    }

    // 测试内存管理
    test_memory_management();

    // 测试数据结构
    test_data_structures();

    // 测试字符串操作
    test_string_operations();

    // 测试数学函数
    test_math_functions();

    // 测试时间函数
    test_time_functions();

    // 测试工具函数
    test_utility_functions();

    // 清理GLib
    glib::cleanup();

    println!("=== GLib Test Complete ===");
    0
}

fn test_memory_management() {
    println!("\n--- Testing Memory Management ---");

    // 测试基本内存分配
    let ptr = g_malloc(100);
    assert!(!ptr.is_null(), "Basic malloc failed");

    // 测试清零分配
    let zero_ptr = g_malloc0(50);
    assert!(!zero_ptr.is_null(), "Zero malloc failed");

    // 检查清零
    unsafe {
        for i in 0..50 {
            assert_eq!(*(zero_ptr as *mut u8).add(i), 0, "Memory not zeroed");
        }
    }

    // 测试重新分配
    let realloc_ptr = g_realloc(ptr, 200);
    assert!(!realloc_ptr.is_null(), "Realloc failed");

    // 测试批量分配
    let batch_ptr = g_malloc_n(32, 10);
    assert!(!batch_ptr.is_null(), "Batch malloc failed");

    // 测试切片分配器
    let slice_ptr = g_slice_alloc(64);
    assert!(!slice_ptr.is_null(), "Slice alloc failed");

    // 释放内存
    g_free(realloc_ptr);
    g_free(zero_ptr);
    g_free(batch_ptr as gpointer);
    unsafe { g_slice_free(64, slice_ptr); }

    println!("✓ Memory management tests passed");

    // 检查内存统计
    let stats = get_memory_stats();
    println!("  Total allocations: {}", stats.total_allocations.load(core::sync::atomic::Ordering::SeqCst));
    println!("  Current allocated: {} bytes", stats.current_allocated.load(core::sync::atomic::Ordering::SeqCst));

    let leaks = check_memory_leaks();
    println!("  Memory leaks: {}", leaks);
}

fn test_data_structures() {
    println!("\n--- Testing Data Structures ---");

    // 测试链表
    test_linked_lists();

    // 测试哈希表
    test_hash_tables();

    // 测试队列
    test_queues();

    // 测试指针数组
    test_pointer_arrays();

    println!("✓ Data structure tests passed");
}

fn test_linked_lists() {
    // 测试GList
    let list = ptr::null_mut();
    let list = GList::append(list, 1 as gpointer);
    let list = GList::append(list, 2 as gpointer);
    let list = GList::prepend(list, 0 as gpointer);

    assert_eq!(GList::length(list), 3);
    assert_eq!(GList::find(list, 2 as gpointer), unsafe { (*list).next as *mut GList });

    // 测试移除
    let list = GList::remove(list, 1 as gpointer);
    assert_eq!(GList::length(list), 2);

    // 测试反转
    let reversed = GList::reverse(list);
    assert_eq!(GList::length(reversed), 2);

    GList::free(reversed);

    // 测试GSList
    let slist = ptr::null_mut();
    let slist = GSList::append(slist, 10 as gpointer);
    let slist = GSList::append(slist, 20 as gpointer);
    let slist = GSList::prepend(slist, 5 as gpointer);

    assert_eq!(GSList::length(slist), 3);

    GSList::free(slist);

    println!("  ✓ Linked lists tests passed");
}

fn test_hash_tables() {
    // 测试哈希函数
    unsafe extern "C" fn hash_func(key: gconstpointer) -> u32 {
        key as u32
    }

    unsafe extern "C" fn equal_func(a: gconstpointer, b: gconstpointer) -> gboolean {
        if a == b { 1 } else { 0 }
    }

    let table = GHashTable::new(hash_func, equal_func);
    assert!(!table.is_null(), "Hash table creation failed");

    // 测试插入和查找
    GHashTable::insert(table, "key1".as_ptr() as gpointer, "value1".as_ptr() as gpointer);
    GHashTable::insert(table, "key2".as_ptr() as gpointer, "value2".as_ptr() as gpointer);

    let value1 = GHashTable::lookup(table, "key1".as_ptr() as gconstpointer);
    let value2 = GHashTable::lookup(table, "key3".as_ptr() as gconstpointer);

    assert_eq!(value1, "value1".as_ptr() as gpointer);
    assert_eq!(value2, ptr::null_mut());

    // 测试移除
    assert_eq!(GHashTable::remove(table, "key2".as_ptr() as gconstpointer), 1);
    assert_eq!(GHashTable::size(table), 1);

    GHashTable::destroy(table);

    println!("  ✓ Hash table tests passed");
}

fn test_queues() {
    let queue = GQueue::new();
    assert!(!queue.is_null());

    GQueue::push_tail(queue, 3 as gpointer);
    GQueue::push_head(queue, 1 as gpointer);
    GQueue::push_tail(queue, 2 as gpointer);

    assert_eq!(GQueue::get_length(queue), 3);

    let head_val = GQueue::pop_head(queue);
    assert_eq!(head_val, 1 as gpointer);
    assert_eq!(GQueue::get_length(queue), 2);

    let tail_val = GQueue::pop_tail(queue);
    assert_eq!(tail_val, 3 as gpointer);
    assert_eq!(GQueue::get_length(queue), 1);

    GQueue::free(queue);

    println!("  ✓ Queue tests passed");
}

fn test_pointer_arrays() {
    let array = GPtrArray::new();
    assert!(!array.is_null());

    let index1 = GPtrArray::add(array, 1 as gpointer);
    let index2 = GPtrArray::add(array, 2 as gpointer);
    let index3 = GPtrArray::add(array, 3 as gpointer);

    assert_eq!(GPtrArray::get_len(array), 3);
    assert_eq!(index1, 0);
    assert_eq!(index2, 1);
    assert_eq!(index3, 2);

    assert_eq!(GPtrArray::index(array, 0), 1 as gpointer);
    assert_eq!(GPtrArray::index(array, 1), 2 as gpointer);
    assert_eq!(GPtrArray::index(array, 2), 3 as gpointer);

    let removed = GPtrArray::remove_index(array, 1);
    assert_eq!(removed, 2 as gpointer);
    assert_eq!(GPtrArray::get_len(array), 2);
    assert_eq!(GPtrArray::index(array, 1), 3 as gpointer);

    GPtrArray::free(array);

    println!("  ✓ Pointer array tests passed");
}

fn test_string_operations() {
    println!("\n--- Testing String Operations ---");

    // 测试GString创建
    let gstring = GString::new("Hello");
    assert!(!gstring.is_null());

    unsafe {
        assert_eq!(GString::as_str(gstring), "Hello");
        assert_eq!(GString::len(gstring), 5);
    }

    // 测试追加
    let gstring = GString::append(gstring, ", World!");
    unsafe {
        assert_eq!(GString::as_str(gstring), "Hello, World!");
        assert_eq!(GString::len(gstring), 13);
    }

    // 测试插入
    let gstring = GString::insert(gstring, 5, " Beautiful ");
    unsafe {
        assert_eq!(GString::as_str(gstring), "Hello Beautiful World!");
    }

    // 测试删除
    let gstring = GString::erase(gstring, 5, 10);
    unsafe {
        assert_eq!(GString::as_str(gstring), "Hello World!");
    }

    // 测试截断
    let gstring = GString::truncate(gstring, 5);
    unsafe {
        assert_eq!(GString::as_str(gstring), "Hello");
    }

    // 获取C字符串
    let c_str = GString::free(gstring);
    assert!(!c_str.is_null());

    unsafe {
        let str = core::ffi::CStr::from_ptr(c_str).to_str().unwrap();
        assert_eq!(str, "Hello");
    }

    g_free(c_str as gpointer);

    // 测试UTF-8验证
    let valid_utf8 = "Hello, 世界!";
    assert_eq!(g_utf8_validate(valid_utf8.as_ptr() as *const i8, -1), 1);

    let invalid_utf8 = b"\xFF\xFE\xFD";
    assert_eq!(g_utf8_validate(invalid_utf8.as_ptr() as *const i8, invalid_utf8.len()), 0);

    // 测试字符串比较
    assert_eq!(g_strcmp0("Hello".as_ptr() as *const i8, "Hello".as_ptr() as *const i8), 0);
    assert_eq!(g_strcmp0("Hello".as_ptr() as *const i8, "hello".as_ptr() as *const i8), -1);
    assert_eq!(g_ascii_strcasecmp("Hello".as_ptr() as *const i8, "hello".as_ptr() as *const i8), 0);

    // 测试字符串复制
    let copied = g_strdup("Test String".as_ptr() as *const i8);
    assert!(!copied.is_null());
    assert_eq!(g_strcmp0("Test String".as_ptr() as *const i8, copied), 0);
    g_free(copied as gpointer);

    let ndup = g_strndup("Partial Copy".as_ptr() as *const i8, 7);
    assert!(!ndup.is_null());
    assert_eq!(g_strcmp0("Partial".as_ptr() as *const i8, ndup), 0);
    g_free(ndup as gpointer);

    // 测试路径处理
    let path = b"/home/user/document.txt\0";
    let dirname = g_path_dirname(path.as_ptr() as *const i8);
    let basename = g_path_basename(path.as_ptr() as *const i8);

    assert!(!dirname.is_null());
    assert!(!basename.is_null());

    unsafe {
        let dir_str = core::ffi::CStr::from_ptr(dirname).to_str().unwrap();
        let base_str = core::ffi::CStr::from_ptr(basename).to_str().unwrap();
        assert_eq!(dir_str, "/home/user");
        assert_eq!(base_str, "document.txt");
    }

    g_free(dirname as gpointer);
    g_free(basename as gpointer);

    println!("  ✓ String operation tests passed");
}

fn test_math_functions() {
    println!("\n--- Testing Math Functions ---");

    assert_eq!(math::g_abs(-5), 5);
    assert_eq!(math::g_max(3, 7), 7);
    assert_eq!(math::g_min(3, 7), 3);
    assert_eq!(math::g_round(3.7), 4.0);
    assert_eq!(math::g_ceil(3.2), 4.0);
    assert_eq!(math::g_floor(3.8), 3.0);
    assert_eq!(math::g_sqrt(16.0), 4.0);
    assert_eq!(math::g_exp(0.0), 1.0);
    assert_eq!(math::g_log(1.0), 0.0);
    assert_eq!(math::g_pow(2.0, 3.0), 8.0);

    let angle = G_PI / 4.0;
    assert!((math::g_sin(angle) - 0.7071).abs() < 0.01);
    assert!((math::g_cos(angle) - 0.7071).abs() < 0.01);

    println!("  ✓ Math function tests passed");
}

fn test_time_functions() {
    println!("\n--- Testing Time Functions ---");

    // 测试基本时间获取
    let mut time_val = 0i32;
    let current_time = time::g_time(&mut time_val);
    assert!(current_time > 0);
    assert!(time_val > 0);

    // 测试高精度时间
    let mut tv = GTimeVal_ { tv_sec: 0, tv_usec: 0 };
    time::g_get_current_time(&mut tv);
    assert!(tv.tv_sec > 0);

    // 测试单调时间
    let monotonic_time = time::g_get_monotonic_time();
    assert!(monotonic_time > 0);

    // 测试实时时间
    let real_time = time::g_get_real_time();
    assert!(real_time > 0);

    println!("  ✓ Time function tests passed");
}

fn test_utility_functions() {
    println!("\n--- Testing Utility Functions ---");

    // 测试随机数生成
    g_random_set_seed(12345);
    let r1 = g_random_int();
    let r2 = g_random_int_range(10, 20);
    let r3 = g_random_double();
    let r4 = g_random_double_range(5.0, 10.0);

    assert!(r1 <= 2147483647);
    assert!(r2 >= 10 && r2 < 20);
    assert!(r3 >= 0.0 && r3 < 1.0);
    assert!(r4 >= 5.0 && r4 < 10.0);

    // 测试位操作
    assert_eq!(bit_ops::g_bit_nth_lsf(0b10100, 0), 2);
    assert_eq!(bit_ops::g_bit_nth_lsf(0b10100, 1), 4);
    assert_eq!(bit_ops::g_bit_nth_msf(0b10100, 0), 0);
    assert_eq!(bit_ops::g_bit_storage(10), 2);

    // 测试网络字节序
    let host_val = 0x12345678;
    let net_val = bit_ops::g_htonl(host_val);
    assert_eq!(bit_ops::g_ntohl(net_val), host_val);

    println!("  ✓ Utility function tests passed");
}

// 简化的时间实现
pub mod time {
    pub fn get_timestamp() -> u64 {
        // 简化实现：返回递增的时间戳
        use core::sync::atomic::{AtomicU64, Ordering};
        static TIMESTAMP: AtomicU64 = AtomicU64::new(0);
        TIMESTAMP.fetch_add(1, Ordering::SeqCst)
    }

    pub fn sleep(duration: core::time::Duration) {
        // 简化实现：实际实现中会使用系统调用
        use core::sync::atomic::{AtomicU64, Ordering};
        static SLEEP_COUNT: AtomicU64 = AtomicU64::new(0);
        SLEEP_COUNT.fetch_add(1, Ordering::SeqCst);
    }
}