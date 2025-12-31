//! Quantum Computing Interface Module
//!
//! This module provides a comprehensive quantum computing framework including:
//! - Quantum bit (qubit) simulation with state vectors
//! - Quantum circuit construction and optimization
//! - Quantum algorithms (Grover, Shor, QFT)
//! - Quantum error correction codes
//! - Quantum key distribution protocols
//! - Post-quantum cryptography implementations
//!
//! ## Architecture
//!
//! The quantum module is organized into several submodules:
//!
//! - **qubit**: Core qubit representation and quantum gates
//! - **circuit**: Quantum circuit construction and simulation
//! - **algorithms**: High-level quantum algorithms
//! - **error_correction**: Quantum error correction codes
//! - **qkd**: Quantum key distribution protocols
//! - **pqc**: Post-quantum cryptographic algorithms
//!
//! ## Usage
//!
//! ```rust,no_run
//! use kernel::quantum::{Qubit, Circuit, GroverAlgorithm};
//!
//! // Create a quantum circuit
//! let mut circuit = Circuit::new(2);
//! circuit.h(0)?;
//! circuit.cnot(0, 1)?;
//! let result = circuit.measure_all()?;
//!
//! // Run Grover's algorithm
//! let grover = GroverAlgorithm::new(4, 0b1010)?;
//! let found = grover.search()?;
//! ```
//!
//! ## Performance Considerations
//!
//! - State vector simulation scales as O(2^n) with n qubits
//! - Circuit optimization reduces gate count
//! - Parallel simulation available for independent operations
//! - Memory-efficient for up to ~20 qubits on typical systems

extern crate alloc;

use alloc::string::String;

pub mod qubit;
pub mod circuit;
pub mod algorithms;
pub mod error_correction;
pub mod qkd;
pub mod pqc;
pub mod gates;
pub mod simulator;
pub mod optimization;

// Re-export commonly used types
pub use qubit::{
    Qubit, QuantumState, QuantumGate, PauliGate, RotationGate,
    MeasurementResult, NoiseModel, TwoQubitGate
};
// Also re-export the matrix-based gate from gates module as a separate name
pub use gates::QuantumGateMatrix;

pub use circuit::{
    Circuit, CircuitOptimizer, CircuitDepth, QuantumOperation
};

pub use algorithms::{
    QuantumAlgorithm, GroverAlgorithm, ShorAlgorithm,
    QuantumFourierTransform, VariationalQuantumAlgorithm
};

pub use error_correction::{
    QuantumCode, SurfaceCode, SteaneCode, ErrorSyndrome, DecodingResult
};

pub use qkd::{
    QKDProtocol, BB84Protocol, E91Protocol, DecoyStateProtocol,
    QuantumKey, KeySifting, ErrorReconciliation
};

pub use pqc::{
    PostQuantumScheme, Kyber, Dilithium, SPHINCSPlus, McEliece,
    PQCKeyPair, PQCSignature, PQCEncryption
};

// Re-export gates module (avoid conflicts with qubit module)
pub use gates::{
    QuantumGateMatrix as Gate, PhaseGate, StandardGate,
    MultiQubitGate, RotationAxis, GateComposer,
    GateOptimizer as GateOpt, GateMatrix
};

// Re-export simulator module
pub use simulator::{
    StateVector, StateVectorSnapshot, QuantumSimulator
};

// Re-export optimization module
pub use optimization::{
    OptimizationLevel, OptimizationConfig, CircuitDepth as OptCircuitDepth,
    OptimizedCircuit, OptimizedOperation, HardwareType,
    CircuitOptimizer as CircuitOpt, CircuitCutter, GateSynthesizer,
    CircuitAnalysis
};

/// Version information for the quantum computing module
pub const QUANTUM_VERSION: &str = "1.0.0";

/// Maximum number of qubits supported in state vector simulation
pub const MAX_QUBITS_STATE_VECTOR: usize = 20;

/// Default precision for quantum state amplitudes
pub const DEFAULT_AMPLITUDE_PRECISION: f64 = 1e-10;

/// Threshold for considering a state amplitude as zero
pub const AMPLITUDE_ZERO_THRESHOLD: f64 = 1e-12;

/// Global quantum computing configuration
#[derive(Debug, Clone)]
pub struct QuantumConfig {
    /// Maximum number of qubits
    pub max_qubits: usize,
    /// Whether to enable circuit optimization
    pub enable_optimization: bool,
    /// Noise model to use
    pub noise_model: Option<NoiseModel>,
    /// Number of parallel workers for simulation
    pub parallel_workers: Option<usize>,
    /// Random seed for measurements
    pub random_seed: Option<u64>,
}

impl Default for QuantumConfig {
    fn default() -> Self {
        Self {
            max_qubits: MAX_QUBITS_STATE_VECTOR,
            enable_optimization: true,
            noise_model: None,
            parallel_workers: None,
            random_seed: None,
        }
    }
}

/// Error types for quantum operations
#[derive(Debug)]
pub enum QuantumError {
    InvalidQubitCount(usize),

    QubitIndexOutOfBounds(usize),

    InvalidGate(String),

    CircuitDepthTooLarge(usize),

    MeasurementFailed(String),

    InsufficientEntanglement,

    ErrorCorrectionFailed(String),

    QKDError(String),

    PQCError(String),

    NumericalError(String),

    InsufficientResources,
}

impl core::fmt::Display for QuantumError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidQubitCount(n) => write!(f, "Invalid number of qubits: {}", n),
            Self::QubitIndexOutOfBounds(i) => write!(f, "Qubit index out of bounds: {}", i),
            Self::InvalidGate(g) => write!(f, "Invalid quantum gate: {}", g),
            Self::CircuitDepthTooLarge(d) => write!(f, "Circuit depth too large: {}", d),
            Self::MeasurementFailed(m) => write!(f, "Quantum measurement failed: {}", m),
            Self::InsufficientEntanglement => write!(f, "Insufficient entanglement for operation"),
            Self::ErrorCorrectionFailed(e) => write!(f, "Error correction failed: {}", e),
            Self::QKDError(e) => write!(f, "QKD protocol error: {}", e),
            Self::PQCError(e) => write!(f, "Post-quantum cryptography error: {}", e),
            Self::NumericalError(e) => write!(f, "Numerical error in quantum computation: {}", e),
            Self::InsufficientResources => write!(f, "Insufficient resources for quantum operation"),
        }
    }
}

/// Result type for quantum operations
pub type QuantumResult<T> = Result<T, QuantumError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_config_default() {
        let config = QuantumConfig::default();
        assert_eq!(config.max_qubits, MAX_QUBITS_STATE_VECTOR);
        assert!(config.enable_optimization);
        assert!(config.noise_model.is_none());
    }

    #[test]
    fn test_max_qubits_constant() {
        assert!(MAX_QUBITS_STATE_VECTOR <= 20);
    }

    #[test]
    fn test_error_display() {
        let err = QuantumError::InvalidQubitCount(5);
        assert!(err.to_string().contains("5"));
    }
}
