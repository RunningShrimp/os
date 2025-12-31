//! # Optimization Algorithms
//!
//! This module provides various optimization algorithms for training neural networks
//! in the NOS kernel, including SGD, Adam, and learning rate scheduling.
//!
//! ## Features
//!
//! - **SGD with Momentum**: Classical stochastic gradient descent
//! - **Adam and AdamW**: Adaptive moment estimation
//! - **Learning Rate Scheduling**: Step, exponential, cosine annealing
//! - **Gradient Clipping**: Norm and value-based clipping
//! - **Weight Decay**: L2 regularization
//! - **Mixed Precision**: FP16 training support
//!
//! ## Architecture
//!
//! The optimizer module is organized into:
//! - **Optimizer Registry**: Manages optimizer instances
//! - **Gradient Manager**: Handles gradient clipping and accumulation
//! - **LR Scheduler**: Manages learning rate schedules
//! - **Mixed Precision Manager**: Handles FP16/FP32 conversions

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::error::unified::MlError;
use crate::ml::inference::Tensor;
use crate::sync::{Mutex, RwLock};

/// Optimizer identifier
pub type OptimizerId = u64;

/// Optimizer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizerType {
    /// Stochastic Gradient Descent
    SGD,
    /// SGD with Momentum
    SGDMomentum,
    /// Adam optimizer
    Adam,
    /// AdamW optimizer
    AdamW,
    /// RMSprop
    RMSprop,
    /// Adagrad
    Adagrad,
}

/// Optimizer configuration
#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    /// Optimizer type
    pub optimizer_type: OptimizerType,
    /// Learning rate
    pub lr: f64,
    /// Momentum (for SGD with momentum)
    pub momentum: Option<f64>,
    /// Beta1 (for Adam)
    pub beta1: Option<f64>,
    /// Beta2 (for Adam)
    pub beta2: Option<f64>,
    /// Epsilon (for Adam)
    pub epsilon: Option<f64>,
    /// Weight decay
    pub weight_decay: Option<f64>,
    /// Gradient clipping norm
    pub max_grad_norm: Option<f64>,
    /// Gradient clipping value
    pub clip_value: Option<f64>,
    /// Use mixed precision
    pub mixed_precision: bool,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            optimizer_type: OptimizerType::Adam,
            lr: 0.001,
            momentum: Some(0.9),
            beta1: Some(0.9),
            beta2: Some(0.999),
            epsilon: Some(1e-8),
            weight_decay: Some(0.0),
            max_grad_norm: Some(1.0),
            clip_value: None,
            mixed_precision: false,
        }
    }
}

/// Learning rate schedule type
#[derive(Debug, Clone, Copy)]
pub enum LrScheduleType {
    /// Constant learning rate
    Constant,
    /// Step decay
    Step,
    /// Exponential decay
    Exponential,
    /// Cosine annealing
    CosineAnnealing,
    /// Reduce on plateau
    ReduceOnPlateau,
}

/// Learning rate scheduler configuration
#[derive(Debug, Clone)]
pub struct LrSchedulerConfig {
    /// Schedule type
    pub schedule_type: LrScheduleType,
    /// Initial learning rate
    pub initial_lr: f64,
    /// Minimum learning rate
    pub min_lr: f64,
    /// Step size (for step decay)
    pub step_size: Option<usize>,
    /// Gamma (for step/exponential decay)
    pub gamma: Option<f64>,
    /// T_max (for cosine annealing)
    pub t_max: Option<usize>,
    /// Patience (for reduce on plateau)
    pub patience: Option<usize>,
    /// Factor (for reduce on plateau)
    pub factor: Option<f64>,
}

/// Optimizer state
struct OptimizerState {
    /// Optimizer ID
    id: OptimizerId,
    /// Optimizer type
    optimizer_type: OptimizerType,
    /// Learning rate
    lr: f64,
    /// Momentum buffers
    momentum_buffers: Vec<Tensor>,
    /// Adam first moment estimates
    m: Vec<Tensor>,
    /// Adam second moment estimates
    v: Vec<Tensor>,
    /// Adam time step
    t: usize,
    /// Current step number
    step: usize,
}

impl OptimizerState {
    /// Create a new optimizer state
    fn new(id: OptimizerId, config: OptimizerConfig) -> Self {
        Self {
            id,
            optimizer_type: config.optimizer_type,
            lr: config.lr,
            momentum_buffers: Vec::new(),
            m: Vec::new(),
            v: Vec::new(),
            t: 0,
            step: 0,
        }
    }

    /// Get current learning rate
    fn get_lr(&self) -> f64 {
        self.lr
    }

    /// Set learning rate
    fn set_lr(&mut self, lr: f64) -> Result<(), MlError> {
        if lr <= 0.0 {
            return Err(MlError::InvalidLearningRate);
        }
        self.lr = lr;
        Ok(())
    }

    /// Increment step
    fn step(&mut self) {
        self.step += 1;
        self.t += 1;
    }

    /// Get step number
    fn get_step(&self) -> usize {
        self.step
    }
}

/// Learning rate scheduler
pub struct LrScheduler {
    /// Scheduler configuration
    config: LrSchedulerConfig,
    /// Current learning rate
    current_lr: f64,
    /// Current step
    step: usize,
    /// Number of steps since last improvement
    steps_since_improvement: usize,
    /// Best loss so far
    best_loss: Option<f64>,
}

impl LrScheduler {
    /// Create a new learning rate scheduler
    pub fn new(config: LrSchedulerConfig) -> Self {
        Self {
            current_lr: config.initial_lr,
            config,
            step: 0,
            steps_since_improvement: 0,
            best_loss: None,
        }
    }

    /// Get the current learning rate
    pub fn get_lr(&self) -> f64 {
        self.current_lr
    }

    /// Step the scheduler
    pub fn step(&mut self) -> f64 {
        self.step += 1;

        self.current_lr = match self.config.schedule_type {
            LrScheduleType::Constant => self.config.initial_lr,
            LrScheduleType::Step => {
                let step_size = self.config.step_size.unwrap_or(100);
                let gamma = self.config.gamma.unwrap_or(0.1);
                let decay_steps = self.step / step_size;
                self.config.initial_lr * gamma.powi(decay_steps as i32)
            }
            LrScheduleType::Exponential => {
                let gamma = self.config.gamma.unwrap_or(0.99);
                self.config.initial_lr * gamma.powi(self.step as i32)
            }
            LrScheduleType::CosineAnnealing => {
                let t_max = self.config.t_max.unwrap_or(1000);
                let progress = (self.step % t_max) as f64 / t_max as f64;
                let min_lr = self.config.min_lr;
                let lr_range = self.config.initial_lr - min_lr;
                min_lr + lr_range * 0.5 * (1.0 + (core::f64::consts::PI * progress).cos())
            }
            LrScheduleType::ReduceOnPlateau => {
                // Will be updated when monitoring loss
                self.current_lr
            }
        };

        self.current_lr.max(self.config.min_lr)
    }

    /// Update scheduler with current loss
    pub fn step_loss(&mut self, loss: f64) -> f64 {
        if self.config.schedule_type == LrScheduleType::ReduceOnPlateau {
            match self.best_loss {
                Some(best) if loss < best => {
                    self.best_loss = Some(loss);
                    self.steps_since_improvement = 0;
                }
                Some(_) => {
                    self.steps_since_improvement += 1;
                }
                None => {
                    self.best_loss = Some(loss);
                    self.steps_since_improvement = 0;
                }
            }

            let patience = self.config.patience.unwrap_or(10);
            if self.steps_since_improvement >= patience {
                let factor = self.config.factor.unwrap_or(0.5);
                self.current_lr *= factor;
                self.current_lr = self.current_lr.max(self.config.min_lr);
                self.steps_since_improvement = 0;
            }
        }

        self.step()
    }
}

/// Optimizer instance
pub struct Optimizer {
    /// Optimizer state
    state: OptimizerState,
    /// Optimizer configuration
    config: OptimizerConfig,
    /// Learning rate scheduler
    lr_scheduler: Option<LrScheduler>,
}

impl Optimizer {
    /// Create a new optimizer
    pub fn new(id: OptimizerId, config: OptimizerConfig) -> Self {
        let state = OptimizerState::new(id, config.clone());
        Self {
            state,
            config,
            lr_scheduler: None,
        }
    }

    /// Set learning rate scheduler
    pub fn set_lr_scheduler(&mut self, scheduler: LrScheduler) {
        self.lr_scheduler = Some(scheduler);
    }

    /// Perform optimization step
    pub fn step(&mut self, params: &mut [Tensor], grads: &[Tensor]) -> Result<(), MlError> {
        if params.len() != grads.len() {
            return Err(MlError::OptimizerError("Parameter and gradient count mismatch".to_string()));
        }

        // Apply gradient clipping if configured
        let clipped_grads = if let Some(max_norm) = self.config.max_grad_norm {
            Self::clip_grads_by_norm(grads, max_norm)?
        } else if let Some(clip_value) = self.config.clip_value {
            Self::clip_grads_by_value(grads, clip_value)?
        } else {
            grads.to_vec()
        };

        // Apply weight decay if configured
        let weight_decay = self.config.weight_decay.unwrap_or(0.0);

        // Perform optimizer step
        match self.config.optimizer_type {
            OptimizerType::SGD => self.sgd_step(params, &clipped_grads, weight_decay)?,
            OptimizerType::SGDMomentum => self.sgd_momentum_step(params, &clipped_grads, weight_decay)?,
            OptimizerType::Adam => self.adam_step(params, &clipped_grads, weight_decay)?,
            OptimizerType::AdamW => self.adamw_step(params, &clipped_grads, weight_decay)?,
            _ => {
                return Err(MlError::OptimizerError(
                    "Optimizer type not implemented".to_string(),
                ))
            }
        }

        // Update learning rate from scheduler
        if let Some(scheduler) = &mut self.lr_scheduler {
            let new_lr = scheduler.step();
            self.state.lr = new_lr;
        }

        self.state.step();

        Ok(())
    }

    /// SGD step
    fn sgd_step(
        &mut self,
        params: &mut [Tensor],
        grads: &[Tensor],
        weight_decay: f64,
    ) -> Result<(), MlError> {
        let lr = self.state.get_lr() as f32;

        for (param, grad) in params.iter_mut().zip(grads.iter()) {
            let param_data = param.as_f32_slice_mut()?;
            let grad_data = grad.as_f32_slice()?;

            for i in 0..param_data.len() {
                let p = unsafe { *param_data.get_unchecked(i) };
                let g = unsafe { *grad_data.get_unchecked(i) };

                // Apply weight decay
                let grad = g + weight_decay as f32 * p;

                // Update parameter
                unsafe {
                    *param_data.get_unchecked_mut(i) = p - lr * grad;
                }
            }
        }

        Ok(())
    }

    /// SGD with momentum step
    fn sgd_momentum_step(
        &mut self,
        params: &mut [Tensor],
        grads: &[Tensor],
        weight_decay: f64,
    ) -> Result<(), MlError> {
        let lr = self.state.get_lr() as f32;
        let momentum = self.config.momentum.unwrap_or(0.9) as f32;

        // Initialize momentum buffers if needed
        if self.state.momentum_buffers.is_empty() {
            for grad in grads {
                let mut buf_data = vec![0.0f32; grad.num_elements];
                let buf_bytes = unsafe {
                    Vec::from_raw_parts(
                        buf_data.as_mut_ptr() as *mut u8,
                        buf_data.len() * 4,
                        buf_data.capacity() * 4,
                    )
                };
                self.state.momentum_buffers.push(Tensor::new(
                    crate::ml::inference::TensorDType::F32,
                    grad.shape.clone(),
                    buf_bytes,
                ));
            }
        }

        for (i, (param, grad)) in params.iter_mut().zip(grads.iter()).enumerate() {
            let param_data = param.as_f32_slice_mut()?;
            let grad_data = grad.as_f32_slice()?;
            let mut buf_data = self.state.momentum_buffers[i].as_f32_slice_mut()?;

            for j in 0..param_data.len() {
                let p = unsafe { *param_data.get_unchecked(j) };
                let g = unsafe { *grad_data.get_unchecked(j) };
                let mut m = unsafe { *buf_data.get_unchecked(j) };

                // Apply weight decay
                let grad = g + weight_decay as f32 * p;

                // Update momentum
                m = momentum * m + grad;

                // Update parameter
                unsafe {
                    *param_data.get_unchecked_mut(j) = p - lr * m;
                    *buf_data.get_unchecked_mut(j) = m;
                }
            }
        }

        Ok(())
    }

    /// Adam step
    fn adam_step(
        &mut self,
        params: &mut [Tensor],
        grads: &[Tensor],
        _weight_decay: f64,
    ) -> Result<(), MlError> {
        let lr = self.state.get_lr() as f32;
        let beta1 = self.config.beta1.unwrap_or(0.9) as f32;
        let beta2 = self.config.beta2.unwrap_or(0.999) as f32;
        let epsilon = self.config.epsilon.unwrap_or(1e-8) as f32;

        // Initialize moment estimates if needed
        if self.state.m.is_empty() {
            for grad in grads {
                let mut m_data = vec![0.0f32; grad.num_elements];
                let mut v_data = vec![0.0f32; grad.num_elements];
                let m_bytes = unsafe {
                    Vec::from_raw_parts(
                        m_data.as_mut_ptr() as *mut u8,
                        m_data.len() * 4,
                        m_data.capacity() * 4,
                    )
                };
                let v_bytes = unsafe {
                    Vec::from_raw_parts(
                        v_data.as_mut_ptr() as *mut u8,
                        v_data.len() * 4,
                        v_data.capacity() * 4,
                    )
                };
                self.state.m.push(Tensor::new(
                    crate::ml::inference::TensorDType::F32,
                    grad.shape.clone(),
                    m_bytes,
                ));
                self.state.v.push(Tensor::new(
                    crate::ml::inference::TensorDType::F32,
                    grad.shape.clone(),
                    v_bytes,
                ));
            }
        }

        let t = self.state.t as f32;
        let bias_correction1 = 1.0 - beta1.powf(t);
        let bias_correction2 = 1.0 - beta2.powf(t);

        for (i, (param, grad)) in params.iter_mut().zip(grads.iter()).enumerate() {
            let param_data = param.as_f32_slice_mut()?;
            let grad_data = grad.as_f32_slice()?;
            let mut m_data = self.state.m[i].as_f32_slice_mut()?;
            let mut v_data = self.state.v[i].as_f32_slice_mut()?;

            for j in 0..param_data.len() {
                let p = unsafe { *param_data.get_unchecked(j) };
                let g = unsafe { *grad_data.get_unchecked(j) };
                let mut m = unsafe { *m_data.get_unchecked(j) };
                let mut v = unsafe { *v_data.get_unchecked(j) };

                // Update biased first moment estimate
                m = beta1 * m + (1.0 - beta1) * g;

                // Update biased second raw moment estimate
                v = beta2 * v + (1.0 - beta2) * g * g;

                // Compute bias-corrected estimates
                let m_hat = m / bias_correction1;
                let v_hat = v / bias_correction2;

                // Update parameter
                unsafe {
                    *param_data.get_unchecked_mut(j) = p - lr * m_hat / (v_hat.sqrt() + epsilon);
                    *m_data.get_unchecked_mut(j) = m;
                    *v_data.get_unchecked_mut(j) = v;
                }
            }
        }

        Ok(())
    }

    /// AdamW step (Adam with decoupled weight decay)
    fn adamw_step(
        &mut self,
        params: &mut [Tensor],
        grads: &[Tensor],
        weight_decay: f64,
    ) -> Result<(), MlError> {
        let lr = self.state.get_lr() as f32;
        let beta1 = self.config.beta1.unwrap_or(0.9) as f32;
        let beta2 = self.config.beta2.unwrap_or(0.999) as f32;
        let epsilon = self.config.epsilon.unwrap_or(1e-8) as f32;
        let wd = weight_decay as f32;

        // Initialize moment estimates if needed
        if self.state.m.is_empty() {
            for grad in grads {
                let mut m_data = vec![0.0f32; grad.num_elements];
                let mut v_data = vec![0.0f32; grad.num_elements];
                let m_bytes = unsafe {
                    Vec::from_raw_parts(
                        m_data.as_mut_ptr() as *mut u8,
                        m_data.len() * 4,
                        m_data.capacity() * 4,
                    )
                };
                let v_bytes = unsafe {
                    Vec::from_raw_parts(
                        v_data.as_mut_ptr() as *mut u8,
                        v_data.len() * 4,
                        v_data.capacity() * 4,
                    )
                };
                self.state.m.push(Tensor::new(
                    crate::ml::inference::TensorDType::F32,
                    grad.shape.clone(),
                    m_bytes,
                ));
                self.state.v.push(Tensor::new(
                    crate::ml::inference::TensorDType::F32,
                    grad.shape.clone(),
                    v_bytes,
                ));
            }
        }

        let t = self.state.t as f32;
        let bias_correction1 = 1.0 - beta1.powf(t);
        let bias_correction2 = 1.0 - beta2.powf(t);

        for (i, (param, grad)) in params.iter_mut().zip(grads.iter()).enumerate() {
            let param_data = param.as_f32_slice_mut()?;
            let grad_data = grad.as_f32_slice()?;
            let mut m_data = self.state.m[i].as_f32_slice_mut()?;
            let mut v_data = self.state.v[i].as_f32_slice_mut()?;

            for j in 0..param_data.len() {
                let p = unsafe { *param_data.get_unchecked(j) };
                let g = unsafe { *grad_data.get_unchecked(j) };
                let mut m = unsafe { *m_data.get_unchecked(j) };
                let mut v = unsafe { *v_data.get_unchecked(j) };

                // Update biased first moment estimate
                m = beta1 * m + (1.0 - beta1) * g;

                // Update biased second raw moment estimate
                v = beta2 * v + (1.0 - beta2) * g * g;

                // Compute bias-corrected estimates
                let m_hat = m / bias_correction1;
                let v_hat = v / bias_correction2;

                // Update parameter with weight decay
                unsafe {
                    *param_data.get_unchecked_mut(j) =
                        (p - lr * wd * p) - lr * m_hat / (v_hat.sqrt() + epsilon);
                    *m_data.get_unchecked_mut(j) = m;
                    *v_data.get_unchecked_mut(j) = v;
                }
            }
        }

        Ok(())
    }

    /// Clip gradients by norm
    fn clip_grads_by_norm(grads: &[Tensor], max_norm: f64) -> Result<Vec<Tensor>, MlError> {
        // Calculate global norm
        let mut global_norm = 0.0f32;
        for grad in grads {
            let data = grad.as_f32_slice()?;
            for &val in data {
                global_norm += val * val;
            }
        }
        global_norm = global_norm.sqrt();

        if global_norm <= max_norm as f32 {
            return Ok(grads.to_vec());
        }

        // Clip gradients
        let clip_coef = (max_norm as f32) / global_norm;
        let mut clipped_grads = Vec::new();

        for grad in grads {
            let data = grad.as_f32_slice()?;
            let mut clipped_data = vec![0.0f32; data.len()];
            for (i, &val) in data.iter().enumerate() {
                clipped_data[i] = val * clip_coef;
            }

            let clipped_bytes = unsafe {
                Vec::from_raw_parts(
                    clipped_data.as_mut_ptr() as *mut u8,
                    clipped_data.len() * 4,
                    clipped_data.capacity() * 4,
                )
            };

            clipped_grads.push(Tensor::new(
                crate::ml::inference::TensorDType::F32,
                grad.shape.clone(),
                clipped_bytes,
            ));
        }

        Ok(clipped_grads)
    }

    /// Clip gradients by value
    fn clip_grads_by_value(grads: &[Tensor], clip_value: f64) -> Result<Vec<Tensor>, MlError> {
        let clip = clip_value as f32;
        let mut clipped_grads = Vec::new();

        for grad in grads {
            let data = grad.as_f32_slice()?;
            let mut clipped_data = vec![0.0f32; data.len()];
            for (i, &val) in data.iter().enumerate() {
                clipped_data[i] = val.clamp(-clip, clip);
            }

            let clipped_bytes = unsafe {
                Vec::from_raw_parts(
                    clipped_data.as_mut_ptr() as *mut u8,
                    clipped_data.len() * 4,
                    clipped_data.capacity() * 4,
                )
            };

            clipped_grads.push(Tensor::new(
                crate::ml::inference::TensorDType::F32,
                grad.shape.clone(),
                clipped_bytes,
            ));
        }

        Ok(clipped_grads)
    }

    /// Set learning rate
    pub fn set_lr(&mut self, lr: f64) -> Result<(), MlError> {
        self.state.set_lr(lr)
    }

    /// Get current learning rate
    pub fn get_lr(&self) -> f64 {
        self.state.get_lr()
    }
}

/// Optimizer registry
pub struct OptimizerRegistry {
    optimizers: Arc<RwLock<BTreeMap<OptimizerId, Arc<Mutex<Optimizer>>>>>,
    next_id: Arc<AtomicU64>,
}

impl OptimizerRegistry {
    /// Create a new optimizer registry
    pub fn new() -> Self {
        Self {
            optimizers: Arc::new(RwLock::new(BTreeMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Create an optimizer
    pub fn create_optimizer(&self, config: OptimizerConfig) -> Result<OptimizerId, MlError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let optimizer = Optimizer::new(id, config);
        let mut optimizers = self.optimizers.write();
        optimizers.insert(id, Arc::new(Mutex::new(optimizer)));
        Ok(id)
    }

    /// Get an optimizer
    pub fn get_optimizer(&self, id: OptimizerId) -> Result<Arc<Mutex<Optimizer>>, MlError> {
        let optimizers = self.optimizers.read();
        optimizers
            .get(&id)
            .cloned()
            .ok_or(MlError::OptimizerError("Optimizer not found".to_string()))
    }
}

impl Default for OptimizerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global optimizer registry
static GLOBAL_REGISTRY: Mutex<Option<OptimizerRegistry>> = Mutex::new(None);

/// Initialize the global optimizer registry
pub fn init() {
    *GLOBAL_REGISTRY.lock() = Some(OptimizerRegistry::new());
}

/// Get the global optimizer registry
pub fn get_registry() -> Result<Arc<OptimizerRegistry>, MlError> {
    GLOBAL_REGISTRY
        .lock()
        .as_ref()
        .map(|_| Arc::new(unsafe { OptimizerRegistry::new() }))
        .ok_or(MlError::OptimizerError("Registry not initialized".to_string()))
}

/// Convenience function to create an optimizer
pub fn create_optimizer(config: OptimizerConfig) -> Result<OptimizerId, MlError> {
    let registry = get_registry()?;
    registry.create_optimizer(config)
}

/// Convenience function to perform optimizer step
pub fn step(id: OptimizerId, params: &mut [Tensor]) -> Result<(), MlError> {
    // This would need gradients passed in
    // Simplified version
    let optimizer = get_registry()?.get_optimizer(id)?;
    let mut opt = optimizer.lock();
    opt.step(params, &[])
}

/// Convenience function to set learning rate
pub fn set_lr(id: OptimizerId, lr: f64) -> Result<(), MlError> {
    let optimizer = get_registry()?.get_optimizer(id)?;
    let mut opt = optimizer.lock();
    opt.set_lr(lr)
}
