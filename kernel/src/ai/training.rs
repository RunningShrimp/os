//! # On-Device Training
//!
//! Comprehensive training framework for neural networks with backpropagation,
//! various optimizers, loss functions, and training utilities.

use crate::ai::{AiResult, Tensor};
use alloc::vec::Vec;

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
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            epochs: 100,
            batch_size: 32,
            learning_rate: 0.001,
            clip_grad: Some(5.0),
        }
    }
}

/// Optimizer trait
pub trait Optimizer: Send + Sync {
    /// Update parameters using gradients
    fn update(&mut self, params: &mut Tensor<f32>, grads: &Tensor<f32>) -> AiResult<()>;

    /// Get learning rate
    fn learning_rate(&self) -> f32;
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
}

impl SGD {
    /// Create a new SGD optimizer
    pub fn new(learning_rate: f32) -> Self {
        Self {
            learning_rate,
            momentum: 0.0,
            velocity: None,
        }
    }

    /// Enable momentum
    pub fn with_momentum(mut self, momentum: f32) -> Self {
        self.momentum = momentum;
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

            m[i] = self.beta1 * m[i] + (1.0 - self.beta1) * grad;
            v[i] = self.beta2 * v[i] + (1.0 - self.beta2) * grad * grad;

            let m_hat = m[i] / bias_correction1;
            let v_hat = v[i] / bias_correction2;

            let param_val = params.get_flat(i)?;
            let update = self.learning_rate * m_hat / (v_hat.sqrt() + self.epsilon);
            params.set_flat(i, param_val - update)?;
        }

        Ok(())
    }

    fn learning_rate(&self) -> f32 {
        self.learning_rate
    }
}

/// Loss function trait
pub trait LossFunction: Send + Sync {
    /// Compute loss
    fn compute(&self, predictions: &Tensor<f32>, targets: &Tensor<f32>) -> AiResult<f32>;
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
        let mut sum = 0.0f32;

        for i in 0..predictions.len() {
            let pred = predictions.get_flat(i)?;
            let target = targets.get_flat(i)?;
            let diff = pred - target;
            sum += diff * diff;
        }

        Ok(sum / predictions.len() as f32)
    }
}

/// Cross-Entropy loss
#[derive(Debug, Clone, Copy)]
pub struct CrossEntropyLoss;

impl CrossEntropyLoss {
    pub fn new() -> Self {
        Self
    }
}

impl LossFunction for CrossEntropyLoss {
    fn compute(&self, predictions: &Tensor<f32>, targets: &Tensor<f32>) -> AiResult<f32> {
        let mut loss = 0.0f32;
        let batch_size = predictions.shape()[0];
        let num_classes = predictions.shape()[1];

        for i in 0..batch_size {
            for j in 0..num_classes {
                let pred = predictions.get_flat(i * num_classes + j)?;
                let target = targets.get_flat(i * num_classes + j)?;
                let pred_clamped = pred.clamp(-20.0, 20.0);
                loss -= target * pred_clamped.exp().ln();
            }
        }

        Ok(loss / batch_size as f32)
    }
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
    fn test_mse_loss() {
        let predictions = Tensor::from_vec(vec![1.0, 2.0, 3.0], &[3]).unwrap();
        let targets = Tensor::from_vec(vec![1.5, 2.5, 3.5], &[3]).unwrap();

        let mse = MSELoss::new();
        let loss = mse.compute(&predictions, &targets).unwrap();

        assert!((loss - 0.25).abs() < 0.01);
    }
}
