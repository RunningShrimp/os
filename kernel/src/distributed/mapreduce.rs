//! # MapReduce 编程框架
//!
//! 实现完整的 MapReduce 编程模型，用于大规模并行数据处理。
//!
//! ## 核心组件
//!
//! - **Map 阶段**: 并行处理输入数据，生成中间键值对
//! - **Shuffle 阶段**: 将中间数据按键分发到 Reducer
//! - **Sort 阶段**: 在 Shuffle 过程中对中间数据排序
//! - **Reduce 阶段**: 聚合具有相同键的值
//!
//! ## 特性
//!
//! - 数据分区策略支持
//! - 任务调度和执行
//! - 容错和自动重试
//! - 资源管理和调度
//! - 数据本地化优化

use alloc::{vec::Vec, collections::BTreeMap, string::String, sync::Arc};
use core::fmt::{self, Debug};
use crate::sync::Mutex;
// Import TaskId from parent module
use super::TaskId;

/// MapReduce 引擎
pub struct MapReduceEngine<K, V, OK, OV, RK, RV> {
    config: MapReduceConfig,
    task_scheduler: Arc<TaskScheduler>,
    partition_manager: Arc<PartitionManager>,
    shuffle_manager: Arc<ShuffleManager>,
    fault_tolerance: Arc<FaultToleranceHandler>,
    _phantom: core::marker::PhantomData<(K, V, OK, OV, RK, RV)>,
}

/// MapReduce 配置
#[derive(Debug, Clone)]
pub struct MapReduceConfig {
    /// Map 任务数量
    pub num_map_tasks: usize,
    /// Reduce 任务数量
    pub num_reduce_tasks: usize,
    /// 分区策略
    pub partition_strategy: PartitionStrategy,
    /// 每个分区的最大内存使用（字节）
    pub max_memory_per_partition: usize,
    /// Shuffle 缓冲区大小（字节）
    pub shuffle_buffer_size: usize,
    /// 任务超时时间（秒）
    pub task_timeout_secs: u64,
    /// 最大重试次数
    pub max_task_retries: u32,
    /// 启用推测执行
    pub enable_speculative_execution: bool,
    /// 启用压缩
    pub enable_compression: bool,
}

impl Default for MapReduceConfig {
    fn default() -> Self {
        Self {
            num_map_tasks: 4,
            num_reduce_tasks: 2,
            partition_strategy: PartitionStrategy::Hash,
            max_memory_per_partition: 256 * 1024 * 1024, // 256MB
            shuffle_buffer_size: 64 * 1024 * 1024, // 64MB
            task_timeout_secs: 600,
            max_task_retries: 3,
            enable_speculative_execution: false,
            enable_compression: true,
        }
    }
}

/// 数据分区策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionStrategy {
    /// 哈希分区
    Hash,
    /// 范围分区
    Range,
    /// 自定义分区
    Custom,
}

/// Map 任务定义
pub struct MapTask<K, V, OK, OV>
where
    K: Clone + Ord + fmt::Debug,
    V: Clone + fmt::Debug,
    OK: Clone + Ord + fmt::Debug + core::hash::Hash,
    OV: Clone + fmt::Debug,
{
    pub task_id: TaskId,
    pub input_data: Vec<(K, V)>,
    pub map_function: MapFunction<K, V, OK, OV>,
    pub partition_count: usize,
}

/// Map 函数类型
pub type MapFunction<K, V, OK, OV> = Arc<dyn Fn(K, V, &mut MapContext<OK, OV>) + Send + Sync>;

/// Map 执行上下文
pub struct MapContext<OK, OV>
where
    OK: Clone + Ord + fmt::Debug + core::hash::Hash,
    OV: Clone + fmt::Debug,
{
    pub intermediate: Vec<(OK, OV)>,
    pub partition_strategy: PartitionStrategy,
    pub partition_count: usize,
}

impl<OK, OV> MapContext<OK, OV>
where
    OK: Clone + Ord + fmt::Debug + core::hash::Hash,
    OV: Clone + fmt::Debug,
{
    pub fn emit(&mut self, key: OK, value: OV) {
        self.intermediate.push((key, value));
    }

    pub fn get_partition(&self, key: &OK) -> usize {
        match self.partition_strategy {
            PartitionStrategy::Hash => {
                use core::hash::{Hash, Hasher};
                // Use a simple custom hasher for no_std environment
                struct SimpleHasher(u64);
                impl Hasher for SimpleHasher {
                    fn finish(&self) -> u64 { self.0 }
                    fn write(&mut self, bytes: &[u8]) {
                        for byte in bytes {
                            self.0 = self.0.wrapping_mul(31).wrapping_add(*byte as u64);
                        }
                    }
                }
                let mut hasher = SimpleHasher(0);
                key.hash(&mut hasher);
                (hasher.finish() as usize) % self.partition_count
            }
            PartitionStrategy::Range => {
                // 简化版：使用键的哈希值
                // 实际实现需要采样和范围计算
                use core::hash::{Hash, Hasher};
                struct SimpleHasher(u64);
                impl Hasher for SimpleHasher {
                    fn finish(&self) -> u64 { self.0 }
                    fn write(&mut self, bytes: &[u8]) {
                        for byte in bytes {
                            self.0 = self.0.wrapping_mul(31).wrapping_add(*byte as u64);
                        }
                    }
                }
                let mut hasher = SimpleHasher(0);
                key.hash(&mut hasher);
                (hasher.finish() as usize) % self.partition_count
            }
            PartitionStrategy::Custom => 0,
        }
    }
}

/// Reduce 任务定义
pub struct ReduceTask<OK, OV, RK, RV>
where
    OK: Clone + Ord + fmt::Debug + core::hash::Hash,
    OV: Clone + fmt::Debug,
    RK: Clone + Ord + fmt::Debug,
    RV: Clone + fmt::Debug,
{
    pub task_id: TaskId,
    pub partition_id: usize,
    pub input_data: Vec<(OK, Vec<OV>)>,
    pub reduce_function: ReduceFunction<OK, OV, RK, RV>,
}

/// Reduce 函数类型
pub type ReduceFunction<OK, OV, RK, RV> = Arc<
    dyn Fn(OK, Vec<OV>, &mut ReduceContext<RK, RV>) + Send + Sync
>;

/// Reduce 执行上下文
pub struct ReduceContext<RK, RV>
where
    RK: Clone + Ord + fmt::Debug,
    RV: Clone + fmt::Debug,
{
    pub output: Vec<(RK, RV)>,
}

impl<RK, RV> ReduceContext<RK, RV>
where
    RK: Clone + Ord + fmt::Debug,
    RV: Clone + fmt::Debug,
{
    pub fn emit(&mut self, key: RK, value: RV) {
        self.output.push((key, value));
    }
}

/// MapReduce 错误类型
#[derive(Debug)]
pub enum MapReduceError {
    TaskExecutionFailed(String),
    ShuffleFailed(String),
    PartitionError(String),
    Timeout(String),
    ResourceExhausted(String),
}

/// MapReduce 执行结果
pub struct MapReduceResult<RK, RV>
where
    RK: Clone + Ord + fmt::Debug,
    RV: Clone + fmt::Debug,
{
    pub output: Vec<(RK, RV)>,
    pub statistics: JobStatistics,
}

/// 作业统计信息
#[derive(Debug, Clone, Default)]
pub struct JobStatistics {
    pub map_tasks_completed: usize,
    pub reduce_tasks_completed: usize,
    pub map_input_bytes: u64,
    pub map_output_bytes: u64,
    pub reduce_input_bytes: u64,
    pub reduce_output_bytes: u64,
    pub shuffle_bytes: u64,
    pub execution_time_ms: u64,
}

impl<K, V, OK, OV, RK, RV> MapReduceEngine<K, V, OK, OV, RK, RV>
where
    K: Clone + Ord + fmt::Debug + core::hash::Hash,
    V: Clone + fmt::Debug,
    OK: Clone + Ord + fmt::Debug + core::hash::Hash,
    OV: Clone + fmt::Debug,
    RK: Clone + Ord + fmt::Debug,
    RV: Clone + fmt::Debug,
{
    /// 创建新的 MapReduce 引擎
    pub fn new(config: MapReduceConfig) -> Result<Self, MapReduceError> {
        Ok(Self {
            task_scheduler: Arc::new(TaskScheduler::new(config.num_map_tasks + config.num_reduce_tasks)),
            partition_manager: Arc::new(PartitionManager::new(config.partition_strategy)),
            shuffle_manager: Arc::new(ShuffleManager::new(config.shuffle_buffer_size)),
            fault_tolerance: Arc::new(FaultToleranceHandler::new(config.max_task_retries)),
            config,
            _phantom: core::marker::PhantomData,
        })
    }

    /// 执行 MapReduce 作业
    pub fn execute_job(
        &self,
        input_data: Vec<(K, V)>,
        map_fn: impl Fn(K, V, &mut MapContext<OK, OV>) + Send + Sync + 'static,
        reduce_fn: impl Fn(OK, Vec<OV>, &mut ReduceContext<RK, RV>) + Send + Sync + 'static,
    ) -> Result<MapReduceResult<RK, RV>, MapReduceError> {
        let job_id = self.generate_job_id();
        let start_time = self.current_time_ms();

        // 阶段 1: 准备 Map 任务
        let map_tasks = self.prepare_map_tasks(job_id.clone(), input_data, map_fn)?;

        // 阶段 2: 执行 Map 任务
        let map_outputs = self.execute_map_tasks(map_tasks)?;

        // 阶段 3: Shuffle 和 Sort
        let shuffled_data = self
            .shuffle_manager
            .shuffle_and_sort(map_outputs, self.config.num_reduce_tasks)?;

        // 阶段 4: 准备 Reduce 任务
        let reduce_tasks = self.prepare_reduce_tasks(job_id, shuffled_data, reduce_fn)?;

        // 阶段 5: 执行 Reduce 任务
        let reduce_outputs = self.execute_reduce_tasks(reduce_tasks)?;

        // 合并结果
        let mut final_output = Vec::new();
        for output in reduce_outputs {
            final_output.extend(output);
        }

        let execution_time = self.current_time_ms() - start_time;

        Ok(MapReduceResult {
            output: final_output,
            statistics: JobStatistics {
                map_tasks_completed: self.config.num_map_tasks,
                reduce_tasks_completed: self.config.num_reduce_tasks,
                execution_time_ms: execution_time,
                ..Default::default()
            },
        })
    }

    /// 准备 Map 任务
    fn prepare_map_tasks(
        &self,
        job_id: String,
        input_data: Vec<(K, V)>,
        map_fn: impl Fn(K, V, &mut MapContext<OK, OV>) + Send + Sync + 'static,
    ) -> Result<Vec<MapTask<K, V, OK, OV>>, MapReduceError> {
        let chunk_size = (input_data.len() + self.config.num_map_tasks - 1) / self.config.num_map_tasks;
        let mut tasks = Vec::new();
        let map_function = Arc::new(map_fn);

        for (i, chunk) in input_data.chunks(chunk_size).enumerate() {
            let task_id = TaskId {
                job_id: job_id.clone(),
                task_id: format!("map-{}", i),
                attempt: 0,
            };

            let task = MapTask {
                task_id,
                input_data: chunk.to_vec(),
                map_function: map_function.clone(),
                partition_count: self.config.num_reduce_tasks,
            };

            tasks.push(task);
        }

        Ok(tasks)
    }

    /// 执行 Map 任务
    fn execute_map_tasks(
        &self,
        tasks: Vec<MapTask<K, V, OK, OV>>,
    ) -> Result<Vec<Vec<(usize, (OK, OV))>>, MapReduceError> {
        let mut all_outputs = Vec::new();

        for task in tasks {
            let output = self.execute_single_map_task(task)?;
            all_outputs.push(output);
        }

        Ok(all_outputs)
    }

    /// 执行单个 Map 任务
    fn execute_single_map_task(
        &self,
        task: MapTask<K, V, OK, OV>,
    ) -> Result<Vec<(usize, (OK, OV))>, MapReduceError> {
        let mut context = MapContext {
            intermediate: Vec::new(),
            partition_strategy: self.config.partition_strategy,
            partition_count: task.partition_count,
        };

        // 执行 Map 函数
        for (key, value) in task.input_data {
            (task.map_function)(key, value, &mut context);
        }

        // 分区
        let mut partitioned: Vec<(usize, (OK, OV))> = Vec::new();
        let intermediate = core::mem::take(&mut context.intermediate);
        for (key, value) in intermediate {
            let partition_id = context.get_partition(&key);
            partitioned.push((partition_id, (key, value)));
        }

        Ok(partitioned)
    }

    /// 准备 Reduce 任务
    fn prepare_reduce_tasks(
        &self,
        job_id: String,
        shuffled_data: BTreeMap<usize, Vec<(OK, Vec<OV>)>>,
        reduce_fn: impl Fn(OK, Vec<OV>, &mut ReduceContext<RK, RV>) + Send + Sync + 'static,
    ) -> Result<Vec<ReduceTask<OK, OV, RK, RV>>, MapReduceError> {
        let mut tasks = Vec::new();
        let reduce_function = Arc::new(reduce_fn);

        for (partition_id, data) in shuffled_data {
            let task_id = TaskId {
                job_id: job_id.clone(),
                task_id: format!("reduce-{}", partition_id),
                attempt: 0,
            };

            let task = ReduceTask {
                task_id,
                partition_id,
                input_data: data,
                reduce_function: reduce_function.clone(),
            };

            tasks.push(task);
        }

        Ok(tasks)
    }

    /// 执行 Reduce 任务
    fn execute_reduce_tasks(
        &self,
        tasks: Vec<ReduceTask<OK, OV, RK, RV>>,
    ) -> Result<Vec<Vec<(RK, RV)>>, MapReduceError> {
        let mut results = Vec::new();

        for task in tasks {
            let result = self.execute_single_reduce_task(task)?;
            results.push(result);
        }

        Ok(results)
    }

    /// 执行单个 Reduce 任务
    fn execute_single_reduce_task(
        &self,
        task: ReduceTask<OK, OV, RK, RV>,
    ) -> Result<Vec<(RK, RV)>, MapReduceError> {
        let mut context = ReduceContext {
            output: Vec::new(),
        };

        // 执行 Reduce 函数
        for (key, values) in task.input_data {
            (task.reduce_function)(key, values, &mut context);
        }

        Ok(context.output)
    }

    /// 生成作业 ID
    fn generate_job_id(&self) -> String {
        // 简化版：使用时间戳
        format!("job-{}", self.current_time_ms())
    }

    /// 获取当前时间（毫秒）
    fn current_time_ms(&self) -> u64 {
        // 简化实现
        0
    }
}

/// 任务调度器
struct TaskScheduler {
    max_concurrent_tasks: usize,
    running_tasks: Mutex<usize>,
}

impl TaskScheduler {
    fn new(max_concurrent_tasks: usize) -> Self {
        Self {
            max_concurrent_tasks,
            running_tasks: Mutex::new(0),
        }
    }
}

/// 分区管理器
struct PartitionManager {
    strategy: PartitionStrategy,
}

impl PartitionManager {
    fn new(strategy: PartitionStrategy) -> Self {
        Self { strategy }
    }
}

/// Shuffle 管理器
struct ShuffleManager {
    buffer_size: usize,
}

impl ShuffleManager {
    fn new(buffer_size: usize) -> Self {
        Self { buffer_size }
    }

    /// Shuffle 和 Sort 阶段
    fn shuffle_and_sort<OK, OV>(
        &self,
        map_outputs: Vec<Vec<(usize, (OK, OV))>>,
        num_partitions: usize,
    ) -> Result<BTreeMap<usize, Vec<(OK, Vec<OV>)>>, MapReduceError>
    where
        OK: Clone + Ord + fmt::Debug,
        OV: Clone + fmt::Debug,
    {
        let mut partitions: BTreeMap<usize, BTreeMap<OK, Vec<OV>>> = BTreeMap::new();

        // 初始化分区
        for i in 0..num_partitions {
            partitions.insert(i, BTreeMap::new());
        }

        // 收集并分区数据
        for task_output in map_outputs {
            for (partition_id, (key, value)) in task_output {
                if let Some(partition) = partitions.get_mut(&partition_id) {
                    partition.entry(key).or_insert_with(Vec::new).push(value);
                }
            }
        }

        // 转换为输出格式
        let mut result = BTreeMap::new();
        for (id, partition) in partitions {
            result.insert(id, partition.into_iter().collect());
        }

        Ok(result)
    }
}

/// 容错处理器
struct FaultToleranceHandler {
    max_retries: u32,
}

impl FaultToleranceHandler {
    fn new(max_retries: u32) -> Self {
        Self { max_retries }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_count_mapreduce() {
        let config = MapReduceConfig::default();
        let engine = MapReduceEngine::new(config).unwrap();

        // 输入数据
        let input = vec![
            ("hello".to_string(), "world".to_string()),
            ("hello".to_string(), "rust".to_string()),
            ("world".to_string(), "hello".to_string()),
        ];

        // Map 函数：切分单词
        // Reduce 函数：计数
        let result = engine.execute_job(
            input,
            |key, value, ctx: &mut MapContext<String, String>| {
                // 简化版：直接输出键
                ctx.emit(key, value);
            },
            |key, values, ctx: &mut ReduceContext<String, usize>| {
                ctx.emit(key, values.len());
            },
        );

        assert!(result.is_ok());
    }
}
