// Resilience Framework Implementation
//
// 弹性框架实现
// 提供熔断器、限流、超时重试、舱壁和回退机制

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use spin::Mutex;

use crate::reliability::EIO;

/// 弹性策略管理器
pub struct ResilienceManager {
    /// 熔断器注册表
    pub circuit_breakers: BTreeMap<String, Arc<Mutex<CircuitBreaker>>>,
    /// 限流器注册表
    pub rate_limiters: BTreeMap<String, Arc<Mutex<RateLimiter>>>,
    /// 超时策略注册表
    pub timeout_policies: BTreeMap<String, TimeoutPolicy>,
    /// 重试策略注册表
    pub retry_policies: BTreeMap<String, RetryPolicy>,
    /// 舱壁策略注册表
    pub bulkheads: BTreeMap<String, Arc<Mutex<Bulkhead>>>,
    /// 回退策略注册表
    pub fallback_policies: BTreeMap<String, FallbackPolicy>,
    /// 统计信息
    pub stats: Arc<Mutex<ResilienceStats>>,
}

/// 熔断器
pub struct CircuitBreaker {
    /// 名称
    pub name: String,
    /// 状态
    pub state: CircuitBreakerState,
    /// 配置
    pub config: CircuitBreakerConfig,
    /// 滑动窗口
    pub sliding_window: SlidingWindow,
    /// 连续失败计数
    pub consecutive_failures: usize,
    /// 连续成功计数
    pub consecutive_successes: usize,
    /// 最后状态变更时间
    pub last_state_change: u64,
    /// 半开状态尝试次数
    pub half_open_attempts: usize,
}

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    /// 关闭(正常)
    Closed,
    /// 打开(熔断)
    Open,
    /// 半开(尝试恢复)
    HalfOpen,
}

/// 滑动窗口
pub struct SlidingWindow {
    /// 窗口类型
    pub window_type: WindowType,
    /// 基于计数的窗口
    pub count_based: Option<CountBasedWindow>,
    /// 基于时间的窗口
    pub time_based: Option<TimeBasedWindow>,
}

/// 窗口类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowType {
    /// 基于计数
    CountBased,
    /// 基于时间
    TimeBased,
}

/// 基于计数的窗口
pub struct CountBasedWindow {
    /// 窗口大小
    pub size: usize,
    /// 请求结果
    pub requests: Vec<RequestResult>,
    /// 总请求数
    pub total_count: usize,
    /// 失败数
    pub failure_count: usize,
}

/// 基于时间的窗口
pub struct TimeBasedWindow {
    /// 窗口时长(毫秒)
    pub duration_ms: u64,
    /// 分桶数量
    pub buckets: usize,
    /// 请求桶
    pub request_buckets: Vec<RequestBucket>,
}

/// 请求桶
#[derive(Debug, Clone)]
pub struct RequestBucket {
    /// 时间戳
    pub timestamp: u64,
    /// 总请求数
    pub total: u64,
    /// 失败数
    pub failures: u64,
}

/// 请求结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestResult {
    /// 成功
    Success,
    /// 失败
    Failure,
}

/// 熔断器配置
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// 错误阈值(0.0-1.0)
    pub error_threshold: f64,
    /// 滑动窗口大小
    pub sliding_window_size: usize,
    /// 滑动窗口时长(毫秒)
    pub sliding_window_duration_ms: u64,
    /// 最小请求数
    pub minimum_requests: usize,
    /// 半开状态的最大尝试次数
    pub half_open_max_calls: usize,
    /// 打开到半开的等待时间(毫秒)
    pub open_to_half_open_wait_ms: u64,
    /// 是否连续错误触发
    pub consecutive_errors: bool,
    /// 连续错误阈值
    pub consecutive_error_threshold: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            error_threshold: 0.5,
            sliding_window_size: 100,
            sliding_window_duration_ms: 60000, // 1分钟
            minimum_requests: 10,
            half_open_max_calls: 3,
            open_to_half_open_wait_ms: 60000, // 1分钟
            consecutive_errors: false,
            consecutive_error_threshold: 5,
        }
    }
}

/// 限流器
pub struct RateLimiter {
    /// 名称
    pub name: String,
    /// 算法
    pub algorithm: RateLimitAlgorithm,
    /// 令牌桶
    pub token_bucket: Option<TokenBucket>,
    /// 漏桶
    pub leaky_bucket: Option<LeakyBucket>,
    /// 固定窗口
    pub fixed_window: Option<FixedWindow>,
    /// 滑动窗口
    pub sliding_window_counter: Option<SlidingWindowCounter>,
}

/// 限流算法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitAlgorithm {
    /// 令牌桶
    TokenBucket,
    /// 漏桶
    LeakyBucket,
    /// 固定窗口
    FixedWindow,
    /// 滑动窗口
    SlidingWindow,
}

/// 令牌桶
pub struct TokenBucket {
    /// 容量
    pub capacity: u64,
    /// 当前令牌数
    pub tokens: u64,
    /// 填充速率(令牌/秒)
    pub refill_rate: u64,
    /// 最后填充时间
    pub last_refill: u64,
}

/// 漏桶
pub struct LeakyBucket {
    /// 容量
    pub capacity: u64,
    /// 当前水量
    pub water: u64,
    /// 漏水速率(请求/秒)
    pub leak_rate: u64,
    /// 最后漏水时间
    pub last_leak: u64,
}

/// 固定窗口
pub struct FixedWindow {
    /// 窗口大小(秒)
    pub window_size_sec: u64,
    /// 最大请求数
    pub max_requests: u64,
    /// 当前计数
    pub current_count: u64,
    /// 窗口开始时间
    pub window_start: u64,
}

/// 滑动窗口计数器
pub struct SlidingWindowCounter {
    /// 窗口大小(毫秒)
    pub window_size_ms: u64,
    /// 最大请求数
    pub max_requests: u64,
    /// 请求时间戳
    pub request_timestamps: Vec<u64>,
}

/// 超时策略
#[derive(Debug, Clone)]
pub struct TimeoutPolicy {
    /// 名称
    pub name: String,
    /// 超时时间(毫秒)
    pub timeout_ms: u64,
    /// 是否使用指数退避
    pub use_exponential_backoff: bool,
}

/// 重试策略
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// 名称
    pub name: String,
    /// 最大重试次数
    pub max_attempts: usize,
    /// 重试间隔(毫秒)
    pub wait_duration_ms: u64,
    /// 是否使用指数退避
    pub use_exponential_backoff: bool,
    /// 退避倍数
    pub backoff_multiplier: f64,
    /// 最大重试间隔(毫秒)
    pub max_wait_duration_ms: u64,
    /// 可重试的错误
    pub retryable_errors: Vec<String>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            max_attempts: 3,
            wait_duration_ms: 100,
            use_exponential_backoff: true,
            backoff_multiplier: 2.0,
            max_wait_duration_ms: 1000,
            retryable_errors: Vec::new(),
        }
    }
}

/// 舱壁策略
pub struct Bulkhead {
    /// 名称
    pub name: String,
    /// 最大并发数
    pub max_concurrent_calls: usize,
    /// 最大等待时间(毫秒)
    pub max_wait_duration_ms: u64,
    /// 当前并发数
    pub current_concurrent_calls: usize,
    /// 等待队列
    pub waiting_queue: usize,
}

/// 回退策略
#[derive(Debug, Clone)]
pub struct FallbackPolicy {
    /// 名称
    pub name: String,
    /// 回退函数类型
    pub fallback_type: FallbackType,
    /// 回退值
    pub fallback_value: Option<String>,
}

/// 回退类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackType {
    /// 返回默认值
    DefaultValue,
    /// 返回缓存值
    CachedValue,
    /// 抛出异常
    ThrowException,
    /// 执行回退函数
    FallbackFunction,
}

/// 弹性统计信息
#[derive(Debug, Clone)]
pub struct ResilienceStats {
    /// 熔断器统计
    pub circuit_breaker_stats: BTreeMap<String, CircuitBreakerStats>,
    /// 限流统计
    pub rate_limiter_stats: BTreeMap<String, RateLimiterStats>,
    /// 超时统计
    pub timeout_stats: BTreeMap<String, TimeoutStats>,
    /// 重试统计
    pub retry_stats: BTreeMap<String, RetryStats>,
    /// 舱壁统计
    pub bulkhead_stats: BTreeMap<String, BulkheadStats>,
}

/// 熔断器统计
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    /// 名称
    pub name: String,
    /// 当前状态
    pub state: CircuitBreakerState,
    /// 总请求数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 失败请求数
    pub failed_requests: u64,
    /// 被拒绝的请求数(熔断器打开)
    pub rejected_requests: u64,
    /// 失败率
    pub failure_rate: f64,
}

/// 限流器统计
#[derive(Debug, Clone)]
pub struct RateLimiterStats {
    /// 名称
    pub name: String,
    /// 总请求数
    pub total_requests: u64,
    /// 允许的请求数
    pub allowed_requests: u64,
    /// 被拒绝的请求数
    pub rejected_requests: u64,
}

/// 超时统计
#[derive(Debug, Clone)]
pub struct TimeoutStats {
    /// 名称
    pub name: String,
    /// 总请求数
    pub total_requests: u64,
    /// 超时次数
    pub timeouts: u64,
}

/// 重试统计
#[derive(Debug, Clone)]
pub struct RetryStats {
    /// 名称
    pub name: String,
    /// 总尝试次数
    pub total_attempts: u64,
    /// 成功次数
    pub successful_attempts: u64,
    /// 失败次数
    pub failed_attempts: u64,
    /// 平均重试次数
    pub avg_retry_count: f64,
}

/// 舱壁统计
#[derive(Debug, Clone)]
pub struct BulkheadStats {
    /// 名称
    pub name: String,
    /// 当前并发数
    pub current_concurrent_calls: usize,
    /// 被拒绝的请求数
    pub rejected_requests: u64,
}

impl ResilienceManager {
    /// 创建新的弹性管理器
    pub fn new() -> Self {
        Self {
            circuit_breakers: BTreeMap::new(),
            rate_limiters: BTreeMap::new(),
            timeout_policies: BTreeMap::new(),
            retry_policies: BTreeMap::new(),
            bulkheads: BTreeMap::new(),
            fallback_policies: BTreeMap::new(),
            stats: Arc::new(Mutex::new(ResilienceStats {
                circuit_breaker_stats: BTreeMap::new(),
                rate_limiter_stats: BTreeMap::new(),
                timeout_stats: BTreeMap::new(),
                retry_stats: BTreeMap::new(),
                bulkhead_stats: BTreeMap::new(),
            })),
        }
    }

    /// 创建熔断器
    pub fn create_circuit_breaker(
        &mut self,
        name: &str,
        config: CircuitBreakerConfig,
    ) -> Result<(), i32> {
        let breaker = CircuitBreaker {
            name: name.to_string(),
            state: CircuitBreakerState::Closed,
            config,
            sliding_window: SlidingWindow {
                window_type: WindowType::CountBased,
                count_based: Some(CountBasedWindow {
                    size: 100,
                    requests: Vec::new(),
                    total_count: 0,
                    failure_count: 0,
                }),
                time_based: None,
            },
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_state_change: self.get_current_time_ms(),
            half_open_attempts: 0,
        };

        self.circuit_breakers
            .insert(name.to_string(), Arc::new(Mutex::new(breaker)));

        crate::println!("[resilience] Created circuit breaker: {}", name);
        Ok(())
    }

    /// 执行熔断器保护的调用
    pub fn execute_with_circuit_breaker<F, R>(
        &mut self,
        name: &str,
        f: F,
    ) -> Result<R, i32>
    where
        F: FnOnce() -> Result<R, i32>,
    {
        let breaker = self.circuit_breakers.get(name).ok_or(EIO)?.clone();

        // 检查熔断器状态
        {
            let mut b = breaker.lock();
            if !b.allow_request() {
                b.record_rejection();
                return Err(EIO); // 熔断器打开,拒绝请求
            }
        }

        // 执行调用
        let result = f();

        // 记录结果
        {
            let mut b = breaker.lock();
            if result.is_ok() {
                b.record_success();
            } else {
                b.record_failure();
            }
        }

        result
    }

    /// 创建限流器
    pub fn create_rate_limiter(
        &mut self,
        name: &str,
        algorithm: RateLimitAlgorithm,
        limit: u64,
        window: u64,
    ) -> Result<(), i32> {
        let rate_limiter = match algorithm {
            RateLimitAlgorithm::TokenBucket => RateLimiter {
                name: name.to_string(),
                algorithm,
                token_bucket: Some(TokenBucket {
                    capacity: limit,
                    tokens: limit,
                    refill_rate: limit / (window / 1000),
                    last_refill: self.get_current_time_ms(),
                }),
                leaky_bucket: None,
                fixed_window: None,
                sliding_window_counter: None,
            },
            RateLimitAlgorithm::LeakyBucket => RateLimiter {
                name: name.to_string(),
                algorithm,
                token_bucket: None,
                leaky_bucket: Some(LeakyBucket {
                    capacity: limit,
                    water: 0,
                    leak_rate: limit / (window / 1000),
                    last_leak: self.get_current_time_ms(),
                }),
                fixed_window: None,
                sliding_window_counter: None,
            },
            RateLimitAlgorithm::FixedWindow => RateLimiter {
                name: name.to_string(),
                algorithm,
                token_bucket: None,
                leaky_bucket: None,
                fixed_window: Some(FixedWindow {
                    window_size_sec: window / 1000,
                    max_requests: limit,
                    current_count: 0,
                    window_start: self.get_current_time_ms(),
                }),
                sliding_window_counter: None,
            },
            RateLimitAlgorithm::SlidingWindow => RateLimiter {
                name: name.to_string(),
                algorithm,
                token_bucket: None,
                leaky_bucket: None,
                fixed_window: None,
                sliding_window_counter: Some(SlidingWindowCounter {
                    window_size_ms: window,
                    max_requests: limit,
                    request_timestamps: Vec::new(),
                }),
            },
        };

        self.rate_limiters
            .insert(name.to_string(), Arc::new(Mutex::new(rate_limiter)));

        crate::println!("[resilience] Created rate limiter: {}", name);
        Ok(())
    }

    /// 尝试获取许可(限流)
    pub fn try_acquire_permission(&mut self, name: &str, permits: u64) -> Result<(), i32> {
        let limiter = self.rate_limiters.get(name).ok_or(EIO)?;

        match limiter.lock().try_acquire(permits, self.get_current_time_ms()) {
            true => Ok(()),
            false => Err(EIO), // 限流
        }
    }

    /// 创建重试策略
    pub fn create_retry_policy(&mut self, name: &str, policy: RetryPolicy) -> Result<(), i32> {
        self.retry_policies.insert(name.to_string(), policy);

        crate::println!("[resilience] Created retry policy: {}", name);
        Ok(())
    }

    /// 执行带重试的调用
    pub fn execute_with_retry<F, R>(&mut self, name: &str, mut f: F) -> Result<R, i32>
    where
        F: FnMut() -> Result<R, i32>,
    {
        let policy = self.retry_policies.get(name).ok_or(EIO)?.clone();

        let mut last_error = EIO;
        let mut wait_ms = policy.wait_duration_ms;

        for attempt in 0..policy.max_attempts {
            match f() {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = e;
                    if attempt < policy.max_attempts - 1 {
                        // 等待后重试
                        self.sleep_ms(wait_ms);

                        if policy.use_exponential_backoff {
                            wait_ms = (wait_ms as f64 * policy.backoff_multiplier) as u64;
                            wait_ms = wait_ms.min(policy.max_wait_duration_ms);
                        }
                    }
                },
            }
        }

        Err(last_error)
    }

    /// 创建舱壁
    pub fn create_bulkhead(&mut self, name: &str, max_concurrent: usize, max_wait_ms: u64) -> Result<(), i32> {
        let bulkhead = Bulkhead {
            name: name.to_string(),
            max_concurrent_calls: max_concurrent,
            max_wait_duration_ms: max_wait_ms,
            current_concurrent_calls: 0,
            waiting_queue: 0,
        };

        self.bulkheads
            .insert(name.to_string(), Arc::new(Mutex::new(bulkhead)));

        crate::println!("[resilience] Created bulkhead: {}", name);
        Ok(())
    }

    /// 执行带舱壁保护的调用
    pub fn execute_with_bulkhead<F, R>(&mut self, name: &str, f: F) -> Result<R, i32>
    where
        F: FnOnce() -> Result<R, i32>,
    {
        let bulkhead = self.bulkheads.get(name).ok_or(EIO)?;

        // 尝试进入舱壁
        {
            let mut bh = bulkhead.lock();
            if bh.current_concurrent_calls >= bh.max_concurrent_calls {
                return Err(EIO); // 舱壁已满,拒绝请求
            }
            bh.current_concurrent_calls += 1;
        }

        // 执行调用
        let result = f();

        // 退出舱壁
        {
            let mut bh = bulkhead.lock();
            bh.current_concurrent_calls -= 1;
        }

        result
    }

    /// 创建超时策略
    pub fn create_timeout_policy(&mut self, name: &str, timeout_ms: u64) -> Result<(), i32> {
        let policy = TimeoutPolicy {
            name: name.to_string(),
            timeout_ms,
            use_exponential_backoff: false,
        };

        self.timeout_policies.insert(name.to_string(), policy);

        crate::println!("[resilience] Created timeout policy: {}", name);
        Ok(())
    }

    /// 执行带超时的调用
    pub fn execute_with_timeout<F, R>(&mut self, name: &str, f: F) -> Result<R, i32>
    where
        F: FnOnce() -> Result<R, i32>,
    {
        let policy = self.timeout_policies.get(name).ok_or(EIO)?;
        let start_time = self.get_current_time_ms();

        // 执行调用
        let result = f();

        // 检查超时
        let elapsed = self.get_current_time_ms() - start_time;
        if elapsed > policy.timeout_ms {
            return Err(EIO); // 超时
        }

        result
    }

    /// 创建回退策略
    pub fn create_fallback_policy(&mut self, name: &str, fallback_type: FallbackType) -> Result<(), i32> {
        let policy = FallbackPolicy {
            name: name.to_string(),
            fallback_type,
            fallback_value: None,
        };

        self.fallback_policies.insert(name.to_string(), policy);

        crate::println!("[resilience] Created fallback policy: {}", name);
        Ok(())
    }

    /// 执行带回退的调用
    pub fn execute_with_fallback<F, R>(&mut self, name: &str, f: F) -> Result<R, i32>
    where
        F: FnOnce() -> Result<R, i32>,
    {
        let result = f();

        if result.is_err() {
            // 执行回退
            if let Some(policy) = self.fallback_policies.get(name) {
                return match policy.fallback_type {
                    FallbackType::DefaultValue => Err(EIO),
                    FallbackType::CachedValue => Err(EIO),
                    FallbackType::ThrowException => Err(EIO),
                    FallbackType::FallbackFunction => Err(EIO),
                };
            }
        }

        result
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> ResilienceStats {
        self.stats.lock().clone()
    }

    /// 获取当前时间(毫秒)
    fn get_current_time_ms(&self) -> u64 {
        crate::subsystems::time::rdtsc() as u64 / 1000000
    }

    /// 休眠(毫秒)
    fn sleep_ms(&self, ms: u64) {
        crate::subsystems::time::sleep_ms(ms);
    }
}

impl CircuitBreaker {
    /// 是否允许请求
    fn allow_request(&mut self) -> bool {
        let now = self.get_current_time_ms();

        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                // 检查是否可以转换到半开状态
                if now - self.last_state_change >= self.config.open_to_half_open_wait_ms {
                    self.state = CircuitBreakerState::HalfOpen;
                    self.last_state_change = now;
                    self.half_open_attempts = 0;
                    true
                } else {
                    false
                }
            },
            CircuitBreakerState::HalfOpen => {
                self.half_open_attempts < self.config.half_open_max_calls
            },
        }
    }

    /// 记录成功
    fn record_success(&mut self) {
        self.record_result(RequestResult::Success);
    }

    /// 记录失败
    fn record_failure(&mut self) {
        self.record_result(RequestResult::Failure);
    }

    /// 记录拒绝
    fn record_rejection(&mut self) {
        // 更新统计信息
    }

    /// 记录结果
    fn record_result(&mut self, result: RequestResult) {
        match result {
            RequestResult::Success => {
                self.consecutive_failures = 0;
                self.consecutive_successes += 1;

                if self.state == CircuitBreakerState::HalfOpen {
                    // 半开状态成功,转换到关闭状态
                    if self.consecutive_successes >= self.config.half_open_max_calls {
                        self.state = CircuitBreakerState::Closed;
                        self.last_state_change = self.get_current_time_ms();
                    }
                }
            },
            RequestResult::Failure => {
                self.consecutive_successes = 0;
                self.consecutive_failures += 1;

                // 更新滑动窗口
                if let Some(ref mut window) = self.sliding_window.count_based {
                    window.total_count += 1;
                    window.failure_count += 1;
                    window.requests.push(result);
                    if window.requests.len() > window.size {
                        if window.requests.remove(0) == RequestResult::Failure {
                            window.failure_count -= 1;
                        }
                    }
                }

                // 检查是否应该打开熔断器
                if self.should_open_breaker() {
                    self.state = CircuitBreakerState::Open;
                    self.last_state_change = self.get_current_time_ms();
                }
            },
        }
    }

    /// 是否应该打开熔断器
    fn should_open_breaker(&self) -> bool {
        if self.state == CircuitBreakerState::Open {
            return false;
        }

        if self.config.consecutive_errors {
            self.consecutive_failures >= self.config.consecutive_error_threshold
        } else {
            if let Some(ref window) = self.sliding_window.count_based {
                if window.total_count < self.config.minimum_requests {
                    return false;
                }

                let failure_rate = window.failure_count as f64 / window.total_count as f64;
                failure_rate >= self.config.error_threshold
            } else {
                false
            }
        }
    }

    /// 获取当前时间(毫秒)
    fn get_current_time_ms(&self) -> u64 {
        crate::subsystems::time::rdtsc() as u64 / 1000000
    }
}

impl RateLimiter {
    /// 尝试获取许可
    fn try_acquire(&mut self, permits: u64, now: u64) -> bool {
        match self.algorithm {
            RateLimitAlgorithm::TokenBucket => {
                if let Some(ref mut bucket) = self.token_bucket {
                    // 填充令牌
                    let elapsed = now - bucket.last_refill;
                    if elapsed >= 1000 {
                        let refill = (elapsed / 1000) * bucket.refill_rate;
                        bucket.tokens = (bucket.tokens + refill).min(bucket.capacity);
                        bucket.last_refill = now;
                    }

                    // 消费令牌
                    if bucket.tokens >= permits {
                        bucket.tokens -= permits;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            },
            RateLimitAlgorithm::LeakyBucket => {
                if let Some(ref mut bucket) = self.leaky_bucket {
                    // 漏水
                    let elapsed = now - bucket.last_leak;
                    if elapsed >= 1000 {
                        let leaked = (elapsed / 1000) * bucket.leak_rate;
                        bucket.water = bucket.water.saturating_sub(leaked);
                        bucket.last_leak = now;
                    }

                    // 加水
                    if bucket.water + permits <= bucket.capacity {
                        bucket.water += permits;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            },
            RateLimitAlgorithm::FixedWindow => {
                if let Some(ref mut window) = self.fixed_window {
                    // 检查是否需要重置窗口
                    let elapsed = now - window.window_start;
                    if elapsed >= window.window_size_sec * 1000 {
                        window.current_count = 0;
                        window.window_start = now;
                    }

                    // 检查是否允许
                    if window.current_count + permits <= window.max_requests {
                        window.current_count += permits;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            },
            RateLimitAlgorithm::SlidingWindow => {
                if let Some(ref mut window) = self.sliding_window_counter {
                    // 移除过期的请求时间戳
                    let cutoff = now - window.window_size_ms;
                    window.request_timestamps.retain(|&ts| ts > cutoff);

                    // 检查是否允许
                    if window.request_timestamps.len() as u64 + permits <= window.max_requests {
                        for _ in 0..permits {
                            window.request_timestamps.push(now);
                        }
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            },
        }
    }
}

/// 全局弹性管理器实例
static mut RESILIENCE_MANAGER: Option<ResilienceManager> = None;
static mut RESILIENCE_MANAGER_INITIALIZED: bool = false;

/// 初始化弹性管理器
pub fn init_resilience_manager() -> Result<(), i32> {
    if unsafe { RESILIENCE_MANAGER_INITIALIZED } {
        return Ok(());
    }

    let manager = ResilienceManager::new();

    unsafe {
        RESILIENCE_MANAGER = Some(manager);
        RESILIENCE_MANAGER_INITIALIZED = true;
    }

    crate::println!("[resilience] Resilience manager initialized");
    Ok(())
}

/// 获取弹性管理器引用
pub fn get_resilience_manager() -> Option<&'static mut ResilienceManager> {
    unsafe { RESILIENCE_MANAGER.as_mut() }
}
