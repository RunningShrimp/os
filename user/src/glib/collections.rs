//! GLib数据结构集合模块
//!
//! 提供与GLib兼容的数据结构，包括：
//! - GList/GSList 双向/单向链表
//! - GHashTable 哈希表
//! - GQueue 队列
//! - GPtrArray 指针数组
//! - GByteArray 字节数组
//! - 数据结构迭代器

#![no_std]

extern crate alloc;

use crate::glib::{types::*, g_free, g_malloc, g_malloc0, g_realloc};
use alloc::collections::BTreeMap;
use core::ptr::{self, NonNull};
use core::mem;
use core::ffi::c_void;

/// GList 双向链表节点
#[repr(C)]
#[derive(Debug)]
pub struct GList {
    pub data: gpointer,
    pub next: *mut GList,
    pub prev: *mut GList,
}

impl GList {
    /// 创建新的链表节点
    pub fn new(data: gpointer) -> *mut GList {
        unsafe {
            let node = g_malloc(core::mem::size_of::<GList>()) as *mut GList;
            if !node.is_null() {
                (*node).data = data;
                (*node).next = ptr::null_mut();
                (*node).prev = ptr::null_mut();
            }
            node
        }
    }

    /// 在链表前添加节点
    pub fn prepend(list: *mut GList, data: gpointer) -> *mut GList {
        let new_node = Self::new(data);
        if new_node.is_null() {
            return list;
        }

        unsafe {
            (*new_node).next = list;
            if !list.is_null() {
                (*new_node).prev = (*list).prev;
                (*list).prev = new_node;
            }
        }
        new_node
    }

    /// 在链表后添加节点
    pub fn append(list: *mut GList, data: gpointer) -> *mut GList {
        let new_node = Self::new(data);
        if new_node.is_null() {
            return list;
        }

        if list.is_null() {
            return new_node;
        }

        unsafe {
            // 找到链表尾部
            let mut last = list;
            while !(*last).next.is_null() {
                last = (*last).next;
            }

            (*last).next = new_node;
            (*new_node).prev = last;
        }
        list
    }

    /// 插入节点到指定位置
    pub fn insert(list: *mut GList, data: gpointer, position: i32) -> *mut GList {
        if position <= 0 {
            return Self::prepend(list, data);
        }

        let new_node = Self::new(data);
        if new_node.is_null() {
            return list;
        }

        if list.is_null() {
            return new_node;
        }

        unsafe {
            // 找到插入位置
            let mut current = list;
            let mut pos = 0;
            while !current.is_null() && pos < position {
                current = (*current).next;
                pos += 1;
            }

            if current.is_null() {
                // 插入到末尾
                return Self::append(list, data);
            }

            // 插入到current之前
            (*new_node).next = current;
            (*new_node).prev = (*current).prev;
            (*current).prev = new_node;

            if !(*new_node).prev.is_null() {
                (*(*new_node).prev).next = new_node;
                return list; // 不是插入到头部
            } else {
                return new_node; // 插入到头部
            }
        }
    }

    /// 移除指定数据的节点
    pub fn remove(list: *mut GList, data: gconstpointer) -> *mut GList {
        let mut current = list;
        let mut result = list;

        unsafe {
            while !current.is_null() {
                if (*current).data == data as gpointer {
                    let next = (*current).next;
                    let prev = (*current).prev;

                    if !prev.is_null() {
                        (*prev).next = next;
                    } else {
                        result = next; // 移除头部
                    }

                    if !next.is_null() {
                        (*next).prev = prev;
                    }

                    g_free(current as gpointer);
                    current = next;
                } else {
                    current = (*current).next;
                }
            }
        }

        result
    }

    /// 移除指定位置的节点并返回数据
    pub fn remove_nth(list: *mut GList, n: u32) -> (*mut GList, gpointer) {
        let mut current = list;
        let mut pos = 0;

        unsafe {
            while !current.is_null() && pos < n {
                current = (*current).next;
                pos += 1;
            }

            if current.is_null() {
                return (list, ptr::null_mut());
            }

            let data = (*current).data;
            let next = (*current).next;
            let prev = (*current).prev;

            if !prev.is_null() {
                (*prev).next = next;
            }

            if !next.is_null() {
                (*next).prev = prev;
            }

            g_free(current as gpointer);

            let new_list = if prev.is_null() { next } else { list };
            (new_list, data)
        }
    }

    /// 获取链表长度
    pub fn length(list: *const GList) -> u32 {
        let mut len = 0;
        let mut current = list;

        unsafe {
            while !current.is_null() {
                len += 1;
                current = (*current).next;
            }
        }

        len
    }

    /// 查找指定数据的节点
    pub fn find(list: *const GList, data: gconstpointer) -> *mut GList {
        let mut current = list;

        unsafe {
            while !current.is_null() {
                if (*current).data == data as gpointer {
                    return current as *mut GList;
                }
                current = (*current).next;
            }
        }

        ptr::null_mut()
    }

    /// 使用比较函数查找节点
    pub fn find_custom(list: *const GList, data: gconstpointer,
                      func: GCompareFunc) -> *mut GList {
        let mut current = list;

        unsafe {
            while !current.is_null() {
                if func((*current).data, data) == 0 {
                    return current as *mut GList;
                }
                current = (*current).next;
            }
        }

        ptr::null_mut()
    }

    /// 反转链表
    pub fn reverse(list: *mut GList) -> *mut GList {
        let mut current = list;
        let mut prev = ptr::null_mut();

        unsafe {
            while !current.is_null() {
                let next = (*current).next;
                (*current).next = prev;
                (*current).prev = next;
                prev = current;
                current = next;
            }
        }

        prev
    }

    /// 复制链表
    pub fn copy(list: *const GList) -> *mut GList {
        let mut new_list = ptr::null_mut();
        let mut current = list;

        unsafe {
            while !current.is_null() {
                new_list = Self::append(new_list, (*current).data);
                current = (*current).next;
            }
        }

        new_list
    }

    /// 释放链表
    pub fn free(list: *mut GList) {
        let mut current = list;
        let mut next;

        unsafe {
            while !current.is_null() {
                next = (*current).next;
                g_free(current as gpointer);
                current = next;
            }
        }
    }

    /// 释放链表并调用每个数据的销毁函数
    pub fn free_full(list: *mut GList, free_func: GDestroyNotify) {
        let mut current = list;
        let mut next;

        unsafe {
            while !current.is_null() {
                next = (*current).next;
                if free_func as *const () != core::ptr::null() {
                    free_func((*current).data);
                }
                g_free(current as gpointer);
                current = next;
            }
        }
    }
}

/// GSList 单向链表节点
#[repr(C)]
#[derive(Debug)]
pub struct GSList {
    pub data: gpointer,
    pub next: *mut GSList,
}

impl GSList {
    /// 创建新的单向链表节点
    pub fn new(data: gpointer) -> *mut GSList {
        unsafe {
            let node = g_malloc(core::mem::size_of::<GSList>()) as *mut GSList;
            if !node.is_null() {
                (*node).data = data;
                (*node).next = ptr::null_mut();
            }
            node
        }
    }

    /// 在链表前添加节点
    pub fn prepend(list: *mut GSList, data: gpointer) -> *mut GSList {
        let new_node = Self::new(data);
        if !new_node.is_null() {
            unsafe {
                (*new_node).next = list;
            }
        }
        new_node
    }

    /// 在链表后添加节点
    pub fn append(list: *mut GSList, data: gpointer) -> *mut GSList {
        let new_node = Self::new(data);
        if new_node.is_null() {
            return list;
        }

        if list.is_null() {
            return new_node;
        }

        unsafe {
            // 找到链表尾部
            let mut last = list;
            while !(*last).next.is_null() {
                last = (*last).next;
            }

            (*last).next = new_node;
        }
        list
    }

    /// 获取单向链表长度
    pub fn length(list: *const GSList) -> u32 {
        let mut len = 0;
        let mut current = list;

        unsafe {
            while !current.is_null() {
                len += 1;
                current = (*current).next;
            }
        }

        len
    }

    /// 释放单向链表
    pub fn free(list: *mut GSList) {
        let mut current = list;
        let mut next;

        unsafe {
            while !current.is_null() {
                next = (*current).next;
                g_free(current as gpointer);
                current = next;
            }
        }
    }

    /// 释放单向链表并调用每个数据的销毁函数
    pub fn free_full(list: *mut GSList, free_func: GDestroyNotify) {
        let mut current = list;
        let mut next;

        unsafe {
            while !current.is_null() {
                next = (*current).next;
                if free_func as *const () != core::ptr::null() {
                    free_func((*current).data);
                }
                g_free(current as gpointer);
                current = next;
            }
        }
    }
}

/// GHashTable 哈希表节点
#[derive(Debug)]
pub struct GHashTableNode {
    pub key: gpointer,
    pub value: gpointer,
    pub hash: u32,
    pub next: *mut GHashTableNode,
}

/// GHashTable 哈希表
#[derive(Debug)]
pub struct GHashTable {
    pub size: u32,
    pub nnodes: u32,
    pub nodes: *mut *mut GHashTableNode,
    pub key_hash_func: GHashFunc,
    pub key_equal_func: GEqualFunc,
    pub key_destroy_func: GDestroyNotify,
    pub value_destroy_func: GDestroyNotify,
}

/// 哈希函数类型
pub type GHashFunc = unsafe extern "C" fn(gconstpointer) -> u32;

/// 相等函数类型
pub type GEqualFunc = unsafe extern "C" fn(gconstpointer, gconstpointer) -> gboolean;

impl GHashTable {
    /// 创建新的哈希表
    pub fn new(key_hash_func: GHashFunc, key_equal_func: GEqualFunc) -> *mut GHashTable {
        Self::new_full(key_hash_func, key_equal_func, unsafe { mem::transmute(ptr::null::<()>() as *const ()) }, unsafe { mem::transmute(ptr::null::<()>() as *const ()) })
    }

    /// 创建新的哈希表（带销毁函数）
    pub fn new_full(
        key_hash_func: GHashFunc,
        key_equal_func: GEqualFunc,
        key_destroy_func: GDestroyNotify,
        value_destroy_func: GDestroyNotify,
    ) -> *mut GHashTable {
        unsafe {
            let table = g_malloc0(core::mem::size_of::<GHashTable>()) as *mut GHashTable;
            if table.is_null() {
                return ptr::null_mut();
            }

            (*table).size = 8; // 初始大小
            (*table).nnodes = 0;
            (*table).nodes = g_malloc0((*table).size as usize * core::mem::size_of::<*mut GHashTableNode>())
                as *mut *mut GHashTableNode;
            (*table).key_hash_func = key_hash_func;
            (*table).key_equal_func = key_equal_func;
            (*table).key_destroy_func = key_destroy_func;
            (*table).value_destroy_func = value_destroy_func;

            table
        }
    }

    /// 插入键值对
    pub fn insert(table: *mut GHashTable, key: gpointer, value: gpointer) {
        if table.is_null() {
            return;
        }

        unsafe {
            let hash_func = (*table).key_hash_func;
            let hash = if hash_func as *const () != core::ptr::null() {
                hash_func(key)
            } else {
                default_hash_func(key)
            };

            let index = hash % (*table).size;

            // 检查键是否已存在
            let mut node = *(*table).nodes.add(index as usize);
            let mut prev: *mut GHashTableNode = ptr::null_mut();

            while !node.is_null() {
                let keys_equal = if (*table).key_equal_func as *const () != core::ptr::null() {
                    let equal_func = (*table).key_equal_func;
                    equal_func((*node).key, key) != 0
                } else {
                    (*node).key == key as gpointer
                };

                if keys_equal {
                    // 键已存在，更新值
                    let value_destroy_func = (*table).value_destroy_func;
                    if value_destroy_func as *const () != core::ptr::null() {
                        value_destroy_func((*node).value);
                    }
                    (*node).value = value;
                    return;
                }

                prev = node;
                node = (*node).next;
            }

            // 创建新节点
            let new_node = g_malloc(core::mem::size_of::<GHashTableNode>()) as *mut GHashTableNode;
            (*new_node).key = key;
            (*new_node).value = value;
            (*new_node).hash = hash;
            (*new_node).next = ptr::null_mut();

            if prev.is_null() {
                *(*table).nodes.add(index as usize) = new_node;
            } else {
                (*prev).next = new_node;
            }

            (*table).nnodes += 1;

            // 检查是否需要扩容
            if (*table).nnodes > (*table).size * 3 / 4 {
                resize_table(table);
            }
        }
    }

    /// 查找键对应的值
    pub fn lookup(table: *mut GHashTable, key: gconstpointer) -> gpointer {
        if table.is_null() {
            return ptr::null_mut();
        }

        unsafe {
            let hash_func = (*table).key_hash_func;
            let hash = if hash_func as *const () != core::ptr::null() {
                hash_func(key)
            } else {
                default_hash_func(key)
            };

            let index = hash % (*table).size;
            let mut node = *(*table).nodes.add(index as usize);

            while !node.is_null() {
                let keys_equal = if (*table).key_equal_func as *const () != core::ptr::null() {
                    let equal_func = (*table).key_equal_func;
                    equal_func((*node).key, key) != 0
                } else {
                    (*node).key == key as gpointer
                };

                if keys_equal {
                    return (*node).value;
                }

                node = (*node).next;
            }
        }

        ptr::null_mut()
    }

    /// 移除键值对
    pub fn remove(table: *mut GHashTable, key: gconstpointer) -> gboolean {
        if table.is_null() {
            return 0;
        }

        unsafe {
            let hash_func = (*table).key_hash_func;
            let hash = if hash_func as *const () != core::ptr::null() {
                hash_func(key)
            } else {
                default_hash_func(key)
            };

            let index = hash % (*table).size;
            let mut node = *(*table).nodes.add(index as usize);
            let mut prev: *mut GHashTableNode = ptr::null_mut();

            while !node.is_null() {
                let keys_equal = if (*table).key_equal_func as *const () != core::ptr::null() {
                    let equal_func = (*table).key_equal_func;
                    equal_func((*node).key, key) != 0
                } else {
                    (*node).key == key as gpointer
                };

                if keys_equal {
                    // 调用销毁函数
                    let key_destroy_func = (*table).key_destroy_func;
                    let value_destroy_func = (*table).value_destroy_func;
                    if key_destroy_func as *const () != core::ptr::null() {
                        key_destroy_func((*node).key);
                    }
                    if value_destroy_func as *const () != core::ptr::null() {
                        value_destroy_func((*node).value);
                    }

                    // 从链表中移除节点
                    if prev.is_null() {
                        *(*table).nodes.add(index as usize) = (*node).next;
                    } else {
                        (*prev).next = (*node).next;
                    }

                    g_free(node as gpointer);
                    (*table).nnodes -= 1;
                    return 1; // true
                }

                prev = node;
                node = (*node).next;
            }
        }

        0 // false
    }

    /// 获取哈希表大小
    pub fn size(table: *mut GHashTable) -> u32 {
        if table.is_null() {
            return 0;
        }
        unsafe { (*table).nnodes }
    }

    /// 释放哈希表
    pub fn destroy(table: *mut GHashTable) {
        if table.is_null() {
            return;
        }

        unsafe {
            // 释放所有节点
            for i in 0..(*table).size {
                let mut node = *(*table).nodes.add(i as usize);
                while !node.is_null() {
                    let next = (*node).next;

                    // 调用销毁函数
                    let key_destroy_func = (*table).key_destroy_func;
                    let value_destroy_func = (*table).value_destroy_func;
                    if key_destroy_func as *const () != core::ptr::null() {
                        key_destroy_func((*node).key);
                    }
                    if value_destroy_func as *const () != core::ptr::null() {
                        value_destroy_func((*node).value);
                    }

                    g_free(node as gpointer);
                    node = next;
                }
            }

            // 释放节点数组
            g_free((*table).nodes as gpointer);

            // 释放表结构
            g_free(table as gpointer);
        }
    }
}

/// 默认哈希函数
fn default_hash_func(key: gconstpointer) -> u32 {
    let ptr = key as usize;
    let hash = ptr.wrapping_mul(2654435761);
    (hash >> 32) as u32 ^ (hash as u32)
}

/// 调整哈希表大小
fn resize_table(table: *mut GHashTable) {
    unsafe {
        if table.is_null() {
            return;
        }

        let old_size = (*table).size;
        let new_size = old_size * 2;
        let new_nodes = g_malloc0(new_size as usize * core::mem::size_of::<*mut GHashTableNode>())
            as *mut *mut GHashTableNode;

        // 重新哈希所有节点
        for i in 0..old_size {
            let mut node = *(*table).nodes.add(i as usize);
            while !node.is_null() {
                let next = (*node).next;
                let index = (*node).hash % new_size;

                // 插入到新表
                (*node).next = *new_nodes.add(index as usize);
                *new_nodes.add(index as usize) = node;

                node = next;
            }
        }

        // 更新表
        g_free((*table).nodes as gpointer);
        (*table).nodes = new_nodes;
        (*table).size = new_size;
    }
}

/// GQueue 队列
#[derive(Debug)]
pub struct GQueue {
    pub head: *mut GList,
    pub tail: *mut GList,
    pub length: u32,
}

impl GQueue {
    /// 创建新队列
    pub fn new() -> *mut GQueue {
        unsafe {
            let queue = g_malloc0(core::mem::size_of::<GQueue>()) as *mut GQueue;
            if !queue.is_null() {
                (*queue).length = 0;
            }
            queue
        }
    }

    /// 在队尾添加元素
    pub fn push_tail(queue: *mut GQueue, data: gpointer) {
        if queue.is_null() {
            return;
        }

        unsafe {
            (*queue).head = GList::append((*queue).head, data);
            if (*queue).tail.is_null() || (*(*queue).tail).next.is_null() {
                (*queue).tail = GList::find((*queue).head, data);
            }
            (*queue).length += 1;
        }
    }

    /// 在队头添加元素
    pub fn push_head(queue: *mut GQueue, data: gpointer) {
        if queue.is_null() {
            return;
        }

        unsafe {
            (*queue).head = GList::prepend((*queue).head, data);
            if (*queue).tail.is_null() {
                (*queue).tail = (*queue).head;
            }
            (*queue).length += 1;
        }
    }

    /// 从队尾弹出元素
    pub fn pop_tail(queue: *mut GQueue) -> gpointer {
        if queue.is_null() || (*queue).tail.is_null() {
            return ptr::null_mut();
        }

        unsafe {
            let data = (*(*queue).tail).data;
            let new_tail = (*(*queue).tail).prev;

            if !new_tail.is_null() {
                (*new_tail).next = ptr::null_mut();
            } else {
                (*queue).head = ptr::null_mut();
            }

            g_free((*queue).tail as gpointer);
            (*queue).tail = new_tail;
            (*queue).length = (*queue).length.saturating_sub(1);

            data
        }
    }

    /// 从队头弹出元素
    pub fn pop_head(queue: *mut GQueue) -> gpointer {
        if queue.is_null() || (*queue).head.is_null() {
            return ptr::null_mut();
        }

        unsafe {
            let data = (*(*queue).head).data;
            let new_head = (*(*queue).head).next;

            if !new_head.is_null() {
                (*new_head).prev = ptr::null_mut();
            } else {
                (*queue).tail = ptr::null_mut();
            }

            g_free((*queue).head as gpointer);
            (*queue).head = new_head;
            (*queue).length = (*queue).length.saturating_sub(1);

            data
        }
    }

    /// 获取队列长度
    pub fn get_length(queue: *const GQueue) -> u32 {
        if queue.is_null() {
            return 0;
        }
        unsafe { (*queue).length }
    }

    /// 释放队列
    pub fn free(queue: *mut GQueue) {
        if queue.is_null() {
            return;
        }

        unsafe {
            GList::free((*queue).head);
            g_free(queue as gpointer);
        }
    }

    /// 释放队列并调用销毁函数
    pub fn free_full(queue: *mut GQueue, free_func: GDestroyNotify) {
        if queue.is_null() {
            return;
        }

        unsafe {
            GList::free_full((*queue).head, free_func);
            g_free(queue as gpointer);
        }
    }
}

/// GPtrArray 指针数组
#[derive(Debug)]
pub struct GPtrArray {
    pub pdata: *mut gpointer,
    pub len: u32,
    pub allocated: u32,
}

impl GPtrArray {
    /// 创建新的指针数组
    pub fn new() -> *mut GPtrArray {
        Self::sized_new(0)
    }

    /// 创建指定大小的指针数组
    pub fn sized_new(reserved_size: u32) -> *mut GPtrArray {
        unsafe {
            let array = g_malloc0(core::mem::size_of::<GPtrArray>()) as *mut GPtrArray;
            if array.is_null() {
                return ptr::null_mut();
            }

            if reserved_size > 0 {
                (*array).pdata = g_malloc0(reserved_size as usize * core::mem::size_of::<gpointer>())
                    as *mut gpointer;
                (*array).allocated = reserved_size;
            } else {
                (*array).pdata = ptr::null_mut();
                (*array).allocated = 0;
            }

            (*array).len = 0;
            array
        }
    }

    /// 添加指针到数组
    pub fn add(array: *mut GPtrArray, data: gpointer) -> u32 {
        if array.is_null() {
            return 0;
        }

        unsafe {
            // 检查是否需要扩容
            if (*array).len >= (*array).allocated {
                let new_size = if (*array).allocated == 0 {
                    8
                } else {
                    (*array).allocated * 2
                };

                (*array).pdata = g_realloc((*array).pdata as gpointer,
                    new_size as usize * core::mem::size_of::<gpointer>()) as *mut gpointer;
                (*array).allocated = new_size;
            }

            *((*array).pdata).add((*array).len as usize) = data;
            let index = (*array).len;
            (*array).len += 1;
            index
        }
    }

    /// 获取指定索引的指针
    pub fn index(array: *const GPtrArray, index: u32) -> gpointer {
        if array.is_null() || index >= unsafe { (*array).len } {
            return ptr::null_mut();
        }

        unsafe {
            *((*array).pdata).add(index as usize)
        }
    }

    /// 设置指定索引的指针
    pub fn set(array: *mut GPtrArray, index: u32, data: gpointer) {
        if array.is_null() || index >= unsafe { (*array).len } {
            return;
        }

        unsafe {
            *((*array).pdata).add(index as usize) = data;
        }
    }

    /// 移除指定索引的指针
    pub fn remove_index(array: *mut GPtrArray, index: u32) -> gpointer {
        if array.is_null() || index >= unsafe { (*array).len } {
            return ptr::null_mut();
        }

        unsafe {
            let data = *((*array).pdata).add(index as usize);

            // 移动后续元素
            for i in index..(*array).len - 1 {
                *((*array).pdata).add(i as usize) = *((*array).pdata).add((i + 1) as usize);
            }

            (*array).len -= 1;
            data
        }
    }

    /// 获取数组长度
    pub fn get_len(array: *const GPtrArray) -> u32 {
        if array.is_null() {
            return 0;
        }
        unsafe { (*array).len }
    }

    /// 释放指针数组
    pub fn free(array: *mut GPtrArray) {
        if array.is_null() {
            return;
        }

        unsafe {
            g_free((*array).pdata as gpointer);
            g_free(array as gpointer);
        }
    }

    /// 释放指针数组并调用销毁函数
    pub fn free_full(array: *mut GPtrArray, free_func: GDestroyNotify) {
        if array.is_null() {
            return;
        }

        unsafe {
            for i in 0..(*array).len {
                let data = *((*array).pdata).add(i as usize);
                if free_func as *const () != core::ptr::null() {
                    free_func(data);
                }
            }

            g_free((*array).pdata as gpointer);
            g_free(array as gpointer);
        }
    }
}

/// 数据结构测试
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glist_basic_operations() {
        // 测试链表基本操作
        let list = ptr::null_mut();
        let list = GList::append(list, 1 as gpointer);
        let list = GList::append(list, 2 as gpointer);
        let list = GList::prepend(list, 0 as gpointer);

        assert_eq!(GList::length(list), 3);
        assert_eq!(GList::find(list, 1 as gpointer),
                   unsafe { (*list).next as *mut GList });

        GList::free(list);
    }

    #[test]
    fn test_gslist_operations() {
        let list = ptr::null_mut();
        let list = GSList::append(list, 1 as gpointer);
        let list = GSList::append(list, 2 as gpointer);
        let list = GSList::prepend(list, 0 as gpointer);

        assert_eq!(GSList::length(list), 3);

        GSList::free(list);
    }

    #[test]
    fn test_gqueue_operations() {
        let queue = GQueue::new();
        assert!(!queue.is_null());

        GQueue::push_tail(queue, 1 as gpointer);
        GQueue::push_head(queue, 0 as gpointer);
        GQueue::push_tail(queue, 2 as gpointer);

        assert_eq!(GQueue::get_length(queue), 3);

        let data1 = GQueue::pop_head(queue);
        assert_eq!(data1, 0 as gpointer);

        let data2 = GQueue::pop_tail(queue);
        assert_eq!(data2, 2 as gpointer);

        assert_eq!(GQueue::get_length(queue), 1);

        GQueue::free(queue);
    }

    #[test]
    fn test_gptrarray_operations() {
        let array = GPtrArray::new();
        assert!(!array.is_null());

        let index1 = GPtrArray::add(array, 1 as gpointer);
        let index2 = GPtrArray::add(array, 2 as gpointer);

        assert_eq!(index1, 0);
        assert_eq!(index2, 1);
        assert_eq!(GPtrArray::get_len(array), 2);
        assert_eq!(GPtrArray::index(array, 0), 1 as gpointer);
        assert_eq!(GPtrArray::index(array, 1), 2 as gpointer);

        let removed = GPtrArray::remove_index(array, 0);
        assert_eq!(removed, 1 as gpointer);
        assert_eq!(GPtrArray::get_len(array), 1);
        assert_eq!(GPtrArray::index(array, 0), 2 as gpointer);

        GPtrArray::free(array);
    }
}