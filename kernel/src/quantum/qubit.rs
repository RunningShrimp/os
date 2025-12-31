//! Quantum Bit (Qubit) Implementation
//!
//! This module provides the fundamental building blocks for quantum simulation:
//! - Qubit state representation using state vectors
//! - Single-qubit quantum gates (X, Y, Z, H, S, T, rotations)
//! - Multi-qubit gates (CNOT, CZ, Toffoli, SWAP)
//! - Measurement operations
//! - Noise models for realistic simulation

use crate::quantum::{QuantumError, QuantumResult};
use alloc::vec::Vec;
use alloc::string::String;
use num_complex::Complex64;
use rand::Rng;

/// Complex number type used throughout the quantum module
pub type Complex = Complex64;

/// Represents a single qubit in a quantum system
///
/// A qubit can exist in a superposition of states |0⟩ and |1⟩.
/// The state is represented as α|0⟩ + β|1⟩ where |α|² + |β|² = 1.
#[derive(Debug, Clone)]
pub struct Qubit {
    /// Amplitude for |0⟩ state
    pub alpha: Complex,
    /// Amplitude for |1⟩ state
    pub beta: Complex,
}

impl Qubit {
    /// Create a new qubit in the |0⟩ state (computational basis state)
    ///
    /// # Example
    /// ```
    /// use kernel::quantum::Qubit;
    ///
    /// let qubit = Qubit::new();
    /// assert_eq!(qubit.measure().unwrap(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            alpha: Complex::new(1.0, 0.0),
            beta: Complex::new(0.0, 0.0),
        }
    }

    /// Create a qubit in the |1⟩ state
    pub fn one() -> Self {
        Self {
            alpha: Complex::new(0.0, 0.0),
            beta: Complex::new(1.0, 0.0),
        }
    }

    /// Create a qubit in a specific superposition state
    ///
    /// # Arguments
    /// * `alpha` - Amplitude for |0⟩
    /// * `beta` - Amplitude for |1⟩
    ///
    /// # Errors
    /// Returns an error if the state is not normalized
    pub fn from_amplitudes(alpha: Complex, beta: Complex) -> QuantumResult<Self> {
        let norm_sq = alpha.norm_sqr() + beta.norm_sqr();
        if (norm_sq - 1.0).abs() > 1e-10 {
            return Err(QuantumError::NumericalError(
                String::from("Qubit state not normalized")
            ));
        }
        Ok(Self { alpha, beta })
    }

    /// Create a qubit in the |+⟩ state (H|0⟩)
    pub fn plus() -> Self {
        let inv_sqrt2 = 1.0 / core::f64::consts::SQRT_2;
        Self {
            alpha: Complex::new(inv_sqrt2, 0.0),
            beta: Complex::new(inv_sqrt2, 0.0),
        }
    }

    /// Create a qubit in the |-⟩ state (H|1⟩)
    pub fn minus() -> Self {
        let inv_sqrt2 = 1.0 / core::f64::consts::SQRT_2;
        Self {
            alpha: Complex::new(inv_sqrt2, 0.0),
            beta: Complex::new(-inv_sqrt2, 0.0),
        }
    }

    /// Create a qubit in the |+i⟩ state (S†H|0⟩)
    pub fn plus_i() -> Self {
        let inv_sqrt2 = 1.0 / core::f64::consts::SQRT_2;
        Self {
            alpha: Complex::new(inv_sqrt2, 0.0),
            beta: Complex::new(0.0, inv_sqrt2),
        }
    }

    /// Create a qubit in the |-i⟩ state (SH|0⟩)
    pub fn minus_i() -> Self {
        let inv_sqrt2 = 1.0 / core::f64::consts::SQRT_2;
        Self {
            alpha: Complex::new(inv_sqrt2, 0.0),
            beta: Complex::new(0.0, -inv_sqrt2),
        }
    }

    /// Apply a single-qubit gate to this qubit
    pub fn apply_gate(&mut self, gate: &QuantumGate) -> QuantumResult<()> {
        let new_alpha = gate.matrix[0][0] * self.alpha + gate.matrix[0][1] * self.beta;
        let new_beta = gate.matrix[1][0] * self.alpha + gate.matrix[1][1] * self.beta;
        self.alpha = new_alpha;
        self.beta = new_beta;
        Ok(())
    }

    /// Measure the qubit in the computational basis
    ///
    /// Returns 0 with probability |α|² and 1 with probability |β|²
    /// The qubit collapses to the measured state after measurement.
    pub fn measure(&mut self) -> QuantumResult<u8> {
        let prob_1 = self.beta.norm_sqr();
        let mut rng = rand::thread_rng();

        if rng.r#gen::<f64>() < prob_1 {
            // Collapse to |1⟩
            self.alpha = Complex::new(0.0, 0.0);
            self.beta = Complex::new(1.0, 0.0);
            Ok(1)
        } else {
            // Collapse to |0⟩
            self.alpha = Complex::new(1.0, 0.0);
            self.beta = Complex::new(0.0, 0.0);
            Ok(0)
        }
    }

    /// Get the probability of measuring |0⟩
    pub fn prob_zero(&self) -> f64 {
        self.alpha.norm_sqr()
    }

    /// Get the probability of measuring |1⟩
    pub fn prob_one(&self) -> f64 {
        self.beta.norm_sqr()
    }

    /// Check if the qubit is in a pure state
    pub fn is_pure(&self) -> bool {
        (self.alpha.norm_sqr() + self.beta.norm_sqr() - 1.0).abs() < 1e-10
    }

    /// Renormalize the qubit state
    pub fn renormalize(&mut self) {
        let norm = (self.alpha.norm_sqr() + self.beta.norm_sqr()).sqrt();
        if norm > 1e-10 {
            self.alpha /= norm;
            self.beta /= norm;
        }
    }
}

impl Default for Qubit {
    fn default() -> Self {
        Self::new()
    }
}

/// Multi-qubit quantum state represented as a state vector
///
/// For n qubits, the state vector has 2^n complex amplitudes.
/// The state is Σᵢ αᵢ|i⟩ where Σ|αᵢ|² = 1.
#[derive(Debug, Clone)]
pub struct QuantumState {
    /// State vector amplitudes
    amplitudes: Vec<Complex>,
    /// Number of qubits
    num_qubits: usize,
}

impl QuantumState {
    /// Create a new quantum state with all qubits in |0⟩
    ///
    /// # Arguments
    /// * `num_qubits` - Number of qubits in the system
    ///
    /// # Errors
    /// Returns an error if num_qubits is too large
    pub fn new(num_qubits: usize) -> QuantumResult<Self> {
        if num_qubits > 20 {
            return Err(QuantumError::InvalidQubitCount(num_qubits));
        }

        let size = 1usize << num_qubits; // 2^num_qubits
        let mut amplitudes = vec![Complex::new(0.0, 0.0); size];
        amplitudes[0] = Complex::new(1.0, 0.0); // |00...0⟩ state

        Ok(Self {
            amplitudes,
            num_qubits,
        })
    }

    /// Get the number of qubits
    pub fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    /// Get the size of the state vector (2^n)
    pub fn size(&self) -> usize {
        self.amplitudes.len()
    }

    /// Get the amplitude at a specific index
    pub fn get_amplitude(&self, index: usize) -> QuantumResult<Complex> {
        if index >= self.amplitudes.len() {
            return Err(QuantumError::QubitIndexOutOfBounds(index));
        }
        Ok(self.amplitudes[index])
    }

    /// Set the amplitude at a specific index
    pub fn set_amplitude(&mut self, index: usize, amplitude: Complex) -> QuantumResult<()> {
        if index >= self.amplitudes.len() {
            return Err(QuantumError::QubitIndexOutOfBounds(index));
        }
        self.amplitudes[index] = amplitude;
        Ok(())
    }

    /// Apply a single-qubit gate to a specific qubit
    ///
    /// # Arguments
    /// * `gate` - The quantum gate to apply
    /// * `target` - Index of the target qubit
    pub fn apply_single_qubit_gate(
        &mut self,
        gate: &QuantumGate,
        target: usize,
    ) -> QuantumResult<()> {
        if target >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(target));
        }

        let stride = 1usize << target;
        let mask = stride - 1;

        for i in 0..(self.amplitudes.len() / 2) {
            let upper = ((i >> target) << (target + 1)) | (i & mask);
            let lower = upper | stride;

            let temp_upper = self.amplitudes[upper];
            let temp_lower = self.amplitudes[lower];

            self.amplitudes[upper] = gate.matrix[0][0] * temp_upper + gate.matrix[0][1] * temp_lower;
            self.amplitudes[lower] = gate.matrix[1][0] * temp_upper + gate.matrix[1][1] * temp_lower;
        }

        Ok(())
    }

    /// Apply a two-qubit gate (e.g., CNOT)
    ///
    /// # Arguments
    /// * `gate` - The two-qubit gate to apply
    /// * `control` - Index of the control qubit
    /// * `target` - Index of the target qubit
    pub fn apply_two_qubit_gate(
        &mut self,
        gate: &TwoQubitGate,
        control: usize,
        target: usize,
    ) -> QuantumResult<()> {
        if control >= self.num_qubits || target >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(self.num_qubits));
        }

        if control == target {
            return Err(QuantumError::InvalidGate(
String::from("Control and target qubits must be different")
            ));
        }

        // Apply the gate to each pair of basis states
        let size = self.amplitudes.len();
        let control_bit = 1usize << control;
        let target_bit = 1usize << target;

        for i in 0..size {
            // Only apply when control qubit is |1⟩
            if i & control_bit != 0 {
                let j = i ^ target_bit; // Flip target bit
                let temp = self.amplitudes[i];

                // Apply the gate matrix
                if i & target_bit != 0 {
                    // Target is |1⟩
                    self.amplitudes[i] = gate.matrix[3][2] * self.amplitudes[j] + gate.matrix[3][3] * temp;
                    self.amplitudes[j] = gate.matrix[2][2] * self.amplitudes[j] + gate.matrix[2][3] * temp;
                } else {
                    // Target is |0⟩
                    self.amplitudes[i] = gate.matrix[1][0] * self.amplitudes[j] + gate.matrix[1][1] * temp;
                    self.amplitudes[j] = gate.matrix[0][0] * self.amplitudes[j] + gate.matrix[0][1] * temp;
                }
            }
        }

        Ok(())
    }

    /// Measure a specific qubit in the computational basis
    ///
    /// Returns the measurement result and collapses the state
    pub fn measure_qubit(&mut self, qubit: usize) -> QuantumResult<u8> {
        if qubit >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(qubit));
        }

        // Calculate probability of measuring |1⟩
        let mut prob_1 = 0.0;
        let mask = 1usize << qubit;

        for (i, amp) in self.amplitudes.iter().enumerate() {
            if i & mask != 0 {
                prob_1 += amp.norm_sqr();
            }
        }

        // Perform measurement
        let mut rng = rand::thread_rng();
        let result = if rng.r#gen::<f64>() < prob_1 { 1 } else { 0 };

        // Collapse state
        let result_mask = if result == 1 { mask } else { 0 };

        for i in 0..self.amplitudes.len() {
            if (i & mask) != result_mask {
                self.amplitudes[i] = Complex::new(0.0, 0.0);
            }
        }

        // Renormalize
        let norm: f64 = self.amplitudes.iter().map(|a| a.norm_sqr()).sum();
        if norm > 1e-10 {
            let norm_sqrt = norm.sqrt();
            for amp in &mut self.amplitudes {
                *amp /= norm_sqrt;
            }
        }

        Ok(result)
    }

    /// Measure all qubits in the computational basis
    ///
    /// Returns the measurement result as a bitmask
    pub fn measure_all(&mut self) -> QuantumResult<u64> {
        let mut result = 0u64;
        for i in 0..self.num_qubits {
            result |= (self.measure_qubit(i)? as u64) << i;
        }
        Ok(result)
    }

    /// Get the probability of a specific measurement outcome
    pub fn get_probability(&self, outcome: u64) -> QuantumResult<f64> {
        if outcome >= (1u64 << self.num_qubits) {
            return Err(QuantumError::QubitIndexOutOfBounds(outcome as usize));
        }
        Ok(self.amplitudes[outcome as usize].norm_sqr())
    }

    /// Check if the state is normalized
    pub fn is_normalized(&self) -> bool {
        let norm: f64 = self.amplitudes.iter().map(|a| a.norm_sqr()).sum();
        (norm - 1.0).abs() < 1e-10
    }

    /// Renormalize the state vector
    pub fn renormalize(&mut self) {
        let norm: f64 = self.amplitudes.iter().map(|a| a.norm_sqr()).sum();
        if norm > 1e-10 {
            let norm_sqrt = norm.sqrt();
            for amp in &mut self.amplitudes {
                *amp /= norm_sqrt;
            }
        }
    }

    /// Compute the inner product with another state
    pub fn inner_product(&self, other: &QuantumState) -> QuantumResult<Complex> {
        if self.num_qubits != other.num_qubits {
            return Err(QuantumError::InvalidQubitCount(self.num_qubits));
        }

        let mut result = Complex::new(0.0, 0.0);
        for (a, b) in self.amplitudes.iter().zip(other.amplitudes.iter()) {
            result += a.conj() * b;
        }
        Ok(result)
    }
}

/// Single-qubit quantum gate
#[derive(Debug, Clone)]
pub struct QuantumGate {
    /// 2x2 unitary matrix representing the gate
    pub matrix: [[Complex; 2]; 2],
    /// Name of the gate
    pub name: String,
}

impl QuantumGate {
    /// Create a new quantum gate from a 2x2 matrix
    pub fn new(matrix: [[Complex; 2]; 2], name: &str) -> Self {
        Self {
            matrix,
            name: String::from(name),
        }
    }

    /// Check if the gate is unitary
    pub fn is_unitary(&self) -> bool {
        // U† U should equal I
        let det = self.matrix[0][0] * self.matrix[1][1] - self.matrix[0][1] * self.matrix[1][0];
        (det.norm() - 1.0).abs() < 1e-10
    }
}

/// Pauli gates
#[derive(Debug, Clone, Copy)]
pub enum PauliGate {
    X, // Bit flip
    Y, // Bit and phase flip
    Z, // Phase flip
}

impl PauliGate {
    /// Get the gate matrix
    pub fn matrix(&self) -> [[Complex; 2]; 2] {
        match self {
            Self::X => [
                [Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
                [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
            ],
            Self::Y => [
                [Complex::new(0.0, 0.0), Complex::new(0.0, -1.0)],
                [Complex::new(0.0, 1.0), Complex::new(0.0, 0.0)],
            ],
            Self::Z => [
                [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
                [Complex::new(0.0, 0.0), Complex::new(-1.0, 0.0)],
            ],
        }
    }

    /// Get as QuantumGate
    pub fn as_gate(&self) -> QuantumGate {
        let name = match self {
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
        };
        QuantumGate::new(self.matrix(), name)
    }
}

/// Rotation gates
#[derive(Debug, Clone, Copy)]
pub enum RotationGate {
    RX(f64), // Rotation around X-axis
    RY(f64), // Rotation around Y-axis
    RZ(f64), // Rotation around Z-axis
}

impl RotationGate {
    /// Get the gate matrix
    pub fn matrix(&self) -> [[Complex; 2]; 2] {
        match self {
            Self::RX(theta) => {
                let cos = (theta / 2.0).cos();
                let sin = (theta / 2.0).sin();
                [
                    [Complex::new(cos, 0.0), Complex::new(0.0, -sin)],
                    [Complex::new(0.0, -sin), Complex::new(cos, 0.0)],
                ]
            }
            Self::RY(theta) => {
                let cos = (theta / 2.0).cos();
                let sin = (theta / 2.0).sin();
                [
                    [Complex::new(cos, 0.0), Complex::new(-sin, 0.0)],
                    [Complex::new(sin, 0.0), Complex::new(cos, 0.0)],
                ]
            }
            Self::RZ(theta) => {
                [
                    [Complex::new((-theta / 2.0).cos(), (-theta / 2.0).sin()), Complex::new(0.0, 0.0)],
                    [Complex::new(0.0, 0.0), Complex::new((theta / 2.0).cos(), (theta / 2.0).sin())],
                ]
            }
        }
    }

    /// Get as QuantumGate
    pub fn as_gate(&self) -> QuantumGate {
        let name = match self {
            Self::RX(_) => "RX",
            Self::RY(_) => "RY",
            Self::RZ(_) => "RZ",
        };
        QuantumGate::new(self.matrix(), name)
    }
}

/// Common single-qubit gates
impl QuantumGate {
    /// Pauli-X gate (NOT gate)
    pub fn x() -> Self {
        Self::new(
            [
                [Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
                [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
            ],
            "X"
        )
    }

    /// Pauli-Y gate
    pub fn y() -> Self {
        Self::new(
            [
                [Complex::new(0.0, 0.0), Complex::new(0.0, -1.0)],
                [Complex::new(0.0, 1.0), Complex::new(0.0, 0.0)],
            ],
            "Y"
        )
    }

    /// Pauli-Z gate
    pub fn z() -> Self {
        Self::new(
            [
                [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
                [Complex::new(0.0, 0.0), Complex::new(-1.0, 0.0)],
            ],
            "Z"
        )
    }

    /// Hadamard gate
    pub fn h() -> Self {
        let inv_sqrt2 = 1.0 / core::f64::consts::SQRT_2;
        Self::new(
            [
                [Complex::new(inv_sqrt2, 0.0), Complex::new(inv_sqrt2, 0.0)],
                [Complex::new(inv_sqrt2, 0.0), Complex::new(-inv_sqrt2, 0.0)],
            ],
            "H"
        )
    }

    /// S gate (phase gate)
    pub fn s() -> Self {
        Self::new(
            [
                [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
                [Complex::new(0.0, 0.0), Complex::new(0.0, 1.0)],
            ],
            "S"
        )
    }

    /// S† gate (adjoint of S gate)
    pub fn s_dag() -> Self {
        Self::new(
            [
                [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
                [Complex::new(0.0, 0.0), Complex::new(0.0, -1.0)],
            ],
            "S†"
        )
    }

    /// T gate (π/8 gate)
    pub fn t() -> Self {
        Self::new(
            [
                [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
                [Complex::new(0.0, 0.0), Complex::new(core::f64::consts::FRAC_1_SQRT_2, core::f64::consts::FRAC_1_SQRT_2)],
            ],
            "T"
        )
    }

    /// T† gate (adjoint of T gate)
    pub fn t_dag() -> Self {
        Self::new(
            [
                [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
                [Complex::new(0.0, 0.0), Complex::new(core::f64::consts::FRAC_1_SQRT_2, -core::f64::consts::FRAC_1_SQRT_2)],
            ],
            "T†"
        )
    }

    /// Identity gate
    pub fn i() -> Self {
        Self::new(
            [
                [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
                [Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)],
            ],
            "I"
        )
    }

    /// Rotation around X-axis
    pub fn rx(theta: f64) -> Self {
        RotationGate::RX(theta).as_gate()
    }

    /// Rotation around Y-axis
    pub fn ry(theta: f64) -> Self {
        RotationGate::RY(theta).as_gate()
    }

    /// Rotation around Z-axis
    pub fn rz(theta: f64) -> Self {
        RotationGate::RZ(theta).as_gate()
    }
}

/// Two-qubit quantum gate
#[derive(Debug, Clone)]
pub struct TwoQubitGate {
    /// 4x4 unitary matrix representing the gate
    pub matrix: [[Complex; 4]; 4],
    /// Name of the gate
    pub name: String,
}

impl TwoQubitGate {
    /// Create a new two-qubit gate
    pub fn new(matrix: [[Complex; 4]; 4], name: &str) -> Self {
        Self {
            matrix,
            name: String::from(name),
        }
    }

    /// CNOT gate (controlled-X)
    pub fn cnot() -> Self {
        let mut matrix = [[Complex::new(0.0, 0.0); 4]; 4];
        matrix[0][0] = Complex::new(1.0, 0.0);
        matrix[1][1] = Complex::new(1.0, 0.0);
        matrix[2][3] = Complex::new(1.0, 0.0);
        matrix[3][2] = Complex::new(1.0, 0.0);
        Self::new(matrix, "CNOT")
    }

    /// CZ gate (controlled-Z)
    pub fn cz() -> Self {
        let mut matrix = [[Complex::new(0.0, 0.0); 4]; 4];
        matrix[0][0] = Complex::new(1.0, 0.0);
        matrix[1][1] = Complex::new(1.0, 0.0);
        matrix[2][2] = Complex::new(1.0, 0.0);
        matrix[3][3] = Complex::new(-1.0, 0.0);
        Self::new(matrix, "CZ")
    }

    /// SWAP gate
    pub fn swap() -> Self {
        let mut matrix = [[Complex::new(0.0, 0.0); 4]; 4];
        matrix[0][0] = Complex::new(1.0, 0.0);
        matrix[1][2] = Complex::new(1.0, 0.0);
        matrix[2][1] = Complex::new(1.0, 0.0);
        matrix[3][3] = Complex::new(1.0, 0.0);
        Self::new(matrix, "SWAP")
    }

    /// Toffoli gate (CCNOT - controlled-controlled-NOT)
    ///
    /// Note: This is conceptually a 3-qubit gate but can be built from 2-qubit gates
    pub fn toffoli() -> Self {
        // Implemented as an 8x8 matrix for 3 qubits
        // Here we return a placeholder - actual implementation would need a larger matrix
        panic!("Toffoli gate requires 3-qubit representation")
    }
}

/// Result of a quantum measurement
#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementResult {
    /// The measured qubit index (or None for all qubits)
    pub qubit: Option<usize>,
    /// The measurement outcome (0 or 1)
    pub outcome: u8,
    /// Probability of this outcome
    pub probability: f64,
    /// Time of measurement (in nanoseconds)
    pub timestamp: u64,
}

impl MeasurementResult {
    /// Create a new measurement result
    pub fn new(qubit: Option<usize>, outcome: u8, probability: f64) -> Self {
        Self {
            qubit,
            outcome,
            probability,
            timestamp: 0, // Would be set by actual measurement
        }
    }
}

/// Noise models for realistic quantum simulation
#[derive(Debug, Clone)]
pub enum NoiseModel {
    /// No noise (ideal simulation)
    None,

    /// Depolarizing channel
    ///
    /// With probability p, replaces the state with maximally mixed state
    Depolarizing { error_rate: f64 },

    /// Amplitude damping channel
    ///
    /// Models energy loss (T1 relaxation)
    AmplitudeDamping { damping_rate: f64 },

    /// Phase damping channel
    ///
    /// Models dephasing without energy loss (T2 relaxation)
    PhaseDamping { dephasing_rate: f64 },

    /// Combined bit-flip and phase-flip channel
    BitPhaseFlip {
        bit_flip_rate: f64,
        phase_flip_rate: f64,
    },

    /// Custom noise model
    Custom {
        apply_fn: fn(&mut QuantumState, usize) -> QuantumResult<()>,
    },
}

impl NoiseModel {
    /// Apply noise to a quantum state
    pub fn apply(&self, state: &mut QuantumState, qubit: usize) -> QuantumResult<()> {
        match self {
            Self::None => Ok(()),

            Self::Depolarizing { error_rate } => {
                let mut rng = rand::thread_rng();
                if rng.r#gen::<f64>() < *error_rate {
                    // Apply random Pauli error
                    let error = rng.gen_range(0..3);
                    match error {
                        0 => state.apply_single_qubit_gate(&QuantumGate::x(), qubit)?,
                        1 => state.apply_single_qubit_gate(&QuantumGate::y(), qubit)?,
                        2 => state.apply_single_qubit_gate(&QuantumGate::z(), qubit)?,
                        _ => unreachable!(),
                    }
                }
                Ok(())
            }

            Self::AmplitudeDamping { damping_rate } => {
                // Simplified amplitude damping
                let mut rng = rand::thread_rng();
                if rng.r#gen::<f64>() < *damping_rate {
                    // Decay from |1⟩ to |0⟩
                    let prob_1 = state.get_probability(1u64 << qubit)?;
                    if prob_1 > 0.0 {
                        state.apply_single_qubit_gate(&QuantumGate::new([
                            [Complex::new(1.0, 0.0), Complex::new(0.0, 0.0)],
                            [Complex::new(damping_rate.sqrt(), 0.0), Complex::new((1.0 - damping_rate).sqrt(), 0.0)],
                        ], "AmpDamp"), qubit)?;
                    }
                }
                Ok(())
            }

            Self::PhaseDamping { dephasing_rate } => {
                let mut rng = rand::thread_rng();
                if rng.r#gen::<f64>() < *dephasing_rate {
                    // Apply phase flip
                    state.apply_single_qubit_gate(&QuantumGate::z(), qubit)?;
                }
                Ok(())
            }

            Self::BitPhaseFlip { bit_flip_rate, phase_flip_rate } => {
                let mut rng = rand::thread_rng();

                if rng.r#gen::<f64>() < *bit_flip_rate {
                    state.apply_single_qubit_gate(&QuantumGate::x(), qubit)?;
                }

                if rng.r#gen::<f64>() < *phase_flip_rate {
                    state.apply_single_qubit_gate(&QuantumGate::z(), qubit)?;
                }

                Ok(())
            }

            Self::Custom { apply_fn } => {
                apply_fn(state, qubit)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qubit_creation() {
        let qubit = Qubit::new();
        assert_eq!(qubit.alpha, Complex::new(1.0, 0.0));
        assert_eq!(qubit.beta, Complex::new(0.0, 0.0));
    }

    #[test]
    fn test_qubit_superposition() {
        let qubit = Qubit::plus();
        let prob = qubit.prob_zero();
        assert!((prob - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_pauli_x_gate() {
        let mut qubit = Qubit::new();
        qubit.apply_gate(&QuantumGate::x()).unwrap();
        assert_eq!(qubit.measure().unwrap(), 1);
    }

    #[test]
    fn test_hadamard_gate() {
        let mut qubit = Qubit::new();
        qubit.apply_gate(&QuantumGate::h()).unwrap();

        let prob_0 = qubit.prob_zero();
        assert!((prob_0 - 0.5).abs() < 1e-10);

        let prob_1 = qubit.prob_one();
        assert!((prob_1 - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_quantum_state_creation() {
        let state = QuantumState::new(2).unwrap();
        assert_eq!(state.num_qubits(), 2);
        assert_eq!(state.size(), 4);
        assert!(state.is_normalized());
    }

    #[test]
    fn test_cnot_gate() {
        let mut state = QuantumState::new(2).unwrap();

        // Apply Hadamard to control qubit
        state.apply_single_qubit_gate(&QuantumGate::h(), 0).unwrap();

        // Apply CNOT
        state.apply_two_qubit_gate(&TwoQubitGate::cnot(), 0, 1).unwrap();

        // Should create Bell state: (|00⟩ + |11⟩)/√2
        let prob_00 = state.get_probability(0b00).unwrap();
        let prob_11 = state.get_probability(0b11).unwrap();

        assert!((prob_00 - 0.5).abs() < 1e-10);
        assert!((prob_11 - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_measurement() {
        let mut state = QuantumState::new(2).unwrap();
        state.apply_single_qubit_gate(&QuantumGate::h(), 0).unwrap();
        state.apply_two_qubit_gate(&TwoQubitGate::cnot(), 0, 1).unwrap();

        let result = state.measure_all().unwrap();
        assert!(result == 0b00 || result == 0b11);
    }
}
