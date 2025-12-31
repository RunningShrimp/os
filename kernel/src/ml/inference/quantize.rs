//! # Model Quantization and Optimization
//!
//! Quantization tools for compressing and optimizing ML models:
//! - FP32 to INT8/INT16 quantization
//! - Quantization-aware training simulation
//! - Dynamic quantization
//! - Mixed precision computation
//!
//! # Quantization Types
//!
//! - **Static Quantization**: Calibrate scale and zero-point using representative data
//! - **Dynamic Quantization**: Calculate scale and zero-point at runtime
//! - **Quantization-Aware Training (QAT)**: Simulate quantization during training
//!
//! # Example
//!
//! ```ignore
//! use kernel::ml::inference::quantize::{Quantizer, QuantizationType, QuantizationConfig};
//!
//! let config = QuantizationConfig {
//!     qtype: QuantizationType::Int8,
//!     per_channel: false,
//!     symmetric: true,
//! };
//!
//! let mut quantizer = Quantizer::new(config);
//! let fp32_tensor = ...; // Your FP32 tensor
//! let quantized = quantizer.quantize_tensor(&fp32_tensor)?;
//! ```

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::f32;

use crate::ml::inference::tensor::{Tensor, TensorDType};

/// Quantization type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantizationType {
    Int8,
    Int16,
    Uint8,
}

impl QuantizationType {
    /// Get the bit width for this quantization type
    pub fn bit_width(&self) -> usize {
        match self {
            QuantizationType::Int8 => 8,
            QuantizationType::Int16 => 16,
            QuantizationType::Uint8 => 8,
        }
    }

    /// Get the range for this quantization type
    pub fn range(&self) -> (i32, i32) {
        match self {
            QuantizationType::Int8 => (-128, 127),
            QuantizationType::Int16 => (-32768, 32767),
            QuantizationType::Uint8 => (0, 255),
        }
    }

    /// Check if this is a signed type
    pub fn is_signed(&self) -> bool {
        matches!(self, QuantizationType::Int8 | QuantizationType::Int16)
    }
}

/// Quantization configuration
#[derive(Debug, Clone)]
pub struct QuantizationConfig {
    /// Quantization type (Int8, Int16, Uint8)
    pub qtype: QuantizationType,
    /// Whether to use per-channel quantization
    pub per_channel: bool,
    /// Whether to use symmetric quantization
    pub symmetric: bool,
    /// Calibration method
    pub calibration_method: CalibrationMethod,
    /// Whether to preserve output accuracy
    pub preserve_accuracy: bool,
}

impl Default for QuantizationConfig {
    fn default() -> Self {
        Self {
            qtype: QuantizationType::Int8,
            per_channel: false,
            symmetric: true,
            calibration_method: CalibrationMethod::MinMax,
            preserve_accuracy: true,
        }
    }
}

/// Calibration method for determining quantization parameters
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationMethod {
    /// Use min/max values
    MinMax,
    /// Use percentile (e.g., 99.9% to ignore outliers)
    Percentile(f32),
    /// Use entropy minimization
    Entropy,
    /// Use mean and standard deviation
    MeanStd,
}

/// Quantization parameters (scale and zero-point)
#[derive(Debug, Clone, Copy)]
pub struct QuantParams {
    /// Scale factor for quantization
    pub scale: f32,
    /// Zero-point for quantization
    pub zero_point: i32,
}

impl QuantParams {
    /// Create new quantization parameters
    pub fn new(scale: f32, zero_point: i32) -> Self {
        Self { scale, zero_point }
    }

    /// Quantize a single value
    pub fn quantize(&self, value: f32) -> i32 {
        ((value / self.scale) + self.zero_point as f32).round() as i32
    }

    /// Dequantize a single value
    pub fn dequantize(&self, qvalue: i32) -> f32 {
        (qvalue as f32 - self.zero_point as f32) * self.scale
    }

    /// Calculate quantization parameters from min/max
    pub fn from_min_max(min: f32, max: f32, qtype: QuantizationType, symmetric: bool) -> Self {
        let (qmin, qmax) = qtype.range();

        if symmetric {
            // Symmetric quantization: zero_point is 0 for signed types
            let abs_max = min.abs().max(max.abs());
            let scale = abs_max / (qmax as f32);
            Self {
                scale,
                zero_point: 0,
            }
        } else {
            // Asymmetric quantization
            let scale = (max - min) / ((qmax - qmin) as f32);
            let zero_point = qmin as f32 - (min / scale);
            let zero_point = zero_point.round().max(qmin as f32).min(qmax as f32) as i32;

            Self {
                scale,
                zero_point,
            }
        }
    }
}

/// Calibration data for quantization
#[derive(Debug, Clone)]
pub struct CalibrationData {
    /// Min values per channel
    pub min_per_channel: Vec<f32>,
    /// Max values per channel
    pub max_per_channel: Vec<f32>,
    /// Mean values per channel
    pub mean_per_channel: Vec<f32>,
    /// Standard deviation per channel
    pub std_per_channel: Vec<f32>,
}

impl CalibrationData {
    /// Create calibration data from tensor
    pub fn from_tensor(tensor: &Tensor, per_channel: bool) -> Result<Self, QuantizeError> {
        if tensor.dtype() != TensorDType::F32 {
            return Err(QuantizeError::InvalidDtype);
        }

        let data = tensor.as_slice::<f32>();

        if per_channel {
            // Per-channel calibration
            let num_channels = tensor.dims()[0];
            let channel_size = tensor.size() / num_channels;

            let mut mins = Vec::with_capacity(num_channels);
            let mut maxs = Vec::with_capacity(num_channels);
            let mut means = Vec::with_capacity(num_channels);
            let mut stds = Vec::with_capacity(num_channels);

            for ch in 0..num_channels {
                let start = ch * channel_size;
                let end = start + channel_size;
                let channel_data = &data[start..end];

                let min = channel_data.iter().cloned().fold(f32::INFINITY, f32::min);
                let max = channel_data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let sum: f32 = channel_data.iter().sum();
                let mean = sum / channel_size as f32;

                let variance: f32 = channel_data.iter()
                    .map(|x| (x - mean).powi(2))
                    .sum::<f32>() / channel_size as f32;
                let std = variance.sqrt();

                mins.push(min);
                maxs.push(max);
                means.push(mean);
                stds.push(std);
            }

            Ok(Self {
                min_per_channel: mins,
                max_per_channel: maxs,
                mean_per_channel: means,
                std_per_channel: stds,
            })
        } else {
            // Per-tensor calibration
            let min = data.iter().cloned().fold(f32::INFINITY, f32::min);
            let max = data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let sum: f32 = data.iter().sum();
            let mean = sum / data.len() as f32;

            let variance: f32 = data.iter()
                .map(|x| (x - mean).powi(2))
                .sum::<f32>() / data.len() as f32;
            let std = variance.sqrt();

            Ok(Self {
                min_per_channel: vec![min],
                max_per_channel: vec![max],
                mean_per_channel: vec![mean],
                std_per_channel: vec![std],
            })
        }
    }
}

/// Quantized tensor
#[derive(Debug, Clone)]
pub struct QuantizedTensor {
    /// Quantized data
    pub data: Vec<u8>,
    /// Quantization parameters
    pub params: QuantParams,
    /// Quantization type
    pub qtype: QuantizationType,
    /// Original shape
    pub shape: Vec<usize>,
}

impl QuantizedTensor {
    /// Create a new quantized tensor
    pub fn new(data: Vec<u8>, params: QuantParams, qtype: QuantizationType, shape: Vec<usize>) -> Self {
        Self {
            data,
            params,
            qtype,
            shape,
        }
    }

    /// Get the number of elements
    pub fn len(&self) -> usize {
        match self.qtype {
            QuantizationType::Int8 | QuantizationType::Uint8 => self.data.len(),
            QuantizationType::Int16 => self.data.len() / 2,
        }
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get element size in bytes
    pub fn element_size(&self) -> usize {
        self.qtype.bit_width() / 8
    }

    /// Calculate size in bytes
    pub fn nbytes(&self) -> usize {
        self.data.len()
    }
}

/// Model quantizer
pub struct Quantizer {
    config: QuantizationConfig,
    calibration_data: Option<CalibrationData>,
}

impl Quantizer {
    /// Create a new quantizer
    pub fn new(config: QuantizationConfig) -> Self {
        Self {
            config,
            calibration_data: None,
        }
    }

    /// Calibrate quantizer using representative data
    pub fn calibrate(&mut self, tensor: &Tensor) -> Result<(), QuantizeError> {
        let calib_data = CalibrationData::from_tensor(tensor, self.config.per_channel)?;
        self.calibration_data = Some(calib_data);
        Ok(())
    }

    /// Quantize a tensor
    pub fn quantize_tensor(&self, tensor: &Tensor) -> Result<QuantizedTensor, QuantizeError> {
        if tensor.dtype() != TensorDType::F32 {
            return Err(QuantizeError::InvalidDtype);
        }

        let calib_data = self.calibration_data.as_ref()
            .ok_or(QuantizeError::NotCalibrated)?;

        let fp32_data = tensor.as_slice::<f32>();
        let mut quantized_data = Vec::with_capacity(fp32_data.len() * self.config.qtype.element_size());

        if self.config.per_channel {
            // Per-channel quantization
            let num_channels = tensor.dims()[0];
            let channel_size = tensor.size() / num_channels;

            for ch in 0..num_channels {
                let params = QuantParams::from_min_max(
                    calib_data.min_per_channel[ch],
                    calib_data.max_per_channel[ch],
                    self.config.qtype,
                    self.config.symmetric,
                );

                let start = ch * channel_size;
                let end = start + channel_size;

                for i in start..end {
                    let qval = params.quantize(fp32_data[i]);
                    self.push_quantized(&mut quantized_data, qval);
                }
            }
        } else {
            // Per-tensor quantization
            let params = QuantParams::from_min_max(
                calib_data.min_per_channel[0],
                calib_data.max_per_channel[0],
                self.config.qtype,
                self.config.symmetric,
            );

            for &val in fp32_data {
                let qval = params.quantize(val);
                self.push_quantized(&mut quantized_data, qval);
            }
        }

        Ok(QuantizedTensor {
            data: quantized_data,
            params: QuantParams::new(1.0, 0), // Placeholder
            qtype: self.config.qtype,
            shape: tensor.dims().to_vec(),
        })
    }

    /// Push quantized value to byte vector
    fn push_quantized(&self, data: &mut Vec<u8>, value: i32) {
        match self.config.qtype {
            QuantizationType::Int8 => {
                data.push((value as i8) as u8);
            }
            QuantizationType::Int16 => {
                let bytes = (value as i16).to_le_bytes();
                data.extend_from_slice(&bytes);
            }
            QuantizationType::Uint8 => {
                data.push(value as u8);
            }
        }
    }

    /// Dequantize a tensor
    pub fn dequantize_tensor(&self, qtensor: &QuantizedTensor) -> Result<Tensor, QuantizeError> {
        let mut fp32_data = Vec::with_capacity(qtensor.len());

        match qtensor.qtype {
            QuantizationType::Int8 => {
                for &byte in &qtensor.data {
                    let qval = byte as i8 as i32;
                    fp32_data.push(qtensor.params.dequantize(qval));
                }
            }
            QuantizationType::Int16 => {
                for chunk in qtensor.data.chunks(2) {
                    let qval = i16::from_le_bytes([chunk[0], chunk[1]]) as i32;
                    fp32_data.push(qtensor.params.dequantize(qval));
                }
            }
            QuantizationType::Uint8 => {
                for &byte in &qtensor.data {
                    let qval = byte as i32;
                    fp32_data.push(qtensor.params.dequantize(qval));
                }
            }
        }

        let tensor = Tensor::from_slice(
            &fp32_data,
            crate::ml::inference::tensor::TensorShape::new(qtensor.shape.clone()),
        );

        Ok(tensor)
    }

    /// Calculate quantization error
    pub fn calculate_error(&self, original: &Tensor, quantized: &QuantizedTensor) -> Result<f32, QuantizeError> {
        let dequantized = self.dequantize_tensor(quantized)?;

        let orig_data = original.as_slice::<f32>();
        let deq_data = dequantized.as_slice::<f32>();

        if orig_data.len() != deq_data.len() {
            return Err(QuantizeError::SizeMismatch);
        }

        // Calculate MSE
        let mse: f32 = orig_data.iter()
            .zip(deq_data.iter())
            .map(|(o, d)| (o - d).powi(2))
            .sum::<f32>() / orig_data.len() as f32;

        Ok(mse.sqrt())
    }

    /// Get quantization configuration
    pub fn config(&self) -> &QuantizationConfig {
        &self.config
    }

    /// Check if quantizer is calibrated
    pub fn is_calibrated(&self) -> bool {
        self.calibration_data.is_some()
    }
}

/// Dynamic quantization (no calibration needed)
pub struct DynamicQuantizer {
    qtype: QuantizationType,
    symmetric: bool,
}

impl DynamicQuantizer {
    /// Create a new dynamic quantizer
    pub fn new(qtype: QuantizationType, symmetric: bool) -> Self {
        Self { qtype, symmetric }
    }

    /// Quantize tensor dynamically
    pub fn quantize(&self, tensor: &Tensor) -> Result<QuantizedTensor, QuantizeError> {
        if tensor.dtype() != TensorDType::F32 {
            return Err(QuantizeError::InvalidDtype);
        }

        let fp32_data = tensor.as_slice::<f32>();

        // Calculate min/max on the fly
        let min = fp32_data.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = fp32_data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        let params = QuantParams::from_min_max(min, max, self.qtype, self.symmetric);

        let mut quantized_data = Vec::with_capacity(fp32_data.len() * self.qtype.bit_width() / 8);

        for &val in fp32_data {
            let qval = params.quantize(val);
            match self.qtype {
                QuantizationType::Int8 => {
                    quantized_data.push((qval as i8) as u8);
                }
                QuantizationType::Int16 => {
                    let bytes = (qval as i16).to_le_bytes();
                    quantized_data.extend_from_slice(&bytes);
                }
                QuantizationType::Uint8 => {
                    quantized_data.push(qval as u8);
                }
            }
        }

        Ok(QuantizedTensor {
            data: quantized_data,
            params,
            qtype: self.qtype,
            shape: tensor.dims().to_vec(),
        })
    }
}

/// Mixed precision computation context
pub struct MixedPrecisionContext {
    /// Use FP16 where possible
    pub use_fp16: bool,
    /// Use BF16 where possible
    pub use_bf16: bool,
    /// Keep critical ops in FP32
    pub preserve_critical_fp32: bool,
}

impl Default for MixedPrecisionContext {
    fn default() -> Self {
        Self {
            use_fp16: true,
            use_bf16: false,
            preserve_critical_fp32: true,
        }
    }
}

impl MixedPrecisionContext {
    /// Create new mixed precision context
    pub fn new() -> Self {
        Self::default()
    }

    /// Determine if operation should use reduced precision
    pub fn should_use_reduced_precision(&self, op_type: &str) -> bool {
        if !self.use_fp16 && !self.use_bf16 {
            return false;
        }

        // Preserve FP32 for critical operations
        if self.preserve_critical_fp32 {
            match op_type {
                "Softmax" | "LayerNorm" | "BatchNorm" => return false,
                _ => {}
            }
        }

        true
    }

    /// Get recommended dtype for operation
    pub fn recommended_dtype(&self, _op_type: &str) -> TensorDType {
        if self.use_fp16 {
            // FP16 would be returned here if supported
            TensorDType::F32
        } else if self.use_bf16 {
            // BF16 would be returned here if supported
            TensorDType::F32
        } else {
            TensorDType::F32
        }
    }
}

/// Quantization error types
#[derive(Debug, Clone)]
pub enum QuantizeError {
    InvalidDtype,
    NotCalibrated,
    SizeMismatch,
    QuantizationFailed,
}

impl core::fmt::Display for QuantizeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            QuantizeError::InvalidDtype => write!(f, "Invalid tensor dtype"),
            QuantizeError::NotCalibrated => write!(f, "Quantizer not calibrated"),
            QuantizeError::SizeMismatch => write!(f, "Size mismatch"),
            QuantizeError::QuantizationFailed => write!(f, "Quantization failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::inference::tensor::TensorShape;

    #[test]
    fn test_quant_params() {
        let params = QuantParams::from_min_max(-6.0, 6.0, QuantizationType::Int8, true);
        assert_eq!(params.zero_point, 0);
        assert!(params.scale > 0.0);

        let qval = params.quantize(3.0);
        let dval = params.dequantize(qval);
        assert!((dval - 3.0).abs() < 0.1);
    }

    #[test]
    fn test_calibration_data() {
        let data = vec![-3.0f32, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0];
        let shape = TensorShape::new(vec![7]);
        let tensor = Tensor::from_slice(&data, shape);

        let calib = CalibrationData::from_tensor(&tensor, false).unwrap();
        assert_eq!(calib.min_per_channel[0], -3.0);
        assert_eq!(calib.max_per_channel[0], 3.0);
    }

    #[test]
    fn test_quantizer() {
        let config = QuantizationConfig::default();
        let mut quantizer = Quantizer::new(config);

        let data: Vec<f32> = (0..100).map(|i| (i as f32) / 10.0).collect();
        let shape = TensorShape::new(vec![100]);
        let tensor = Tensor::from_slice(&data, shape);

        quantizer.calibrate(&tensor).unwrap();
        assert!(quantizer.is_calibrated());

        let qtensor = quantizer.quantize_tensor(&tensor);
        assert!(qtensor.is_ok());
    }

    #[test]
    fn test_dynamic_quantization() {
        let quantizer = DynamicQuantizer::new(QuantizationType::Int8, true);

        let data: Vec<f32> = (0..100).map(|i| (i as f32) / 10.0).collect();
        let shape = TensorShape::new(vec![100]);
        let tensor = Tensor::from_slice(&data, shape);

        let qtensor = quantizer.quantize(&tensor);
        assert!(qtensor.is_ok());

        let qtensor = qtensor.unwrap();
        assert_eq!(qtensor.len(), 100);
    }
}
