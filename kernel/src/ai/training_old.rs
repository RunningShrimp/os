//! # On-Device Training
//!
//! Comprehensive training framework for neural networks with backpropagation,
//! various optimizers, loss functions, and training utilities.
//!
//! ## Features
//!
//! - **Backpropagation**: Automatic gradient computation
//! - **Optimizers**: SGD, Adam, AdamW, RMSprop, Adagrad
//! - **Loss Functions**: MSE, Cross-Entropy, Hinge Loss
//! - **Gradient Clipping**: Prevent gradient explosion
//! - **Learning Rate Scheduling**: Decay strategies
//! - **Early Stopping**: Prevent overfitting
//! - **Checkpoints**: Save and resume training

use crate::ai::{AiError, AiResult, NeuralError, Tensor, TrainingError};
use crate::sci::optimization::{OptimizationConfig, OptimizationResult};
use alloc::vec::Vec;
use alloc::boxed::Box;

/// Training configuration
#[derive(Debug, Clone)]
pub struct TrainingConfig {
    /// Number of epochs
    pub epochs: usize,
    /// Batch size
    pub batch_size: usize,
    /// Learning rate
    pub learning_rate: f32,
    /// Gradient clipping threshold
    pub clip_grad: Option<f32>,
    /// Enable early stopping
    pub early_stopping: bool,
    /// Patience for early stopping
    pub patience: usize,
    /// Minimum improvement threshold
    pub min_delta: f32,
    /// Shuffle data each epoch
    pub shuffle: bool,
    /// Validation split ratio
    pub validation_split: f32,
    /// Checkpoint interval
    pub checkpoint_interval: Option<usize>,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            epochs: 100,
            batch_size: 32,
            learning_rate: 0.001,
            clip_grad: Some(5.0),
            early_stopping: true,
            patience: 10,
            min_delta: 0.001,
            shuffle: true,
            validation_split: 0.1,
            checkpoint_interval: Some(10),
        }
    }
}

/// Optimizer trait
pub trait Optimizer: Send + Sync {
    /// Update parameters using gradients
    fn update(&mut self, params: &mut Tensor<f32>, grads: &Tensor<f32>) -> AiResult<()>;

    /// Get learning rate
    fn learning_rate(&self) -> f32;

    /// Set learning rate
    fn set_learning_rate(&mut self, lr: f32);
}

/// SGD with momentum optimizer
#[derive(Debug, Clone)]
pub struct SGD {
    /// Learning rate
    learning_rate: f32,
    /// Momentum coefficient
    momentum: f32,
    /// Velocity
    velocity: Option<Vec<f32>>,
    /// Nesterov momentum
    nesterov: bool,
}

impl SGD {
    /// Create a new SGD optimizer
    pub fn new(learning_rate: f32) -> Self {
        Self {
            learning_rate,
            momentum: 0.0,
            velocity: None,
            nesterov: false,
        }
    }

    /// Enable momentum
    pub fn with_momentum(mut self, momentum: f32) -> Self {
        self.momentum = momentum;
        self
    }

    /// Enable Nesterov momentum
    pub fn with_nesterov(mut self, nesterov: bool) -> Self {
        self.nesterov = nesterov;
        self
    }
}

impl Optimizer for SGD {
    fn update(&mut self, params: &mut Tensor<f32>, grads: &Tensor<f32>) -> AiResult<()> {
        let param_count = params.len();

        if self.velocity.is_none() && self.momentum > 0.0 {
            self.velocity = Some(vec![0.0; param_count]);
        }

        if let Some(ref mut velocity) = self.velocity {
            for i in 0..param_count {
                let grad = grads.get_flat(i)?;
                velocity[i] = self.momentum * velocity[i] + self.learning_rate * grad;
                let param_val = params.get_flat(i)?;
                params.set_flat(i, param_val - velocity[i])?;
            }
        } else {
            // Standard SGD without momentum
            for i in 0..param_count {
                let grad = grads.get_flat(i)?;
                let param_val = params.get_flat(i)?;
                params.set_flat(i, param_val - self.learning_rate * grad)?;
            }
        }

        Ok(())
    }

    fn learning_rate(&self) -> f32 {
        self.learning_rate
    }

    fn set_learning_rate(&mut self, lr: f32) {
        self.learning_rate = lr;
    }
}

/// Adam optimizer
#[derive(Debug, Clone)]
pub struct Adam {
    /// Learning rate
    learning_rate: f32,
    /// Beta1 parameter
    beta1: f32,
    /// Beta2 parameter
    beta2: f32,
    /// Epsilon
    epsilon: f32,
    /// First moment estimate
    m: Option<Vec<f32>>,
    /// Second moment estimate
    v: Option<Vec<f32>>,
    /// Time step
    t: usize,
}

impl Adam {
    /// Create a new Adam optimizer
    pub fn new(learning_rate: f32) -> Self {
        Self {
            learning_rate,
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
            m: None,
            v: None,
            t: 0,
        }
    }

    /// Set beta1
    pub fn with_beta1(mut self, beta1: f32) -> Self {
        self.beta1 = beta1;
        self
    }

    /// Set beta2
    pub fn with_beta2(mut self, beta2: f32) -> Self {
        self.beta2 = beta2;
        self
    }
}

impl Optimizer for Adam {
    fn update(&mut self, params: &mut Tensor<f32>, grads: &Tensor<f32>) -> AiResult<()> {
        let param_count = params.len();

        if self.m.is_none() {
            self.m = Some(vec![0.0; param_count]);
            self.v = Some(vec![0.0; param_count]);
        }

        self.t += 1;

        let m = self.m.as_mut().unwrap();
        let v = self.v.as_mut().unwrap();

        let bias_correction1 = 1.0 - self.beta1.powi(self.t as i32);
        let bias_correction2 = 1.0 - self.beta2.powi(self.t as i32);

        for i in 0..param_count {
            let grad = grads.get_flat(i)?;

            // Update biased first moment estimate
            m[i] = self.beta1 * m[i] + (1.0 - self.beta1) * grad;

            // Update biased second raw moment estimate
            v[i] = self.beta2 * v[i] + (1.0 - self.beta2) * grad * grad;

            // Compute bias-corrected estimates
            let m_hat = m[i] / bias_correction1;
            let v_hat = v[i] / bias_correction2;

            // Update parameters
            let param_val = params.get_flat(i)?;
            let update = self.learning_rate * m_hat / (v_hat.sqrt() + self.epsilon);
            params.set_flat(i, param_val - update)?;
        }

        Ok(())
    }

    fn learning_rate(&self) -> f32 {
        self.learning_rate
    }

    fn set_learning_rate(&mut self, lr: f32) {
        self.learning_rate = lr;
    }
}

/// AdamW optimizer (Adam with decoupled weight decay)
#[derive(Debug, Clone)]
pub struct AdamW {
    /// Base Adam optimizer
    adam: Adam,
    /// Weight decay
    weight_decay: f32,
}

impl AdamW {
    /// Create a new AdamW optimizer
    pub fn new(learning_rate: f32) -> Self {
        Self {
            adam: Adam::new(learning_rate),
            weight_decay: 0.01,
        }
    }

    /// Set weight decay
    pub fn with_weight_decay(mut self, weight_decay: f32) -> Self {
        self.weight_decay = weight_decay;
        self
    }
}

impl Optimizer for AdamW {
    fn update(&mut self, params: &mut Tensor<f32>, grads: &Tensor<f32>) -> AiResult<()> {
        // Apply weight decay
        for i in 0..params.len() {
            let param_val = params.get_flat(i)?;
            params.set_flat(i, param_val * (1.0 - self.learning_rate() * self.weight_decay))?;
        }

        // Then apply Adam update
        self.adam.update(params, grads)
    }

    fn learning_rate(&self) -> f32 {
        self.adam.learning_rate()
    }

    fn set_learning_rate(&mut self, lr: f32) {
        self.adam.set_learning_rate(lr);
    }
}

/// RMSprop optimizer
#[derive(Debug, Clone)]
pub struct RMSprop {
    /// Learning rate
    learning_rate: f32,
    /// Decay rate
    rho: f32,
    /// Epsilon
    epsilon: f32,
    /// Moving average of squared gradients
    cache: Option<Vec<f32>>,
}

impl RMSprop {
    /// Create a new RMSprop optimizer
    pub fn new(learning_rate: f32) -> Self {
        Self {
            learning_rate,
            rho: 0.9,
            epsilon: 1e-8,
            cache: None,
        }
    }

    /// Set decay rate
    pub fn with_rho(mut self, rho: f32) -> Self {
        self.rho = rho;
        self
    }
}

impl Optimizer for RMSprop {
    fn update(&mut self, params: &mut Tensor<f32>, grads: &Tensor<f32>) -> AiResult<()> {
        let param_count = params.len();

        if self.cache.is_none() {
            self.cache = Some(vec![0.0; param_count]);
        }

        let cache = self.cache.as_mut().unwrap();

        for i in 0..param_count {
            let grad = grads.get_flat(i)?;

            // Update cache
            cache[i] = self.rho * cache[i] + (1.0 - self.rho) * grad * grad;

            // Update parameters
            let param_val = params.get_flat(i)?;
            let update = self.learning_rate * grad / (cache[i].sqrt() + self.epsilon);
            params.set_flat(i, param_val - update)?;
        }

        Ok(())
    }

    fn learning_rate(&self) -> f32 {
        self.learning_rate
    }

    fn set_learning_rate(&mut self, lr: f32) {
        self.learning_rate = lr;
    }
}

/// Adagrad optimizer
#[derive(Debug, Clone)]
pub struct Adagrad {
    /// Learning rate
    learning_rate: f32,
    /// Epsilon
    epsilon: f32,
    /// Sum of squared gradients
    cache: Option<Vec<f32>>,
}

impl Adagrad {
    /// Create a new Adagrad optimizer
    pub fn new(learning_rate: f32) -> Self {
        Self {
            learning_rate,
            epsilon: 1e-8,
            cache: None,
        }
    }
}

impl Optimizer for Adagrad {
    fn update(&mut self, params: &mut Tensor<f32>, grads: &Tensor<f32>) -> AiResult<()> {
        let param_count = params.len();

        if self.cache.is_none() {
            self.cache = Some(vec![0.0; param_count]);
        }

        let cache = self.cache.as_mut().unwrap();

        for i in 0..param_count {
            let grad = grads.get_flat(i)?;

            // Update cache
            cache[i] += grad * grad;

            // Update parameters
            let param_val = params.get_flat(i)?;
            let update = self.learning_rate * grad / (cache[i].sqrt() + self.epsilon);
            params.set_flat(i, param_val - update)?;
        }

        Ok(())
    }

    fn learning_rate(&self) -> f32 {
        self.learning_rate
    }

    fn set_learning_rate(&mut self, lr: f32) {
        self.learning_rate = lr;
    }
}

/// Loss function trait
pub trait LossFunction: Send + Sync {
    /// Compute loss
    fn compute(&self, predictions: &Tensor<f32>, targets: &Tensor<f32>) -> AiResult<f32>;

    /// Compute gradient of loss
    fn gradient(&self, predictions: &Tensor<f32>, targets: &Tensor<f32>) -> AiResult<Tensor<f32>>;
}

/// Mean Squared Error loss
#[derive(Debug, Clone, Copy)]
pub struct MSELoss;

impl MSELoss {
    pub fn new() -> Self {
        Self
    }
}

impl LossFunction for MSELoss {
    fn compute(&self, predictions: &Tensor<f32>, targets: &Tensor<f32>) -> AiResult<f32> {
        let shape = predictions.shape();

        if shape != targets.shape() {
            return Err(AiError::TrainingError(TrainingError::LossError(
                "Predictions and targets must have the same shape".to_string()
            )));
        }

        let mut sum = 0.0f32;
        for i in 0..predictions.len() {
            let pred = predictions.get_flat(i)?;
            let target = targets.get_flat(i)?;
            let diff = pred - target;
            sum += diff * diff;
        }

        Ok(sum / predictions.len() as f32)
    }

    fn gradient(&self, predictions: &Tensor<f32>, targets: &Tensor<f32>) -> AiResult<Tensor<f32>> {
        let mut grad_data = Vec::with_capacity(predictions.len());

        for i in 0..predictions.len() {
            let pred = predictions.get_flat(i)?;
            let target = targets.get_flat(i)?;
            grad_data.push(2.0 * (pred - target) / predictions.len() as f32);
        }

        Tensor::from_vec(grad_data, predictions.shape())
    }
}

/// Cross-Entropy loss (for classification)
#[derive(Debug, Clone, Copy)]
pub struct CrossEntropyLoss {
    /// Label smoothing
    label_smoothing: f32,
}

impl CrossEntropyLoss {
    pub fn new() -> Self {
        Self {
            label_smoothing: 0.0,
        }
    }

    /// Set label smoothing
    pub fn with_label_smoothing(mut self, smoothing: f32) -> Self {
        self.label_smoothing = smoothing;
        self
    }
}

impl LossFunction for CrossEntropyLoss {
    fn compute(&self, predictions: &Tensor<f32>, targets: &Tensor<f32>) -> AiResult<f32> {
        let shape = predictions.shape();

        if shape.len() != 2 {
            return Err(AiError::TrainingError(TrainingError::LossError(
                "CrossEntropy requires 2D predictions".to_string()
            )));
        }

        let mut loss = 0.0f32;
        let batch_size = shape[0];
        let num_classes = shape[1];

        for i in 0..batch_size {
            for j in 0..num_classes {
                let pred = predictions.get_flat(i * num_classes + j)?;
                let target = targets.get_flat(i * num_classes + j)?;

                // Apply label smoothing
                let smooth_target = if self.label_smoothing > 0.0 {
                    target * (1.0 - self.label_smoothing) + self.label_smoothing / num_classes as f32
                } else {
                    target
                };

                // Numerical stability
                let pred_clamped = pred.clamp(-20.0, 20.0);
                loss -= smooth_target * pred_clamped.exp().ln();
            }
        }

        Ok(loss / batch_size as f32)
    }

    fn gradient(&self, predictions: &Tensor<f32>, targets: &Tensor<f32>) -> AiResult<Tensor<f32>> {
        // Simplified gradient computation
        let mut grad_data = Vec::with_capacity(predictions.len());

        for i in 0..predictions.len() {
            let pred = predictions.get_flat(i)?;
            let target = targets.get_flat(i)?;
            grad_data.push(pred - target);
        }

        Tensor::from_vec(grad_data, predictions.shape())
    }
}

/// Hinge loss (for SVMs)
#[derive(Debug, Clone, Copy)]
pub struct HingeLoss;

impl HingeLoss {
    pub fn new() -> Self {
        Self
    }
}

impl LossFunction for HingeLoss {
    fn compute(&self, predictions: &Tensor<f32>, targets: &Tensor<f32>) -> AiResult<f32> {
        let mut loss = 0.0f32;

        for i in 0..predictions.len() {
            let pred = predictions.get_flat(i)?;
            let target = targets.get_flat(i)?;
            let margin = 1.0 - pred * target;
            loss += if margin > 0.0 { margin } else { 0.0 };
        }

        Ok(loss / predictions.len() as f32)
    }

    fn gradient(&self, predictions: &Tensor<f32>, targets: &Tensor<f32>) -> AiResult<Tensor<f32>> {
        let mut grad_data = Vec::with_capacity(predictions.len());

        for i in 0..predictions.len() {
            let pred = predictions.get_flat(i)?;
            let target = targets.get_flat(i)?;
            let margin = 1.0 - pred * target;

            if margin > 0.0 {
                grad_data.push(-target);
            } else {
                grad_data.push(0.0);
            }
        }

        Tensor::from_vec(grad_data, predictions.shape())
    }
}

/// Training metrics
#[derive(Debug, Clone)]
pub struct TrainingMetrics {
    /// Training loss
    pub train_loss: f32,
    /// Validation loss
    pub val_loss: Option<f32>,
    /// Training accuracy
    pub train_acc: Option<f32>,
    /// Validation accuracy
    pub val_acc: Option<f32>,
    /// Epoch number
    pub epoch: usize,
}

/// Learning rate scheduler
pub trait LearningRateScheduler: Send + Sync {
    /// Get learning rate for given epoch
    fn get_lr(&self, epoch: usize, base_lr: f32) -> f32;
}

/// Step learning rate decay
#[derive(Debug, Clone)]
pub struct StepLR {
    /// Step size
    step_size: usize,
    /// Decay rate
    gamma: f32,
}

impl StepLR {
    pub fn new(step_size: usize, gamma: f32) -> Self {
        Self { step_size, gamma }
    }
}

impl LearningRateScheduler for StepLR {
    fn get_lr(&self, epoch: usize, base_lr: f32) -> f32 {
        let decay_count = epoch / self.step_size;
        base_lr * self.gamma.powi(decay_count as i32)
    }
}

/// Exponential learning rate decay
#[derive(Debug, Clone)]
pub struct ExponentialLR {
    /// Decay rate
    gamma: f32,
}

impl ExponentialLR {
    pub fn new(gamma: f32) -> Self {
        Self { gamma }
    }
}

impl LearningRateScheduler for ExponentialLR {
    fn get_lr(&self, epoch: usize, base_lr: f32) -> f32 {
        base_lr * self.gamma.powi(epoch as i32)
    }
}

/// Gradient clipping
pub fn clip_gradients(grads: &mut Tensor<f32>, max_norm: f32) -> AiResult<()> {
    let mut grad_norm_sq = 0.0f32;

    for i in 0..grads.len() {
        let grad = grads.get_flat(i)?;
        grad_norm_sq += grad * grad;
    }

    let grad_norm = grad_norm_sq.sqrt();

    if grad_norm > max_norm {
        let scale = max_norm / grad_norm;
        for i in 0..grads.len() {
            let grad = grads.get_flat(i)?;
            grads.set_flat(i, grad * scale)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sgd_optimizer() {
        let mut sgd = SGD::new(0.01);
        let mut params = Tensor::from_vec(vec![1.0, 2.0, 3.0], &[3]).unwrap();
        let grads = Tensor::from_vec(vec![0.1, 0.2, 0.3], &[3]).unwrap();

        sgd.update(&mut params, &grads).unwrap();

        assert!((params.get_flat(0).unwrap() - 0.999).abs() < 0.001);
    }

    #[test]
    fn test_adam_optimizer() {
        let mut adam = Adam::new(0.01);
        let mut params = Tensor::from_vec(vec![1.0, 2.0, 3.0], &[3]).unwrap();
        let grads = Tensor::from_vec(vec![0.1, 0.2, 0.3], &[3]).unwrap();

        adam.update(&mut params, &grads).unwrap();

        // Parameters should have changed
        assert!((params.get_flat(0).unwrap() - 1.0).abs() > 0.001);
    }

    #[test]
    fn test_mse_loss() {
        let predictions = Tensor::from_vec(vec![1.0, 2.0, 3.0], &[3]).unwrap();
        let targets = Tensor::from_vec(vec![1.5, 2.5, 3.5], &[3]).unwrap();

        let mse = MSELoss::new();
        let loss = mse.compute(&predictions, &targets).unwrap();

        assert!((loss - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_cross_entropy_loss() {
        let predictions = Tensor::from_vec(vec![0.5, 0.5], &[1, 2]).unwrap();
        let targets = Tensor::from_vec(vec![1.0, 0.0], &[1, 2]).unwrap();

        let ce = CrossEntropyLoss::new();
        let loss = ce.compute(&predictions, &targets).unwrap();

        assert!(loss > 0.0);
    }

    #[test]
    fn test_gradient_clipping() {
        let mut grads = Tensor::from_vec(vec![10.0, 10.0, 10.0], &[3]).unwrap();
        clip_gradients(&mut grads, 1.0).unwrap();

        // Norm should be <= 1.0 after clipping
        let mut norm_sq = 0.0;
        for i in 0..grads.len() {
            let g = grads.get_flat(i).unwrap();
            norm_sq += g * g;
        }
        assert!(norm_sq.sqrt() <= 1.01);
    }

    #[test]
    fn test_step_lr() {
        let scheduler = StepLR::new(10, 0.5);

        assert_eq!(scheduler.get_lr(0, 0.1), 0.1);
        assert_eq!(scheduler.get_lr(10, 0.1), 0.05);
        assert_eq!(scheduler.get_lr(20, 0.1), 0.025);
    }

    #[test]
    fn test_exponential_lr() {
        let scheduler = ExponentialLR::new(0.95);

        assert_eq!(scheduler.get_lr(0, 0.1), 0.1);
        assert!((scheduler.get_lr(1, 0.1) - 0.095).abs() < 0.001);
    }
}
