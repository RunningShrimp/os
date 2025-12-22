# NOS Bootloader 使用指南

本指南提供NOS引导加载程序主要功能的完整使用示例和最佳实践，帮助开发者快速上手并避免常见陷阱。

## 目录

- [快速开始](#快速开始)
- [基本使用](#基本使用)
- [高级功能](#高级功能)
- [最佳实践](#最佳实践)
- [常见陷阱](#常见陷阱)
- [故障排除](#故障排除)
- [性能优化](#性能优化)

## 快速开始

### 环境准备

确保您的开发环境已安装以下工具：

```bash
# 安装Rust工具链
rustup update stable
rustup component add rust-src

# 安装目标平台
rustup target add x86_64-unknown-uefi
rustup target add aarch64-unknown-uefi
rustup target add riscv64-unknown-uefi

# 安装构建工具
cargo install cargo-binutils
rustup component add llvm-tools-preview
```

### 构建引导加载程序

```bash
# 构建x86_64 UEFI版本
cargo build --release --target x86_64-unknown-uefi

# 构建AArch64 UEFI版本
cargo build --release --target aarch64-unknown-uefi

# 构建RISC-V UEFI版本
cargo build --release --target riscv64-unknown-uefi
```

### 基本引导流程

```rust
use nos_bootloader::application::BootApplicationService;
use nos_bootloader::protocol::BootProtocolType;
use nos_bootloader::utils::error::Result;

fn main() -> Result<()> {
    // 创建引导应用服务
    let mut service = BootApplicationService::new(BootProtocolType::Uefi)?;
    
    // 执行完整引导序列
    let boot_info = service.boot_system(Some("quiet splash"))?;
    
    println!("引导完成，内核地址: {:#x}", boot_info.kernel_address);
    
    Ok(())
}
```

## 基本使用

### 内存管理

#### 使用双级分配器

```rust
use nos_bootloader::core::allocator::DualLevelAllocator;
use nos_bootloader::domain::boot_config::MemoryRegion;

fn example_memory_allocation() -> Result<()> {
    // 创建分配器
    let allocator = DualLevelAllocator::new();
    
    // 检查内存状态
    println!("已分配: {} 字节", allocator.allocated());
    println!("剩余: {} 字节", allocator.free());
    println!("利用率: {:.1}%", allocator.utilization());
    
    // 分配小内存块（使用空闲列表）
    let layout = core::alloc::Layout::new::<u32>();
    let small_ptr = unsafe { allocator.alloc(layout) };
    
    // 分配大内存块（使用bump分配器）
    let large_layout = core::alloc::Layout::from_size_align(1024, 64)?;
    let large_ptr = unsafe { allocator.alloc(large_layout) };
    
    // 使用内存...
    
    // 释放小内存块（重用）
    unsafe {
        allocator.dealloc(small_ptr, layout);
    }
    
    // 大内存块不释放（bump分配器特性）
    
    Ok(())
}
```

#### 内存区域管理

```rust
use nos_bootloader::domain::boot_config::{MemoryRegion, MemoryRegionType};

fn example_memory_regions() -> Result<()> {
    // 创建可用内存区域
    let available = MemoryRegion::new(0x100000, 0x1000000, MemoryRegionType::Available)?;
    
    // 创建保留内存区域
    let reserved = MemoryRegion::new(0xA0000, 0xC0000, MemoryRegionType::Reserved)?;
    
    // 检查区域属性
    println!("可用区域大小: {} 字节", available.size());
    println!("是否可用: {}", available.is_available());
    
    // 检查重叠
    if available.overlaps(&reserved) {
        println!("警告：内存区域重叠！");
    }
    
    // 检查地址包含
    let test_addr = 0x200000;
    if available.contains(test_addr) {
        println!("地址 {:#x} 在可用区域内", test_addr);
    }
    
    Ok(())
}
```

### 图形渲染

#### 基本图形操作

```rust
use nos_bootloader::graphics::{GraphicsRenderer, FramebufferInfo, Color};

fn example_basic_graphics() -> Result<()> {
    // 创建帧缓冲区信息
    let fb_info = FramebufferInfo::new(
        0xE0000000,  // 帧缓冲区地址
        1024,         // 宽度
        768,          // 高度
        4096,         // 扫描线字节
        32            // 色深
    );
    
    // 创建图形渲染器
    let mut renderer = GraphicsRenderer::new_with_double_buffer(fb_info)?;
    
    // 初始化帧缓冲区
    renderer.initialize_framebuffer()?;
    
    // 清屏为深蓝色
    renderer.clear_screen(Color::rgb(0, 51, 102))?;
    
    // 绘制白色矩形
    renderer.draw_filled_rect(100, 100, 200, 150, Color::white())?;
    
    // 绘制红色边框
    renderer.draw_h_line(100, 100, 200, Color::red())?;
    renderer.draw_h_line(100, 250, 200, Color::red())?;
    renderer.draw_v_line(100, 100, 150, Color::red())?;
    renderer.draw_v_line(300, 100, 150, Color::red())?;
    
    // 绘制像素点
    for i in 0..100 {
        let x = 150 + (i as i32 * 2).cos() as u32 * 50;
        let y = 175 + (i as i32 * 2).sin() as u32 * 50;
        renderer.draw_pixel(x, y, Color::yellow())?;
    }
    
    Ok(())
}
```

#### 双缓冲渲染

```rust
use nos_bootloader::graphics::{GraphicsRenderer, FramebufferInfo, Color};

fn example_double_buffering() -> Result<()> {
    let fb_info = FramebufferInfo::new(0xE0000000, 1024, 768, 4096, 32);
    let mut renderer = GraphicsRenderer::new_with_double_buffer(fb_info)?;
    
    // 初始化
    renderer.initialize_framebuffer()?;
    
    // 启用脏区域跟踪
    renderer.set_dirty_tracking(true);
    
    // 动画循环示例
    for frame in 0..100 {
        // 开始渲染会话
        renderer.begin_render()?;
        
        // 清屏
        renderer.clear_screen(Color::rgb(0, 51, 102))?;
        
        // 绘制移动的矩形
        let x = (frame * 5) % 800;
        let y = 200;
        renderer.draw_filled_rect(x, y, 100, 50, Color::green())?;
        
        // 结束渲染会话
        renderer.end_render()?;
        
        // 交换缓冲区（只更新脏区域）
        renderer.swap_buffers()?;
        
        // 短暂延迟（实际引导加载程序中可能不需要）
        // delay_ms(16);
    }
    
    Ok(())
}
```

#### 批量像素绘制

```rust
use nos_bootloader::graphics::{GraphicsRenderer, FramebufferInfo, Color};

fn example_batch_drawing() -> Result<()> {
    let fb_info = FramebufferInfo::new(0xE0000000, 1024, 768, 4096, 32);
    let mut renderer = GraphicsRenderer::new_with_double_buffer(fb_info)?;
    
    renderer.initialize_framebuffer()?;
    renderer.begin_render()?;
    
    // 创建像素数组
    let mut pixels = Vec::new();
    
    // 生成随机点
    for i in 0..1000 {
        let x = (i * 137) % 1024;  // 伪随机分布
        let y = (i * 89) % 768;
        let color = Color::rgb(
            ((i * 13) % 256) as u8,
            ((i * 17) % 256) as u8,
            ((i * 23) % 256) as u8,
        );
        
        pixels.push((x, y, color));
    }
    
    // 批量绘制像素
    renderer.draw_pixels_batch(&pixels)?;
    
    renderer.end_render()?;
    renderer.swap_buffers()?;
    
    Ok(())
}
```

### VBE图形模式

#### VBE模式设置

```rust
use nos_bootloader::graphics::vbe::{VbeController, VbeGraphicsManager};

fn example_vbe_modes() -> Result<()> {
    // 创建VBE控制器
    let mut controller = VbeController::new();
    
    // 初始化控制器
    controller.initialize()?;
    
    // 列出所有支持的模式
    let modes = controller.get_supported_modes();
    println!("支持的VBE模式数量: {}", modes.len());
    
    // 查找最佳模式
    if let Some(mode) = controller.find_best_mode(1920, 1080, 32) {
        println!("找到最佳模式: 0x{:04X}", mode);
    } else {
        println!("未找到1920x1080x32模式");
    }
    
    // 设置图形模式
    let mut graphics_manager = VbeGraphicsManager::new();
    graphics_manager.initialize()?;
    
    let fb_info = graphics_manager.set_mode(1024, 768, 32)?;
    println!("帧缓冲区地址: {:#X}", fb_info.address);
    println!("分辨率: {}x{}", fb_info.width, fb_info.height);
    
    Ok(())
}
```

### 引导菜单

#### 创建自定义菜单

```rust
use nos_bootloader::boot_menu::{BootMenu, UIMode, MenuOption};

fn example_custom_menu() -> Result<()> {
    // 创建图形模式菜单
    let mut menu = BootMenu::new(UIMode::Graphical);
    
    // 初始化菜单
    menu.initialize()?;
    
    // 添加自定义选项
    menu.add_option(MenuOption::new(1, "正常启动", "启动NOS操作系统"))?;
    menu.add_option(MenuOption::new(2, "安全模式", "以最小驱动启动"))?;
    menu.add_option(MenuOption::new(3, "恢复模式", "进入系统恢复"))?;
    
    // 添加带回调的选项
    let debug_option = MenuOption::with_callback(
        4,
        "调试模式",
        "启用详细调试信息",
        || {
            println!("进入调试模式");
            // 设置调试标志
            // enable_debug_mode();
            Ok(())
        }
    );
    menu.add_option(debug_option)?;
    
    // 处理用户输入
    loop {
        // 渲染菜单（需要图形渲染器）
        // menu.render_graphical(&mut renderer)?;
        
        // 等待输入
        // let key = wait_for_input();
        
        // 处理输入
        // if let Some(selected_id) = menu.process_input(key) {
        //     match selected_id {
        //         1 => { /* 正常启动 */ }
        //         2 => { /* 安全模式 */ }
        //         3 => { /* 恢复模式 */ }
        //         4 => { /* 调试模式 */ }
        //         _ => {}
        //     }
        //     break;
        // }
        
        break; // 示例中直接退出
    }
    
    Ok(())
}
```

#### 文本模式菜单

```rust
use nos_bootloader::boot_menu::{BootMenu, UIMode};

fn example_text_menu() -> Result<()> {
    let mut menu = BootMenu::new(UIMode::Text);
    menu.initialize()?;
    
    // 渲染文本菜单
    menu.render_text()?;
    
    // 模拟用户选择
    let selected_id = menu.process_input(b'\n'); // 模拟回车键
    
    if let Some(id) = selected_id {
        println!("用户选择了选项: {}", id);
        
        // 执行相应操作
        match id {
            1 => println!("启动内核..."),
            2 => println!("运行诊断..."),
            3 => println!("进入恢复模式..."),
            _ => println!("未知选项"),
        }
    }
    
    Ok(())
}
```

## 高级功能

### 硬件检测

#### CPU特性检测

```rust
use nos_bootloader::infrastructure::HardwareDetectionService;

fn example_cpu_detection() -> Result<()> {
    // 获取硬件检测服务
    let hw_service = create_hardware_detection_service(BootProtocolType::Uefi)?;
    
    // 检测硬件
    let hw_info = hw_service.detect_hardware()?;
    
    // 显示CPU信息
    println!("CPU型号: {}", hw_info.cpu.model);
    println!("CPU频率: {} MHz", hw_info.cpu.frequency_mhz);
    println!("核心数: {}", hw_info.cpu.cores);
    println!("线程数: {}", hw_info.cpu.threads);
    
    // 检查CPU特性
    if hw_info.cpu.features.has_sse2() {
        println!("支持SSE2");
    }
    if hw_info.cpu.features.has_avx2() {
        println!("支持AVX2");
    }
    if hw_info.cpu.features.has_long_mode() {
        println!("支持64位长模式");
    }
    
    Ok(())
}
```

#### 内存检测

```rust
use nos_bootloader::infrastructure::HardwareDetectionService;

fn example_memory_detection() -> Result<()> {
    let hw_service = create_hardware_detection_service(BootProtocolType::Uefi)?;
    let hw_info = hw_service.detect_hardware()?;
    
    // 显示内存信息
    println!("总内存: {} MB", hw_info.total_memory / (1024 * 1024));
    println!("可用内存: {} MB", hw_info.available_memory / (1024 * 1024));
    
    // 遍历内存区域
    for region in &hw_info.memory_regions {
        println!("区域: {:#X}-{:#X} ({:?})",
                 region.start,
                 region.end,
                 region.region_type);
        
        if region.is_available() {
            println!("  大小: {} MB", region.size() / (1024 * 1024));
        }
    }
    
    Ok(())
}
```

### 引导配置

#### 自定义配置

```rust
use nos_bootloader::domain::boot_config::{BootConfig, GraphicsMode, LogLevel};

fn example_custom_config() -> Result<()> {
    // 创建自定义配置
    let mut config = BootConfig::new();
    
    // 设置日志级别
    config.verbosity = LogLevel::Verbose;
    
    // 启用功能
    config.enable_post = true;
    config.enable_paging = true;
    config.enable_memory_check = true;
    config.enable_device_detect = true;
    
    // 设置图形模式
    config.graphics_mode = Some(GraphicsMode::new(1920, 1080, 32)?);
    
    // 设置内核路径
    let kernel_path = b"/EFI/NOS/kernel.efi";
    config.kernel_path = Some([0u8; 256]);
    if let Some(ref mut path) = config.kernel_path {
        path[..kernel_path.len()].copy_from_slice(kernel_path);
        config.kernel_path_len = kernel_path.len();
    }
    
    // 设置命令行
    let cmdline = b"quiet splash root=/dev/sda2";
    config.cmdline = Some([0u8; 512]);
    if let Some(ref mut cmd) = config.cmdline {
        cmd[..cmdline.len()].copy_from_slice(cmdline);
        config.cmdline_len = cmdline.len();
    }
    
    // 验证配置
    config.validate()?;
    
    println!("配置验证通过");
    println!("图形模式: {:?}", config.graphics_mode);
    println!("日志级别: {:?}", config.verbosity);
    
    Ok(())
}
```

### 事件系统

#### 事件订阅

```rust
use nos_bootloader::domain::events::{DomainEventPublisher, SimpleEventPublisher, LoggingSubscriber};

fn example_event_system() -> Result<()> {
    // 创建事件发布器
    let mut publisher = SimpleEventPublisher::new();
    
    // 创建日志订阅者
    let logging_subscriber = LoggingSubscriber::new();
    
    // 订阅事件
    publisher.subscribe(Box::new(logging_subscriber))?;
    
    // 发布事件
    let boot_event = Box::new(BootPhaseStartedEvent::new("initialization"));
    publisher.publish(boot_event)?;
    
    let graphics_event = Box::new(GraphicsInitializedEvent::new(1024, 768, 0xE0000000, 4096));
    publisher.publish(graphics_event)?;
    
    Ok(())
}
```

#### 自定义事件订阅者

```rust
use nos_bootloader::domain::events::{DomainEvent, DomainEventSubscriber, BootPhaseCompletedEvent};

struct CustomSubscriber;

impl DomainEventSubscriber for CustomSubscriber {
    fn handle_event(&self, event: &dyn DomainEvent) -> Result<()> {
        match event.event_type() {
            "BootPhaseCompleted" => {
                if let Some(phase_event) = event.as_any().downcast_ref::<BootPhaseCompletedEvent>() {
                    println!("引导阶段完成: {}", phase_event.phase());
                }
            }
            "GraphicsInitialized" => {
                println!("图形系统初始化完成");
            }
            _ => {
                println!("未知事件类型: {}", event.event_type());
            }
        }
        Ok(())
    }
}
```

## 最佳实践

### 内存管理

1. **使用双级分配器**:
   ```rust
   // 好的做法：使用分配器统计信息
   let allocator = DualLevelAllocator::new();
   if allocator.utilization() > 80.0 {
       println!("警告：内存使用率过高");
   }
   ```

2. **小块重用**:
   ```rust
   // 好的做法：小块会自动重用
   let layout = Layout::new::<u32>();
   let ptr1 = unsafe { allocator.alloc(layout) };
   unsafe { allocator.dealloc(ptr1, layout) };
   let ptr2 = unsafe { allocator.alloc(layout) }; // 可能重用ptr1的内存
   ```

3. **避免内存泄漏**:
   ```rust
   // 好的做法：及时释放不需要的内存
   let ptr = unsafe { allocator.alloc(layout) };
   // 使用ptr...
   unsafe { allocator.dealloc(ptr, layout) }; // 及时释放
   ```

### 图形渲染

1. **使用双缓冲**:
   ```rust
   // 好的做法：始终使用双缓冲避免闪烁
   let mut renderer = GraphicsRenderer::new_with_double_buffer(fb_info)?;
   renderer.begin_render()?;
   // 绘制操作...
   renderer.end_render()?;
   renderer.swap_buffers()?;
   ```

2. **启用脏区域跟踪**:
   ```rust
   // 好的做法：启用脏区域跟踪提高性能
   renderer.set_dirty_tracking(true);
   renderer.begin_render()?;
   // 只更新变化区域
   renderer.draw_filled_rect(100, 100, 50, 50, Color::red())?;
   renderer.end_render()?;
   renderer.swap_buffers()?; // 只更新脏区域
   ```

3. **批量操作**:
   ```rust
   // 好的做法：使用批量操作减少函数调用
   let mut pixels = Vec::new();
   for i in 0..1000 {
       pixels.push((x, y, color));
   }
   renderer.draw_pixels_batch(&pixels)?;
   ```

### 错误处理

1. **统一错误处理**:
   ```rust
   // 好的做法：使用Result和?操作符
   fn boot_system() -> Result<()> {
       let config = load_config()?;
       let hw_info = detect_hardware()?;
       validate_prerequisites(&config, &hw_info)?;
       initialize_graphics(&config)?;
       Ok(())
   }
   ```

2. **提供有意义的错误信息**:
   ```rust
   // 好的做法：提供详细的错误信息
   Err(BootError::HardwareError("VBE控制器初始化失败：无法获取控制器信息"))
   ```

3. **错误恢复**:
   ```rust
   // 好的做法：提供错误恢复机制
   match initialize_graphics(&config) {
       Ok(_) => Ok(()),
       Err(e) => {
           println!("图形初始化失败，回退到文本模式: {}", e);
           fallback_to_text_mode();
           Ok(())
       }
   }
   ```

## 常见陷阱

### 内存相关陷阱

1. **忘记对齐**:
   ```rust
   // 错误：未考虑内存对齐
   let layout = Layout::from_size_align(15, 1)?; // 15字节，1字节对齐
   
   // 正确：使用适当对齐
   let layout = Layout::from_size_align(15, 8)?; // 15字节，8字节对齐
   ```

2. **忽略分配失败**:
   ```rust
   // 错误：未检查分配结果
   let ptr = unsafe { allocator.alloc(layout) };
   *ptr = 42; // 可能空指针解引用
   
   // 正确：检查分配结果
   let ptr = unsafe { allocator.alloc(layout) };
   if ptr.is_null() {
       return Err(BootError::OutOfMemory);
   }
   ```

3. **内存泄漏**:
   ```rust
   // 错误：忘记释放内存
   fn process_data() {
       let ptr = unsafe { allocator.alloc(layout) };
       // 使用ptr但忘记释放
   }
   
   // 正确：确保释放内存
   fn process_data() {
       let ptr = unsafe { allocator.alloc(layout) };
       // 使用ptr
       unsafe { allocator.dealloc(ptr, layout) };
   }
   ```

### 图形相关陷阱

1. **坐标越界**:
   ```rust
   // 错误：未检查坐标边界
   renderer.draw_pixel(2000, 2000, Color::red())?; // 可能越界
   
   // 正确：检查坐标或使用安全API
   if renderer.point_in_bounds(100, 100) {
       renderer.draw_pixel(100, 100, Color::red())?;
   }
   ```

2. **忘记交换缓冲区**:
   ```rust
   // 错误：忘记交换缓冲区
   renderer.begin_render()?;
   renderer.draw_filled_rect(100, 100, 50, 50, Color::red())?;
   renderer.end_render()?;
   // 忘记调用swap_buffers()，绘制内容不会显示
   
   // 正确：记得交换缓冲区
   renderer.begin_render()?;
   renderer.draw_filled_rect(100, 100, 50, 50, Color::red())?;
   renderer.end_render()?;
   renderer.swap_buffers()?; // 显示绘制内容
   ```

3. **渲染会话嵌套**:
   ```rust
   // 错误：嵌套渲染会话
   renderer.begin_render()?;
   draw_complex_shape(&mut renderer)?; // 内部也调用了begin_render()
   renderer.end_render()?;
   
   // 正确：避免嵌套或检查状态
   fn draw_complex_shape(renderer: &mut GraphicsRenderer) -> Result<()> {
       if !renderer.is_rendering() {
           renderer.begin_render()?;
           // 绘制操作...
           renderer.end_render()?;
           renderer.swap_buffers()?;
       }
       Ok(())
   }
   ```

### 配置相关陷阱

1. **未验证配置**:
   ```rust
   // 错误：未验证配置
   let mut config = BootConfig::new();
   config.graphics_mode = Some(GraphicsMode::new(10000, 10000, 32)?); // 可能无效
   // 直接使用未验证的配置
   
   // 正确：验证配置
   let mut config = BootConfig::new();
   config.graphics_mode = Some(GraphicsMode::new(10000, 10000, 32)?);
   config.validate()?; // 验证配置
   ```

2. **忽略配置依赖**:
   ```rust
   // 错误：忽略配置依赖
   config.enable_paging = false;
   config.graphics_mode = Some(GraphicsMode::new(1920, 1080, 32)?); // 高分辨率需要分页
   
   // 正确：考虑配置依赖
   if config.graphics_mode.is_some() && config.graphics_mode.unwrap().is_high_resolution() {
       config.enable_paging = true; // 高分辨率需要分页
   }
   ```

## 故障排除

### 常见问题

1. **引导失败**:
   - 检查引导协议是否正确配置
   - 验证内核镜像是否存在
   - 确认内存布局是否正确

2. **图形问题**:
   - 检查VBE/GOP初始化
   - 验证帧缓冲区地址
   - 确认图形模式是否支持

3. **内存问题**:
   - 检查内存分配是否成功
   - 验证内存对齐
   - 确认没有内存泄漏

### 调试技巧

1. **启用详细日志**:
   ```rust
   let mut config = BootConfig::new();
   config.verbosity = LogLevel::Debug; // 启用调试日志
   ```

2. **使用性能分析**:
   ```rust
   use nos_bootloader::diagnostics::BootTimer;
   
   let timer = BootTimer::new();
   // 执行操作...
   println!("操作耗时: {:.2} ms", timer.elapsed_ms());
   ```

3. **内存状态检查**:
   ```rust
   let allocator = DualLevelAllocator::new();
   println!("内存使用: {} / {} ({:.1}%)",
            allocator.allocated(),
            BOOTLOADER_HEAP_SIZE,
            allocator.utilization());
   ```

## 性能优化

### 内存优化

1. **使用适当的数据结构**:
   ```rust
   // 好的做法：使用Vec预分配容量
   let mut pixels = Vec::with_capacity(1000);
   
   // 好的做法：使用小数组避免堆分配
   let small_buffer = [0u8; 64]; // 栈分配
   ```

2. **减少内存分配**:
   ```rust
   // 好的做法：重用缓冲区
   static mut DRAW_BUFFER: [u8; 1024 * 768 * 4] = [0; 1024 * 768 * 4];
   
   fn draw_frame() {
       // 使用静态缓冲区避免重复分配
       let buffer = unsafe { &mut DRAW_BUFFER };
       // 绘制操作...
   }
   ```

### 图形优化

1. **使用SIMD指令**:
   ```rust
   // 好的做法：使用SIMD优化的批量操作
   renderer.draw_pixels_batch(&pixels)?; // 内部使用SIMD优化
   
   // 好的做法：对齐内存访问
   let aligned_data = align_to_64_bytes(data);
   ```

2. **减少绘制调用**:
   ```rust
   // 好的做法：合并相邻绘制操作
   renderer.draw_filled_rect(100, 100, 50, 50, Color::red())?;
   renderer.draw_filled_rect(150, 100, 50, 50, Color::red())?;
   
   // 更好的做法：合并为一个矩形
   renderer.draw_filled_rect(100, 100, 100, 50, Color::red())?;
   ```

3. **启用脏区域跟踪**:
   ```rust
   // 好的做法：只更新变化区域
   renderer.set_dirty_tracking(true);
   renderer.begin_render()?;
   // 只绘制变化的部分...
   renderer.end_render()?;
   renderer.swap_buffers()?; // 只更新脏区域
   ```

### 算法优化

1. **使用高效算法**:
   ```rust
   // 好的做法：使用哈希表O(1)查找
   use hashbrown::HashMap;
   let mut mode_cache = HashMap::new();
   mode_cache.insert(mode_id, mode_info);
   let info = mode_cache.get(&mode_id); // O(1)查找
   
   // 避免：线性搜索O(n)
   // let info = modes.iter().find(|m| m.id == mode_id);
   ```

2. **避免重复计算**:
   ```rust
   // 好的做法：缓存计算结果
   let scanline_bytes = mode.scanline_bytes();
   for y in 0..height {
       let offset = y * scanline_bytes; // 重用计算结果
       // ...
   }
   ```

这个使用指南为NOS引导加载程序提供了全面的使用示例和最佳实践，帮助开发者快速上手并避免常见问题。