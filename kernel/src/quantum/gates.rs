//! Quantum Gate Operations
//!
//! This module provides comprehensive quantum gate implementations including:
//! - Single-qubit gates (Pauli X, Y, Z, Hadamard, Phase gates)
//! - Two-qubit gates (CNOT, CZ, SWAP, CX)
//! - Multi-qubit gates (Toffoli, Fredkin)
//! - Parameterized rotation gates (Rx, Ry, Rz)
//! - Gate composition and decomposition
//! - Gate matrix representations
//! - Gate optimization and simplification

extern crate alloc;

use crate::quantum::{QuantumError, QuantumResult};
use alloc::vec::Vec;
use num_complex::Complex64 as Complex;
use alloc::string::String;
use alloc::format;

/// Complex number type for gate matrices
pub type GateMatrix = Vec<Vec<Complex>>;

/// Pauli gates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauliGate {
    /// Pauli-X gate (bit flip)
    X,
    /// Pauli-Y gate
    Y,
    /// Pauli-Z gate (phase flip)
    Z,
    /// Identity gate
    I,
}

/// Phase gates
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PhaseGate {
    /// S gate (phase gate, sqrt(Z))
    S,
    /// S-dagger gate (inverse of S)
    Sdg,
    /// T gate (pi/8 gate)
    T,
    /// T-dagger gate
    Tdg,
    /// Rz(φ) phase gate
    Rz(f64),
    /// Ry rotation
    Ry(f64),
    /// Rx rotation
    Rx(f64),
}

/// Standard single-qubit gates
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StandardGate {
    /// Hadamard gate
    H,
    /// Pauli gate
    Pauli(PauliGate),
    /// Phase gate
    Phase(PhaseGate),
    /// U1 gate (phase)
    U1(f64),
    /// U2 gate
    U2(f64, f64),
    /// U3 gate
    U3(f64, f64, f64),
}

/// Two-qubit controlled gates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoQubitGate {
    /// CNOT gate (controlled-X)
    CNOT,
    /// CZ gate (controlled-Z)
    CZ,
    /// CY gate (controlled-Y)
    CY,
    /// SWAP gate
    SWAP,
    /// sqrt(SWAP) gate
    SqrtSwap,
    /// Controlled-Hadamard
    CH,
}

/// Multi-qubit gates
#[derive(Debug, Clone)]
pub enum MultiQubitGate {
    /// Toffoli gate (CCX - controlled-controlled-NOT)
    Toffoli,
    /// Fredkin gate (CSWAP - controlled-SWAP)
    Fredkin,
    /// General controlled rotation
    ControlledRotation {
        control: Vec<usize>,
        target: usize,
        axis: RotationAxis,
        angle: f64,
    },
}

/// Rotation axis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationAxis {
    X,
    Y,
    Z,
}

/// Generic quantum gate with matrix representation
#[derive(Debug, Clone)]
pub struct QuantumGateMatrix {
    /// Name of the gate
    pub name: String,
    /// Number of qubits the gate acts on
    pub num_qubits: usize,
    /// Unitary matrix representation
    pub matrix: GateMatrix,
    /// Parameters (if any)
    pub params: Vec<f64>,
}

impl QuantumGateMatrix {
    /// Create a new quantum gate from a matrix
    ///
    /// # Arguments
    /// * `name` - Gate name
    /// * `matrix` - Unitary matrix representation
    ///
    /// # Errors
    /// Returns an error if the matrix is not unitary or not square
    pub fn new(name: String, matrix: GateMatrix) -> QuantumResult<Self> {
        let num_qubits = if matrix.is_empty() {
            return Err(QuantumError::InvalidGate(String::from("Empty matrix")));
        } else {
            let dim = matrix.len();
            // Check if matrix is square
            for row in &matrix {
                if row.len() != dim {
                    return Err(QuantumError::InvalidGate(
                        String::from("Matrix is not square")
                    ));
                }
            }
            // Calculate number of qubits: matrix is 2^n x 2^n
            let mut n = 0;
            let mut size = 1;
            while size < dim {
                size *= 2;
                n += 1;
            }
            if size != dim {
                return Err(QuantumError::InvalidGate(
                    String::from("Matrix dimension is not a power of 2")
                ));
            }
            // Check unitary: U†U = I
            if !Self::is_unitary(&matrix) {
                return Err(QuantumError::InvalidGate(
                    String::from("Matrix is not unitary")
                ));
            }
            n
        };

        Ok(Self {
            name,
            num_qubits,
            matrix,
            params: Vec::new(),
        })
    }

    /// Check if a matrix is unitary (U†U = I)
    fn is_unitary(matrix: &GateMatrix) -> bool {
        let n = matrix.len();
        let epsilon = 1e-9;

        // Compute U†U
        for i in 0..n {
            for j in 0..n {
                let mut sum = Complex::new(0.0, 0.0);
                for k in 0..n {
                    sum += matrix[k][i].conj() * matrix[k][j];
                }
                // Check if it's identity
                let expected = if i == j {
                    Complex::new(1.0, 0.0)
                } else {
                    Complex::new(0.0, 0.0)
                };
                if (sum - expected).norm() > epsilon {
                    return false;
                }
            }
        }
        true
    }

    /// Create Pauli-X gate (bit flip)
    ///
    /// Matrix: [[0, 1], [1, 0]]
    pub fn pauli_x() -> Self {
        let matrix = vec![
            vec![Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
            vec![Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
        ];
        Self {
            name: String::from("X"),
            num_qubits: 1,
            matrix,
            params: Vec::new(),
        }
    }

    /// Create Pauli-Y gate
    ///
    /// Matrix: [[0, -i], [i, 0]]
    pub fn pauli_y() -> Self {
        let matrix = vec![
            vec![Complex::new(0.0, 0.0), Complex::new(0.0, -1.0)],
            vec![Complex::new(0.0, 1.0), Complex::new(0.0, 0.0)],
        ];
        Self {
            name: String::from("Y"),
            num_qubits: 1,
            matrix,
            params: Vec::new(),
        }
    }

    /// Create Pauli-Z gate (phase flip)
    ///
    /// Matrix: [[1, 0], [0, -1]]
    pub fn pauli_z() -> Self {
        let matrix = vec![
            vec![Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
            vec![Complex::new(0.0, 0.0), Complex::new(-1.0, 0.0)],
        ];
        Self {
            name: String::from("Z"),
            num_qubits: 1,
            matrix,
            params: Vec::new(),
        }
    }

    /// Create Hadamard gate
    ///
    /// Matrix: (1/√2)[[1, 1], [1, -1]]
    pub fn hadamard() -> Self {
        let inv_sqrt2 = 1.0 / core::f64::consts::SQRT_2;
        let matrix = vec![
            vec![Complex::new(inv_sqrt2, 0.0), Complex::new(inv_sqrt2, 0.0)],
            vec![Complex::new(inv_sqrt2, 0.0), Complex::new(-inv_sqrt2, 0.0)],
        ];
        Self {
            name: String::from("H"),
            num_qubits: 1,
            matrix,
            params: Vec::new(),
        }
    }

    /// Create S gate (phase gate, sqrt(Z))
    ///
    /// Matrix: [[1, 0], [0, i]]
    pub fn s_gate() -> Self {
        let matrix = vec![
            vec![Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
            vec![Complex::new(0.0, 0.0), Complex::new(0.0, 1.0)],
        ];
        Self {
            name: String::from("S"),
            num_qubits: 1,
            matrix,
            params: Vec::new(),
        }
    }

    /// Create S-dagger gate
    ///
    /// Matrix: [[1, 0], [0, -i]]
    pub fn s_dagger() -> Self {
        let matrix = vec![
            vec![Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
            vec![Complex::new(0.0, 0.0), Complex::new(0.0, -1.0)],
        ];
        Self {
            name: String::from("S†"),
            num_qubits: 1,
            matrix,
            params: Vec::new(),
        }
    }

    /// Create T gate (pi/8 gate)
    ///
    /// Matrix: [[1, 0], [0, e^(iπ/4)]]
    pub fn t_gate() -> Self {
        let phase = core::f64::consts::FRAC_PI_4; // π/4
        let matrix = vec![
            vec![Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
            vec![
                Complex::new(0.0, 0.0),
                Complex::new(phase.cos(), phase.sin()),
            ],
        ];
        Self {
            name: String::from("T"),
            num_qubits: 1,
            matrix,
            params: Vec::new(),
        }
    }

    /// Create T-dagger gate
    ///
    /// Matrix: [[1, 0], [0, e^(-iπ/4)]]
    pub fn t_dagger() -> Self {
        let phase = -core::f64::consts::FRAC_PI_4; // -π/4
        let matrix = vec![
            vec![Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
            vec![
                Complex::new(0.0, 0.0),
                Complex::new(phase.cos(), phase.sin()),
            ],
        ];
        Self {
            name: String::from("T†"),
            num_qubits: 1,
            matrix,
            params: Vec::new(),
        }
    }

    /// Create Rx rotation gate
    ///
    /// Matrix: [[cos(θ/2), -i*sin(θ/2)], [-i*sin(θ/2), cos(θ/2)]]
    pub fn rx(theta: f64) -> Self {
        let half = theta / 2.0;
        let cos_h = half.cos();
        let sin_h = half.sin();
        let matrix = vec![
            vec![Complex::new(cos_h, 0.0), Complex::new(0.0, -sin_h)],
            vec![Complex::new(0.0, -sin_h), Complex::new(cos_h, 0.0)],
        ];
        Self {
            name: format!("Rx({:.4})", theta),
            num_qubits: 1,
            matrix,
            params: vec![theta],
        }
    }

    /// Create Ry rotation gate
    ///
    /// Matrix: [[cos(θ/2), -sin(θ/2)], [sin(θ/2), cos(θ/2)]]
    pub fn ry(theta: f64) -> Self {
        let half = theta / 2.0;
        let cos_h = half.cos();
        let sin_h = half.sin();
        let matrix = vec![
            vec![Complex::new(cos_h, 0.0), Complex::new(-sin_h, 0.0)],
            vec![Complex::new(sin_h, 0.0), Complex::new(cos_h, 0.0)],
        ];
        Self {
            name: format!("Ry({:.4})", theta),
            num_qubits: 1,
            matrix,
            params: vec![theta],
        }
    }

    /// Create Rz rotation gate
    ///
    /// Matrix: [[e^(-iθ/2), 0], [0, e^(iθ/2)]]
    pub fn rz(theta: f64) -> Self {
        let half = theta / 2.0;
        let matrix = vec![
            vec![
                Complex::new(half.cos(), -half.sin()),
                Complex::new(0.0, 0.0),
            ],
            vec![
                Complex::new(0.0, 0.0),
                Complex::new(half.cos(), half.sin()),
            ],
        ];
        Self {
            name: format!("Rz({:.4})", theta),
            num_qubits: 1,
            matrix,
            params: vec![theta],
        }
    }

    /// Create identity gate
    pub fn identity() -> Self {
        let matrix = vec![
            vec![Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
            vec![Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
        ];
        Self {
            name: String::from("I"),
            num_qubits: 1,
            matrix,
            params: Vec::new(),
        }
    }

    /// Create CNOT gate (controlled-NOT)
    ///
    /// Matrix: [[1,0,0,0], [0,1,0,0], [0,0,0,1], [0,0,1,0]]
    pub fn cnot() -> Self {
        let matrix = vec![
            vec![Complex::new(1.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0)],
            vec![Complex::new(0.0, 0.0), Complex::new(1.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0)],
            vec![Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
            vec![Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
        ];
        Self {
            name: String::from("CNOT"),
            num_qubits: 2,
            matrix,
            params: Vec::new(),
        }
    }

    /// Create CZ gate (controlled-Z)
    pub fn cz() -> Self {
        let matrix = vec![
            vec![Complex::new(1.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0)],
            vec![Complex::new(0.0, 0.0), Complex::new(1.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0)],
            vec![Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
            vec![Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(-1.0, 0.0)],
        ];
        Self {
            name: String::from("CZ"),
            num_qubits: 2,
            matrix,
            params: Vec::new(),
        }
    }

    /// Create SWAP gate
    pub fn swap() -> Self {
        let matrix = vec![
            vec![Complex::new(1.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0)],
            vec![Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
            vec![Complex::new(0.0, 0.0), Complex::new(1.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0)],
            vec![Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
        ];
        Self {
            name: String::from("SWAP"),
            num_qubits: 2,
            matrix,
            params: Vec::new(),
        }
    }

    /// Create Toffoli gate (CCX - controlled-controlled-NOT)
    pub fn toffoli() -> Self {
        // 8x8 matrix for 3 qubits
        let mut matrix = Vec::with_capacity(8);
        for i in 0..8 {
            let mut row = vec![Complex::new(0.0, 0.0); 8];
            if i < 6 {
                row[i] = Complex::new(1.0, 0.0);
            } else {
                // Swap last two rows
                row[if i == 6 { 7 } else { 6 }] = Complex::new(1.0, 0.0);
            }
            matrix.push(row);
        }
        Self {
            name: String::from("Toffoli"),
            num_qubits: 3,
            matrix,
            params: Vec::new(),
        }
    }

    /// Create Fredkin gate (CSWAP - controlled-SWAP)
    pub fn fredkin() -> Self {
        // 8x8 matrix for 3 qubits
        let mut matrix = Vec::with_capacity(8);
        for i in 0..8 {
            let mut row = vec![Complex::new(0.0, 0.0); 8];
            if i < 4 {
                row[i] = Complex::new(1.0, 0.0);
            } else {
                // Apply SWAP to last 4 rows
                let offset = i - 4;
                match offset {
                    0 => row[4] = Complex::new(1.0, 0.0),
                    1 => row[5] = Complex::new(1.0, 0.0),
                    2 => row[7] = Complex::new(1.0, 0.0),
                    3 => row[6] = Complex::new(1.0, 0.0),
                    _ => {}
                }
            }
            matrix.push(row);
        }
        Self {
            name: String::from("Fredkin"),
            num_qubits: 3,
            matrix,
            params: Vec::new(),
        }
    }

    /// Compute the tensor product (Kronecker product) of two gates
    pub fn tensor_product(&self, other: &Self) -> QuantumResult<Self> {
        let n1 = self.matrix.len();
        let n2 = other.matrix.len();
        let new_dim = n1 * n2;

        let mut new_matrix = Vec::with_capacity(new_dim);
        for i in 0..n1 {
            for k in 0..n2 {
                let mut new_row = Vec::with_capacity(new_dim);
                for j in 0..n1 {
                    for l in 0..n2 {
                        let product = self.matrix[i][j] * other.matrix[k][l];
                        new_row.push(product);
                    }
                }
                new_matrix.push(new_row);
            }
        }

        Ok(Self {
            name: format!("{} ⊗ {}", self.name, other.name),
            num_qubits: self.num_qubits + other.num_qubits,
            matrix: new_matrix,
            params: [self.params.clone(), other.params.clone()].concat(),
        })
    }

    /// Compose two gates (multiply matrices: G2 after G1)
    pub fn compose(&self, other: &Self) -> QuantumResult<Self> {
        if self.num_qubits != other.num_qubits {
            return Err(QuantumError::InvalidGate(
                format!("Cannot compose gates acting on different numbers of qubits: {} vs {}",
                       self.num_qubits, other.num_qubits)
            ));
        }

        let n = self.matrix.len();
        let mut result_matrix = vec![vec![Complex::new(0.0, 0.0); n]; n];

        // Matrix multiplication: other * self
        for i in 0..n {
            for j in 0..n {
                let mut sum = Complex::new(0.0, 0.0);
                for k in 0..n {
                    sum += other.matrix[i][k] * self.matrix[k][j];
                }
                result_matrix[i][j] = sum;
            }
        }

        Ok(Self {
            name: format!("{} · {}", other.name, self.name),
            num_qubits: self.num_qubits,
            matrix: result_matrix,
            params: [other.params.clone(), self.params.clone()].concat(),
        })
    }

    /// Get the adjoint (conjugate transpose) of the gate
    pub fn adjoint(&self) -> Self {
        let n = self.matrix.len();
        let mut adj_matrix = vec![vec![Complex::new(0.0, 0.0); n]; n];

        for i in 0..n {
            for j in 0..n {
                adj_matrix[j][i] = self.matrix[i][j].conj();
            }
        }

        Self {
            name: format!("{}†", self.name),
            num_qubits: self.num_qubits,
            matrix: adj_matrix,
            params: self.params.iter().map(|&p| -p).collect(), // Invert rotation angles
        }
    }

    /// Check if gate is equal to identity (within numerical precision)
    pub fn is_identity(&self) -> bool {
        let epsilon = 1e-9;
        let n = self.matrix.len();

        for i in 0..n {
            for j in 0..n {
                let expected = if i == j {
                    Complex::new(1.0, 0.0)
                } else {
                    Complex::new(0.0, 0.0)
                };
                if (self.matrix[i][j] - expected).norm() > epsilon {
                    return false;
                }
            }
        }
        true
    }

    /// Decompose gate into simpler gates if possible
    pub fn decompose(&self) -> Vec<QuantumGateMatrix> {
        match self.name.as_str() {
            "SWAP" => {
                // SWAP = CNOT(0,1) · CNOT(1,0) · CNOT(0,1)
                vec![
                    QuantumGateMatrix::cnot(),
                    QuantumGateMatrix::cnot(),
                    QuantumGateMatrix::cnot(),
                ]
            }
            "Toffoli" => {
                // Decompose Toffoli into H, T, T†, and CNOT gates
                // This is a simplified decomposition
                vec![
                    QuantumGateMatrix::hadamard(),
                    QuantumGateMatrix::cnot(),
                    QuantumGateMatrix::t_dagger(),
                    QuantumGateMatrix::cnot(),
                    QuantumGateMatrix::t_gate(),
                    QuantumGateMatrix::hadamard(),
                ]
            }
            _ => vec![self.clone()],
        }
    }
}

impl StandardGate {
    /// Convert to QuantumGate
    pub fn to_gate(&self) -> QuantumGateMatrix {
        match self {
            StandardGate::H => QuantumGateMatrix::hadamard(),
            StandardGate::Pauli(p) => match p {
                PauliGate::X => QuantumGateMatrix::pauli_x(),
                PauliGate::Y => QuantumGateMatrix::pauli_y(),
                PauliGate::Z => QuantumGateMatrix::pauli_z(),
                PauliGate::I => QuantumGateMatrix::identity(),
            },
            StandardGate::Phase(p) => match p {
                PhaseGate::S => QuantumGateMatrix::s_gate(),
                PhaseGate::Sdg => QuantumGateMatrix::s_dagger(),
                PhaseGate::T => QuantumGateMatrix::t_gate(),
                PhaseGate::Tdg => QuantumGateMatrix::t_dagger(),
                PhaseGate::Rz(theta) => QuantumGateMatrix::rz(*theta),
                PhaseGate::Ry(theta) => QuantumGateMatrix::ry(*theta),
                PhaseGate::Rx(theta) => QuantumGateMatrix::rx(*theta),
            },
            StandardGate::U1(lambda) => {
                // U1(λ) = [[1, 0], [0, e^(iλ)]]
                let matrix = vec![
                    vec![Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
                    vec![
                        Complex::new(0.0, 0.0),
                        Complex::new(lambda.cos(), lambda.sin()),
                    ],
                ];
                QuantumGateMatrix {
                    name: format!("U1({:.4})", lambda),
                    num_qubits: 1,
                    matrix,
                    params: vec![*lambda],
                }
            }
            StandardGate::U2(phi, lambda) => {
                // U2(φ,λ) = (1/√2)[[1, -e^(iλ)], [e^(iφ), e^(i(φ+λ))]]
                let inv_sqrt2 = 1.0 / core::f64::consts::SQRT_2;
                let matrix = vec![
                    vec![
                        Complex::new(inv_sqrt2, 0.0),
                        Complex::new(-inv_sqrt2 * lambda.cos(), -inv_sqrt2 * lambda.sin()),
                    ],
                    vec![
                        Complex::new(inv_sqrt2 * phi.cos(), inv_sqrt2 * phi.sin()),
                        Complex::new(
                            inv_sqrt2 * (phi + lambda).cos(),
                            inv_sqrt2 * (phi + lambda).sin(),
                        ),
                    ],
                ];
                QuantumGateMatrix {
                    name: format!("U2({:.4},{:.4})", phi, lambda),
                    num_qubits: 1,
                    matrix,
                    params: vec![*phi, *lambda],
                }
            }
            StandardGate::U3(theta, phi, lambda) => {
                // U3(θ,φ,λ) = [[cos(θ/2), -e^(iλ)sin(θ/2)], [e^(iφ)sin(θ/2), e^(i(φ+λ))cos(θ/2)]]
                let half_theta = theta / 2.0;
                let cos_t = half_theta.cos();
                let sin_t = half_theta.sin();
                let matrix = vec![
                    vec![
                        Complex::new(cos_t, 0.0),
                        Complex::new(-sin_t * lambda.cos(), -sin_t * lambda.sin()),
                    ],
                    vec![
                        Complex::new(sin_t * phi.cos(), sin_t * phi.sin()),
                        Complex::new(
                            cos_t * (phi + lambda).cos(),
                            cos_t * (phi + lambda).sin(),
                        ),
                    ],
                ];
                QuantumGateMatrix {
                    name: format!("U3({:.4},{:.4},{:.4})", theta, phi, lambda),
                    num_qubits: 1,
                    matrix,
                    params: vec![*theta, *phi, *lambda],
                }
            }
        }
    }
}

impl TwoQubitGate {
    /// Convert to QuantumGate
    pub fn to_gate(&self) -> QuantumGateMatrix {
        match self {
            TwoQubitGate::CNOT => QuantumGateMatrix::cnot(),
            TwoQubitGate::CZ => QuantumGateMatrix::cz(),
            TwoQubitGate::CY => {
                // CY gate (controlled-Y)
                let matrix = vec![
                    vec![Complex::new(1.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0)],
                    vec![Complex::new(0.0, 0.0), Complex::new(1.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0)],
                    vec![Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, -1.0)],
                    vec![Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 1.0), Complex::new(0.0, 0.0)],
                ];
                QuantumGateMatrix {
                    name: String::from("CY"),
                    num_qubits: 2,
                    matrix,
                    params: Vec::new(),
                }
            }
            TwoQubitGate::SWAP => QuantumGateMatrix::swap(),
            TwoQubitGate::SqrtSwap => {
                // sqrt(SWAP) gate
                let half = 0.5;
                let _i_half = Complex::new(0.0, 0.5);
                let matrix = vec![
                    vec![Complex::new(1.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0)],
                    vec![Complex::new(0.0, 0.0), Complex::new(half, 0.0), Complex::new(half, 0.0), Complex::new(0.0, 0.0)],
                    vec![Complex::new(0.0, 0.0), Complex::new(half, 0.0), Complex::new(half, 0.0), Complex::new(0.0, 0.0)],
                    vec![Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
                ];
                QuantumGateMatrix {
                    name: String::from("√SWAP"),
                    num_qubits: 2,
                    matrix,
                    params: Vec::new(),
                }
            }
            TwoQubitGate::CH => {
                // Controlled-Hadamard
                let inv_sqrt2 = 1.0 / core::f64::consts::SQRT_2;
                let matrix = vec![
                    vec![Complex::new(1.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0)],
                    vec![Complex::new(0.0, 0.0), Complex::new(1.0, 0.0), Complex::new(0.0, 0.0), Complex::new(0.0, 0.0)],
                    vec![Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(inv_sqrt2, 0.0), Complex::new(inv_sqrt2, 0.0)],
                    vec![Complex::new(0.0, 0.0), Complex::new(0.0, 0.0), Complex::new(inv_sqrt2, 0.0), Complex::new(-inv_sqrt2, 0.0)],
                ];
                QuantumGateMatrix {
                    name: String::from("CH"),
                    num_qubits: 2,
                    matrix,
                    params: Vec::new(),
                }
            }
        }
    }
}

/// Gate composer for building complex gates from basic ones
pub struct GateComposer;

impl GateComposer {
    /// Create a controlled version of a single-qubit gate
    pub fn controlled(gate: &QuantumGateMatrix) -> QuantumResult<QuantumGateMatrix> {
        if gate.num_qubits != 1 {
            return Err(QuantumError::InvalidGate(
                String::from("Can only create controlled version of single-qubit gates")
            ));
        }

        // For a controlled gate, matrix is [[I, 0], [0, U]]
        let dim = 2;
        let u = &gate.matrix;
        let new_dim = dim * 2;

        let mut matrix = vec![vec![Complex::new(0.0, 0.0); new_dim]; new_dim];

        // Top-left block: Identity
        for i in 0..dim {
            matrix[i][i] = Complex::new(1.0, 0.0);
        }

        // Bottom-right block: U
        for i in 0..dim {
            for j in 0..dim {
                matrix[i + dim][j + dim] = u[i][j];
            }
        }

        Ok(QuantumGateMatrix {
            name: format!("C-{}", gate.name),
            num_qubits: 2,
            matrix,
            params: gate.params.clone(),
        })
    }

    /// Create a rotation gate around specified axis
    pub fn rotation(axis: RotationAxis, angle: f64) -> QuantumGateMatrix {
        match axis {
            RotationAxis::X => QuantumGateMatrix::rx(angle),
            RotationAxis::Y => QuantumGateMatrix::ry(angle),
            RotationAxis::Z => QuantumGateMatrix::rz(angle),
        }
    }

    /// Create a global phase gate
    pub fn global_phase(phase: f64) -> QuantumGateMatrix {
        let c = Complex::new(phase.cos(), phase.sin());
        let matrix = vec![
            vec![c, Complex::new(0.0, 0.0)],
            vec![Complex::new(0.0, 0.0), c],
        ];
        QuantumGateMatrix {
            name: format!("P({:.4})", phase),
            num_qubits: 1,
            matrix,
            params: vec![phase],
        }
    }
}

/// Gate optimizer for simplifying gate sequences
pub struct GateOptimizer;

impl GateOptimizer {
    /// Cancel adjacent gates that are inverses of each other
    pub fn cancel_inverse_gates(gates: &[QuantumGateMatrix]) -> Vec<QuantumGateMatrix> {
        let mut optimized = Vec::new();

        for gate in gates {
            // Check if last gate is the inverse of current gate
            let should_cancel = optimized.last().map_or(false, |last| {
                Self::are_inverses(last, gate)
            });

            if should_cancel {
                optimized.pop();
            } else {
                optimized.push(gate.clone());
            }
        }

        optimized
    }

    /// Check if two gates are inverses of each other
    fn are_inverses(g1: &QuantumGateMatrix, g2: &QuantumGateMatrix) -> bool {
        // Check by name for common gates
        let inverse_pairs = [
            ("X", "X"),
            ("Y", "Y"),
            ("Z", "Z"),
            ("H", "H"),
            ("S", "S†"),
            ("T", "T†"),
        ];

        for (name1, name2) in &inverse_pairs {
            if g1.name == *name1 && g2.name == *name2 {
                return true;
            }
        }

        // Check if matrices multiply to identity
        if let Ok(composed) = g1.compose(g2) {
            composed.is_identity()
        } else {
            false
        }
    }

    /// Merge consecutive rotations on the same axis
    pub fn merge_rotations(gates: &[QuantumGateMatrix]) -> Vec<QuantumGateMatrix> {
        let mut result = Vec::new();
        let mut pending_rx: Option<f64> = None;
        let mut pending_ry: Option<f64> = None;
        let mut pending_rz: Option<f64> = None;

        for gate in gates {
            if gate.name.starts_with("Rx(") {
                if let Some(&angle) = gate.params.first() {
                    *pending_rx.get_or_insert(0.0) += angle;
                }
            } else if gate.name.starts_with("Ry(") {
                if let Some(&angle) = gate.params.first() {
                    *pending_ry.get_or_insert(0.0) += angle;
                }
            } else if gate.name.starts_with("Rz(") {
                if let Some(&angle) = gate.params.first() {
                    *pending_rz.get_or_insert(0.0) += angle;
                }
            } else {
                // Flush pending rotations
                if let Some(angle) = pending_rx.take() {
                    if angle.abs() > 1e-10 {
                        result.push(QuantumGateMatrix::rx(angle));
                    }
                }
                if let Some(angle) = pending_ry.take() {
                    if angle.abs() > 1e-10 {
                        result.push(QuantumGateMatrix::ry(angle));
                    }
                }
                if let Some(angle) = pending_rz.take() {
                    if angle.abs() > 1e-10 {
                        result.push(QuantumGateMatrix::rz(angle));
                    }
                }
                result.push(gate.clone());
            }
        }

        // Flush remaining rotations
        if let Some(angle) = pending_rx {
            if angle.abs() > 1e-10 {
                result.push(QuantumGateMatrix::rx(angle));
            }
        }
        if let Some(angle) = pending_ry {
            if angle.abs() > 1e-10 {
                result.push(QuantumGateMatrix::ry(angle));
            }
        }
        if let Some(angle) = pending_rz {
            if angle.abs() > 1e-10 {
                result.push(QuantumGateMatrix::rz(angle));
            }
        }

        result
    }

    /// Full optimization pipeline
    pub fn optimize(gates: &[QuantumGateMatrix]) -> Vec<QuantumGateMatrix> {
        let mut result = gates.to_vec();

        // Apply optimizations in sequence
        result = Self::cancel_inverse_gates(&result);
        result = Self::merge_rotations(&result);

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pauli_gates() {
        let x = QuantumGateMatrix::pauli_x();
        let y = QuantumGateMatrix::pauli_y();
        let z = QuantumGateMatrix::pauli_z();

        assert_eq!(x.num_qubits, 1);
        assert_eq!(y.num_qubits, 1);
        assert_eq!(z.num_qubits, 1);

        // Test that X² = I
        let xx = x.compose(&x).unwrap();
        assert!(xx.is_identity());
    }

    #[test]
    fn test_hadamard_gate() {
        let h = QuantumGateMatrix::hadamard();
        assert_eq!(h.num_qubits, 1);

        // Test that H² = I
        let hh = h.compose(&h).unwrap();
        assert!(hh.is_identity());
    }

    #[test]
    fn test_phase_gates() {
        let s = QuantumGateMatrix::s_gate();
        let sdg = QuantumGateMatrix::s_dagger();

        // Test that S · S† = I
        let ssdg = s.compose(&sdg).unwrap();
        assert!(ssdg.is_identity());
    }

    #[test]
    fn test_rotation_gates() {
        let rx = QuantumGateMatrix::rx(core::f64::consts::PI / 2.0);
        let ry = QuantumGateMatrix::ry(core::f64::consts::PI / 4.0);
        let rz = QuantumGateMatrix::rz(core::f64::consts::PI / 8.0);

        assert_eq!(rx.num_qubits, 1);
        assert_eq!(ry.num_qubits, 1);
        assert_eq!(rz.num_qubits, 1);
    }

    #[test]
    fn test_cnot_gate() {
        let cnot = QuantumGateMatrix::cnot();
        assert_eq!(cnot.num_qubits, 2);
    }

    #[test]
    fn test_tensor_product() {
        let x = QuantumGateMatrix::pauli_x();
        let h = QuantumGateMatrix::hadamard();
        let xh = x.tensor_product(&h).unwrap();

        assert_eq!(xh.num_qubits, 2);
    }

    #[test]
    fn test_gate_composition() {
        let x = QuantumGateMatrix::pauli_x();
        let y = QuantumGateMatrix::pauli_y();
        let xy = x.compose(&y).unwrap();

        assert_eq!(xy.num_qubits, 1);
    }

    #[test]
    fn test_gate_adjoint() {
        let x = QuantumGateMatrix::pauli_x();
        let x_adj = x.adjoint();

        // X is self-adjoint
        let xx = x.compose(&x_adj).unwrap();
        assert!(xx.is_identity());
    }

    #[test]
    fn test_gate_optimizer() {
        let gates = vec![
            QuantumGateMatrix::hadamard(),
            QuantumGateMatrix::hadamard(), // Should cancel with previous
            QuantumGateMatrix::pauli_x(),
        ];

        let optimized = GateOptimizer::cancel_inverse_gates(&gates);

        assert_eq!(optimized.len(), 1);
        assert_eq!(optimized[0].name, "X");
    }

    #[test]
    fn test_rotation_merge() {
        let gates = vec![
            QuantumGateMatrix::rx(0.5),
            QuantumGateMatrix::rx(0.3),
            QuantumGateMatrix::pauli_y(),
        ];

        let optimized = GateOptimizer::merge_rotations(&gates);

        assert_eq!(optimized.len(), 2);
        assert!(optimized[0].name.starts_with("Rx("));
        assert!((optimized[0].params[0] - 0.8).abs() < 1e-9);
    }

    #[test]
    fn test_toffoli_gate() {
        let toffoli = QuantumGateMatrix::toffoli();
        assert_eq!(toffoli.num_qubits, 3);
    }

    #[test]
    fn test_fredkin_gate() {
        let fredkin = QuantumGateMatrix::fredkin();
        assert_eq!(fredkin.num_qubits, 3);
    }
}
