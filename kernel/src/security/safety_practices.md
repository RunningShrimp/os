# Rust 安全最佳实践指南

本文档为 NOS 内核开发团队提供 Rust 安全编程的最佳实践指南，涵盖内存安全、并发安全、类型安全等方面。

## 目录

1. [内存安全](#内存安全)
2. [并发安全](#并发安全)
3. [类型安全](#类型安全)
4. [错误处理](#错误处理)
5. [无标准库 (no_std) 开发](#无标准库-nostd-开发)
6. [性能优化](#性能优化)
7. [代码审查检查清单](#代码审查检查清单)

## 内存安全

### 1. 避免裸指针和 unsafe 代码

**原则**: 尽可能使用 Rust 的安全抽象，只在必要时使用 `unsafe`。

```rust
use alloc::vec::Vec;

let mut data = Vec::new();
data.push(42);

// 避免: 使用裸指针
unsafe {
    let ptr = data.as_ptr();
    *ptr = 100; // 危险
}

// 推荐: 使用安全接口
data[0] = 100; // 安全
```

### 2. 正确处理生命周期

**原则**: 确保引用的生命周期不超过被引用的数据。

```rust
// 错误示例: 悬垂引用
fn dangling_ref() -> &'static i32 {
    let x = 42;
    &x // 编译错误: 返回局部变量的引用
}

// 正确示例: 返回拥有所有权的值
fn owned_value() -> i32 {
    let x = 42;
    x
}

// 正确示例: 使用生命周期参数
fn borrow_data<'a>(data: &'a i32) -> &'a i32 {
    data
}
```

### 3. 使用智能指针管理内存

```rust
use alloc::sync::Arc;
use alloc::rc::Rc;

// Arc: 原子引用计数，线程安全
let shared_data = Arc::new(vec![1, 2, 3]);
let cloned = Arc::clone(&shared_data);

// Rc: 非原子引用计数，单线程
#[cfg(not(feature = "thread_safe"))]
let local_data = Rc::new(vec![1, 2, 3]);
```

### 4. 避免内存泄漏

```rust
use alloc::sync::{Arc, Weak};

// 避免循环引用导致的内存泄漏
struct Node {
    data: i32,
    next: Option<Weak<Node>>,
    prev: Option<Weak<Node>>,
}

let node1 = Arc::new(Node {
    data: 1,
    next: None,
    prev: None,
});
```

## 并发安全

### 1. 使用原子操作

```rust
use core::sync::atomic::{AtomicUsize, Ordering};

// 正确的原子操作模式
struct Counter {
    value: AtomicUsize,
}

impl Counter {
    fn new() -> Self {
        Self { value: AtomicUsize::new(0) }
    }
    
    fn increment(&self) -> usize {
        self.value.fetch_add(1, Ordering::Relaxed) + 1
    }
    
    fn get(&self) -> usize {
        self.value.load(Ordering::Acquire)
    }
}
```

### 2. 正确选择内存序

```rust
use core::sync::atomic::{AtomicBool, Ordering};

// 生产者-消费者模式
struct Channel<T> {
    ready: AtomicBool,
    data: Option<T>,
}

impl<T> Channel<T> {
    fn send(&mut self, data: T) {
        self.data = Some(data);
        self.ready.store(true, Ordering::Release);
    }
    
    fn try_recv(&self) -> Option<T> {
        if self.ready.load(Ordering::Acquire) {
            self.data.clone()
        } else {
            None
        }
    }
}
```

### 3. 使用适当的同步原语

```rust
use spin::Mutex;
use spin::RwLock;

// Mutex: 互斥锁，独占访问
struct SharedState {
    counter: Mutex<usize>,
}

// RwLock: 读写锁，多读单写
struct Cache {
    data: RwLock<Vec<String>>,
}

impl Cache {
    fn read(&self) -> Vec<String> {
        let guard = self.data.read();
        guard.clone()
    }
    
    fn write(&self, new_data: Vec<String>) {
        let mut guard = self.data.write();
        *guard = new_data;
    }
}
```

### 4. 避免死锁

```rust
use spin::Mutex;

struct ResourceA(Mutex<()>);
struct ResourceB(Mutex<()>);

// 危险: 可能死锁
fn dangerous_lock(a: &ResourceA, b: &ResourceB) {
    let _guard1 = a.0.lock();
    let _guard2 = b.0.lock();
}

// 安全: 按固定顺序获取锁
fn safe_lock(a: &ResourceA, b: &ResourceB) {
    if a.0.try_lock().is_ok() {
        if b.0.try_lock().is_ok() {
        }
    }
}
```

## 类型安全

### 1. 使用枚举代替魔法数字

```rust
// 避免: 魔法数字
fn handle_error(code: i32) {
    match code {
        0 => println!("OK"),
        1 => println!("Error"),
        2 => println!("Warning"),
        _ => println!("Unknown"),
    }
}

// 推荐: 使用枚举
#[derive(Debug, Clone, Copy)]
enum Status {
    Ok,
    Error,
    Warning,
}

fn handle_status(status: Status) {
    match status {
        Status::Ok => println!("OK"),
        Status::Error => println!("Error"),
        Status::Warning => println!("Warning"),
    }
}
```

### 2. 使用新类型包装器

```rust
use core::ops::Deref;

// 定义类型别名增强可读性和安全性
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProcessId(u64);

impl ProcessId {
    fn new(id: u64) -> Self {
        Self(id)
    }
    
    fn value(self) -> u64 {
        self.0
    }
}

impl Deref for ProcessId {
    type Target = u64;
    
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
```

### 3. 使用 Option 和 Result

```rust
// Option: 可选值
fn divide(a: i32, b: i32) -> Option<i32> {
    if b == 0 {
        None
    } else {
        Some(a / b)
    }
}

// Result: 可能失败的操作
#[derive(Debug)]
enum MathError {
    DivisionByZero,
    Overflow,
}

fn safe_divide(a: i32, b: i32) -> Result<i32, MathError> {
    if b == 0 {
        Err(MathError::DivisionByZero)
    } else {
        Ok(a / b)
    }
}
```

## 错误处理

### 1. 使用 Result 类型

```rust
type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
enum Error {
    InvalidArgument,
    OutOfMemory,
    IoError,
    PermissionDenied,
}

fn read_config() -> Result<Vec<u8>> {
    let data = Vec::new();
    if data.is_empty() {
        Err(Error::IoError)
    } else {
        Ok(data)
    }
}
```

### 2. 使用 ? 操作符传播错误

```rust
fn parse_config(data: &[u8]) -> Result<Config> {
    let header = parse_header(data).map_err(Error::InvalidHeader)?;
    let body = parse_body(&data[header.len()..])?;
    Ok(Config { header, body })
}
```

### 3. 提供有意义的错误信息

```rust
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    Interrupted,
    UnexpectedEof,
}

impl Error {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
```

## 无标准库 (no_std) 开发

### 1. 使用 alloc 而非 std

```rust
// 在 no_std 环境中
#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
```

### 2. 实现自定义内存分配器

```rust
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;

struct MyAllocator;

unsafe impl GlobalAlloc for MyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();
        todo!()
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        todo!()
    }
}

#[global_allocator]
static GLOBAL: MyAllocator = MyAllocator;
```

### 3. 使用 panic 处理

```rust
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("PANIC: {}", info);
    loop {}
}

#[alloc_error_handler]
fn alloc_error(layout: Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
```

## 性能优化

### 1. 避免不必要的克隆

```rust
use alloc::string::String;

// 避免: 不必要的克隆
fn process_data(data: String) {
    let _ = data.clone();
}

// 推荐: 使用引用
fn process_data_ref(data: &String) {
    let _ = data;
}

// 更好: 使用 &str
fn process_str(data: &str) {
    let _ = data;
}
```

### 2. 使用迭代器

```rust
use alloc::vec::Vec;

let data = vec![1, 2, 3, 4, 5];

// 推荐: 使用迭代器链
let sum: i32 = data.iter().map(|x| x * 2).filter(|x| x > &5).sum();

// 避免: 多次遍历和中间集合
let doubled: Vec<i32> = data.iter().map(|x| x * 2).collect();
let filtered: Vec<i32> = doubled.into_iter().filter(|x| x > 5).collect();
let sum: i32 = filtered.iter().sum();
```

### 3. 使用 Cow (Copy on Write)

```rust
use alloc::borrow::Cow;

fn process(input: Cow<str>) -> Cow<str> {
    if input.contains("pattern") {
        let modified = input.replace("pattern", "replacement");
        Cow::Owned(modified)
    } else {
        input
    }
}
```

### 4. 零成本抽象

```rust
// 使用 const 泛型实现零成本抽象
struct Array<T, const N: usize> {
    data: [T; N],
}

impl<T, const N: usize> Array<T, N> {
    fn new(data: [T; N]) -> Self {
        Self { data }
    }
    
    fn len(&self) -> usize {
        N
    }
}
```

## 代码审查检查清单

### 内存安全
- [ ] 没有裸指针的越界访问
- [ ] 所有 `unsafe` 代码都有清晰的文档说明
- [ ] 生命周期标注正确，没有悬垂引用
- [ ] 没有未定义行为（UB）

### 并发安全
- [ ] 共享可变状态正确同步
- [ ] 原子操作使用正确的内存序
- [ ] 没有数据竞争
- [ ] 没有死锁风险

### 错误处理
- [ ] 所有错误情况都被处理
- [ ] 使用 `Result` 而非 `panic`
- [ ] 错误信息清晰有意义
- [ ] 错误传播使用 `?` 操作符

### 性能
- [ ] 没有不必要的内存分配
- [ ] 没有不必要的克隆
- [ ] 使用迭代器而非循环
- [ ] 避免不必要的 Box 包装

### 代码质量
- [ ] 函数命名清晰
- [ ] 变量命名清晰
- [ ] 代码注释充分
- [ ] 遵循 Rust 命名规范
