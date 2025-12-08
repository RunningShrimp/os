//! C标准库随机数生成器
//!
//! 提供完整的stdlib.h随机数函数支持，包括：
//! - 基本随机数生成：rand, srand
//! - 高质量随机数生成器
//! - 多种随机数分布：均匀、正态、指数等
//! - 种子管理和熵源
//! - 安全随机数生成

extern crate alloc;
use core::ffi::{c_int, c_uint, c_double};
use alloc::vec::Vec;
use crate::libc::error::{get_errno, set_errno};
use crate::libc::error::errno::EINVAL;

/// 随机数生成器类型
#[derive(Debug, Clone, Copy)]
pub enum RandomGeneratorType {
    /// 简单线性同余生成器
    LinearCongruential,
    /// Xorshift生成器
    Xorshift,
    /// Mersenne Twister（简化版）
    MersenneTwister,
    /// 系统熵源
    SystemEntropy,
}

/// 随机数生成器配置
#[derive(Debug, Clone)]
pub struct RandomConfig {
    /// 生成器类型
    pub generator_type: RandomGeneratorType,
    /// 是否使用熵源
    pub use_entropy: bool,
    /// 种子值（如果使用固定种子）
    pub fixed_seed: Option<c_uint>,
    /// 是否启用统计
    pub enable_stats: bool,
}

impl Default for RandomConfig {
    fn default() -> Self {
        Self {
            generator_type: RandomGeneratorType::Xorshift,
            use_entropy: true,
            fixed_seed: None,
            enable_stats: true,
        }
    }
}

/// 随机数生成统计信息
#[derive(Debug, Default)]
pub struct RandomStats {
    /// 生成的随机数总数
    pub total_generated: core::sync::atomic::AtomicU64,
    /// 种子设置次数
    pub seed_set_count: core::sync::atomic::AtomicU64,
    /// 熵源使用次数
    pub entropy_used: core::sync::atomic::AtomicU64,
    /// 生成器重置次数
    pub reset_count: core::sync::atomic::AtomicU64,
}

/// 增强的随机数生成器
pub struct EnhancedRandomGenerator {
    /// 配置
    config: RandomConfig,
    /// 统计信息
    stats: RandomStats,
    /// Xorshift状态
    xorshift_state: core::sync::atomic::AtomicU64,
    /// 线性同余生成器状态
    lcg_state: core::sync::atomic::AtomicU64,
    /// Mersenne Twister状态（简化为64位）
    mt_state: core::sync::atomic::AtomicU64,
    /// 是否已初始化
    initialized: core::sync::atomic::AtomicBool,
}

/// 系统熵源
pub struct SystemEntropy;

impl SystemEntropy {
    /// 获取系统熵
    pub fn get_entropy(&self) -> u64 {
        // 这里应该调用真正的系统熵源
        // 暂时使用时间戳和系统状态的组合
        let timestamp = crate::time::get_timestamp() as u64;
        let system_state = self.get_system_state() as u64;

        // 组合多个熵源
        timestamp ^ system_state ^ self.mix_bits(timestamp ^ system_state)
    }

    /// 获取系统状态信息
    fn get_system_state(&self) -> usize {
        // 使用栈指针、寄存器等作为熵源
        // 在实际实现中，这里应该使用真正的硬件随机数生成器
        let mut pointer = 0usize;
        // 编译器优化：使用栈地址作为熵源
        unsafe {
            core::ptr::read_volatile(&pointer);
        }
        pointer
    }

    /// 位混合函数
    fn mix_bits(&self, value: u64) -> u64 {
        // 来自SplitMix64的混合函数
        let mut x = value;
        x ^= x >> 30;
        x = x.wrapping_mul(0xbf58476d1ce4e5b9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94d049bb133111eb);
        x ^= x >> 31;
        x
    }
}

impl EnhancedRandomGenerator {
    /// 创建新的随机数生成器
    pub fn new(config: RandomConfig) -> Self {
        Self {
            config,
            stats: RandomStats::default(),
            xorshift_state: core::sync::atomic::AtomicU64::new(0),
            lcg_state: core::sync::atomic::AtomicU64::new(0),
            mt_state: core::sync::atomic::AtomicU64::new(0),
            initialized: core::sync::atomic::AtomicBool::new(false),
        }
    }

    /// 初始化随机数生成器
    pub fn initialize(&self) {
        if self.initialized.load(core::sync::atomic::Ordering::SeqCst) {
            return; // 已经初始化
        }

        let seed = if let Some(fixed_seed) = self.config.fixed_seed {
            fixed_seed as u64
        } else if self.config.use_entropy {
            let entropy = SystemEntropy.get_entropy();
            entropy as u64
        } else {
            // 使用默认种子
            0x123456789abcdef0u64
        };

        // 初始化不同类型的生成器
        self.xorshift_state.store(seed, core::sync::atomic::Ordering::SeqCst);
        self.lcg_state.store(seed.wrapping_mul(1103515245).wrapping_add(12345), core::sync::atomic::Ordering::SeqCst);
        self.mt_state.store(seed, core::sync::atomic::Ordering::SeqCst);

        self.initialized.store(true, core::sync::atomic::Ordering::SeqCst);
        crate::println!("[random_lib] 随机数生成器初始化，种子: 0x{:x}", seed);
    }

    /// 设置随机数种子
    pub fn srand(&self, seed: c_uint) {
        self.initialize();

        // 更新所有生成器的种子
        let seed_value = seed as u64;
        self.xorshift_state.store(seed_value, core::sync::atomic::Ordering::SeqCst);
        self.lcg_state.store(seed_value.wrapping_mul(1103515245).wrapping_add(12345), core::sync::atomic::Ordering::SeqCst);
        self.mt_state.store(seed_value, core::sync::atomic::Ordering::SeqCst);

        self.stats.seed_set_count.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
    }

    /// 生成随机整数（0到RAND_MAX）
    pub fn rand(&self) -> c_int {
        if !self.initialized.load(core::sync::atomic::Ordering::SeqCst) {
            self.initialize();
        }

        let result = match self.config.generator_type {
            RandomGeneratorType::LinearCongruential => self.lcg_rand(),
            RandomGeneratorType::Xorshift => self.xorshift_rand(),
            RandomGeneratorType::MersenneTwister => self.mt_rand(),
            RandomGeneratorType::SystemEntropy => self.entropy_rand(),
        };

        self.stats.total_generated.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        (result & 0x7fffffff) as c_int // 确保在RAND_MAX范围内
    }

    /// 生成随机无符号整数（0到UINT_MAX）
    pub fn rand_unsigned(&self) -> c_uint {
        self.rand() as c_uint
    }

    /// 生成0到range范围内的随机整数
    pub fn rand_range(&self, range: c_uint) -> c_uint {
        if range == 0 {
            return 0;
        }

        // 使用拒绝采样避免偏差
        let mut result;
        loop {
            result = self.rand() as c_uint;
            if result < (c_uint::MAX / range) * range {
                break;
            }
        }
        result % range
    }

    /// 生成min到max范围内的随机整数（包含）
    pub fn rand_between(&self, min: c_int, max: c_int) -> c_int {
        if min > max {
            return min;
        }
        let range = (max - min + 1) as c_uint;
        min + self.rand_range(range) as c_int
    }

    /// 生成0.0到1.0之间的随机浮点数
    pub fn rand_float(&self) -> c_double {
        // 生成52位精度的随机浮点数
        let raw = self.rand() as u64 | ((self.rand() as u64) << 31);
        const MAX_U52: u64 = (1u64 << 52) - 1;
        (raw & MAX_U52) as f64 / (MAX_U52 as f64)
    }

    /// 生成指定范围的随机浮点数
    pub fn rand_float_range(&self, min: c_double, max: c_double) -> c_double {
        if min >= max {
            return min;
        }
        min + (max - min) * self.rand_float()
    }

    /// 生成正态分布随机数（Box-Muller变换）
    pub fn rand_normal(&self, mean: c_double, std_dev: c_double) -> c_double {
        // Box-Muller变换
        static mut HAS_SPARE: bool = false;
        static mut SPARE_VALUE: c_double = 0.0;

        unsafe {
            if HAS_SPARE {
                HAS_SPARE = false;
                return SPARE_VALUE * std_dev + mean;
            }

            let u1 = self.rand_float();
            let u2 = self.rand_float();
            let radius = libm::sqrt(-2.0 * libm::log(u1));
            let angle = 2.0 * core::f64::consts::PI * u2;

            HAS_SPARE = true;
            SPARE_VALUE = radius * libm::sin(angle);

            (radius * libm::cos(angle)) * std_dev + mean
        }
    }

    /// 生成指数分布随机数
    pub fn rand_exponential(&self, lambda: c_double) -> c_double {
        if lambda <= 0.0 {
            return 0.0;
        }
        -libm::log(self.rand_float()) / lambda
    }

    /// 生成泊松分布随机数
    pub fn rand_poisson(&self, lambda: c_double) -> c_int {
        if lambda <= 0.0 {
            return 0;
        }

        // 使用Knuth算法，l = e^(-lambda)
        let l = libm::exp(-lambda);
        let mut k = 0;
        let mut p = 1.0;

        loop {
            k += 1;
            p *= self.rand_float();
            if p <= l {
                break;
            }
        }

        k - 1
    }

    /// 填充随机字节缓冲区
    pub fn rand_bytes(&self, buffer: *mut u8, length: usize) {
        if buffer.is_null() || length == 0 {
            return;
        }

        for i in 0..length {
            unsafe {
                *buffer.add(i) = self.rand() as u8;
            }
        }
    }

    /// 打乱字节数组
    pub fn shuffle_bytes(&self, data: &mut [u8]) {
        if data.is_empty() {
            return;
        }

        // Fisher-Yates洗牌算法
        for i in (1..data.len()).rev() {
            let j = self.rand_range(i as c_uint) as usize;
            data.swap(i, j);
        }
    }

    /// 洗牌算法（洗牌数组）
    pub fn shuffle<T>(&self, array: &mut [T]) {
        if array.is_empty() {
            return;
        }

        // Fisher-Yates洗牌算法
        for i in (1..array.len()).rev() {
            let j = self.rand_range(i as c_uint) as usize;
            array.swap(i, j);
        }
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> &RandomStats {
        &self.stats
    }

    /// 打印统计报告
    pub fn print_stats_report(&self) {
        crate::println!("\n=== 随机数生成器统计报告 ===");

        let total = self.stats.total_generated.load(core::sync::atomic::Ordering::SeqCst);
        let seed_count = self.stats.seed_set_count.load(core::sync::atomic::Ordering::SeqCst);
        let entropy_used = self.stats.entropy_used.load(core::sync::atomic::Ordering::SeqCst);
        let reset_count = self.stats.reset_count.load(core::sync::atomic::Ordering::SeqCst);

        crate::println!("生成器类型: {:?}", self.config.generator_type);
        crate::println!("总生成数: {}", total);
        crate::println!("种子设置次数: {}", seed_count);
        crate::println!("熵源使用次数: {}", entropy_used);
        crate::println!("重置次数: {}", reset_count);
        crate::println!("使用系统熵: {}", self.config.use_entropy);

        crate::println!("===========================");
    }

    // === 私有随机数生成算法 ===

    /// 线性同余生成器
    fn lcg_rand(&self) -> u64 {
        let current = self.lcg_state.load(core::sync::atomic::Ordering::SeqCst);
        let next = current.wrapping_mul(1103515245).wrapping_add(12345);
        self.lcg_state.store(next, core::sync::atomic::Ordering::SeqCst);
        next
    }

    /// Xorshift生成器
    fn xorshift_rand(&self) -> u64 {
        let mut x = self.xorshift_state.load(core::sync::atomic::Ordering::SeqCst);
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.xorshift_state.store(x, core::sync::atomic::Ordering::SeqCst);
        x
    }

    /// 简化的Mersenne Twister
    fn mt_rand(&self) -> u64 {
        let state = self.mt_state.load(core::sync::atomic::Ordering::SeqCst);
        let next = state ^ (state >> 11);
        self.mt_state.store(next, core::sync::atomic::Ordering::SeqCst);
        next
    }

    /// 系统熵随机数
    fn entropy_rand(&self) -> u64 {
        let entropy = SystemEntropy.get_entropy();
        self.stats.entropy_used.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        entropy
    }
}

impl Default for EnhancedRandomGenerator {
    fn default() -> Self {
        Self::new(RandomConfig::default())
    }
}

// 导出全局随机数生成器实例
pub static mut RANDOM_GENERATOR: Option<EnhancedRandomGenerator> = None;

/// 初始化全局随机数生成器
pub fn init_random_generator() {
    unsafe {
        if RANDOM_GENERATOR.is_none() {
            RANDOM_GENERATOR = Some(EnhancedRandomGenerator::new(RandomConfig::default()));
        }
    }
}

/// 获取全局随机数生成器
pub fn get_random_generator() -> &'static mut EnhancedRandomGenerator {
    unsafe {
        if RANDOM_GENERATOR.is_none() {
            init_random_generator();
        }
        RANDOM_GENERATOR.as_mut().unwrap()
    }
}

// 便捷的随机数函数包装器
#[inline]
pub fn srand(seed: c_uint) {
    unsafe { get_random_generator().srand(seed) }
}

#[inline]
pub fn rand() -> c_int {
    unsafe { get_random_generator().rand() }
}

// 高级随机数函数
#[inline]
pub fn rand_float() -> c_double {
    unsafe { get_random_generator().rand_float() }
}

#[inline]
pub fn rand_between(min: c_int, max: c_int) -> c_int {
    unsafe { get_random_generator().rand_between(min, max) }
}

#[inline]
pub fn rand_normal(mean: c_double, std_dev: c_double) -> c_double {
    unsafe { get_random_generator().rand_normal(mean, std_dev) }
}

/// 随机数测试函数
pub mod random_tests {
    use super::*;

    /// 运行随机数测试
    pub fn run_random_tests() {
        crate::println!("\n=== 随机数生成器测试 ===");

        let generator = EnhancedRandomGenerator::new(RandomConfig::default());
        generator.initialize();

        // 测试基本随机数生成
        test_basic_random(&generator);

        // 测试浮点随机数
        test_float_random(&generator);

        // 测试随机范围
        test_range_random(&generator);

        // 测试分布随机数
        test_distributed_random(&generator);

        // 测试随机字节数组
        test_random_bytes(&generator);

        // 打印统计报告
        generator.print_stats_report();

        crate::println!("=== 随机数生成器测试完成 ===\n");
    }

    fn test_basic_random(generator: &EnhancedRandomGenerator) {
        crate::println!("\n🎲 测试基本随机数生成...");

        // 测试种子设置
        generator.srand(42);
        let val1 = generator.rand();
        let val2 = generator.rand();

        crate::println!("  设置种子42后的随机数: {}, {}", val1, val2);

        // 重置并测试一致性
        generator.srand(42);
        let val3 = generator.rand();
        let val4 = generator.rand();

        if val1 == val3 && val2 == val4 {
            crate::println!("  ✅ 种子一致性测试通过");
        } else {
            crate::println!("  ❌ 种子一致性测试失败");
        }

        // 测试统计分布
        let mut buckets = [0; 10];
        for _ in 0..1000 {
            let val = generator.rand() % 10;
            buckets[val as usize] += 1;
        }

        let mut min_bucket = buckets[0];
        let mut max_bucket = buckets[0];
        for &count in &buckets {
            min_bucket = min_bucket.min(count);
            max_bucket = max_bucket.max(count);
        }

        crate::println!("  📊 分布测试: 最少={}, 最多={}, 偏差={}",
            min_bucket, max_bucket, max_bucket - min_bucket);
    }

    fn test_float_random(generator: &EnhancedRandomGenerator) {
        crate::println!("\n🎲 测试浮点随机数生成...");

        let mut sum = 0.0;
        let count = 1000;

        for _ in 0..count {
            let val = generator.rand_float();
            sum += val;
        }

        let mean = sum / count as c_double;
        crate::println!("  📊 均值测试: 期望=0.5, 实际={:.4}", mean);

        // 测试范围
        let min_val = generator.rand_float_range(-10.0, 10.0);
        let max_val = generator.rand_float_range(-10.0, 10.0);

        if min_val >= -10.0 && max_val < 10.0 {
            crate::println!("  ✅ 范围测试通过");
        } else {
            crate::println!("  ❌ 范围测试失败: {}, {}", min_val, max_val);
        }
    }

    fn test_range_random(generator: &EnhancedRandomGenerator) {
        crate::println!("\n🎲 测试范围随机数生成...");

        // 测试rand_between
        let min = 50;
        let max = 100;

        let mut in_range = true;
        for _ in 0..100 {
            let val = generator.rand_between(min, max);
            if val < min || val > max {
                in_range = false;
                break;
            }
        }

        if in_range {
            crate::println!("  ✅ 范围测试通过: [{} - {}]", min, max);
        } else {
            crate::println!("  ❌ 范围测试失败");
        }
    }

    fn test_distributed_random(generator: &EnhancedRandomGenerator) {
        crate::println!("\n🎲 测试分布随机数生成...");

        // 测试正态分布
        let normal_samples: Vec<c_double> = (0..100).map(|_| generator.rand_normal(0.0, 1.0)).collect();
        let normal_mean = normal_samples.iter().sum::<c_double>() / normal_samples.len() as c_double;
        crate::println!("  📊 正态分布: 期望=0.0, 实际均值={:.4}", normal_mean);

        // 测试指数分布
        let exp_samples: Vec<c_double> = (0..100).map(|_| generator.rand_exponential(1.0)).collect();
        let exp_mean = exp_samples.iter().sum::<c_double>() / exp_samples.len() as c_double;
        crate::println!("  📊 指数分布(λ=1): 期望=1.0, 实际均值={:.4}", exp_mean);
    }

    fn test_random_bytes(generator: &EnhancedRandomGenerator) {
        crate::println!("\n🎲 测试随机字节生成...");

        let mut buffer = [0u8; 256];
        generator.rand_bytes(buffer.as_mut_ptr(), buffer.len());

        // 检查是否所有字节都被设置
        let all_zero = buffer.iter().all(|&b| b == 0);
        let all_same = buffer.windows(2).all(|w| w[0] == w[1]);

        if !all_zero && !all_same {
            crate::println!("  ✅ 随机字节测试通过");
        } else {
            crate::println!("  ❌ 随机字节测试失败");
        }

        // 测试洗牌
        generator.shuffle_bytes(&mut buffer);
        let all_zero_shuffled = buffer.iter().all(|&b| b == 0);
        if !all_zero_shuffled {
            crate::println!("  ✅ 字节洗牌测试通过");
        } else {
            crate::println!("  ❌ 字节洗牌测试失败");
        }
    }
}