# NOS Bootloader API 文档

本文档提供NOS引导加载程序的公共API详细文档，包括使用示例和边界条件说明。

## 目录

- [核心模块](#核心模块)
- [基础设施层](#基础设施层)
- [应用层](#应用层)
- [领域层](#领域层)
- [图形子系统](#图形子系统)
- [引导菜单](#引导菜单)

## 核心模块

### 内存分配器 (`core::allocator`)

#### `DualLevelAllocator`

双级内存分配器，专为引导加载程序环境设计，结合了bump分配器和分离空闲列表的优势。

```rust
use nos_bootloader::core::allocator::DualLevelAllocator;

// 创建新的分配器
let allocator = DualLevelAllocator::new();

// 获取内存使用统计
let allocated = allocator.allocated();
let free = allocator.free();
let utilization = allocator.utilization();
```

**方法文档:**

- `new()`: 创建新的双级分配器实例
  - 返回: `DualLevelAllocator`
  - 注意: 分配器初始化为空堆，4MB大小

- `allocated()`: 获取已分配内存字节数
  - 返回: `usize`
  - 线程安全: 是，使用原子操作

- `free()`: 获取剩余可用内存字节数
  - 返回: `usize`
  - 计算: `BOOTLOADER_HEAP_SIZE - allocated()`

- `utilization()`: 获取内存利用率百分比
  - 返回: `f32` (0.0-100.0)
  - 公式: `(allocated / BOOTLOADER_HEAP_SIZE) * 100.0`

**边界条件:**
- 最大分配大小: 4MB
- 小块阈值: 256字节
- 对齐要求: 64字节边界
- 空闲列表最大条目: 1024

## 基础设施层

### 依赖注入容器 (`infrastructure::di_container`)

#### `BootDIContainer`

管理引导加载程序服务的生命周期和依赖关系，实现控制反转以提高可测试性。

```rust
use nos_bootloader::infrastructure::BootDIContainer;
use nos_bootloader::protocol::BootProtocolType;

// 创建并初始化容器
let mut container = BootDIContainer::new(BootProtocolType::Bios);
container.initialize()?;

// 获取服务
let hw_service = container.hardware_detection_service()?;
let graphics_backend = container.graphics_backend()?;
```

**方法文档:**

- `new(protocol_type)`: 创建新的DI容器
  - 参数: `protocol_type: BootProtocolType`
  - 返回: `BootDIContainer`
  - 说明: 根据协议类型初始化相应服务

- `initialize()`: 初始化所有服务
  - 返回: `Result<(), BootError>`
  - 错误: 初始化失败时返回错误

- `hardware_detection_service()`: 获取硬件检测服务
  - 返回: `Result<Box<dyn HardwareDetectionService>, BootError>`
  - 错误: 服务不可用时返回错误

### 图形后端 (`infrastructure::graphics_backend`)

#### `GraphicsBackend`

提供硬件无关的图形操作抽象，支持BIOS VBE和UEFI GOP。

```rust
use nos_bootloader::infrastructure::GraphicsBackend;

// 初始化图形后端
let mut backend = GraphicsBackend::new()?;

// 设置图形模式
let fb_info = backend.set_mode(1024, 768, 32)?;

// 获取帧缓冲区信息
let address = fb_info.address;
let width = fb_info.width;
let height = fb_info.height;
```

**方法文档:**

- `new()`: 创建新的图形后端实例
  - 返回: `Result<GraphicsBackend, BootError>`
  - 错误: 初始化失败时返回错误

- `set_mode(width, height, bpp)`: 设置图形模式
  - 参数:
    - `width: u16` - 宽度 (320-4096)
    - `height: u16` - 高度 (200-2160)
    - `bpp: u8` - 色深 (8, 16, 24, 32)
  - 返回: `Result<FramebufferInfo, BootError>`
  - 错误: 不支持的分辨率或色深

## 应用层

### 引导编排器 (`application::boot_orchestrator`)

#### `BootApplicationService`

高级服务，实现完整的引导序列，协调领域服务和基础设施组件。

```rust
use nos_bootloader::application::BootApplicationService;
use nos_bootloader::protocol::BootProtocolType;

// 创建引导应用服务
let mut service = BootApplicationService::new(BootProtocolType::Bios)?;

// 执行完整引导序列
let boot_info = service.boot_system(Some("quiet splash"))?;

// 获取硬件检测服务
let hw_service = service.hardware_detection_service();
let hw_info = hw_service.detect_hardware()?;
```

**方法文档:**

- `new(protocol_type)`: 创建新的引导应用服务
  - 参数: `protocol_type: BootProtocolType`
  - 返回: `Result<BootApplicationService, BootError>`
  - 错误: 初始化失败时返回错误

- `boot_system(cmdline)`: 执行完整引导序列
  - 参数: `cmdline: Option<&str>` - 内核命令行
  - 返回: `Result<BootInfo, BootError>`
  - 流程:
    1. 加载配置
    2. 检测硬件
    3. 验证先决条件
    4. 初始化图形
    5. 创建引导信息
    6. 验证引导信息

- `hardware_detection_service()`: 获取硬件检测服务引用
  - 返回: `&dyn HardwareDetectionService`
  - 说明: 遵循依赖倒置原则

## 领域层

### 引导配置 (`domain::boot_config`)

#### `BootConfig`

不可变配置值对象，封装所有引导加载程序配置。

```rust
use nos_bootloader::domain::boot_config::{BootConfig, GraphicsMode, LogLevel};

// 创建默认配置
let config = BootConfig::default();

// 验证配置
config.validate()?;

// 创建自定义图形模式
let mode = GraphicsMode::new(1920, 1080, 32)?;
```

#### `GraphicsMode`

图形显示模式值对象，表示有效的图形显示设置。

```rust
use nos_bootloader::domain::boot_config::GraphicsMode;

// 创建图形模式
let mode = GraphicsMode::new(1024, 768, 32)?;

// 检查是否为高分辨率
if mode.is_high_resolution() {
    println!("高分辨率模式");
}

// 计算帧缓冲区大小
let fb_size = mode.framebuffer_size();
let scanline_bytes = mode.scanline_bytes();
```

**方法文档:**

- `new(width, height, bpp)`: 创建新的图形模式
  - 参数:
    - `width: u16` - 宽度 (320-4096)
    - `height: u16` - 高度 (200-2160)
    - `bpp: u8` - 色深 (8, 16, 24, 32)
  - 返回: `Result<GraphicsMode, &'static str>`
  - 错误: 无效的分辨率或色深

- `is_high_resolution()`: 检查是否为高分辨率模式
  - 返回: `bool`
  - 条件: width >= 1024 && height >= 768

- `scanline_bytes()`: 获取扫描线字节数（64字节对齐）
  - 返回: `usize`
  - 计算: `((width * bytes_per_pixel + 63) / 64) * 64`

- `framebuffer_size()`: 获取总帧缓冲区大小
  - 返回: `usize`
  - 计算: `scanline_bytes() * height`

#### `MemoryRegion`

内存区域值对象，表示内存区域及其类型和属性。

```rust
use nos_bootloader::domain::boot_config::{MemoryRegion, MemoryRegionType};

// 创建可用内存区域
let region = MemoryRegion::new(0x100000, 0x1000000, MemoryRegionType::Available)?;

// 检查区域大小
let size = region.size();

// 检查是否包含特定地址
if region.contains(0x200000) {
    println!("地址在区域内");
}

// 检查与另一个区域是否重叠
let other = MemoryRegion::new(0x200000, 0x300000, MemoryRegionType::Reserved)?;
if region.overlaps(&other) {
    println!("区域重叠");
}
```

**方法文档:**

- `new(start, end, region_type)`: 创建新的内存区域
  - 参数:
    - `start: u64` - 起始物理地址
    - `end: u64` - 结束物理地址（不包含）
    - `region_type: MemoryRegionType` - 区域类型
  - 返回: `Result<MemoryRegion, &'static str>`
  - 错误: start >= end 时返回错误

- `size()`: 获取内存区域大小
  - 返回: `u64`
  - 计算: `end - start`

- `overlaps(other)`: 检查与另一个区域是否重叠
  - 参数: `other: &MemoryRegion`
  - 返回: `bool`
  - 条件: `start < other.end && other.start < end`

- `contains(addr)`: 检查是否包含特定地址
  - 参数: `addr: u64`
  - 返回: `bool`
  - 条件: `addr >= start && addr < end`

### 引导信息 (`domain::boot_info`)

#### `BootInfo`

引导信息聚合根，包含传递给内核的所有信息。

```rust
use nos_bootloader::domain::boot_info::BootInfo;
use nos_bootloader::protocol::BootProtocolType;

// 创建新的引导信息
let mut boot_info = BootInfo::new(BootProtocolType::Bios);

// 设置内核信息
boot_info.kernel_address = 0x100000;
boot_info.kernel_size = 0x500000;
boot_info.boot_timestamp = 0;

// 验证引导信息
boot_info.validate()?;
```

## 图形子系统

### 图形渲染器 (`graphics`)

#### `GraphicsRenderer`

ARGB8888帧缓冲区的图形渲染器，支持双缓冲。

```rust
use nos_bootloader::graphics::{GraphicsRenderer, FramebufferInfo, Color};

// 创建帧缓冲区信息
let fb_info = FramebufferInfo::new(0x10000000, 1024, 768, 4096, 32);

// 创建图形渲染器
let mut renderer = GraphicsRenderer::new_with_double_buffer(fb_info)?;

// 初始化帧缓冲区
renderer.initialize_framebuffer()?;

// 清屏
renderer.clear_screen(Color::black())?;

// 绘制像素
renderer.draw_pixel(100, 100, Color::red())?;

// 绘制水平线
renderer.draw_h_line(50, 50, 100, Color::green())?;

// 绘制垂直线
renderer.draw_v_line(150, 50, 100, Color::blue())?;

// 绘制填充矩形
renderer.draw_filled_rect(200, 200, 100, 50, Color::white())?;

// 开始渲染会话
renderer.begin_render()?;

// 执行绘制操作...

// 结束渲染会话
renderer.end_render()?;

// 交换缓冲区
renderer.swap_buffers()?;
```

**方法文档:**

- `new(fb)`: 创建新的图形渲染器
  - 参数: `fb: FramebufferInfo` - 帧缓冲区信息
  - 返回: `GraphicsRenderer`

- `new_with_double_buffer(fb)`: 创建带双缓冲的图形渲染器
  - 参数: `fb: FramebufferInfo` - 帧缓冲区信息
  - 返回: `Result<GraphicsRenderer, &'static str>`
  - 错误: 后台缓冲区分配失败

- `initialize_framebuffer()`: 初始化帧缓冲区（零填充）
  - 返回: `Result<(), &'static str>`
  - 错误: 帧缓冲区地址为空

- `clear_screen(color)`: 用单一颜色清屏
  - 参数: `color: Color` - 清屏颜色
  - 返回: `Result<(), &'static str>`
  - 错误: 帧缓冲区地址为空

- `draw_pixel(x, y, color)`: 在坐标处绘制单个像素
  - 参数:
    - `x: u32` - X坐标
    - `y: u32` - Y坐标
    - `color: Color` - 像素颜色
  - 返回: `Result<(), &'static str>`
  - 错误: 坐标超出边界或地址为空

- `draw_h_line(x, y, length, color)`: 绘制水平线
  - 参数:
    - `x: u32` - 起始X坐标
    - `y: u32` - Y坐标
    - `length: u32` - 线长度
    - `color: Color` - 线颜色
  - 返回: `Result<(), &'static str>`
  - 错误: 起始坐标超出边界

- `draw_v_line(x, y, length, color)`: 绘制垂直线
  - 参数:
    - `x: u32` - X坐标
    - `y: u32` - 起始Y坐标
    - `length: u32` - 线长度
    - `color: Color` - 线颜色
  - 返回: `Result<(), &'static str>`
  - 错误: 起始坐标超出边界

- `draw_filled_rect(x, y, width, height, color)`: 绘制填充矩形
  - 参数:
    - `x: u32` - 矩形X坐标
    - `y: u32` - 矩形Y坐标
    - `width: u32` - 矩形宽度
    - `height: u32` - 矩形高度
    - `color: Color` - 填充颜色
  - 返回: `Result<(), &'static str>`
  - 错误: 起始坐标超出边界

#### `Color`

ARGB8888颜色表示。

```rust
use nos_bootloader::graphics::Color;

// 创建不透明RGB颜色
let red = Color::rgb(255, 0, 0);
let green = Color::rgb(0, 255, 0);
let blue = Color::rgb(0, 0, 255);

// 创建ARGB颜色
let semi_transparent_red = Color::argb(128, 255, 0, 0);

// 转换为ARGB8888格式
let argb_value = red.as_argb8888();

// 使用预定义颜色
let black = Color::black();
let white = Color::white();
```

**方法文档:**

- `rgb(red, green, blue)`: 创建不透明RGB颜色
  - 参数:
    - `red: u8` - 红色分量 (0-255)
    - `green: u8` - 绿色分量 (0-255)
    - `blue: u8` - 蓝色分量 (0-255)
  - 返回: `Color`
  - 说明: Alpha分量自动设置为255（不透明）

- `argb(alpha, red, green, blue)`: 创建ARGB颜色
  - 参数:
    - `alpha: u8` - Alpha分量 (0-255)
    - `red: u8` - 红色分量 (0-255)
    - `green: u8` - 绿色分量 (0-255)
    - `blue: u8` - 蓝色分量 (0-255)
  - 返回: `Color`

- `as_argb8888()`: 转换为ARGB8888 u32格式
  - 返回: `u32`
  - 格式: `0xAARRGGBB`

#### `DoubleBuffer`

双缓冲管理器，提供无闪烁渲染和脏区域跟踪。

```rust
use nos_bootloader::graphics::{DoubleBuffer, FramebufferInfo};

// 创建帧缓冲区信息
let fb_info = FramebufferInfo::new(0x10000000, 1024, 768, 4096, 32);

// 创建双缓冲管理器
let mut double_buffer = DoubleBuffer::new(fb_info)?;

// 开始渲染会话
double_buffer.begin_render()?;

// 执行渲染操作...

// 结束渲染会话
double_buffer.end_render()?;

// 交换缓冲区
double_buffer.swap_buffers()?;

// 启用/禁用脏区域跟踪
double_buffer.set_dirty_tracking(true);
```

**方法文档:**

- `new(fb_info)`: 创建新的双缓冲管理器
  - 参数: `fb_info: FramebufferInfo` - 帧缓冲区信息
  - 返回: `Result<DoubleBuffer, &'static str>`
  - 错误: 帧缓冲区地址为空或后台缓冲区分配失败

- `begin_render()`: 开始渲染会话
  - 返回: `Result<(), &'static str>`
  - 错误: 已在渲染状态

- `end_render()`: 结束渲染会话
  - 返回: `Result<(), &'static str>`
  - 错误: 不在渲染状态

- `swap_buffers()`: 交换缓冲区
  - 返回: `Result<(), &'static str>`
  - 错误: 正在渲染时不能交换

- `set_dirty_tracking(enabled)`: 启用/禁用脏区域跟踪
  - 参数: `enabled: bool` - 是否启用脏区域跟踪

### VBE支持 (`graphics::vbe`)

#### `VbeController`

VESA BIOS扩展(VBE)控制器接口，支持模式缓存。

```rust
use nos_bootloader::graphics::vbe::{VbeController, VBE_MODE_1024x768x32};

// 创建VBE控制器
let mut controller = VbeController::new();

// 初始化控制器
controller.initialize()?;

// 设置图形模式
let fb_info = controller.set_graphics_mode(1024, 768, 32)?;

// 查找最佳模式
let mode = controller.find_best_mode(1920, 1080, 32);

// 获取支持的模式列表
let modes = controller.get_supported_modes();
```

**方法文档:**

- `new()`: 创建新的VBE控制器
  - 返回: `VbeController`

- `initialize()`: 初始化VBE控制器
  - 返回: `Result<(), BootError>`
  - 错误: VBE控制器信息获取失败或签名无效

- `set_graphics_mode(width, height, bpp)`: 设置图形模式
  - 参数:
    - `width: u16` - 宽度
    - `height: u16` - 高度
    - `bpp: u8` - 色深
  - 返回: `Result<FramebufferInfo, BootError>`
  - 错误: 未初始化或未找到合适模式

- `find_best_mode(width, height, bpp)`: 查找最佳VBE模式
  - 参数:
    - `width: u16` - 期望宽度
    - `height: u16` - 期望高度
    - `bpp: u8` - 期望色深
  - 返回: `Option<u16>`
  - 说明: 首先尝试精确匹配，然后尝试更高分辨率

## 引导菜单

### 统一引导菜单 (`boot_menu`)

#### `BootMenu`

统一引导菜单接口，支持图形和文本UI模式。

```rust
use nos_bootloader::boot_menu::{BootMenu, UIMode, MenuOption};

// 创建文本模式引导菜单
let mut menu = BootMenu::new(UIMode::Text);

// 初始化菜单
menu.initialize()?;

// 添加自定义选项
let custom_option = MenuOption::new(4, "Custom", "Custom boot option");
menu.add_option(custom_option)?;

// 处理输入
let selected_id = menu.process_input(b'\n');

// 获取当前选中项
if let Some(selected) = menu.get_selected() {
    println!("选中: {}", selected.name);
}
```

**方法文档:**

- `new(mode)`: 创建新的引导菜单
  - 参数: `mode: UIMode` - UI模式（图形/文本/串行）
  - 返回: `BootMenu`

- `initialize()`: 初始化菜单（延迟初始化以提高性能）
  - 返回: `Result<(), BootError>`
  - 说明: 添加默认引导选项

- `add_option(option)`: 添加菜单选项
  - 参数: `option: MenuOption` - 菜单选项
  - 返回: `Result<(), BootError>`
  - 错误: 菜单选项已满（最多8个）

- `process_input(key)`: 处理键盘输入
  - 参数: `key: u8` - 按键码
  - 返回: `Option<u8>` - 选中的选项ID
  - 说明: 支持上下箭头导航和回车选择

- `render_graphical(renderer)`: 在图形模式下渲染菜单
  - 参数: `renderer: &mut GraphicsRenderer` - 图形渲染器
  - 返回: `Result<(), BootError>`

- `render_text()`: 在文本模式下渲染菜单
  - 返回: `Result<(), BootError>`

#### `MenuOption`

引导菜单选项。

```rust
use nos_bootloader::boot_menu::MenuOption;

// 创建简单菜单选项
let option1 = MenuOption::new(1, "Boot", "Normal boot");

// 创建带回调的菜单选项
let option2 = MenuOption::with_callback(
    2,
    "Recovery",
    "Boot into recovery mode",
    || {
        println!("进入恢复模式");
        Ok(())
    }
);
```

**方法文档:**

- `new(id, name, description)`: 创建新的菜单选项
  - 参数:
    - `id: u8` - 选项ID
    - `name: &'static str` - 选项名称
    - `description: &'static str` - 选项描述
  - 返回: `MenuOption`

- `with_callback(id, name, description, callback)`: 创建带回调的菜单选项
  - 参数:
    - `id: u8` - 选项ID
    - `name: &'static str` - 选项名称
    - `description: &'static str` - 选项描述
    - `callback: fn() -> Result<()>` - 回调函数
  - 返回: `MenuOption`

## 错误处理

所有公共API都使用统一的错误处理机制：

```rust
use nos_bootloader::utils::error::{BootError, Result};

// 定义返回类型
fn example_function() -> Result<()> {
    // 可能失败的操作
    if some_condition {
        Err(BootError::HardwareError("硬件错误描述"))
    } else {
        Ok(())
    }
}

// 处理错误
match example_function() {
    Ok(_) => println!("操作成功"),
    Err(e) => println!("操作失败: {:?}", e),
}
```

### 错误类型

- `BootError::HardwareError`: 硬件相关错误
- `BootError::ProtocolInitializationFailed`: 协议初始化失败
- `BootError::NotInitialized`: 组件未初始化
- `BootError::DeviceError`: 设备错误
- `BootError::InvalidParameter`: 无效参数

## 最佳实践

1. **内存管理**:
   - 使用双缓冲避免闪烁
   - 启用脏区域跟踪提高性能
   - 注意内存对齐要求

2. **错误处理**:
   - 始终检查返回结果
   - 使用`?`操作符传播错误
   - 提供有意义的错误信息

3. **性能优化**:
   - 使用批量操作减少函数调用开销
   - 利用SIMD优化（x86_64平台）
   - 缓存频繁访问的数据

4. **安全考虑**:
   - 验证所有输入参数
   - 检查边界条件
   - 正确处理unsafe代码

## 常见陷阱

1. **坐标系统**:
   - 图形坐标从(0,0)开始
   - 注意边界检查避免越界

2. **内存对齐**:
   - 所有分配64字节对齐
   - 小块使用空闲列表重用

3. **双缓冲**:
   - 必须在渲染会话中进行绘制
   - 记得交换缓冲区显示结果

4. **VBE模式**:
   - 检查模式是否支持
   - 使用线性帧缓冲区模式