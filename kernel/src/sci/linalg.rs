//! # Linear Algebra Library
//!
//! Comprehensive linear algebra operations:
//! - Matrix operations (addition, subtraction, multiplication, transpose)
//! - Matrix decompositions (LU, QR, SVD)
//! - Eigenvalue and eigenvector computation
//! - Linear system solving
//! - Sparse matrix support

use crate::sci::{SciError, SciFloat, SciResult};
use alloc::vec::Vec;
use core::ops::Mul;

/// Dense matrix stored in column-major order
#[derive(Debug, Clone)]
pub struct Matrix {
    /// Number of rows
    pub rows: usize,
    /// Number of columns
    pub cols: usize,
    /// Data in column-major order
    data: Vec<SciFloat>,
}

impl Matrix {
    /// Create a new matrix with the given dimensions
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }

    /// Create a matrix from a 2D vector
    pub fn from_vec(data: &[Vec<SciFloat>]) -> Self {
        let rows = data.len();
        let cols = if rows > 0 { data[0].len() } else { 0 };

        let mut matrix = Self::new(rows, cols);
        for (i, row) in data.iter().enumerate() {
            for (j, &val) in row.iter().enumerate() {
                matrix.data[i + j * rows] = val;
            }
        }

        matrix
    }

    /// Create a matrix from a flat vector (column-major)
    pub fn from_flat(rows: usize, cols: usize, data: Vec<SciFloat>) -> Self {
        assert_eq!(data.len(), rows * cols);
        Self { rows, cols, data }
    }

    /// Get element at (row, col)
    pub fn get(&self, row: usize, col: usize) -> SciFloat {
        assert!(row < self.rows && col < self.cols);
        self.data[row + col * self.rows]
    }

    /// Set element at (row, col)
    pub fn set(&mut self, row: usize, col: usize, value: SciFloat) {
        assert!(row < self.rows && col < self.cols);
        self.data[row + col * self.rows] = value;
    }

    /// Get the underlying data (column-major)
    pub fn data(&self) -> &[SciFloat] {
        &self.data
    }

    /// Transpose the matrix
    pub fn transpose(&self) -> Self {
        let mut result = Self::new(self.cols, self.rows);
        for i in 0..self.rows {
            for j in 0..self.cols {
                result.data[j + i * self.cols] = self.data[i + j * self.rows];
            }
        }
        result
    }

    /// Matrix multiplication
    pub fn mul(&self, other: &Matrix) -> SciResult<Matrix> {
        if self.cols != other.rows {
            return Err(SciError::InvalidDimensions);
        }

        let mut result = Matrix::new(self.rows, other.cols);

        for i in 0..self.rows {
            for j in 0..other.cols {
                let mut sum = 0.0;
                for k in 0..self.cols {
                    sum += self.get(i, k) * other.get(k, j);
                }
                result.set(i, j, sum);
            }
        }

        Ok(result)
    }

    /// Element-wise addition
    pub fn add(&self, other: &Matrix) -> SciResult<Matrix> {
        if self.rows != other.rows || self.cols != other.cols {
            return Err(SciError::InvalidDimensions);
        }

        let mut result = Matrix::new(self.rows, self.cols);
        for i in 0..self.data.len() {
            result.data[i] = self.data[i] + other.data[i];
        }

        Ok(result)
    }

    /// Element-wise subtraction
    pub fn sub(&self, other: &Matrix) -> SciResult<Matrix> {
        if self.rows != other.rows || self.cols != other.cols {
            return Err(SciError::InvalidDimensions);
        }

        let mut result = Matrix::new(self.rows, self.cols);
        for i in 0..self.data.len() {
            result.data[i] = self.data[i] - other.data[i];
        }

        Ok(result)
    }

    /// Scalar multiplication
    pub fn scale(&self, scalar: SciFloat) -> Matrix {
        let mut result = self.clone();
        for val in result.data.iter_mut() {
            *val *= scalar;
        }
        result
    }

    /// Create identity matrix
    pub fn identity(n: usize) -> Self {
        let mut m = Self::new(n, n);
        for i in 0..n {
            m.set(i, i, 1.0);
        }
        m
    }

    /// Get diagonal elements
    pub fn diagonal(&self) -> Vec<SciFloat> {
        let n = self.rows.min(self.cols);
        (0..n).map(|i| self.get(i, i)).collect()
    }

    /// Trace of square matrix
    pub fn trace(&self) -> SciResult<SciFloat> {
        if self.rows != self.cols {
            return Err(SciError::InvalidDimensions);
        }
        Ok(self.diagonal().iter().sum())
    }

    /// Frobenius norm
    pub fn norm(&self) -> SciFloat {
        self.data.iter().map(|&x| x * x).sum::<SciFloat>().sqrt()
    }

    /// Check if matrix is symmetric
    pub fn is_symmetric(&self, eps: SciFloat) -> bool {
        if self.rows != self.cols {
            return false;
        }
        for i in 0..self.rows {
            for j in i + 1..self.rows {
                if (self.get(i, j) - self.get(j, i)).abs() > eps {
                    return false;
                }
            }
        }
        true
    }
}

/// LU decomposition with partial pivoting
#[derive(Debug, Clone)]
pub struct LUDecomposition {
    lu: Matrix,
    piv: Vec<usize>,
    piv_sign: i32,
}

impl LUDecomposition {
    /// Compute LU decomposition
    pub fn new(a: &Matrix) -> SciResult<Self> {
        if a.rows != a.cols {
            return Err(SciError::InvalidDimensions);
        }

        let n = a.rows;
        let mut lu = a.clone();
        let mut piv: Vec<usize> = (0..n).collect();
        let mut piv_sign = 1;

        for i in 0..n {
            // Find pivot
            let mut max_row = i;
            let mut max_val = lu.get(i, i).abs();
            for row in (i + 1)..n {
                let val = lu.get(row, i).abs();
                if val > max_val {
                    max_val = val;
                    max_row = row;
                }
            }

            if max_val < 1e-10 {
                return Err(SciError::SingularMatrix);
            }

            // Swap rows
            if max_row != i {
                piv.swap(i, max_row);
                piv_sign *= -1;
                for col in 0..n {
                    let temp = lu.get(i, col);
                    lu.set(i, col, lu.get(max_row, col));
                    lu.set(max_row, col, temp);
                }
            }

            // Compute multipliers and eliminate
            for row in (i + 1)..n {
                let multiplier = lu.get(row, i) / lu.get(i, i);
                lu.set(row, i, multiplier);

                for col in (i + 1)..n {
                    let val = lu.get(row, col) - multiplier * lu.get(i, col);
                    lu.set(row, col, val);
                }
            }
        }

        Ok(Self { lu, piv, piv_sign })
    }

    /// Get lower triangular matrix
    pub fn l(&self) -> Matrix {
        let n = self.lu.rows;
        let mut l = Matrix::new(n, n);
        for i in 0..n {
            l.set(i, i, 1.0);
            for j in 0..i {
                l.set(i, j, self.lu.get(i, j));
            }
        }
        l
    }

    /// Get upper triangular matrix
    pub fn u(&self) -> Matrix {
        let n = self.lu.rows;
        let mut u = Matrix::new(n, n);
        for i in 0..n {
            for j in i..n {
                u.set(i, j, self.lu.get(i, j));
            }
        }
        u
    }

    /// Get pivot vector
    pub fn piv(&self) -> &[usize] {
        &self.piv
    }

    /// Solve linear system Ax = b
    pub fn solve(&self, b: &[SciFloat]) -> SciResult<Vec<SciFloat>> {
        let n = self.lu.rows;
        if b.len() != n {
            return Err(SciError::InvalidDimensions);
        }

        let x = b.to_vec();

        // Permute
        let mut x_perm = vec![0.0; n];
        for i in 0..n {
            x_perm[i] = x[self.piv[i]];
        }

        // Forward substitution
        for i in 0..n {
            for j in 0..i {
                x_perm[i] -= self.lu.get(i, j) * x_perm[j];
            }
        }

        // Backward substitution
        for i in (0..n).rev() {
            for j in (i + 1)..n {
                x_perm[i] -= self.lu.get(i, j) * x_perm[j];
            }
            x_perm[i] /= self.lu.get(i, i);
        }

        Ok(x_perm)
    }

    /// Compute determinant
    pub fn det(&self) -> SciFloat {
        let n = self.lu.rows;
        let mut det = self.piv_sign as SciFloat;
        for i in 0..n {
            det *= self.lu.get(i, i);
        }
        det
    }
}

/// QR decomposition using Householder reflections
#[derive(Debug, Clone)]
pub struct QRDecomposition {
    qr: Matrix,
    r_diag: Vec<SciFloat>,
}

impl QRDecomposition {
    /// Compute QR decomposition
    pub fn new(a: &Matrix) -> Self {
        let m = a.rows;
        let n = a.cols;
        let mut qr = a.clone();
        let mut r_diag = vec![0.0; n.min(m)];

        for k in 0..n.min(m) {
            // Compute 2-norm of k-th column
            let mut norm = 0.0;
            for i in k..m {
                norm += qr.get(i, k) * qr.get(i, k);
            }
            norm = norm.sqrt();

            if norm != 0.0 {
                // Form k-th Householder vector
                if qr.get(k, k) < 0.0 {
                    r_diag[k] = -norm;
                } else {
                    r_diag[k] = norm;
                }

                let alpha = 1.0 / (norm * (norm + qr.get(k, k).abs()));
                qr.set(k, k, qr.get(k, k) + r_diag[k]);

                for j in (k + 1)..n {
                    let mut sum = 0.0;
                    for i in k..m {
                        sum += qr.get(i, k) * qr.get(i, j);
                    }
                    sum *= alpha;

                    for i in k..m {
                        let val = qr.get(i, j) - sum * qr.get(i, k);
                        qr.set(i, j, val);
                    }
                }
            }
        }

        Self { qr, r_diag }
    }

    /// Get Q matrix (orthogonal)
    pub fn q(&self) -> Matrix {
        let m = self.qr.rows;
        let n = self.qr.cols;
        let mut q = Matrix::identity(m);

        for k in (0..n.min(m)).rev() {
            for j in k..m {
                let mut sum = 0.0;
                for i in k..m {
                    sum += self.qr.get(i, k) * q.get(i, j);
                }

                if sum != 0.0 {
                    let alpha = -sum / self.r_diag[k];
                    for i in k..m {
                        let val = q.get(i, j) + alpha * self.qr.get(i, k);
                        q.set(i, j, val);
                    }
                }
            }
        }

        q
    }

    /// Get R matrix (upper triangular)
    pub fn r(&self) -> Matrix {
        let n = self.qr.cols;
        let m = self.qr.rows.min(n);
        let mut r = Matrix::new(m, n);

        for i in 0..m {
            for j in 0..n {
                if i < j {
                    r.set(i, j, self.qr.get(i, j));
                } else if i == j {
                    r.set(i, j, self.r_diag[i]);
                } else {
                    r.set(i, j, 0.0);
                }
            }
        }

        r
    }

    /// Solve least squares problem
    pub fn solve_least_squares(&self, b: &[SciFloat]) -> SciResult<Vec<SciFloat>> {
        let m = self.qr.rows;
        let n = self.qr.cols;

        if b.len() != m {
            return Err(SciError::InvalidDimensions);
        }

        let mut y = b.to_vec();

        // Compute Q'b
        for k in 0..n.min(m) {
            let mut sum = 0.0;
            for i in k..m {
                sum += self.qr.get(i, k) * y[i];
            }
            if sum != 0.0 {
                let alpha = -sum / self.r_diag[k];
                for i in k..m {
                    y[i] += alpha * self.qr.get(i, k);
                }
            }
        }

        // Solve R x = y
        let n_min = n.min(m);
        let mut x = vec![0.0; n];
        for i in (0..n_min).rev() {
            let mut sum = 0.0;
            for j in (i + 1)..n_min {
                sum += self.qr.get(i, j) * x[j];
            }
            x[i] = (y[i] - sum) / self.r_diag[i];
        }

        Ok(x)
    }
}

/// Singular Value Decomposition (SVD)
#[derive(Debug, Clone)]
pub struct SVD {
    u: Option<Matrix>,
    s: Vec<SciFloat>,
    v: Option<Matrix>,
}

impl SVD {
    /// Compute SVD: A = U * S * V^T
    pub fn new(a: &Matrix) -> Self {
        // Simplified SVD using power iteration
        // In production, use Golub-Reinsch or similar
        let m = a.rows;
        let n = a.cols;
        let k = m.min(n);

        let mut s = vec![0.0; k];

        // Compute singular values via eigenvalues of A^T * A
        let ata = a.transpose().mul(a).unwrap();
        let eigen = Self::symmetric_eigenpower(&ata, k);

        for i in 0..k {
            s[i] = eigen[i].sqrt();
        }

        Self {
            u: None,
            s,
            v: None,
        }
    }

    /// Power iteration for eigenvalues of symmetric matrix
    fn symmetric_eigenpower(a: &Matrix, num: usize) -> Vec<SciFloat> {
        let n = a.rows;
        let mut eigenvalues = vec![0.0; num];
        let mut a_work = a.clone();

        for i in 0..num {
            // Power iteration
            let mut v = vec![1.0 / (n as SciFloat).sqrt(); n];
            let mut lambda = 0.0;

            for _ in 0..100 {
                let mut v_new = vec![0.0; n];
                for row in 0..n {
                    for col in 0..n {
                        v_new[row] += a_work.get(row, col) * v[col];
                    }
                }

                let norm: SciFloat = v_new.iter().map(|&x| x * x).sum::<SciFloat>().sqrt();
                v = v_new.iter().map(|&x| x / norm).collect();

                // Rayleigh quotient
                let mut lambda_new = 0.0;
                for row in 0..n {
                    for col in 0..n {
                        lambda_new += v[row] * a_work.get(row, col) * v[col];
                    }
                }

                if (lambda_new - lambda).abs() < 1e-10 {
                    lambda = lambda_new;
                    break;
                }
                lambda = lambda_new;
            }

            eigenvalues[i] = lambda;

            // Deflate
            for row in 0..n {
                for col in 0..n {
                    a_work.set(row, col, a_work.get(row, col) - lambda * v[row] * v[col]);
                }
            }
        }

        eigenvalues
    }

    /// Get singular values
    pub fn singular_values(&self) -> &[SciFloat] {
        &self.s
    }

    /// Get condition number (max singular value / min singular value)
    pub fn condition_number(&self) -> SciFloat {
        if self.s.is_empty() {
            return 0.0;
        }
        let max = self.s[0];
        let min = self.s[self.s.len() - 1];
        if min.abs() < 1e-10 {
            return SciFloat::INFINITY;
        }
        max / min
    }

    /// Get rank (number of non-zero singular values)
    pub fn rank(&self, tol: SciFloat) -> usize {
        let max_s = self.s.iter().cloned().fold(0.0, SciFloat::max);
        let threshold = max_s * tol;
        self.s.iter().filter(|&&s| s > threshold).count()
    }
}

/// Eigenvalue decomposition for symmetric matrices
#[derive(Debug, Clone)]
pub struct EigenDecomposition {
    eigenvalues: Vec<SciFloat>,
    eigenvectors: Option<Matrix>,
}

impl EigenDecomposition {
    /// Compute eigenvalues of symmetric matrix using QR algorithm
    pub fn symmetric(a: &Matrix) -> SciResult<Self> {
        if !a.is_symmetric(1e-10) {
            return Err(SciError::InvalidParameters);
        }

        let n = a.rows;
        let mut q = a.clone();
        let mut eigenvalues = vec![0.0; n];

        // QR algorithm
        for _iter in 0..1000 {
            let qr = QRDecomposition::new(&q);
            let r = qr.r();
            let q_new = r.mul(&qr.q()).unwrap();
            q = q_new;

            // Check convergence
            let mut converged = true;
            for i in 0..n {
                for j in 0..i {
                    if q.get(i, j).abs() > 1e-10 {
                        converged = false;
                        break;
                    }
                }
            }

            if converged {
                break;
            }
        }

        for i in 0..n {
            eigenvalues[i] = q.get(i, i);
        }

        Ok(Self {
            eigenvalues,
            eigenvectors: None,
        })
    }

    /// Get eigenvalues
    pub fn eigenvalues(&self) -> &[SciFloat] {
        &self.eigenvalues
    }

    /// Get eigenvectors
    pub fn eigenvectors(&self) -> Option<&Matrix> {
        self.eigenvectors.as_ref()
    }
}

/// Sparse matrix in CSR format
#[derive(Debug, Clone)]
pub struct SparseMatrix {
    /// Number of rows
    pub rows: usize,
    /// Number of columns
    pub cols: usize,
    /// Column indices
    pub col_indices: Vec<usize>,
    /// Row pointers
    pub row_ptr: Vec<usize>,
    /// Non-zero values
    pub values: Vec<SciFloat>,
}

impl SparseMatrix {
    /// Create a new sparse matrix
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            col_indices: Vec::new(),
            row_ptr: vec![0; rows + 1],
            values: Vec::new(),
        }
    }

    /// Create from COO format (list of (row, col, value) tuples)
    pub fn from_coo(
        rows: usize,
        cols: usize,
        entries: &[(usize, usize, SciFloat)],
    ) -> SciResult<Self> {
        let mut mat = Self::new(rows, cols);

        // Sort entries by row then column
        let mut sorted = entries.to_vec();
        sorted.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

        let mut current_row = 0;
        for &(row, col, val) in &sorted {
            if row >= rows || col >= cols {
                return Err(SciError::IndexOutOfBounds);
            }

            while current_row < row {
                current_row += 1;
                mat.row_ptr[current_row] = mat.values.len();
            }

            if val.abs() > 1e-10 {
                mat.col_indices.push(col);
                mat.values.push(val);
            }
        }

        for row in (current_row + 1)..=rows {
            mat.row_ptr[row] = mat.values.len();
        }

        Ok(mat)
    }

    /// Sparse matrix-vector multiplication
    pub fn mul_vec(&self, x: &[SciFloat]) -> SciResult<Vec<SciFloat>> {
        if x.len() != self.cols {
            return Err(SciError::InvalidDimensions);
        }

        let mut y = vec![0.0; self.rows];
        for i in 0..self.rows {
            for j in self.row_ptr[i]..self.row_ptr[i + 1] {
                y[i] += self.values[j] * x[self.col_indices[j]];
            }
        }

        Ok(y)
    }

    /// Get number of non-zero elements
    pub fn nnz(&self) -> usize {
        self.values.len()
    }

    /// Sparsity ratio
    pub fn sparsity(&self) -> SciFloat {
        1.0 - (self.nnz() as SciFloat) / (self.rows * self.cols) as SciFloat
    }
}

/// Solve linear system using conjugate gradient (for symmetric positive-definite)
pub fn conjugate_gradient_solve(
    a: &SparseMatrix,
    b: &[SciFloat],
    tol: SciFloat,
    max_iter: usize,
) -> SciResult<Vec<SciFloat>> {
    let n = a.rows;
    if b.len() != n {
        return Err(SciError::InvalidDimensions);
    }

    let mut x = vec![0.0; n];
    let mut r = b.to_vec();
    let mut p = r.clone();
    let mut rs_old: SciFloat = r.iter().map(|&ri| ri * ri).sum();

    for _iter in 0..max_iter {
        let ap = a.mul_vec(&p)?;
        let dot_product: SciFloat = p.iter().zip(ap.iter()).map(|(&pi, &api)| pi * api).sum();
        let alpha: SciFloat = rs_old / dot_product;

        for i in 0..n {
            x[i] += alpha * p[i];
            r[i] -= alpha * ap[i];
        }

        let rs_new: SciFloat = r.iter().map(|&ri| ri * ri).sum();

        if rs_new.sqrt() < tol {
            return Ok(x);
        }

        let beta = rs_new / rs_old;
        for i in 0..n {
            p[i] = r[i] + beta * p[i];
        }

        rs_old = rs_new;
    }

    Ok(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_creation() {
        let m = Matrix::new(3, 2);
        assert_eq!(m.rows, 3);
        assert_eq!(m.cols, 2);
    }

    #[test]
    fn test_matrix_from_vec() {
        let data = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];
        let m = Matrix::from_vec(&data);
        assert_eq!(m.get(0, 0), 1.0);
        assert_eq!(m.get(1, 1), 4.0);
        assert_eq!(m.get(2, 0), 5.0);
    }

    #[test]
    fn test_matrix_transpose() {
        let data = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        let m = Matrix::from_vec(&data);
        let mt = m.transpose();
        assert_eq!(mt.rows, 3);
        assert_eq!(mt.cols, 2);
        assert_eq!(mt.get(0, 0), 1.0);
        assert_eq!(mt.get(2, 1), 6.0);
    }

    #[test]
    fn test_matrix_mul() {
        let a = Matrix::from_vec(&vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let b = Matrix::from_vec(&vec![vec![5.0, 6.0], vec![7.0, 8.0]]);
        let c = a.mul(&b).unwrap();
        assert_eq!(c.get(0, 0), 19.0);
        assert_eq!(c.get(0, 1), 22.0);
        assert_eq!(c.get(1, 0), 43.0);
        assert_eq!(c.get(1, 1), 50.0);
    }

    #[test]
    fn test_matrix_identity() {
        let i = Matrix::identity(3);
        assert_eq!(i.get(0, 0), 1.0);
        assert_eq!(i.get(1, 1), 1.0);
        assert_eq!(i.get(2, 2), 1.0);
        assert_eq!(i.get(0, 1), 0.0);
    }

    #[test]
    fn test_matrix_trace() {
        let m = Matrix::from_vec(&vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        assert_eq!(m.trace().unwrap(), 5.0);
    }

    #[test]
    fn test_lu_decomposition() {
        let a = Matrix::from_vec(&vec![vec![2.0, 1.0], vec![1.0, 2.0]]);
        let lu = LUDecomposition::new(&a).unwrap();

        let l = lu.l();
        let u = lu.u();

        assert_eq!(l.get(0, 0), 1.0);
        assert_eq!(u.get(0, 0), 2.0);

        // Check determinant
        assert!((lu.det() - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_lu_solve() {
        let a = Matrix::from_vec(&vec![vec![2.0, 1.0], vec![1.0, 2.0]]);
        let b = vec![3.0, 3.0];
        let lu = LUDecomposition::new(&a).unwrap();
        let x = lu.solve(&b).unwrap();

        assert!((x[0] - 1.0).abs() < 1e-10);
        assert!((x[1] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_qr_decomposition() {
        let a = Matrix::from_vec(&vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]]);
        let qr = QRDecomposition::new(&a);

        let q = qr.q();
        let r = qr.r();

        // Check that Q is orthogonal (Q^T * Q = I)
        let qt = q.transpose();
        let qt_q = qt.mul(&q).unwrap();
        for i in 0..2 {
            for j in 0..2 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((qt_q.get(i, j) - expected).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_svd() {
        let a = Matrix::from_vec(&vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]]);
        let svd = SVD::new(&a);

        let s = svd.singular_values();
        assert_eq!(s.len(), 2);
        assert!(s[0] >= s[1]); // Should be sorted descending

        let rank = svd.rank(1e-10);
        assert_eq!(rank, 2);
    }

    #[test]
    fn test_eigen_decomposition() {
        let a = Matrix::from_vec(&vec![vec![4.0, 1.0], vec![1.0, 3.0]]);
        let eigen = EigenDecomposition::symmetric(&a).unwrap();

        let vals = eigen.eigenvalues();
        assert_eq!(vals.len(), 2);

        // Trace should equal sum of eigenvalues
        let trace = a.trace().unwrap();
        let sum: SciFloat = vals.iter().sum();
        assert!((trace - sum).abs() < 1e-6);
    }

    #[test]
    fn test_sparse_matrix() {
        let entries = vec![(0, 0, 1.0), (0, 2, 2.0), (1, 1, 3.0), (2, 0, 4.0)];
        let mat = SparseMatrix::from_coo(3, 3, &entries).unwrap();

        assert_eq!(mat.nnz(), 4);

        let x = vec![1.0, 2.0, 3.0];
        let y = mat.mul_vec(&x).unwrap();

        assert_eq!(y[0], 7.0); // 1*1 + 2*3
        assert_eq!(y[1], 6.0); // 3*2
        assert_eq!(y[2], 4.0); // 4*1
    }

    #[test]
    fn test_conjugate_gradient() {
        // Simple SPD matrix
        let entries = vec![
            (0, 0, 4.0),
            (0, 1, 1.0),
            (1, 0, 1.0),
            (1, 1, 3.0),
            (1, 2, 1.0),
            (2, 1, 1.0),
            (2, 2, 5.0),
        ];
        let a = SparseMatrix::from_coo(3, 3, &entries).unwrap();
        let b = vec![1.0, 2.0, 3.0];

        let x = conjugate_gradient_solve(&a, &b, 1e-10, 1000).unwrap();

        // Verify solution
        let ax = a.mul_vec(&x).unwrap();
        for i in 0..3 {
            assert!((ax[i] - b[i]).abs() < 1e-6);
        }
    }
}
