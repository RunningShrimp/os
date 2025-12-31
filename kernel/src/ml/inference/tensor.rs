//! # Tensor Data Structures
//!
//! Multi-dimensional tensor implementation for ML operations with:
//! - Zero-copy memory management
//! - Efficient shape operations
//! - SIMD-optimized data layout
//! - Memory view support

use alloc::vec::Vec;
use alloc::sync::Arc;
use core::ops::{Deref, DerefMut};
use core::fmt;

/// Tensor data type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TensorDType {
    F32,
    F64,
    I32,
    I64,
    I8,
    I16,
    U8,
    U16,
    Bool,
}

impl TensorDType {
    pub fn size(&self) -> usize {
        match self {
            TensorDType::F32 | TensorDType::I32 => 4,
            TensorDType::F64 | TensorDType::I64 => 8,
            TensorDType::I8 | TensorDType::U8 => 1,
            TensorDType::I16 | TensorDType::U16 => 2,
            TensorDType::Bool => 1,
        }
    }

    pub fn is_float(&self) -> bool {
        matches!(self, TensorDType::F32 | TensorDType::F64)
    }

    pub fn is_int(&self) -> bool {
        matches!(self, TensorDType::I32 | TensorDType::I64 | TensorDType::I8 | TensorDType::I16)
    }
}

/// Tensor shape with dimensions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TensorShape {
    dims: Vec<usize>,
}

impl TensorShape {
    pub fn new(dims: Vec<usize>) -> Self {
        Self { dims }
    }

    pub fn from_slice(dims: &[usize]) -> Self {
        Self { dims: dims.to_vec() }
    }

    pub fn ndim(&self) -> usize {
        self.dims.len()
    }

    pub fn dims(&self) -> &[usize] {
        &self.dims
    }

    pub fn size(&self) -> usize {
        self.dims.iter().product()
    }

    pub fn is_empty(&self) -> bool {
        self.dims.is_empty() || self.size() == 0
    }

    /// Calculate strides for contiguous memory layout
    pub fn strides(&self) -> Vec<usize> {
        let mut strides = vec![0; self.ndim()];
        if self.ndim() == 0 {
            return strides;
        }

        strides[self.ndim() - 1] = 1;
        for i in (0..self.ndim() - 1).rev() {
            strides[i] = strides[i + 1] * self.dims[i + 1];
        }
        strides
    }

    /// Reshape if compatible
    pub fn reshape(&self, new_dims: &[usize]) -> Option<TensorShape> {
        if self.size() != new_dims.iter().product::<usize>() {
            return None;
        }
        Some(TensorShape::new(new_dims.to_vec()))
    }
}

impl fmt::Display for TensorShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, dim) in self.dims.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", dim)?;
        }
        write!(f, "]")
    }
}

/// Memory storage for tensor data
pub enum TensorStorage {
    Owned(Vec<u8>),
    Borrowed(*const u8, usize),
    Shared(Arc<Vec<u8>>),
}

unsafe impl Send for TensorStorage {}
unsafe impl Sync for TensorStorage {}

/// Multi-dimensional tensor
pub struct Tensor {
    dtype: TensorDType,
    shape: TensorShape,
    storage: TensorStorage,
    strides: Vec<usize>,
    offset: usize,
}

impl Tensor {
    /// Create a new owned tensor
    pub fn new<T: TensorElement>(dtype: TensorDType, shape: TensorShape) -> Self {
        let total_bytes = shape.size() * dtype.size();
        let mut data = Vec::with_capacity(total_bytes);
        data.resize(total_bytes, 0);

        Self {
            dtype,
            shape,
            storage: TensorStorage::Owned(data),
            strides: shape.strides(),
            offset: 0,
        }
    }

    /// Create tensor from raw data
    pub fn from_raw<T: TensorElement>(
        data: Vec<u8>,
        dtype: TensorDType,
        shape: TensorShape,
    ) -> Self {
        Self {
            dtype,
            shape,
            storage: TensorStorage::Owned(data),
            strides: shape.strides(),
            offset: 0,
        }
    }

    /// Create tensor from slice
    pub fn from_slice<T: TensorElement>(data: &[T], shape: TensorShape) -> Self {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                data.len() * core::mem::size_of::<T>(),
            )
        };

        Self {
            dtype: T::dtype(),
            shape,
            storage: TensorStorage::Owned(bytes.to_vec()),
            strides: shape.strides(),
            offset: 0,
        }
    }

    /// Get tensor dtype
    pub fn dtype(&self) -> TensorDType {
        self.dtype
    }

    /// Get tensor shape
    pub fn shape(&self) -> &TensorShape {
        &self.shape
    }

    /// Get tensor dimensions
    pub fn dims(&self) -> &[usize] {
        self.shape.dims()
    }

    /// Get number of dimensions
    pub fn ndim(&self) -> usize {
        self.shape.ndim()
    }

    /// Get total number of elements
    pub fn size(&self) -> usize {
        self.shape.size()
    }

    /// Get tensor size in bytes
    pub fn nbytes(&self) -> usize {
        self.size() * self.dtype.size()
    }

    /// Check if tensor is contiguous
    pub fn is_contiguous(&self) -> bool {
        let expected_strides = self.shape.strides();
        self.strides == expected_strides
    }

    /// Get raw data pointer
    pub fn as_ptr(&self) -> *const u8 {
        match &self.storage {
            TensorStorage::Owned(data) => data.as_ptr().add(self.offset),
            TensorStorage::Borrowed(ptr, _) => unsafe { ptr.add(self.offset) },
            TensorStorage::Shared(data) => data.as_ptr().add(self.offset),
        }
    }

    /// Get mutable raw data pointer
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        match &mut self.storage {
            TensorStorage::Owned(data) => data.as_mut_ptr().add(self.offset),
            TensorStorage::Shared(_) => panic!("Cannot get mutable pointer to shared storage"),
            TensorStorage::Borrowed(_, _) => panic!("Cannot get mutable pointer to borrowed storage"),
        }
    }

    /// Get data as typed slice
    pub fn as_slice<T: TensorElement>(&self) -> &[T] {
        assert_eq!(self.dtype, T::dtype(), "Type mismatch");
        unsafe {
            core::slice::from_raw_parts(
                self.as_ptr() as *const T,
                self.size(),
            )
        }
    }

    /// Get data as mutable typed slice
    pub fn as_mut_slice<T: TensorElement>(&mut self) -> &mut [T] {
        assert_eq!(self.dtype, T::dtype(), "Type mismatch");
        unsafe {
            core::slice::from_raw_parts_mut(
                self.as_mut_ptr() as *mut T,
                self.size(),
            )
        }
    }

    /// Get value at indices
    pub fn get<T: TensorElement>(&self, indices: &[usize]) -> Option<T> {
        if indices.len() != self.ndim() {
            return None;
        }

        let offset = self.offset
            + indices.iter().zip(self.strides.iter())
            .map(|(idx, stride)| idx * stride)
            .sum::<usize>();

        unsafe {
            let ptr = self.as_ptr().add(offset) as *const T;
            Some(ptr.read_unaligned())
        }
    }

    /// Set value at indices
    pub fn set<T: TensorElement>(&mut self, indices: &[usize], value: T) -> bool {
        if indices.len() != self.ndim() {
            return false;
        }

        let offset = self.offset
            + indices.iter().zip(self.strides.iter())
            .map(|(idx, stride)| idx * stride)
            .sum::<usize>();

        unsafe {
            let ptr = self.as_mut_ptr().add(offset) as *mut T;
            ptr.write_unaligned(value);
        }
        true
    }

    /// Reshape tensor
    pub fn reshape(&self, new_shape: TensorShape) -> Option<Tensor> {
        if !self.is_contiguous() {
            return None;
        }

        if self.size() != new_shape.size() {
            return None;
        }

        Some(Tensor {
            dtype: self.dtype,
            shape: new_shape,
            storage: match &self.storage {
                TensorStorage::Owned(data) => TensorStorage::Owned(data.clone()),
                TensorStorage::Shared(data) => TensorStorage::Shared(Arc::clone(data)),
                TensorStorage::Borrowed(ptr, len) => TensorStorage::Borrowed(*ptr, *len),
            },
            strides: new_shape.strides(),
            offset: self.offset,
        })
    }

    /// Create a view of this tensor
    pub fn view(&self) -> Tensor {
        Tensor {
            dtype: self.dtype,
            shape: self.shape.clone(),
            storage: match &self.storage {
                TensorStorage::Owned(data) => TensorStorage::Shared(Arc::new(data.clone())),
                TensorStorage::Shared(data) => TensorStorage::Shared(Arc::clone(data)),
                TensorStorage::Borrowed(ptr, len) => TensorStorage::Borrowed(*ptr, *len),
            },
            strides: self.strides.clone(),
            offset: self.offset,
        }
    }

    /// Slice tensor along dimension
    pub fn slice(&self, dim: usize, start: usize, end: usize) -> Option<Tensor> {
        if dim >= self.ndim() {
            return None;
        }

        if start > end || end > self.dims()[dim] {
            return None;
        }

        let mut new_shape = self.shape.clone();
        new_shape.dims[dim] = end - start;

        let mut new_strides = self.strides.clone();
        let new_offset = self.offset + start * self.strides[dim];

        Some(Tensor {
            dtype: self.dtype,
            shape: new_shape,
            storage: match &self.storage {
                TensorStorage::Owned(data) => TensorStorage::Shared(Arc::new(data.clone())),
                TensorStorage::Shared(data) => TensorStorage::Shared(Arc::clone(data)),
                TensorStorage::Borrowed(ptr, len) => TensorStorage::Borrowed(*ptr, *len),
            },
            strides: new_strides,
            offset: new_offset,
        })
    }

    /// Transpose tensor
    pub fn transpose(&self, dim1: usize, dim2: usize) -> Option<Tensor> {
        if dim1 >= self.ndim() || dim2 >= self.ndim() {
            return None;
        }

        let mut new_dims = self.dims().to_vec();
        new_dims.swap(dim1, dim2);

        let mut new_strides = self.strides.clone();
        new_strides.swap(dim1, dim2);

        Some(Tensor {
            dtype: self.dtype,
            shape: TensorShape::new(new_dims),
            storage: match &self.storage {
                TensorStorage::Owned(data) => TensorStorage::Shared(Arc::new(data.clone())),
                TensorStorage::Shared(data) => TensorStorage::Shared(Arc::clone(data)),
                TensorStorage::Borrowed(ptr, len) => TensorStorage::Borrowed(*ptr, *len),
            },
            strides: new_strides,
            offset: self.offset,
        })
    }

    /// Clone tensor data
    pub fn to_owned(&self) -> Tensor {
        let mut data = Vec::with_capacity(self.nbytes());
        unsafe {
            data.extend_from_slice(core::slice::from_raw_parts(self.as_ptr(), self.nbytes()));
        }

        Tensor {
            dtype: self.dtype,
            shape: self.shape.clone(),
            storage: TensorStorage::Owned(data),
            strides: self.shape.strides(),
            offset: 0,
        }
    }

    /// Fill tensor with value
    pub fn fill<T: TensorElement>(&mut self, value: T) {
        let slice = self.as_mut_slice::<T>();
        for elem in slice {
            *elem = value;
        }
    }

    /// Zero tensor
    pub fn zero(&mut self) {
        match self.dtype {
            TensorDType::F32 => self.fill(0.0f32),
            TensorDType::F64 => self.fill(0.0f64),
            TensorDType::I32 => self.fill(0i32),
            TensorDType::I64 => self.fill(0i64),
            TensorDType::I8 => self.fill(0i8),
            TensorDType::I16 => self.fill(0i16),
            TensorDType::U8 => self.fill(0u8),
            TensorDType::U16 => self.fill(0u16),
            TensorDType::Bool => self.fill(false),
        }
    }
}

impl Clone for Tensor {
    fn clone(&self) -> Self {
        self.to_owned()
    }
}

/// Marker trait for tensor element types
pub trait TensorElement: Copy + 'static {
    fn dtype() -> TensorDType;
}

macro_rules! impl_tensor_element {
    ($ty:ty, $dtype:expr) => {
        impl TensorElement for $ty {
            fn dtype() -> TensorDType {
                $dtype
            }
        }
    };
}

impl_tensor_element!(f32, TensorDType::F32);
impl_tensor_element!(f64, TensorDType::F64);
impl_tensor_element!(i32, TensorDType::I32);
impl_tensor_element!(i64, TensorDType::I64);
impl_tensor_element!(i8, TensorDType::I8);
impl_tensor_element!(i16, TensorDType::I16);
impl_tensor_element!(u8, TensorDType::U8);
impl_tensor_element!(u16, TensorDType::U16);
impl_tensor_element!(bool, TensorDType::Bool);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_shape() {
        let shape = TensorShape::new(vec![2, 3, 4]);
        assert_eq!(shape.ndim(), 3);
        assert_eq!(shape.size(), 24);
        assert_eq!(shape.strides(), vec![12, 4, 1]);
    }

    #[test]
    fn test_tensor_creation() {
        let shape = TensorShape::new(vec![2, 3]);
        let tensor = Tensor::new::<f32>(TensorDType::F32, shape);
        assert_eq!(tensor.size(), 6);
        assert_eq!(tensor.nbytes(), 24);
        assert!(tensor.is_contiguous());
    }

    #[test]
    fn test_tensor_from_slice() {
        let data = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let shape = TensorShape::new(vec![2, 3]);
        let tensor = Tensor::from_slice(&data, shape);
        assert_eq!(tensor.size(), 6);
        assert_eq!(tensor.as_slice::<f32>(), &data[..]);
    }

    #[test]
    fn test_tensor_reshape() {
        let data = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let shape = TensorShape::new(vec![2, 3]);
        let tensor = Tensor::from_slice(&data, shape);

        let new_shape = TensorShape::new(vec![3, 2]);
        let reshaped = tensor.reshape(new_shape);
        assert!(reshaped.is_some());
        assert_eq!(reshaped.unwrap().size(), 6);
    }

    #[test]
    fn test_tensor_slice() {
        let data: Vec<f32> = (0..12).map(|i| i as f32).collect();
        let shape = TensorShape::new(vec![3, 4]);
        let tensor = Tensor::from_slice(&data, shape);

        let sliced = tensor.slice(0, 1, 2);
        assert!(sliced.is_some());
        let sliced = sliced.unwrap();
        assert_eq!(sliced.dims(), &[2, 4]);
        assert_eq!(sliced.size(), 8);
    }
}
