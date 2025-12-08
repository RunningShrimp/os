# NOS硬件抽象层（HAL）设计文档

## 概述

本文档描述了NOS内核硬件抽象层（Hardware Abstraction Layer, HAL）的设计，旨在将架构相关代码与通用内核代码分离，提供统一的硬件接口抽象，支持多平台架构。

## 设计目标

### 1. 架构分离
- 将架构特定代码（x86_64、AArch64、RISC-V）与通用内核逻辑完全分离
- 提供统一的硬件接口，隐藏底层架构差异
- 支持运行时架构检测和适配

### 2. 接口标准化
- 定义标准的硬件操作接口
- 统一的中断、内存、设备管理接口
- 可扩展的设备驱动框架

### 3. 性能优化
- 最小化抽象层开销
- 支持架构特定的优化路径
- 高效的硬件资源管理

## HAL架构设计

### 1. 分层结构

```
┌─────────────────────────────────────────────────────────┐
│                 应用层和内核服务                      │
├─────────────────────────────────────────────────────────┤
│                  HAL接口层                        │
├─────────────────────────────────────────────────────────┤
│                HAL核心层                          │
├─────────────────────────────────────────────────────────┤
│              架构适配层                          │
│  ┌─────────────┬─────────────┬─────────────┐    │
│  │   x86_64    │   AArch64    │   RISC-V    │    │
│  │   适配器      │   适配器      │   适配器      │    │
│  └─────────────┴─────────────┴─────────────┘    │
└─────────────────────────────────────────────────────────┘
```

### 2. 模块组织

#### 2.1 HAL接口层 (`kernel/hal/interface/`)

提供统一的硬件抽象接口：

```rust
// 基础HAL接口
pub trait Hal {
    fn architecture(&self) -> Architecture;
    fn cpu_features(&self) -> CpuFeatures;
    fn memory_info(&self) -> MemoryInfo;
    fn interrupt_controller(&self) -> &dyn InterruptController;
    fn timer(&self) -> &dyn Timer;
    fn device_manager(&self) -> &dyn DeviceManager;
}

// CPU操作接口
pub trait CpuOps {
    fn current_id(&self) -> usize;
    fn current_core(&self) -> &CpuCore;
    fn set_affinity(&self, cpu_id: usize) -> Result<(), HalError>;
    fn get_frequency(&self, cpu_id: usize) -> u64;
    fn set_frequency(&self, cpu_id: usize, freq: u64) -> Result<(), HalError>;
}

// 内存操作接口
pub trait MemoryOps {
    fn physical_memory(&self) -> &PhysicalMemory;
    fn virtual_memory(&self) -> &VirtualMemory;
    fn cache_info(&self) -> CacheInfo;
    fn tlb_info(&self) -> TlbInfo;
}

// 中断控制接口
pub trait InterruptController {
    fn enable_interrupt(&self, vector: u32) -> Result<(), HalError>;
    fn disable_interrupt(&self, vector: u32) -> Result<(), HalError>;
    fn set_priority(&self, vector: u32, priority: u8) -> Result<(), HalError>;
    fn register_handler(&self, vector: u32, handler: InterruptHandler) -> Result<(), HalError>;
}

// 定时器接口
pub trait Timer {
    fn get_ticks(&self) -> u64;
    fn set_alarm(&self, deadline: u64) -> Result<(), HalError>;
    fn cancel_alarm(&self, alarm_id: u32) -> Result<(), HalError>;
    fn get_frequency(&self) -> u64;
}
```

#### 2.2 HAL核心层 (`kernel/hal/core/`)

实现HAL接口的核心逻辑和架构无关的功能：

```rust
pub struct HalCore {
    architecture: Architecture,
    cpu_ops: Box<dyn CpuOps>,
    memory_ops: Box<dyn MemoryOps>,
    interrupt_controller: Box<dyn InterruptController>,
    timer: Box<dyn Timer>,
    device_manager: Box<dyn DeviceManager>,
}

impl HalCore {
    pub fn new() -> Result<Self, HalError> {
        // 检测当前架构
        let arch = Self::detect_architecture();
        
        // 创建架构特定的适配器
        let (cpu_ops, memory_ops, interrupt_controller, timer, device_manager) = 
            Self::create_arch_adapters(arch)?;
        
        Ok(Self {
            architecture: arch,
            cpu_ops,
            memory_ops,
            interrupt_controller,
            timer,
            device_manager,
        })
    }
    
    fn detect_architecture() -> Architecture {
        // 运行时架构检测
        match cfg!(target_arch) {
            "x86_64" => Architecture::X86_64,
            "aarch64" => Architecture::AArch64,
            "riscv64" => Architecture::RiscV64,
            _ => Architecture::Unknown,
        }
    }
    
    fn create_arch_adapters(arch: Architecture) -> Result<(Box<dyn CpuOps>, /* ... */), HalError> {
        match arch {
            Architecture::X86_64 => {
                // 创建x86_64特定的适配器
                Ok((
                    Box::new(x86_64::X86_64CpuOps::new()),
                    Box::new(x86_64::X86_64MemoryOps::new()),
                    Box::new(x86_64::X86_64InterruptController::new()),
                    Box::new(x86_64::X86_64Timer::new()),
                    Box::new(x86_64::X86_64DeviceManager::new()),
                ))
            }
            Architecture::AArch64 => {
                // 创建AArch64特定的适配器
                Ok((
                    Box::new(aarch64::AArch64CpuOps::new()),
                    Box::new(aarch64::AArch64MemoryOps::new()),
                    Box::new(aarch64::AArch64InterruptController::new()),
                    Box::new(aarch64::AArch64Timer::new()),
                    Box::new(aarch64::AArch64DeviceManager::new()),
                ))
            }
            Architecture::RiscV64 => {
                // 创建RISC-V特定的适配器
                Ok((
                    Box::new(riscv64::RiscV64CpuOps::new()),
                    Box::new(riscv64::RiscV64MemoryOps::new()),
                    Box::new(riscv64::RiscV64InterruptController::new()),
                    Box::new(riscv64::RiscV64Timer::new()),
                    Box::new(riscv64::RiscV64DeviceManager::new()),
                ))
            }
            _ => Err(HalError::UnsupportedArchitecture),
        }
    }
}
```

#### 2.3 架构适配层 (`kernel/hal/arch/`)

每个架构的特定实现：

##### 2.3.1 x86_64适配器 (`kernel/hal/arch/x86_64/`)

```rust
pub struct X86_64CpuOps {
    features: CpuFeatures,
    core_count: usize,
}

impl X86_64CpuOps {
    pub fn new() -> Self {
        Self {
            features: Self::detect_cpu_features(),
            core_count: Self::detect_core_count(),
        }
    }
    
    fn detect_cpu_features() -> CpuFeatures {
        // CPUID指令检测CPU特性
        let mut features = CpuFeatures::new();
        
        // 检测SSE、AVX等特性
        if Self::has_cpuid_feature(CpuIdFeature::SSE2) {
            features.set_sse2(true);
        }
        if Self::has_cpuid_feature(CpuIdFeature::AVX2) {
            features.set_avx2(true);
        }
        
        features
    }
}

impl CpuOps for X86_64CpuOps {
    fn current_id(&self) -> usize {
        // 读取GS寄存器获取当前CPU ID
        unsafe {
            let mut cpu_id: usize;
            core::arch::asm!(
                "mov {0}, gs:[0x8]",
                out(reg) cpu_id,
            );
            cpu_id
        }
    }
    
    fn set_affinity(&self, cpu_id: usize) -> Result<(), HalError> {
        // 设置CPU亲和性
        unsafe {
            let mask = 1u64 << cpu_id;
            core::arch::asm!(
                "mov {0}, rcx",
                "wrmsr",
                in(reg) 0x110, // IA32_TSC_AUXILIARY
                in(reg) mask,
            );
        }
        Ok(())
    }
}
```

##### 2.3.2 AArch64适配器 (`kernel/hal/arch/aarch64/`)

```rust
pub struct AArch64CpuOps {
    features: CpuFeatures,
    core_count: usize,
}

impl AArch64CpuOps {
    pub fn new() -> Self {
        Self {
            features: Self::detect_cpu_features(),
            core_count: Self::detect_core_count(),
        }
    }
    
    fn detect_cpu_features() -> CpuFeatures {
        // 读取ID_AA64PFR0_EL1等系统寄存器
        let mut features = CpuFeatures::new();
        
        unsafe {
            let mut pfr0: u64;
            core::arch::asm!(
                "mrs {0}, id_aa64pfr0_el1",
                out(reg) pfr0,
            );
            
            // 检测NEON、AES等特性
            if (pfr0 & 0x1) != 0 {
                features.set_neon(true);
            }
            if (pfr0 & 0x2) != 0 {
                features.set_aes(true);
            }
        }
        
        features
    }
}

impl CpuOps for AArch64CpuOps {
    fn current_id(&self) -> usize {
        // 读取TPIDR_EL0获取当前CPU ID
        unsafe {
            let mut cpu_id: usize;
            core::arch::asm!(
                "mrs {0}, tpidr_el0",
                out(reg) cpu_id,
            );
            cpu_id
        }
    }
    
    fn set_affinity(&self, cpu_id: usize) -> Result<(), HalError> {
        // 通过系统调用设置CPU亲和性
        syscall::set_affinity(cpu_id).map_err(|_| HalError::OperationFailed)
    }
}
```

##### 2.3.3 RISC-V适配器 (`kernel/hal/arch/riscv64/`)

```rust
pub struct RiscV64CpuOps {
    features: CpuFeatures,
    core_count: usize,
}

impl RiscV64CpuOps {
    pub fn new() -> Self {
        Self {
            features: Self::detect_cpu_features(),
            core_count: Self::detect_core_count(),
        }
    }
    
    fn detect_cpu_features() -> CpuFeatures {
        // 读取misa、mvendorid等CSR寄存器
        let mut features = CpuFeatures::new();
        
        unsafe {
            let mut misa: u64;
            core::arch::asm!(
                "csrr {0}, misa",
                out(reg) misa,
            );
            
            // 检测扩展特性
            if (misa & (1 << 0)) != 0 {
                features.set_integer_division(true);
            }
            if (misa & (1 << 2)) != 0 {
                features.set_multiplication(true);
            }
        }
        
        features
    }
}

impl CpuOps for RiscV64CpuOps {
    fn current_id(&self) -> usize {
        // 读取tp寄存器获取当前CPU ID
        unsafe {
            let mut cpu_id: usize;
            core::arch::asm!(
                "mv {0}, tp",
                out(reg) cpu_id,
            );
            cpu_id
        }
    }
    
    fn set_affinity(&self, cpu_id: usize) -> Result<(), HalError> {
        // 通过SBI调用设置CPU亲和性
        sbi::set_affinity(cpu_id).map_err(|_| HalError::OperationFailed)
    }
}
```

## HAL初始化流程

### 1. 启动时初始化

```rust
// 在main.rs中的早期初始化
fn hal_early_init() -> Result<(), HalError> {
    // 1. 检测架构
    let arch = detect_architecture();
    
    // 2. 初始化架构特定硬件
    arch_specific_early_init(arch)?;
    
    // 3. 创建HAL实例
    let hal = HalCore::new()?;
    
    // 4. 设置全局HAL实例
    set_global_hal(hal);
    
    Ok(())
}

// 架构特定早期初始化
fn arch_specific_early_init(arch: Architecture) -> Result<(), HalError> {
    match arch {
        Architecture::X86_64 => {
            // x86_64特定初始化
            x86_64::init_gdt();
            x86_64::init_idt();
            x86_64::init_pic();
        }
        Architecture::AArch64 => {
            // AArch64特定初始化
            aarch64::init_gic();
            aarch64::init_mmu();
        }
        Architecture::RiscV64 => {
            // RISC-V特定初始化
            riscv64::init_plic();
            riscv64::init_sbi();
        }
        _ => return Err(HalError::UnsupportedArchitecture),
    }
    Ok(())
}
```

### 2. 完整HAL初始化

```rust
fn hal_full_init() -> Result<(), HalError> {
    let hal = get_global_hal();
    
    // 1. 初始化CPU管理
    hal.cpu_ops().init()?;
    
    // 2. 初始化内存管理
    hal.memory_ops().init()?;
    
    // 3. 初始化中断控制器
    hal.interrupt_controller().init()?;
    
    // 4. 初始化定时器
    hal.timer().init()?;
    
    // 5. 初始化设备管理器
    hal.device_manager().init()?;
    
    // 6. 注册HAL服务
    register_hal_services(hal);
    
    Ok(())
}
```

## 设备抽象框架

### 1. 设备接口标准化

```rust
// 通用设备接口
pub trait Device: Send + Sync {
    fn device_type(&self) -> DeviceType;
    fn device_id(&self) -> DeviceId;
    fn name(&self) -> &str;
    fn state(&self) -> DeviceState;
    fn capabilities(&self) -> DeviceCapabilities;
    
    fn initialize(&mut self) -> Result<(), DeviceError>;
    fn shutdown(&mut self) -> Result<(), DeviceError>;
    fn reset(&mut self) -> Result<(), DeviceError>;
    fn suspend(&mut self) -> Result<(), DeviceError>;
    fn resume(&mut self) -> Result<(), DeviceError>;
}

// 块设备接口
pub trait BlockDevice: Device {
    fn read(&self, lba: u64, buffer: &mut [u8]) -> Result<usize, DeviceError>;
    fn write(&self, lba: u64, buffer: &[u8]) -> Result<usize, DeviceError>;
    fn flush(&self) -> Result<(), DeviceError>;
    fn block_size(&self) -> usize;
    fn block_count(&self) -> u64;
}

// 网络设备接口
pub trait NetworkDevice: Device {
    fn mac_address(&self) -> [u8; 6];
    fn mtu(&self) -> usize;
    fn transmit(&self, packet: &[u8]) -> Result<(), DeviceError>;
    fn receive(&self) -> Result<Vec<u8>, DeviceError>;
    fn set_promiscuous(&self, enable: bool) -> Result<(), DeviceError>;
}
```

### 2. 设备管理器

```rust
pub struct DeviceManager {
    devices: HashMap<DeviceId, Arc<Mutex<dyn Device>>>,
    drivers: HashMap<DeviceType, Box<dyn DriverFactory>>,
    next_device_id: AtomicU32,
}

impl DeviceManager {
    pub fn register_driver(&mut self, device_type: DeviceType, factory: Box<dyn DriverFactory>) {
        self.drivers.insert(device_type, factory);
    }
    
    pub fn probe_devices(&mut self) -> Result<(), DeviceError> {
        // 遍历所有驱动工厂，探测设备
        for (deviceType, factory) in &self.drivers {
            if let Some(device) = factory.probe()? {
                let device_id = self.next_device_id.fetch_add(1, Ordering::SeqCst);
                self.devices.insert(device_id, Arc::new(Mutex::new(device)));
            }
        }
        Ok(())
    }
    
    pub fn get_device(&self, device_id: DeviceId) -> Option<Arc<Mutex<dyn Device>>> {
        self.devices.get(&device_id).cloned()
    }
    
    pub fn get_devices_by_type(&self, device_type: DeviceType) -> Vec<Arc<Mutex<dyn Device>>> {
        self.devices.values()
            .filter(|device| {
                let device = device.lock();
                device.device_type() == device_type
            })
            .cloned()
            .collect()
    }
}
```

## 错误处理机制

### 1. 统一错误类型

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum HalError {
    UnsupportedArchitecture,
    OperationFailed,
    InvalidParameter,
    ResourceUnavailable,
    DeviceNotFound,
    PermissionDenied,
    Timeout,
    HardwareFault,
}

// 错误转换trait
impl From<ArchError> for HalError {
    fn from(error: ArchError) -> Self {
        match error {
            ArchError::UnsupportedOperation => HalError::UnsupportedArchitecture,
            ArchError::InvalidState => HalError::OperationFailed,
            // ... 其他转换
        }
    }
}
```

## 性能优化策略

### 1. 快速路径

```rust
impl HalCore {
    // 内联快速路径操作
    #[inline(always)]
    pub fn fast_path_get_cpu_id() -> usize {
        // 直接使用架构特定的快速实现
        match self.architecture {
            Architecture::X86_64 => x86_64::fast_get_cpu_id(),
            Architecture::AArch64 => aarch64::fast_get_cpu_id(),
            Architecture::RiscV64 => riscv64::fast_get_cpu_id(),
        }
    }
    
    // 缓存常用操作
    pub fn cached_memory_info(&self) -> &MemoryInfo {
        static CACHED_INFO: OnceCell<MemoryInfo> = OnceCell::new();
        CACHED_INFO.get_or_init(|| self.memory_ops.get_info())
    }
}
```

### 2. 零开销抽象

```rust
// 编译时特化消除抽象开销
pub trait HalOps {
    fn nop() -> Self;
    fn is_available() -> bool;
}

// 编译时选择最优实现
#[inline(always)]
pub fn get_hal_ops<T: HalOps>() -> T {
    if T::is_available() {
        T::nop()
    } else {
        // 回退到通用实现
        panic!("HAL operation not available on this architecture");
    }
}
```

## 迁移计划

### 阶段1：基础HAL框架（第1-2周）

1. 创建HAL接口定义
2. 实现HAL核心层
3. 创建架构适配器框架
4. 迁移现有arch模块功能

### 阶段2：设备抽象（第3-4周）

1. 设计设备接口标准
2. 实现设备管理器
3. 迁移现有驱动程序
4. 测试设备抽象层

### 阶段3：系统集成（第5-6周）

1. 集成HAL到内核初始化流程
2. 更新系统调用以使用HAL
3. 性能优化和测试
4. 文档和工具完善

## 验收标准

### 1. 功能完整性
- [ ] 支持所有目标架构（x86_64、AArch64、RISC-V）
- [ ] 完整的设备抽象框架
- [ ] 统一的错误处理机制
- [ ] 性能优化路径

### 2. 代码质量
- [ ] 架构代码与通用代码完全分离
- [ ] 接口一致性
- [ ] 测试覆盖率>90%
- [ ] 文档完整性

### 3. 性能指标
- [ ] HAL操作开销<5%
- [ ] 设备访问性能提升>10%
- [ ] 内存使用优化>15%
- [ ] 编译时间不增加

## 结论

通过实施这个HAL设计，NOS内核将获得：

1. **清晰的架构分离**：架构相关代码与通用内核逻辑完全分离
2. **统一的硬件接口**：提供一致的硬件操作抽象
3. **良好的扩展性**：支持新架构和设备的快速添加
4. **优化的性能**：最小化抽象层开销，支持架构特定优化

这个HAL设计为NOS内核的长期发展奠定了坚实的架构基础。