# NOS Bootloader 模块文档

本文档提供NOS引导加载程序各模块的详细说明，包括模块职责、使用场景和模块间交互。

## 目录

- [架构概述](#架构概述)
- [核心模块](#核心模块)
- [领域层](#领域层)
- [基础设施层](#基础设施层)
- [应用层](#应用层)
- [图形子系统](#图形子系统)
- [引导菜单](#引导菜单)
- [BIOS支持](#bios支持)
- [固件接口](#固件接口)
- [内存管理](#内存管理)
- [CPU初始化](#cpu初始化)
- [设备驱动](#设备驱动)
- [安全模块](#安全模块)
- [内核接口](#内核接口)
- [引导协议](#引导协议)
- [引导阶段](#引导阶段)
- [诊断系统](#诊断系统)
- [优化模块](#优化模块)
- [ACPI支持](#acpi支持)
- [平台抽象](#平台抽象)
- [工具库](#工具库)

## 架构概述

NOS引导加载程序采用分层架构，遵循领域驱动设计(DDD)原则：

```
┌─────────────────────────────────────────────────────────────┐
│                    应用层 (Application)                    │
│  ┌─────────────────┐  ┌─────────────────┐                │
│  │ BootOrchestrator│  │   BootMenu      │                │
│  └─────────────────┘  └─────────────────┘                │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│                     领域层 (Domain)                       │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐   │
│  │BootConfig  │ │  BootInfo   │ │   BootServices     │   │
│  └─────────────┘ └─────────────┘ └─────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│                  基础设施层 (Infrastructure)               │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐   │
│  │DIContainer │ │GraphicsBackend│ │HardwareDetection   │   │
│  └─────────────┘ └─────────────┘ └─────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│                     核心层 (Core)                         │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐   │
│  │ Allocator   │ │  BootState  │ │   BootSequence     │   │
│  └─────────────┘ └─────────────┘ └─────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 分层原则

1. **应用层**: 协调领域服务，实现用例
2. **领域层**: 包含业务逻辑和领域模型
3. **基础设施层**: 提供技术实现和外部接口
4. **核心层**: 提供基础框架和工具

## 核心模块

### `core` - 核心框架

**职责**:
- 提供引导加载程序的基础框架
- 实现内存分配器
- 管理引导状态和序列
- 处理初始化逻辑

**主要组件**:
- `allocator`: 双级内存分配器
- `boot_state`: 引导状态管理
- `boot_sequence`: 引导序列控制
- `init`: 初始化逻辑

**使用场景**:
```rust
// 初始化内存分配器
use nos_bootloader::core::allocator::DualLevelAllocator;

let allocator = DualLevelAllocator::new();
let allocated = allocator.allocated();
let utilization = allocator.utilization();

// 管理引导状态
use nos_bootloader::core::boot_state::BootState;

let mut state = BootState::new();
state.transition_to(BootPhase::HardwareDetection);
```

**与其他模块交互**:
- 为所有其他模块提供内存分配服务
- 被应用层用于控制引导流程
- 被基础设施层用于管理初始化状态

## 领域层

### `domain` - 领域模型

**职责**:
- 定义核心业务概念
- 实现业务规则和验证
- 提供领域服务接口
- 发布领域事件

**主要组件**:
- `boot_config`: 引导配置值对象
- `boot_info`: 引导信息聚合根
- `boot_services`: 引导领域服务
- `events`: 领域事件系统
- `hardware_detection`: 硬件检测领域服务

**使用场景**:
```rust
// 创建和验证配置
use nos_bootloader::domain::{BootConfig, GraphicsMode};

let mut config = BootConfig::new();
config.graphics_mode = Some(GraphicsMode::new(1024, 768, 32)?);
config.validate()?;

// 使用领域服务
use nos_bootloader::domain::boot_services::BootValidator;

BootValidator::validate_prerequisites(&config, &hw_info)?;
```

**与其他模块交互**:
- 被应用层调用实现业务逻辑
- 使用基础设施层提供的服务实现
- 发布事件给订阅者（如诊断系统）

### `domain::boot_config`

**职责**:
- 定义引导配置数据结构
- 实现配置验证规则
- 提供配置默认值

**关键类型**:
- `BootConfig`: 主配置对象
- `GraphicsMode`: 图形模式配置
- `MemoryRegion`: 内存区域定义
- `LogLevel`: 日志级别枚举
- `BootPhase`: 引导阶段状态

**验证规则**:
- 图形模式分辨率限制: 320-4096 x 200-2160
- 色深支持: 8, 16, 24, 32位
- 高分辨率图形需要分页支持
- 内存检查需要分页支持

### `domain::boot_info`

**职责**:
- 聚合引导信息
- 提供内核接口所需数据
- 验证引导信息完整性

**关键类型**:
- `BootInfo`: 引导信息聚合根
- `KernelInfo`: 内核信息
- `GraphicsInfo`: 图形信息

### `domain::boot_services`

**职责**:
- 实现跨实体的业务逻辑
- 提供领域服务接口
- 协调多个领域对象

**关键服务**:
- `BootValidator`: 验证引导先决条件
- `GraphicsModeSelector`: 选择图形模式
- `MemoryManager`: 内存管理服务
- `KernelLoader`: 内核加载服务

### `domain::events`

**职责**:
- 定义领域事件
- 实现事件发布订阅
- 支持事件过滤和路由

**关键类型**:
- `DomainEvent`: 事件特征
- `DomainEventPublisher`: 事件发布器
- `DomainEventSubscriber`: 事件订阅者

**事件类型**:
- `BootPhaseCompletedEvent`: 引导阶段完成
- `GraphicsInitializedEvent`: 图形初始化完成
- `KernelLoadedEvent`: 内核加载完成
- `ValidationFailedEvent`: 验证失败

## 基础设施层

### `infrastructure` - 基础设施

**职责**:
- 实现领域定义的接口
- 提供硬件抽象
- 管理外部依赖
- 实现依赖注入

**主要组件**:
- `di_container`: 依赖注入容器
- `graphics_backend`: 图形后端实现
- `hardware_detection`: 硬件检测实现

**使用场景**:
```rust
// 使用依赖注入容器
use nos_bootloader::infrastructure::BootDIContainer;

let mut container = BootDIContainer::new(BootProtocolType::Bios);
container.initialize()?;

let hw_service = container.hardware_detection_service()?;

// 使用图形后端
use nos_bootloader::infrastructure::GraphicsBackend;

let mut backend = GraphicsBackend::new()?;
let fb_info = backend.set_mode(1024, 768, 32)?;
```

**与其他模块交互**:
- 实现领域层定义的接口
- 被应用层通过依赖注入使用
- 与核心层交互进行底层操作

### `infrastructure::di_container`

**职责**:
- 管理服务生命周期
- 实现依赖注入
- 提供服务工厂

**关键功能**:
- 服务注册和解析
- 依赖关系管理
- 生命周期控制
- 平台特定实现选择

### `infrastructure::graphics_backend`

**职责**:
- 实现图形后端接口
- 抽象不同图形协议
- 提供统一的图形API

**支持的后端**:
- BIOS VBE (VESA BIOS Extensions)
- UEFI GOP (Graphics Output Protocol)

### `infrastructure::hardware_detection`

**职责**:
- 实现硬件检测服务
- 提供统一硬件信息
- 支持多种检测方式

**检测能力**:
- CPU信息和特性
- 内存布局和大小
- 图形设备能力
- 系统总线信息

## 应用层

### `application` - 应用服务

**职责**:
- 实现用例编排
- 协调领域服务
- 处理应用程序流程
- 提供高级API

**主要组件**:
- `boot_orchestrator`: 引导编排器

**使用场景**:
```rust
// 执行完整引导序列
use nos_bootloader::application::BootApplicationService;

let mut service = BootApplicationService::new(BootProtocolType::Bios)?;
let boot_info = service.boot_system(Some("quiet splash"))?;
```

**与其他模块交互**:
- 调用领域服务实现业务逻辑
- 使用基础设施层提供的技术服务
- 控制核心层的初始化流程

### `application::boot_orchestrator`

**职责**:
- 编排完整引导流程
- 协调各个阶段
- 处理错误恢复
- 管理引导状态转换

**引导流程**:
1. 加载配置
2. 检测硬件
3. 验证先决条件
4. 初始化图形
5. 加载内核
6. 准备引导信息
7. 发布就绪事件

## 图形子系统

### `graphics` - 图形渲染

**职责**:
- 提供图形渲染API
- 实现双缓冲机制
- 支持多种图形格式
- 优化渲染性能

**主要组件**:
- `GraphicsRenderer`: 主渲染器
- `DoubleBuffer`: 双缓冲管理
- `Color`: 颜色表示
- `FramebufferInfo`: 帧缓冲区信息

**特性**:
- ARGB8888颜色格式支持
- 双缓冲无闪烁渲染
- 脏区域跟踪优化
- SIMD加速（x86_64）
- 跨平台兼容性

**使用场景**:
```rust
// 创建渲染器
use nos_bootloader::graphics::{GraphicsRenderer, FramebufferInfo, Color};

let fb_info = FramebufferInfo::new(0x10000000, 1024, 768, 4096, 32);
let mut renderer = GraphicsRenderer::new_with_double_buffer(fb_info)?;

// 渲染操作
renderer.begin_render()?;
renderer.clear_screen(Color::rgb(0, 51, 102))?;
renderer.draw_filled_rect(100, 100, 200, 100, Color::white())?;
renderer.end_render()?;
renderer.swap_buffers()?;
```

### `graphics::vbe` - VBE支持

**职责**:
- 实现VESA BIOS扩展
- 提供VBE模式管理
- 支持模式缓存
- 处理BIOS中断

**主要组件**:
- `VbeController`: VBE控制器
- `VbeGraphicsManager`: VBE图形管理器
- `VbeModeInfo`: VBE模式信息

**功能**:
- VBE 2.0/3.0支持
- 模式枚举和选择
- 线性帧缓冲区
- 性能优化缓存

## 引导菜单

### `boot_menu` - 引导菜单

**职责**:
- 提供统一菜单接口
- 支持多种UI模式
- 处理用户输入
- 管理菜单选项

**主要组件**:
- `BootMenu`: 统一菜单接口
- `MenuOption`: 菜单选项
- `UIMode`: UI模式枚举

**UI模式**:
- `Graphical`: 图形模式
- `Text`: 文本模式
- `Serial`: 串行控制台模式

**使用场景**:
```rust
// 创建菜单
use nos_bootloader::boot_menu::{BootMenu, UIMode, MenuOption};

let mut menu = BootMenu::new(UIMode::Text);
menu.initialize()?;

// 添加选项
menu.add_option(MenuOption::new(1, "Normal", "Normal boot"))?;
menu.add_option(MenuOption::new(2, "Recovery", "Recovery mode"))?;

// 处理输入
let selected = menu.process_input(b'\n');
```

## BIOS支持

### `bios` - BIOS接口

**职责**:
- 实现BIOS中断调用
- 提供实模式服务
- 处理BIOS数据结构
- 支持传统硬件访问

**主要组件**:
- `bios_calls`: BIOS调用接口
- `bios_realmode`: 实模式执行
- `disk`: 磁盘访问
- `vga`: VGA图形
- `memory`: 内存检测

**功能**:
- BIOS中断10h (视频服务)
- BIOS中断13h (磁盘服务)
- BIOS中断15h (系统服务)
- 实模式内存访问

## 固件接口

### `firmware` - 固件抽象

**职责**:
- 抽象不同固件接口
- 提供统一API
- 支持多种引导协议
- 处理固件特定逻辑

**支持的固件**:
- BIOS (传统BIOS)
- UEFI (统一可扩展固件接口)
- Multiboot2 (多引导协议)

**主要组件**:
- `uefi`: UEFI接口
- `multiboot2`: Multiboot2协议
- `disk_io`: 磁盘I/O抽象

## 内存管理

### `memory_mgmt` - 内存管理

**职责**:
- 管理物理内存
- 实现分页机制
- 支持内存热插拔
- 提供ECC支持

**主要组件**:
- `manager`: 内存管理器
- `paging`: 分页机制
- `layout`: 内存布局
- `hotplug`: 热插拔支持
- `ecc`: ECC内存支持

**功能**:
- 物理页面分配
- 虚拟内存映射
- 内存区域管理
- 内存统计报告

## CPU初始化

### `cpu_init` - CPU初始化

**职责**:
- 初始化CPU状态
- 设置中断处理
- 支持多处理器
- 检测CPU特性

**主要组件**:
- `interrupt`: 中断处理
- `multiprocessor`: 多处理器支持
- `mode_transition`: 模式转换
- `virtualization_detect`: 虚拟化检测

**功能**:
- 中断描述符表(IDT)
- 全局描述符表(GDT)
- CPU特性检测
- 多核启动

## 设备驱动

### `drivers` - 设备驱动

**职责**:
- 提供设备抽象
- 实现基本驱动
- 支持设备枚举
- 处理设备I/O

**主要组件**:
- `console`: 控制台驱动
- `uart`: 串口驱动
- `timer`: 定时器驱动
- `display`: 显示驱动

**支持的设备**:
- 串行端口(UART)
- 可编程间隔定时器(PIT)
- VGA显示适配器
- 系统控制台

## 安全模块

### `security` - 安全功能

**职责**:
- 实现安全启动
- 提供TPM支持
- 验证数字签名
- 检查完整性

**主要组件**:
- `secure_boot`: 安全启动
- `tpm`: TPM支持
- `verify`: 验证功能
- `integrity`: 完整性检查

**功能**:
- UEFI安全启动
- 数字签名验证
- 平台密钥管理
- 启动度量

## 内核接口

### `kernel_if` - 内核接口

**职责**:
- 加载内核镜像
- 准备引导信息
- 实现引导协议
- 处理内核交接

**主要组件**:
- `kernel_loader`: 内核加载器
- `kernel_handoff`: 内核交接
- `boot_info_builder`: 引导信息构建
- `protocol_manager`: 协议管理

**支持的格式**:
- ELF64内核镜像
- 多引导协议
- 平台特定协议

## 引导协议

### `protocol` - 协议定义

**职责**:
- 定义引导协议
- 提供协议抽象
- 支持多种标准
- 处理协议转换

**支持的协议**:
- BIOS引导协议
- UEFI引导协议
- Multiboot2协议
- 自定义协议

## 引导阶段

### `boot_stage` - 引导阶段

**职责**:
- 管理引导流程
- 控制阶段转换
- 处理阶段错误
- 支持阶段恢复

**主要组件**:
- `boot_orchestrator`: 引导编排器
- `boot_control`: 引导控制
- `boot_preparation`: 引导准备
- `boot_recovery`: 引导恢复

**引导阶段**:
1. 初始化阶段
2. 硬件检测阶段
3. 内存初始化阶段
4. 图形初始化阶段
5. 内核加载阶段
6. 内核交接阶段

## 诊断系统

### `diagnostics` - 诊断功能

**职责**:
- 提供硬件扫描
- 实现性能分析
- 支持日志记录
- 生成诊断报告

**主要组件**:
- `hardware_scan`: 硬件扫描
- `profiling`: 性能分析
- `logging`: 日志记录
- `reporter`: 报告生成

**功能**:
- 硬件兼容性检查
- 性能瓶颈分析
- 错误诊断报告
- 启动时间分析

## 优化模块

### `optimization` - 性能优化

**职责**:
- 实现并行化
- 提供缓存优化
- 支持延迟加载
- 处理错误缓解

**主要组件**:
- `cache`: 缓存优化
- `error_mitigation`: 错误缓解
- `recovery`: 恢复机制

**优化技术**:
- SIMD指令优化
- 缓存友好算法
- 并行初始化
- 预取策略

## ACPI支持

### `acpi_support` - ACPI支持

**职责**:
- 解析ACPI表
- 提供电源管理
- 支持设备配置
- 处理ACPI事件

**主要组件**:
- `acpi_parser`: ACPI解析器
- `power`: 电源管理
- `parser`: 表解析

**支持的表**:
- RSDP (Root System Description Pointer)
- RSDT (Root System Description Table)
- FADT (Fixed ACPI Description Table)
- DSDT (Differentiated System Description Table)

## 平台抽象

### `platform` - 平台抽象

**职责**:
- 提供平台接口
- 抽象平台差异
- 支持系统信息
- 实现平台验证

**主要组件**:
- `console`: 控制台抽象
- `system_info`: 系统信息
- `device`: 设备抽象
- `validation`: 平台验证

## 工具库

### `utils` - 工具函数

**职责**:
- 提供通用工具
- 实现错误处理
- 支持日志记录
- 提供内存工具

**主要组件**:
- `error`: 错误处理
- `log`: 日志记录
- `boot_traits`: 引导特征
- `error_recovery`: 错误恢复

**工具功能**:
- 统一错误类型
- 日志级别控制
- 内存操作工具
- 字符串处理

## 模块间交互图

```
┌─────────────────────────────────────────────────────────────┐
│                    应用层                                │
│  ┌─────────────┐                                       │
│  │BootOrchestr.│ ────────────────────────────────────────┐ │
│  └─────────────┘                                       │ │
└─────────────────────────────────────────────────────────────┘ │
  │                                                       │
  ▼                                                       │
┌─────────────────────────────────────────────────────────────┐ │
│                    领域层                                │ │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐ │ │
│  │BootConfig   │ │  BootInfo   │ │   BootServices     │ │ │
│  └─────────────┘ └─────────────┘ └─────────────────────┘ │ │
└─────────────────────────────────────────────────────────────┘ │
  │                                                       │
  ▼                                                       │
┌─────────────────────────────────────────────────────────────┐ │
│                  基础设施层                               │ │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐ │ │
│  │DIContainer │ │GraphicsBackend│ │HardwareDetection   │ │ │
│  └─────────────┘ └─────────────┘ └─────────────────────┘ │ │
└─────────────────────────────────────────────────────────────┘ │
  │                                                       │
  ▼                                                       │
┌─────────────────────────────────────────────────────────────┐ │
│                     核心层                               │ │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐ │ │
│  │ Allocator   │ │  BootState  │ │   BootSequence     │ │ │
│  └─────────────┘ └─────────────┘ └─────────────────────┘ │ │
└─────────────────────────────────────────────────────────────┘ │
  │                                                       │
  └───────────────────────────────────────────────────────────┘
```

## 设计原则

### 1. 依赖倒置原则
- 高层模块不依赖低层模块
- 抽象不依赖细节
- 细节依赖抽象

### 2. 单一职责原则
- 每个模块只有一个变化原因
- 明确定义模块边界
- 避免功能耦合

### 3. 开闭原则
- 对扩展开放
- 对修改封闭
- 使用接口和抽象

### 4. 接口隔离原则
- 客户端不应依赖不需要的接口
- 接口应该小而专注
- 避免胖接口

### 5. 领域驱动设计
- 以领域为中心
- 明确的边界上下文
- 统一语言

## 最佳实践

### 模块设计
1. **明确职责**: 每个模块应该有单一、明确的职责
2. **最小依赖**: 减少模块间的依赖关系
3. **接口抽象**: 使用接口而非具体实现
4. **错误处理**: 统一的错误处理机制

### 代码组织
1. **分层结构**: 严格按照分层架构组织代码
2. **命名约定**: 使用一致的命名约定
3. **文档完整**: 为所有公共API提供文档
4. **测试覆盖**: 为关键功能提供测试

### 性能考虑
1. **内存管理**: 高效的内存分配和释放
2. **缓存友好**: 优化缓存命中率
3. **并行处理**: 利用多核处理器
4. **延迟优化**: 减少关键路径延迟

## 未来扩展

### 计划中的模块
1. **网络支持**: PXE/iPXE网络引导
2. **加密支持**: 磁盘加密和验证
3. **虚拟化**: 虚拟机引导支持
4. **容器**: 容器化引导支持

### 架构演进
1. **微服务化**: 模块化服务架构
2. **事件驱动**: 更多事件驱动机制
3. **插件系统**: 可扩展插件架构
4. **配置管理**: 动态配置管理

这个模块文档为NOS引导加载程序提供了全面的模块说明，帮助开发者理解系统架构和各模块的职责。