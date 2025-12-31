//! Quantum Circuit Implementation
//!
//! This module provides quantum circuit construction, optimization, and simulation:
//! - Circuit building with quantum gates
//! - Circuit optimization (gate merging, cancellation)
//! - Parallel circuit execution
//! - Circuit depth analysis
//! - State evolution simulation

use crate::quantum::{
    QuantumState, QuantumGate, TwoQubitGate, QuantumError, QuantumResult, NoiseModel
};

use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;
use alloc::collections::BTreeMap;
use hashbrown::HashSet;

/// Quantum circuit containing a sequence of operations
#[derive(Debug, Clone)]
pub struct Circuit {
    /// Number of qubits in the circuit
    num_qubits: usize,
    /// List of quantum operations
    operations: Vec<QuantumOperation>,
    /// Measurement operations (applied at the end)
    measurements: Vec<(usize, String)>,
    /// Circuit metadata
    metadata: CircuitMetadata,
}

/// Metadata about the circuit
#[derive(Debug, Clone, Default)]
struct CircuitMetadata {
    /// Name of the circuit
    name: String,
    /// Description
    description: String,
    /// Number of parameters (for parameterized circuits)
    num_parameters: usize,
    /// Tags for circuit categorization
    tags: alloc::vec::Vec<String>,
}

/// A quantum operation (gate application)
#[derive(Debug, Clone)]
pub enum QuantumOperation {
    /// Single-qubit gate
    SingleQubit {
        gate: QuantumGate,
        target: usize,
        label: Option<String>,
    },
    /// Two-qubit gate
    TwoQubit {
        gate: TwoQubitGate,
        control: usize,
        target: usize,
        label: Option<String>,
    },
    /// Multi-qubit rotation (for variational algorithms)
    Rotation {
        axis: RotationAxis,
        angle: f64,
        targets: Vec<usize>,
        label: Option<String>,
    },
    /// Measurement operation
    Measurement {
        qubit: usize,
        output_bit: Option<usize>,
    },
    /// Barrier (prevents optimization across this point)
    Barrier {
        qubits: Vec<usize>,
    },
    /// Reset operation
    Reset {
        qubit: usize,
    },
}

/// Rotation axis for parameterized gates
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationAxis {
    X,
    Y,
    Z,
}

impl Circuit {
    /// Create a new quantum circuit
    ///
    /// # Arguments
    /// * `num_qubits` - Number of qubits in the circuit
    ///
    /// # Example
    /// ```
    /// use kernel::quantum::Circuit;
    ///
    /// let mut circuit = Circuit::new(2);
    /// circuit.h(0).unwrap();
    /// circuit.cnot(0, 1).unwrap();
    /// ```
    pub fn new(num_qubits: usize) -> Self {
        Self {
            num_qubits,
            operations: Vec::new(),
            measurements: Vec::new(),
            metadata: CircuitMetadata {
                name: format!("circuit_{}", num_qubits),
                ..Default::default()
            },
        }
    }

    /// Get the number of qubits
    pub fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    /// Get the number of operations
    pub fn num_operations(&self) -> usize {
        self.operations.len()
    }

    /// Get the circuit depth (parallel layers of operations)
    pub fn depth(&self) -> usize {
        let mut depth = 0;
        let mut current_layer_qubits = HashSet::new();

        for op in &self.operations {
            let qubits = match op {
                QuantumOperation::SingleQubit { target, .. } => vec![*target],
                QuantumOperation::TwoQubit { control, target, .. } => vec![*control, *target],
                QuantumOperation::Rotation { targets, .. } => targets.clone(),
                QuantumOperation::Measurement { qubit, .. } => vec![*qubit],
                QuantumOperation::Barrier { qubits } => qubits.clone(),
                QuantumOperation::Reset { qubit } => vec![*qubit],
            };

            // Check if we need a new layer
            let overlaps = qubits.iter().any(|q| current_layer_qubits.contains(q));
            if overlaps {
                depth += 1;
                current_layer_qubits.clear();
            }

            current_layer_qubits.extend(qubits);
        }

        if !current_layer_qubits.is_empty() {
            depth += 1;
        }

        depth
    }

    /// Set the circuit name
    pub fn with_name(mut self, name: &str) -> Self {
        self.metadata.name = String::from(name);
        self
    }

    /// Add a description to the circuit
    pub fn with_description(mut self, description: &str) -> Self {
        self.metadata.description = String::from(description);
        self
    }

    /// Add a Hadamard gate to a qubit
    pub fn h(&mut self, qubit: usize) -> QuantumResult<&mut Self> {
        if qubit >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(qubit));
        }
        self.operations.push(QuantumOperation::SingleQubit {
            gate: QuantumGate::h(),
            target: qubit,
            label: Some(format!("h[{}]", qubit)),
        });
        Ok(self)
    }

    /// Add an X gate to a qubit
    pub fn x(&mut self, qubit: usize) -> QuantumResult<&mut Self> {
        if qubit >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(qubit));
        }
        self.operations.push(QuantumOperation::SingleQubit {
            gate: QuantumGate::x(),
            target: qubit,
            label: Some(format!("x[{}]", qubit)),
        });
        Ok(self)
    }

    /// Add a Y gate to a qubit
    pub fn y(&mut self, qubit: usize) -> QuantumResult<&mut Self> {
        if qubit >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(qubit));
        }
        self.operations.push(QuantumOperation::SingleQubit {
            gate: QuantumGate::y(),
            target: qubit,
            label: Some(format!("y[{}]", qubit)),
        });
        Ok(self)
    }

    /// Add a Z gate to a qubit
    pub fn z(&mut self, qubit: usize) -> QuantumResult<&mut Self> {
        if qubit >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(qubit));
        }
        self.operations.push(QuantumOperation::SingleQubit {
            gate: QuantumGate::z(),
            target: qubit,
            label: Some(format!("z[{}]", qubit)),
        });
        Ok(self)
    }

    /// Add an S gate to a qubit
    pub fn s(&mut self, qubit: usize) -> QuantumResult<&mut Self> {
        if qubit >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(qubit));
        }
        self.operations.push(QuantumOperation::SingleQubit {
            gate: QuantumGate::s(),
            target: qubit,
            label: Some(format!("s[{}]", qubit)),
        });
        Ok(self)
    }

    /// Add a T gate to a qubit
    pub fn t(&mut self, qubit: usize) -> QuantumResult<&mut Self> {
        if qubit >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(qubit));
        }
        self.operations.push(QuantumOperation::SingleQubit {
            gate: QuantumGate::t(),
            target: qubit,
            label: Some(format!("t[{}]", qubit)),
        });
        Ok(self)
    }

    /// Add a rotation gate
    pub fn rx(&mut self, qubit: usize, theta: f64) -> QuantumResult<&mut Self> {
        if qubit >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(qubit));
        }
        self.operations.push(QuantumOperation::SingleQubit {
            gate: QuantumGate::rx(theta),
            target: qubit,
            label: Some(format!("rx({}, {})", qubit, theta)),
        });
        Ok(self)
    }

    /// Add a rotation around Y-axis
    pub fn ry(&mut self, qubit: usize, theta: f64) -> QuantumResult<&mut Self> {
        if qubit >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(qubit));
        }
        self.operations.push(QuantumOperation::SingleQubit {
            gate: QuantumGate::ry(theta),
            target: qubit,
            label: Some(format!("ry({}, {})", qubit, theta)),
        });
        Ok(self)
    }

    /// Add a rotation around Z-axis
    pub fn rz(&mut self, qubit: usize, theta: f64) -> QuantumResult<&mut Self> {
        if qubit >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(qubit));
        }
        self.operations.push(QuantumOperation::SingleQubit {
            gate: QuantumGate::rz(theta),
            target: qubit,
            label: Some(format!("rz({}, {})", qubit, theta)),
        });
        Ok(self)
    }

    /// Add a CNOT gate
    pub fn cnot(&mut self, control: usize, target: usize) -> QuantumResult<&mut Self> {
        if control >= self.num_qubits || target >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(self.num_qubits));
        }
        self.operations.push(QuantumOperation::TwoQubit {
            gate: TwoQubitGate::cnot(),
            control,
            target,
            label: Some(format!("cx[{}, {}]", control, target)),
        });
        Ok(self)
    }

    /// Add a CZ gate
    pub fn cz(&mut self, control: usize, target: usize) -> QuantumResult<&mut Self> {
        if control >= self.num_qubits || target >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(self.num_qubits));
        }
        self.operations.push(QuantumOperation::TwoQubit {
            gate: TwoQubitGate::cz(),
            control,
            target,
            label: Some(format!("cz[{}, {}]", control, target)),
        });
        Ok(self)
    }

    /// Add a SWAP gate
    pub fn swap(&mut self, qubit1: usize, qubit2: usize) -> QuantumResult<&mut Self> {
        if qubit1 >= self.num_qubits || qubit2 >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(self.num_qubits));
        }

        // SWAP can be decomposed into 3 CNOTs
        self.cnot(qubit1, qubit2)?;
        self.cnot(qubit2, qubit1)?;
        self.cnot(qubit1, qubit2)?;

        Ok(self)
    }

    /// Add a Toffoli (CCNOT) gate
    pub fn toffoli(&mut self, control1: usize, control2: usize, target: usize) -> QuantumResult<&mut Self> {
        if control1 >= self.num_qubits || control2 >= self.num_qubits || target >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(self.num_qubits));
        }

        // Toffoli decomposition using H, T, T†, and CNOT gates
        self.h(target)?;
        self.cnot(control2, target)?;
        self.t_dag(target)?;
        self.cnot(control1, target)?;
        self.t(target)?;
        self.cnot(control2, target)?;
        self.t_dag(target)?;
        self.h(target)?;

        Ok(self)
    }

    /// Add T† gate
    pub fn t_dag(&mut self, qubit: usize) -> QuantumResult<&mut Self> {
        if qubit >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(qubit));
        }
        self.operations.push(QuantumOperation::SingleQubit {
            gate: QuantumGate::t_dag(),
            target: qubit,
            label: Some(format!("t†[{}]", qubit)),
        });
        Ok(self)
    }

    /// Add a barrier to prevent optimization
    pub fn barrier(&mut self, qubits: Vec<usize>) -> QuantumResult<&mut Self> {
        for &q in &qubits {
            if q >= self.num_qubits {
                return Err(QuantumError::QubitIndexOutOfBounds(q));
            }
        }
        self.operations.push(QuantumOperation::Barrier { qubits });
        Ok(self)
    }

    /// Measure a specific qubit
    pub fn measure(&mut self, qubit: usize) -> QuantumResult<&mut Self> {
        if qubit >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(qubit));
        }
        self.operations.push(QuantumOperation::Measurement {
            qubit,
            output_bit: Some(self.measurements.len()),
        });
        self.measurements.push((qubit, format!("m[{}]", qubit)));
        Ok(self)
    }

    /// Measure all qubits
    pub fn measure_all(&mut self) -> QuantumResult<&mut Self> {
        for i in 0..self.num_qubits {
            self.measure(i)?;
        }
        Ok(self)
    }

    /// Reset a qubit to |0⟩
    pub fn reset(&mut self, qubit: usize) -> QuantumResult<&mut Self> {
        if qubit >= self.num_qubits {
            return Err(QuantumError::QubitIndexOutOfBounds(qubit));
        }
        self.operations.push(QuantumOperation::Reset { qubit });
        Ok(self)
    }

    /// Simulate the circuit and return the final state
    ///
    /// # Arguments
    /// * `noise_model` - Optional noise model to apply
    pub fn simulate(&self, noise_model: Option<&NoiseModel>) -> QuantumResult<QuantumState> {
        let mut state = QuantumState::new(self.num_qubits)?;

        for op in &self.operations {
            match op {
                QuantumOperation::SingleQubit { gate, target, .. } => {
                    state.apply_single_qubit_gate(gate, *target)?;

                    // Apply noise if configured
                    if let Some(noise) = noise_model {
                        noise.apply(&mut state, *target)?;
                    }
                }
                QuantumOperation::TwoQubit { gate, control, target, .. } => {
                    state.apply_two_qubit_gate(gate, *control, *target)?;

                    // Apply noise to both qubits
                    if let Some(noise) = noise_model {
                        noise.apply(&mut state, *control)?;
                        noise.apply(&mut state, *target)?;
                    }
                }
                QuantumOperation::Rotation { axis, angle, targets, .. } => {
                    let gate = match axis {
                        RotationAxis::X => QuantumGate::rx(*angle),
                        RotationAxis::Y => QuantumGate::ry(*angle),
                        RotationAxis::Z => QuantumGate::rz(*angle),
                    };

                    for &target in targets {
                        state.apply_single_qubit_gate(&gate, target)?;

                        if let Some(noise) = noise_model {
                            noise.apply(&mut state, target)?;
                        }
                    }
                }
                QuantumOperation::Measurement { qubit, .. } => {
                    state.measure_qubit(*qubit)?;
                }
                QuantumOperation::Barrier { .. } => {
                    // Barrier prevents optimization but does nothing in simulation
                }
                QuantumOperation::Reset { qubit } => {
                    // Reset qubit to |0⟩ by measuring and then applying X if needed
                    if state.measure_qubit(*qubit)? == 1 {
                        state.apply_single_qubit_gate(&QuantumGate::x(), *qubit)?;
                    }
                }
            }
        }

        Ok(state)
    }

    /// Execute the circuit multiple times and collect measurement statistics
    ///
    /// # Arguments
    /// * `shots` - Number of times to run the circuit
    /// * `noise_model` - Optional noise model
    pub fn execute(
        &self,
        shots: usize,
        noise_model: Option<&NoiseModel>,
    ) -> QuantumResult<BTreeMap<u64, usize>> {
        let mut counts = BTreeMap::new();

        for _ in 0..shots {
            let state = self.simulate(noise_model)?;

            // Collect measurement results
            let mut result = 0u64;
            for (i, &(qubit, _)) in self.measurements.iter().enumerate() {
                let prob = state.get_probability(1u64 << qubit)?;
                let bit = if rand::random::<f64>() < prob { 1 } else { 0 };
                result |= (bit as u64) << i;
            }

            *counts.entry(result).or_insert(0) += 1;
        }

        Ok(counts)
    }

    /// Optimize the circuit using various techniques
    pub fn optimize(&mut self) -> QuantumResult<&mut Self> {
        let optimizer = CircuitOptimizer::new();
        optimizer.optimize(self)?;
        Ok(self)
    }

    /// Get a detailed analysis of the circuit
    pub fn analyze(&self) -> CircuitAnalysis {
        let mut gate_counts = BTreeMap::new();
        let mut two_qubit_gate_count = 0;
        let mut single_qubit_gate_count = 0;

        for op in &self.operations {
            match op {
                QuantumOperation::SingleQubit { gate, .. } => {
                    *gate_counts.entry(gate.name.clone()).or_insert(0) += 1;
                    single_qubit_gate_count += 1;
                }
                QuantumOperation::TwoQubit { gate, .. } => {
                    *gate_counts.entry(gate.name.clone()).or_insert(0) += 1;
                    two_qubit_gate_count += 1;
                }
                QuantumOperation::Rotation { axis, .. } => {
                    let name = format!("R{:?}", axis);
                    *gate_counts.entry(name).or_insert(0) += 1;
                    single_qubit_gate_count += 1;
                }
                _ => {}
            }
        }

        CircuitAnalysis {
            depth: self.depth(),
            total_gates: single_qubit_gate_count + two_qubit_gate_count,
            single_qubit_gates: single_qubit_gate_count,
            two_qubit_gates: two_qubit_gate_count,
            gate_counts,
            num_measurements: self.measurements.len(),
        }
    }
}

/// Analysis results for a circuit
#[derive(Debug, Clone)]
pub struct CircuitAnalysis {
    /// Circuit depth
    pub depth: usize,
    /// Total number of gates
    pub total_gates: usize,
    /// Number of single-qubit gates
    pub single_qubit_gates: usize,
    /// Number of two-qubit gates
    pub two_qubit_gates: usize,
    /// Count of each gate type
    pub gate_counts: BTreeMap<String, usize>,
    /// Number of measurements
    pub num_measurements: usize,
}

/// Circuit optimizer for reducing gate count and depth
pub struct CircuitOptimizer {
    /// Whether to merge consecutive single-qubit gates
    merge_single_qubit: bool,
    /// Whether to cancel inverse gate pairs
    cancel_inverses: bool,
    /// Whether to optimize CNOT gates
    optimize_cnot: bool,
}

impl CircuitOptimizer {
    /// Create a new circuit optimizer with default settings
    pub fn new() -> Self {
        Self {
            merge_single_qubit: true,
            cancel_inverses: true,
            optimize_cnot: true,
        }
    }

    /// Set whether to merge single-qubit gates
    pub fn with_merge(mut self, merge: bool) -> Self {
        self.merge_single_qubit = merge;
        self
    }

    /// Set whether to cancel inverse gates
    pub fn with_cancel(mut self, cancel: bool) -> Self {
        self.cancel_inverses = cancel;
        self
    }

    /// Optimize a quantum circuit
    pub fn optimize(&self, circuit: &mut Circuit) -> QuantumResult<()> {
        let mut optimized_ops = Vec::new();
        let mut i = 0;

        while i < circuit.operations.len() {
            let current_op = circuit.operations[i].clone();

            // Check for barriers - they prevent optimization
            if matches!(current_op, QuantumOperation::Barrier { .. }) {
                optimized_ops.push(current_op);
                i += 1;
                continue;
            }

            // Try to merge with next operation
            if self.merge_single_qubit && i + 1 < circuit.operations.len() {
                if let (QuantumOperation::SingleQubit { gate: gate1, target: t1, .. },
                        QuantumOperation::SingleQubit { gate: gate2, target: t2, .. }) =
                    (&current_op, &circuit.operations[i + 1])
                {
                    if t1 == t2 {
                        // Merge the two gates
                        let merged_gate = self.merge_gates(gate1, gate2)?;
                        optimized_ops.push(QuantumOperation::SingleQubit {
                            gate: merged_gate,
                            target: *t1,
                            label: None,
                        });
                        i += 2;
                        continue;
                    }
                }
            }

            // Try to cancel inverse gates
            if self.cancel_inverses && i + 1 < circuit.operations.len() {
                if self.are_inverses(&current_op, &circuit.operations[i + 1]) {
                    i += 2; // Skip both
                    continue;
                }
            }

            optimized_ops.push(current_op);
            i += 1;
        }

        circuit.operations = optimized_ops;
        Ok(())
    }

    /// Merge two single-qubit gates into one
    fn merge_gates(&self, gate1: &QuantumGate, gate2: &QuantumGate) -> QuantumResult<QuantumGate> {
        // Matrix multiplication: gate2 * gate1 (gate1 is applied first)
        let matrix = [
            [
                gate2.matrix[0][0] * gate1.matrix[0][0] + gate2.matrix[0][1] * gate1.matrix[1][0],
                gate2.matrix[0][0] * gate1.matrix[0][1] + gate2.matrix[0][1] * gate1.matrix[1][1],
            ],
            [
                gate2.matrix[1][0] * gate1.matrix[0][0] + gate2.matrix[1][1] * gate1.matrix[1][0],
                gate2.matrix[1][0] * gate1.matrix[0][1] + gate2.matrix[1][1] * gate1.matrix[1][1],
            ],
        ];

        Ok(QuantumGate {
            matrix,
            name: format!("{}_{}", gate1.name, gate2.name),
        })
    }

    /// Check if two operations are inverses of each other
    fn are_inverses(&self, op1: &QuantumOperation, op2: &QuantumOperation) -> bool {
        match (op1, op2) {
            (QuantumOperation::SingleQubit { gate: g1, target: t1, .. },
             QuantumOperation::SingleQubit { gate: g2, target: t2, .. }) => {
                if t1 != t2 {
                    return false;
                }

                // Check specific gate pairs
                matches!(
                    (g1.name.as_str(), g2.name.as_str()),
                    ("X", "X") | ("Y", "Y") | ("Z", "Z") | ("H", "H") |
                    ("S", "S†") | ("S†", "S") | ("T", "T†") | ("T†", "T")
                )
            }
            _ => false,
        }
    }
}

impl Default for CircuitOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for types that can provide circuit depth information
pub trait CircuitDepth {
    /// Get the circuit depth
    fn get_depth(&self) -> usize;

    /// Get the critical path depth (deepest dependency chain)
    fn get_critical_path(&self) -> usize;
}

impl CircuitDepth for Circuit {
    fn get_depth(&self) -> usize {
        self.depth()
    }

    fn get_critical_path(&self) -> usize {
        // Simplified critical path calculation
        // In a full implementation, this would analyze dependencies
        self.operations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_creation() {
        let circuit = Circuit::new(2);
        assert_eq!(circuit.num_qubits(), 2);
        assert_eq!(circuit.num_operations(), 0);
    }

    #[test]
    fn test_circuit_build() {
        let mut circuit = Circuit::new(2);
        circuit.h(0).unwrap();
        circuit.cnot(0, 1).unwrap();

        assert_eq!(circuit.num_operations(), 2);
        let analysis = circuit.analyze();
        assert_eq!(analysis.total_gates, 2);
    }

    #[test]
    fn test_bell_state() {
        let mut circuit = Circuit::new(2);
        circuit.h(0).unwrap();
        circuit.cnot(0, 1).unwrap();

        let state = circuit.simulate(None).unwrap();

        // Should have 50% probability for |00⟩ and |11⟩
        let prob_00 = state.get_probability(0b00).unwrap();
        let prob_11 = state.get_probability(0b11).unwrap();

        assert!((prob_00 - 0.5).abs() < 1e-10);
        assert!((prob_11 - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_circuit_depth() {
        let mut circuit = Circuit::new(3);
        circuit.h(0).unwrap();
        circuit.h(1).unwrap();
        circuit.h(2).unwrap();
        circuit.cnot(0, 1).unwrap();
        circuit.cnot(1, 2).unwrap();

        // First layer: H on all three qubits (can be parallel)
        // Second layer: CNOT(0,1)
        // Third layer: CNOT(1,2)
        assert_eq!(circuit.depth(), 3);
    }

    #[test]
    fn test_circuit_optimization() {
        let mut circuit = Circuit::new(1);
        circuit.x(0).unwrap();
        circuit.x(0).unwrap(); // Should cancel with first X

        circuit.optimize().unwrap();

        // After optimization, should have no gates (X*X = I)
        assert_eq!(circuit.num_operations(), 0);
    }

    #[test]
    fn test_swap_gate() {
        let mut circuit = Circuit::new(2);
        circuit.x(0).unwrap();
        circuit.swap(0, 1).unwrap();

        let state = circuit.simulate(None).unwrap();
        let prob_10 = state.get_probability(0b10).unwrap();

        // |10⟩ should have probability 1 after swap
        assert!((prob_10 - 1.0).abs() < 1e-10);
    }
}
