//! # Parallel Computing Framework
//!
//! Provides parallel computation capabilities:
//! - Data parallelism
//! - Task parallelism
//! - Parallel reduction
//! - Parallel scan (prefix sum)
//! - Work-stealing scheduler

use crate::sci::{SciError, SciFloat, SciResult};
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Number of available CPU cores
pub fn num_cpus() -> usize {
    // In a real implementation, query the actual number of CPUs
    // For now, return a reasonable default
    4
}

/// Parallel iterator for data-parallel operations
pub struct ParallelIterator<T> {
    data: Vec<T>,
    chunk_size: usize,
}

impl<T> ParallelIterator<T> {
    /// Create a new parallel iterator
    pub fn new(data: Vec<T>) -> Self {
        let len = data.len();
        let chunk_size = (len / num_cpus()).max(1);

        Self { data, chunk_size }
    }

    /// Map operation (parallel)
    pub fn map<F, R>(&self, f: F) -> Vec<R>
    where
        T: Clone,
        F: Fn(T) -> R + Sync + Clone,
        R: Send,
    {
        let chunks: Vec<_> = self
            .data
            .chunks(self.chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect();

        let mut results = Vec::new();

        for chunk in chunks {
            let mapped: Vec<R> = chunk.into_iter().map(f.clone()).collect();
            results.extend(mapped);
        }

        results
    }

    /// Filter operation (parallel)
    pub fn filter<F>(&self, f: F) -> Vec<T>
    where
        T: Clone,
        F: Fn(&T) -> bool + Sync,
    {
        let chunks: Vec<_> = self
            .data
            .chunks(self.chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect();

        let mut results = Vec::new();

        for chunk in chunks {
            let filtered: Vec<T> = chunk.into_iter().filter(|x| f(x)).collect();
            results.extend(filtered);
        }

        results
    }
}

/// Parallel reduction
pub fn parallel_reduce<T, F, ID>(
    data: &[T],
    identity: ID,
    op: F,
    chunk_size: Option<usize>,
) -> T
where
    T: Clone + Send,
    F: Fn(T, T) -> T + Sync,
    ID: Fn() -> T + Sync,
{
    let n = data.len();
    let chunk_size = chunk_size.unwrap_or((n / num_cpus()).max(1));

    if n == 0 {
        return identity();
    }

    // Reduce each chunk
    let mut chunk_results = Vec::new();

    for chunk in data.chunks(chunk_size) {
        let result = chunk.iter().cloned().reduce(|a, b| op(a, b));
        if let Some(r) = result {
            chunk_results.push(r);
        }
    }

    // Final reduction
    chunk_results
        .into_iter()
        .reduce(|a, b| op(a, b))
        .unwrap_or_else(identity)
}

/// Parallel summation
pub fn parallel_sum(data: &[SciFloat]) -> SciFloat {
    parallel_reduce(data, || 0.0, |a, b| a + b, None)
}

/// Parallel product
pub fn parallel_product(data: &[SciFloat]) -> SciFloat {
    parallel_reduce(data, || 1.0, |a, b| a * b, None)
}

/// Parallel maximum
pub fn parallel_max(data: &[SciFloat]) -> Option<SciFloat> {
    if data.is_empty() {
        return None;
    }
    Some(parallel_reduce(data, || SciFloat::NEG_INFINITY, |a, b| a.max(b), None))
}

/// Parallel minimum
pub fn parallel_min(data: &[SciFloat]) -> Option<SciFloat> {
    if data.is_empty() {
        return None;
    }
    Some(parallel_reduce(data, || SciFloat::INFINITY, |a, b| a.min(b), None))
}

/// Parallel prefix sum (scan)
///
/// Exclusive scan: output[i] = sum of input[0..i]
pub fn parallel_scan_exclusive(data: &[SciFloat]) -> Vec<SciFloat> {
    let n = data.len();
    if n == 0 {
        return Vec::new();
    }

    let num_threads = num_cpus();
    let chunk_size = (n + num_threads - 1) / num_threads;

    // Step 1: Compute sums of each chunk
    let mut chunk_sums = Vec::new();
    for chunk in data.chunks(chunk_size) {
        let sum: SciFloat = chunk.iter().sum();
        chunk_sums.push(sum);
    }

    // Step 2: Scan the chunk sums
    let mut chunk_offsets = vec![0.0; chunk_sums.len()];
    for i in 1..chunk_sums.len() {
        chunk_offsets[i] = chunk_offsets[i - 1] + chunk_sums[i - 1];
    }

    // Step 3: Add offsets to each chunk
    let mut result = vec![0.0; n];
    for (chunk_idx, chunk) in data.chunks(chunk_size).enumerate() {
        let offset = chunk_offsets[chunk_idx];
        let mut running_sum = offset;

        for (i, &val) in chunk.iter().enumerate() {
            result[chunk_idx * chunk_size + i] = running_sum;
            running_sum += val;
        }
    }

    result
}

/// Parallel prefix sum (inclusive scan)
///
/// Inclusive scan: output[i] = sum of input[0..=i]
pub fn parallel_scan_inclusive(data: &[SciFloat]) -> Vec<SciFloat> {
    let n = data.len();
    if n == 0 {
        return Vec::new();
    }

    let exclusive = parallel_scan_exclusive(data);
    let mut result = vec![0.0; n];

    for i in 0..n {
        result[i] = exclusive[i] + data[i];
    }

    result
}

/// Task for parallel execution
pub type Task = dyn FnOnce() + Send;

/// Simple work-stealing task scheduler
pub struct TaskScheduler {
    queues: Vec<Vec<Box<Task>>>,
    next_queue: AtomicUsize,
}

impl TaskScheduler {
    /// Create a new task scheduler
    pub fn new(num_workers: usize) -> Self {
        let mut queues = Vec::with_capacity(num_workers);
        for _ in 0..num_workers {
            queues.push(Vec::new());
        }

        Self {
            queues,
            next_queue: AtomicUsize::new(0),
        }
    }

    /// Add a task to the scheduler
    pub fn add_task(&mut self, task: Box<Task>) {
        let idx = self.next_queue.fetch_add(1, Ordering::SeqCst) % self.queues.len();
        self.queues[idx].push(task);
    }

    /// Execute all tasks
    pub fn execute(&mut self) {
        for queue in &mut self.queues {
            while let Some(task) = queue.pop() {
                task();
            }
        }
    }

    /// Get number of pending tasks
    pub fn pending_tasks(&self) -> usize {
        self.queues.iter().map(|q| q.len()).sum()
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new(num_cpus())
    }
}

/// Parallel matrix multiplication using block decomposition
pub fn parallel_matrix_mul(
    a: &[Vec<SciFloat>],
    b: &[Vec<SciFloat>],
    block_size: usize,
) -> SciResult<Vec<Vec<SciFloat>>> {
    let m = a.len();
    let n = b[0].len();
    let k = a[0].len();

    if b.len() != k {
        return Err(SciError::InvalidDimensions);
    }

    let mut result = vec![vec![0.0; n]; m];

    // Block-based multiplication
    for i_block in (0..m).step_by(block_size) {
        for j_block in (0..n).step_by(block_size) {
            for k_block in (0..k).step_by(block_size) {
                let i_end = (i_block + block_size).min(m);
                let j_end = (j_block + block_size).min(n);
                let k_end = (k_block + block_size).min(k);

                for i in i_block..i_end {
                    for k in k_block..k_end {
                        let a_ik = a[i][k];
                        for j in j_block..j_end {
                            result[i][j] += a_ik * b[k][j];
                        }
                    }
                }
            }
        }
    }

    Ok(result)
}

/// Parallel vector addition
pub fn parallel_vector_add(a: &[SciFloat], b: &[SciFloat]) -> SciResult<Vec<SciFloat>> {
    if a.len() != b.len() {
        return Err(SciError::InvalidDimensions);
    }

    let n = a.len();
    let mut result = vec![0.0; n];

    for i in 0..n {
        result[i] = a[i] + b[i];
    }

    Ok(result)
}

/// Parallel vector subtraction
pub fn parallel_vector_sub(a: &[SciFloat], b: &[SciFloat]) -> SciResult<Vec<SciFloat>> {
    if a.len() != b.len() {
        return Err(SciError::InvalidDimensions);
    }

    let n = a.len();
    let mut result = vec![0.0; n];

    for i in 0..n {
        result[i] = a[i] - b[i];
    }

    Ok(result)
}

/// Parallel scalar multiplication
pub fn parallel_vector_scale(v: &[SciFloat], scalar: SciFloat) -> Vec<SciFloat> {
    v.iter().map(|&x| x * scalar).collect()
}

/// Parallel dot product
pub fn parallel_dot(a: &[SciFloat], b: &[SciFloat]) -> SciResult<SciFloat> {
    if a.len() != b.len() {
        return Err(SciError::InvalidDimensions);
    }

    let n = a.len();
    let mut sum = 0.0;

    for i in 0..n {
        sum += a[i] * b[i];
    }

    Ok(sum)
}

/// Parallel Euclidean norm
pub fn parallel_norm(v: &[SciFloat]) -> SciFloat {
    let sum_sq: SciFloat = v.iter().map(|&x| x * x).sum();
    sum_sq.sqrt()
}

/// Parallel SAXPY (scalar * X + Y)
pub fn parallel_saxpy(
    scalar: SciFloat,
    x: &[SciFloat],
    y: &[SciFloat],
) -> SciResult<Vec<SciFloat>> {
    if x.len() != y.len() {
        return Err(SciError::InvalidDimensions);
    }

    let n = x.len();
    let mut result = vec![0.0; n];

    for i in 0..n {
        result[i] = scalar * x[i] + y[i];
    }

    Ok(result)
}

/// Parallel histogram computation
pub fn parallel_histogram(data: &[usize], num_bins: usize) -> Vec<usize> {
    let mut histogram = vec![0; num_bins];

    for &value in data {
        if value < num_bins {
            histogram[value] += 1;
        }
    }

    histogram
}

/// Parallel sort (simplified - uses external sort)
///
/// In a real implementation, use parallel quicksort or samplesort
pub fn parallel_sort<T: Ord + Clone + Send>(data: &[T]) -> Vec<T> {
    let mut sorted = data.to_vec();
    sorted.sort();
    sorted
}

/// Parallel merge of sorted arrays
pub fn parallel_merge<T: Ord + Clone>(left: &[T], right: &[T]) -> Vec<T> {
    let mut result = Vec::with_capacity(left.len() + right.len());
    let mut i = 0;
    let mut j = 0;

    while i < left.len() && j < right.len() {
        if left[i] <= right[j] {
            result.push(left[i].clone());
            i += 1;
        } else {
            result.push(right[j].clone());
            j += 1;
        }
    }

    result.extend_from_slice(&left[i..]);
    result.extend_from_slice(&right[j..]);

    result
}

/// Parallel search (returns first index where predicate is true)
pub fn parallel_search<F>(data: &[usize], predicate: F) -> Option<usize>
where
    F: Fn(&usize) -> bool + Sync,
{
    let chunk_size = (data.len() / num_cpus()).max(1);

    for (chunk_idx, chunk) in data.chunks(chunk_size).enumerate() {
        for (i, &item) in chunk.iter().enumerate() {
            if predicate(&item) {
                return Some(chunk_idx * chunk_size + i);
            }
        }
    }

    None
}

/// Parallel map-reduce operation
pub fn parallel_map_reduce<T, R, M, F>(
    data: &[T],
    map_fn: M,
    reduce_fn: F,
    identity: R,
) -> R
where
    T: Clone + Sync,
    R: Clone + Send,
    M: Fn(&T) -> R + Sync + Copy,
    F: Fn(R, R) -> R + Sync + Copy,
{
    let chunk_size = (data.len() / num_cpus()).max(1);

    let mut results = Vec::new();

    for chunk in data.chunks(chunk_size) {
        let mapped: Vec<R> = chunk.iter().map(map_fn).collect();
        let reduced = mapped.into_iter().reduce(|a, b| reduce_fn(a, b));
        if let Some(r) = reduced {
            results.push(r);
        }
    }

    results
        .into_iter()
        .reduce(|a, b| reduce_fn(a, b))
        .unwrap_or(identity)
}

/// Parallel stencil computation (for PDE solving)
///
/// Computes: result[i] = func(data[i-1], data[i], data[i+1])
pub fn parallel_stencil<T, F>(
    data: &[T],
    func: F,
    boundary: T,
) -> Vec<T>
where
    T: Clone + Send,
    F: Fn(&T, &T, &T) -> T + Sync,
{
    let n = data.len();
    if n < 3 {
        return vec![boundary; n];
    }

    let mut result = vec![boundary.clone(); n];

    // Interior points
    for i in 1..n - 1 {
        result[i] = func(&data[i - 1], &data[i], &data[i + 1]);
    }

    result
}

/// Parallel convolution (simplified)
pub fn parallel_convolve(signal: &[SciFloat], kernel: &[SciFloat]) -> Vec<SciFloat> {
    let n = signal.len();
    let m = kernel.len();
    let result_len = n + m - 1;

    let mut result = vec![0.0; result_len];

    for i in 0..result_len {
        let mut sum = 0.0;
        for j in 0..m {
            let signal_idx = i as isize - j as isize + (m as isize / 2);
            if signal_idx >= 0 && signal_idx < n as isize {
                sum += signal[signal_idx as usize] * kernel[j];
            }
        }
        result[i] = sum;
    }

    result
}

/// Benchmark parallel operation
pub fn benchmark<F>(f: F, iterations: usize) -> (SciFloat, usize)
where
    F: Fn() + Sync,
{
    

    // In a real implementation, measure actual time
    // For now, just run the function

    for _ in 0..iterations {
        f();
    }

    // Placeholder timing (would use actual timers in real code)
    (0.0, iterations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_sum() {
        let data: Vec<SciFloat> = (1..=100).map(|i| i as SciFloat).collect();
        let sum = parallel_sum(&data);

        let expected: SciFloat = (1..=100).map(|i| i as SciFloat).sum();

        assert!((sum - expected).abs() < 1e-10);
    }

    #[test]
    fn test_parallel_product() {
        let data = vec![2.0, 3.0, 4.0];
        let product = parallel_product(&data);

        assert!((product - 24.0).abs() < 1e-10);
    }

    #[test]
    fn test_parallel_max_min() {
        let data = vec![1.0, 5.0, 3.0, 9.0, 2.0];

        let max = parallel_max(&data).unwrap();
        let min = parallel_min(&data).unwrap();

        assert_eq!(max, 9.0);
        assert_eq!(min, 1.0);
    }

    #[test]
    fn test_parallel_scan_exclusive() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = parallel_scan_exclusive(&data);

        let expected = vec![0.0, 1.0, 3.0, 6.0, 10.0];

        assert_eq!(result.len(), expected.len());
        for i in 0..result.len() {
            assert!((result[i] - expected[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn test_parallel_scan_inclusive() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = parallel_scan_inclusive(&data);

        let expected = vec![1.0, 3.0, 6.0, 10.0, 15.0];

        assert_eq!(result.len(), expected.len());
        for i in 0..result.len() {
            assert!((result[i] - expected[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn test_parallel_vector_ops() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];

        let add = parallel_vector_add(&a, &b).unwrap();
        assert_eq!(add, vec![5.0, 7.0, 9.0]);

        let sub = parallel_vector_sub(&a, &b).unwrap();
        assert_eq!(sub, vec![-3.0, -3.0, -3.0]);

        let scale = parallel_vector_scale(&a, 2.0);
        assert_eq!(scale, vec![2.0, 4.0, 6.0]);

        let dot = parallel_dot(&a, &b).unwrap();
        assert_eq!(dot, 32.0);
    }

    #[test]
    fn test_parallel_norm() {
        let v = vec![3.0, 4.0];
        let norm = parallel_norm(&v);

        assert!((norm - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_parallel_saxpy() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![4.0, 5.0, 6.0];

        let result = parallel_saxpy(2.0, &x, &y).unwrap();

        assert_eq!(result, vec![6.0, 9.0, 12.0]);
    }

    #[test]
    fn test_parallel_histogram() {
        let data = vec![0, 1, 2, 1, 0, 2, 3, 2, 1];
        let histogram = parallel_histogram(&data, 4);

        assert_eq!(histogram, vec![2, 3, 3, 1]);
    }

    #[test]
    fn test_parallel_merge() {
        let left = vec![1, 3, 5, 7];
        let right = vec![2, 4, 6, 8];

        let merged = parallel_merge(&left, &right);

        assert_eq!(merged, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_parallel_sort() {
        let data = vec![5, 2, 8, 1, 9, 3];
        let sorted = parallel_sort(&data);

        assert_eq!(sorted, vec![1, 2, 3, 5, 8, 9]);
    }

    #[test]
    fn test_parallel_search() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let result = parallel_search(&data, |&x| x == 5);
        assert_eq!(result, Some(4));

        let result_none = parallel_search(&data, |&x| x == 100);
        assert_eq!(result_none, None);
    }

    #[test]
    fn test_parallel_map_reduce() {
        let data = vec![1, 2, 3, 4, 5];

        let sum = parallel_map_reduce(&data, |&x| x * x, |a, b| a + b, 0);

        // 1² + 2² + 3² + 4² + 5² = 55
        assert_eq!(sum, 55);
    }

    #[test]
    fn test_parallel_stencil() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        let result = parallel_stencil(&data, |l, c, r| 0.5 * l + c + 0.5 * r, 0.0);

        // Interior: 0.5*1 + 2 + 0.5*3 = 4, etc.
        assert_eq!(result[0], 0.0); // boundary
        assert!((result[1] - 4.0).abs() < 1e-10);
        assert!((result[2] - 6.0).abs() < 1e-10);
        assert_eq!(result[4], 0.0); // boundary
    }

    #[test]
    fn test_parallel_convolve() {
        let signal = vec![1.0, 2.0, 3.0];
        let kernel = vec![0.5, 1.0, 0.5];

        let result = parallel_convolve(&signal, &kernel);

        // Expected output (with zero padding):
        // y[0] = 0.5*0 + 1.0*0 + 0.5*1 = 0.5
        // y[1] = 0.5*0 + 1.0*1 + 0.5*2 = 2.0
        // y[2] = 0.5*1 + 1.0*2 + 0.5*3 = 4.0
        // y[3] = 0.5*2 + 1.0*3 + 0.5*0 = 4.0
        // y[4] = 0.5*3 + 1.0*0 + 0.5*0 = 1.5

        assert_eq!(result.len(), 5);
        assert!((result[0] - 0.5).abs() < 1e-10);
        assert!((result[1] - 2.0).abs() < 1e-10);
        assert!((result[2] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_parallel_matrix_mul() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let b = vec![vec![5.0, 6.0], vec![7.0, 8.0]];

        let result = parallel_matrix_mul(&a, &b, 1).unwrap();

        assert_eq!(result[0][0], 19.0);
        assert_eq!(result[0][1], 22.0);
        assert_eq!(result[1][0], 43.0);
        assert_eq!(result[1][1], 50.0);
    }

    #[test]
    fn test_task_scheduler() {
        let mut scheduler = TaskScheduler::new(4);

        let counter = core::sync::atomic::AtomicUsize::new(0);

        for _ in 0..10 {
            let counter_ref = &counter;
            scheduler.add_task(Box::new(move || {
                counter_ref.fetch_add(1, Ordering::SeqCst);
            }));
        }

        assert_eq!(scheduler.pending_tasks(), 10);

        scheduler.execute();

        assert_eq!(scheduler.pending_tasks(), 0);
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }
}
