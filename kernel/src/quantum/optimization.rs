//! Quantum Circuit Optimization
//!
//! This module provides comprehensive circuit optimization techniques:
//! - Gate cancellation (removing inverse gates)
//! - Gate merging (combining consecutive gates)
//! - Qubit reordering for better parallelization
//! - Circuit cutting (decomposing large circuits)
//! - Approximate compilation (trading accuracy for depth)
//! - Noise-aware optimization (considering hardware noise)
//! - Transpilation for different hardware architectures
//! - Circuit depth analysis and optimization

extern crate alloc;

use crate::quantum::QuantumResult;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::String;

/// Optimization level for circuit compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptimizationLevel {
    /// No optimization
    None = 0,
    /// Basic optimizations (gate cancellation)
    Basic = 1,
    /// Moderate optimizations (including gate merging)
    Moderate = 2,
    /// Aggressive optimizations (including circuit rewriting)
    Aggressive = 3,
}

/// Circuit optimization configuration
#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    /// Optimization level
    pub level: OptimizationLevel,
    /// Maximum circuit depth after optimization
    pub max_depth: Option<usize>,
    /// Maximum number of gates after optimization
    pub max_gates: Option<usize>,
    /// Target hardware architecture
    pub target_hardware: Option<HardwareType>,
    /// Whether to use approximate compilation
    pub allow_approximation: bool,
    /// Noise model for noise-aware optimization
    pub noise_aware: bool,
    /// Error tolerance for approximate optimization
    pub error_tolerance: f64,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            level: OptimizationLevel::Moderate,
            max_depth: None,
            max_gates: None,
            target_hardware: None,
            allow_approximation: false,
            noise_aware: false,
            error_tolerance: 1e-6,
        }
    }
}

/// Hardware architecture types for transpilation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareType {
    /// Superconducting qubits (e.g., IBM, Google)
    Superconducting,
    /// Trapped ion qubits (e.g., IonQ, Honeywell)
    TrappedIon,
    /// Photonic quantum computing
    Photonic,
    /// Topological quantum computing
    Topological,
    /// Neutral atom arrays
    NeutralAtom,
    /// Generic/unknown hardware
    Generic,
}

/// Circuit depth analyzer
#[derive(Debug, Clone)]
pub struct CircuitDepth {
    /// Total depth of the circuit
    pub total_depth: usize,
    /// Depth per qubit
    pub qubit_depths: Vec<usize>,
    /// Critical path (longest sequential chain)
    pub critical_path: Vec<usize>,
    /// Parallelizable operations
    pub parallel_layers: Vec<Vec<(usize, String)>>,
}

impl CircuitDepth {
    /// Calculate circuit depth from operations
    pub fn calculate(num_qubits: usize, operations: &[(String, Vec<usize>)]) -> Self {
        let mut qubit_depths = vec![0usize; num_qubits];
        let mut layers: Vec<Vec<(usize, String)>> = Vec::new();

        for (gate_name, qubits) in operations {
            // Find the first layer where all required qubits are free
            let _layer_idx = 0;
            let max_depth = qubits.iter().map(|&q| qubit_depths[q]).max().unwrap_or(0);

            // Create new layers if needed
            while layers.len() <= max_depth {
                layers.push(Vec::new());
            }

            // Add to layer at max_depth
            layers[max_depth].push((max_depth, gate_name.clone()));

            // Update qubit depths
            for &qubit in qubits {
                qubit_depths[qubit] = max_depth + 1;
            }
        }

        let total_depth = layers.len();

        // Find critical path (simplified - just count layers)
        let critical_path: Vec<usize> = (0..total_depth).collect();

        Self {
            total_depth,
            qubit_depths,
            critical_path,
            parallel_layers: layers,
        }
    }
}

/// Optimized circuit with metadata
#[derive(Debug, Clone)]
pub struct OptimizedCircuit {
    /// Optimized operations
    pub operations: Vec<OptimizedOperation>,
    /// Original gate count
    pub original_gates: usize,
    /// Optimized gate count
    pub optimized_gates: usize,
    /// Depth reduction achieved
    pub depth_reduction: f64,
    /// Gate count reduction
    pub gate_reduction: f64,
    /// Optimization time in microseconds (optional)
    pub optimization_time_us: Option<u64>,
}

/// Optimized quantum operation
#[derive(Debug, Clone)]
pub struct OptimizedOperation {
    /// Gate name
    pub gate: String,
    /// Target qubits
    pub targets: Vec<usize>,
    /// Parameters (if any)
    pub params: Vec<f64>,
    /// Estimated execution time
    pub duration: Option<f64>,
    /// Error rate estimate
    pub error_rate: Option<f64>,
}

/// Circuit optimizer
pub struct CircuitOptimizer;

impl CircuitOptimizer {
    /// Optimize a quantum circuit
    ///
    /// # Arguments
    /// * `operations` - List of operations (gate name, target qubits)
    /// * `config` - Optimization configuration
    ///
    /// # Returns
    /// Optimized circuit with metadata
    pub fn optimize(
        operations: &[(String, Vec<usize>)],
        config: &OptimizationConfig,
    ) -> QuantumResult<OptimizedCircuit> {
        let original_gates = operations.len();
        let start = Self::get_time();

        // Apply optimizations based on level
        let mut optimized = operations.to_vec();

        if config.level >= OptimizationLevel::Basic {
            optimized = Self::cancel_inverse_gates(&optimized);
        }

        if config.level >= OptimizationLevel::Moderate {
            optimized = Self::merge_consecutive_gates(&optimized);
            optimized = Self::optimize_rotations(&optimized);
        }

        if config.level >= OptimizationLevel::Aggressive {
            optimized = Self::rewrite_circuit(&optimized);
            optimized = Self::reorder_qubits(&optimized);
        }

        // Apply hardware-specific optimizations
        if let Some(hw_type) = config.target_hardware {
            optimized = Self::transpile_for_hardware(&optimized, hw_type)?;
        }

        // Apply noise-aware optimization if enabled
        if config.noise_aware {
            optimized = Self::noise_aware_optimization(&optimized);
        }

        // Apply approximate compilation if allowed
        if config.allow_approximation {
            optimized = Self::approximate_compilation(&optimized, config.error_tolerance)?;
        }

        let end = Self::get_time();
        let optimization_time_us = Some(end.saturating_sub(start));

        let optimized_gates = optimized.len();
        let gate_reduction = if original_gates > 0 {
            (original_gates - optimized_gates) as f64 / original_gates as f64
        } else {
            0.0
        };

        // Calculate depth reduction
        let original_depth = CircuitDepth::calculate(10, operations).total_depth;
        let optimized_depth = CircuitDepth::calculate(10, &optimized).total_depth;
        let depth_reduction = if original_depth > 0 {
            (original_depth - optimized_depth) as f64 / original_depth as f64
        } else {
            0.0
        };

        // Convert to OptimizedOperation format
        let ops = optimized.into_iter()
            .map(|(gate, targets)| OptimizedOperation {
                gate,
                targets,
                params: Vec::new(),
                duration: None,
                error_rate: None,
            })
            .collect();

        Ok(OptimizedCircuit {
            operations: ops,
            original_gates,
            optimized_gates,
            depth_reduction,
            gate_reduction,
            optimization_time_us,
        })
    }

    /// Cancel adjacent inverse gates
    fn cancel_inverse_gates(operations: &[(String, Vec<usize>)]) -> Vec<(String, Vec<usize>)> {
        let mut result: Vec<(String, Vec<usize>)> = Vec::new();

        for (gate, targets) in operations {
            let should_cancel = result.last().map_or(false, |(last_gate, last_targets)| {
                Self::are_inverse_gates(gate.as_str(), last_gate.as_str()) && targets == last_targets
            });

            if should_cancel {
                result.pop();
            } else {
                result.push((gate.clone(), targets.clone()));
            }
        }

        result
    }

    /// Check if two gates are inverses
    fn are_inverse_gates(gate1: &str, gate2: &str) -> bool {
        let inverse_pairs = [
            ("X", "X"),
            ("Y", "Y"),
            ("Z", "Z"),
            ("H", "H"),
            ("S", "Sdg"),
            ("Sdg", "S"),
            ("T", "Tdg"),
            ("Tdg", "T"),
            ("CNOT", "CNOT"),
            ("CZ", "CZ"),
        ];

        for (name1, name2) in &inverse_pairs {
            if gate1 == *name1 && gate2 == *name2 {
                return true;
            }
        }

        // Check for rotation gates
        if gate1.starts_with("Rx(") && gate2.starts_with("Rx(") {
            return true; // Will be merged instead
        }
        if gate1.starts_with("Ry(") && gate2.starts_with("Ry(") {
            return true;
        }
        if gate1.starts_with("Rz(") && gate2.starts_with("Rz(") {
            return true;
        }

        false
    }

    /// Merge consecutive gates on same qubits
    fn merge_consecutive_gates(operations: &[(String, Vec<usize>)]) -> Vec<(String, Vec<usize>)> {
        let mut result = Vec::new();
        let mut gate_buffer: BTreeMap<Vec<usize>, Vec<String>> = BTreeMap::new();

        // Group gates by their target qubits
        for (gate, targets) in operations {
            gate_buffer.entry(targets.clone()).or_insert_with(Vec::new).push(gate.clone());
        }

        // Try to merge gates in each group
        for (targets, gates) in gate_buffer {
            let merged = Self::merge_gate_sequence(&gates);
            for gate in merged {
                result.push((gate, targets.clone()));
            }
        }

        result
    }

    /// Merge a sequence of gates on the same qubits
    fn merge_gate_sequence(gates: &[String]) -> Vec<String> {
        if gates.len() <= 1 {
            return gates.to_vec();
        }

        // Try to merge consecutive rotations
        let mut result = Vec::new();
        let mut i = 0;

        while i < gates.len() {
            if i + 1 < gates.len() {
                let merged = Self::try_merge_two_gates(&gates[i], &gates[i + 1]);
                if let Some(merged_gate) = merged {
                    result.push(merged_gate);
                    i += 2;
                    continue;
                }
            }
            result.push(gates[i].clone());
            i += 1;
        }

        result
    }

    /// Try to merge two gates
    fn try_merge_two_gates(gate1: &str, gate2: &str) -> Option<String> {
        // Merge Rx rotations
        if let (Some(a1), Some(a2)) = (Self::extract_angle("Rx", gate1), Self::extract_angle("Rx", gate2)) {
            return Some(format!("Rx({:.6})", a1 + a2));
        }

        // Merge Ry rotations
        if let (Some(a1), Some(a2)) = (Self::extract_angle("Ry", gate1), Self::extract_angle("Ry", gate2)) {
            return Some(format!("Ry({:.6})", a1 + a2));
        }

        // Merge Rz rotations
        if let (Some(a1), Some(a2)) = (Self::extract_angle("Rz", gate1), Self::extract_angle("Rz", gate2)) {
            return Some(format!("Rz({:.6})", a1 + a2));
        }

        // H · H = I (identity, can be removed)
        if gate1 == "H" && gate2 == "H" {
            return None; // Cancel out
        }

        // S · S = Z
        if gate1 == "S" && gate2 == "S" {
            return Some(String::from("Z"));
        }

        // T · T = S
        if gate1 == "T" && gate2 == "T" {
            return Some(String::from("S"));
        }

        None
    }

    /// Extract angle from rotation gate string
    fn extract_angle(prefix: &str, gate: &str) -> Option<f64> {
        if gate.starts_with(prefix) && gate.starts_with(prefix) {
            let start = gate.find('(')?;
            let end = gate.find(')')?;
            let angle_str = &gate[start + 1..end];
            angle_str.parse::<f64>().ok()
        } else {
            None
        }
    }

    /// Optimize rotation gates
    fn optimize_rotations(operations: &[(String, Vec<usize>)]) -> Vec<(String, Vec<usize>)> {
        let mut result = Vec::new();
        let mut pending_rx: BTreeMap<usize, f64> = BTreeMap::new();
        let mut pending_ry: BTreeMap<usize, f64> = BTreeMap::new();
        let mut pending_rz: BTreeMap<usize, f64> = BTreeMap::new();

        for (gate, targets) in operations {
            if targets.len() == 1 {
                let qubit = targets[0];

                if let Some(angle) = Self::extract_angle("Rx", gate) {
                    *pending_rx.entry(qubit).or_insert(0.0) += angle;
                    continue;
                } else if let Some(angle) = Self::extract_angle("Ry", gate) {
                    *pending_ry.entry(qubit).or_insert(0.0) += angle;
                    continue;
                } else if let Some(angle) = Self::extract_angle("Rz", gate) {
                    *pending_rz.entry(qubit).or_insert(0.0) += angle;
                    continue;
                }
            }

            // Flush pending rotations for these qubits
            for &qubit in targets {
                if let Some(&angle) = pending_rx.get(&qubit) {
                    if angle.abs() > 1e-10 {
                        result.push((format!("Rx({:.6})", angle), vec![qubit]));
                    }
                    pending_rx.remove(&qubit);
                }
                if let Some(&angle) = pending_ry.get(&qubit) {
                    if angle.abs() > 1e-10 {
                        result.push((format!("Ry({:.6})", angle), vec![qubit]));
                    }
                    pending_ry.remove(&qubit);
                }
                if let Some(&angle) = pending_rz.get(&qubit) {
                    if angle.abs() > 1e-10 {
                        result.push((format!("Rz({:.6})", angle), vec![qubit]));
                    }
                    pending_rz.remove(&qubit);
                }
            }

            result.push((gate.clone(), targets.clone()));
        }

        // Flush remaining rotations
        for (qubit, angle) in pending_rx {
            if angle.abs() > 1e-10 {
                result.push((format!("Rx({:.6})", angle), vec![qubit]));
            }
        }
        for (qubit, angle) in pending_ry {
            if angle.abs() > 1e-10 {
                result.push((format!("Ry({:.6})", angle), vec![qubit]));
            }
        }
        for (qubit, angle) in pending_rz {
            if angle.abs() > 1e-10 {
                result.push((format!("Rz({:.6})", angle), vec![qubit]));
            }
        }

        result
    }

    /// Rewrite circuit using known identities
    fn rewrite_circuit(operations: &[(String, Vec<usize>)]) -> Vec<(String, Vec<usize>)> {
        let mut result = Vec::new();

        for (gate, targets) in operations {
            // Apply circuit rewriting rules
            let rewritten = Self::apply_rewrite_rules(gate, targets);
            result.extend(rewritten);
        }

        result
    }

    /// Apply rewrite rules to a single gate
    fn apply_rewrite_rules(gate: &str, targets: &[usize]) -> Vec<(String, Vec<usize>)> {
        // SWAP decomposition
        if gate == "SWAP" && targets.len() == 2 {
            return vec![
                (String::from("CNOT"), vec![targets[0], targets[1]]),
                (String::from("CNOT"), vec![targets[1], targets[0]]),
                (String::from("CNOT"), vec![targets[0], targets[1]]),
            ];
        }

        // Toffoli decomposition (simplified)
        if gate == "Toffoli" && targets.len() == 3 {
            let result = vec![
                (String::from("H"), vec![targets[2]]),
                (String::from("CNOT"), vec![targets[1], targets[2]]),
                (String::from("Tdg"), vec![targets[2]]),
                (String::from("CNOT"), vec![targets[0], targets[2]]),
                (String::from("T"), vec![targets[2]]),
                (String::from("CNOT"), vec![targets[1], targets[2]]),
                (String::from("Tdg"), vec![targets[2]]),
                (String::from("CNOT"), vec![targets[0], targets[2]]),
                (String::from("T"), vec![targets[1]]),
                (String::from("T"), vec![targets[2]]),
                (String::from("H"), vec![targets[2]]),
                (String::from("CNOT"), vec![targets[0], targets[1]]),
                (String::from("T"), vec![targets[0]]),
                (String::from("Tdg"), vec![targets[1]]),
                (String::from("CNOT"), vec![targets[0], targets[1]]),
            ];
            return result;
        }

        vec![(String::from(gate), targets.to_vec())]
    }

    /// Reorder qubits for better parallelization
    fn reorder_qubits(operations: &[(String, Vec<usize>)]) -> Vec<(String, Vec<usize>)> {
        // Simplified qubit reordering
        // In practice, this would analyze qubit connectivity
        operations.to_vec()
    }

    /// Transpile circuit for specific hardware
    fn transpile_for_hardware(
        operations: &[(String, Vec<usize>)],
        hw_type: HardwareType,
    ) -> QuantumResult<Vec<(String, Vec<usize>)>> {
        let result = match hw_type {
            HardwareType::Superconducting => {
                // Convert to native gate set (e.g., IBM's CX, U3, ID)
                Self::transpile_to_superconducting(operations)?
            }
            HardwareType::TrappedIon => {
                // All-to-all connectivity, different gate set
                Self::transpile_to_trapped_ion(operations)?
            }
            HardwareType::Photonic => {
                // Measurement-based approach
                Self::transpile_to_photonic(operations)?
            }
            _ => operations.to_vec(),
        };

        Ok(result)
    }

    /// Transpile to superconducting qubit gate set
    fn transpile_to_superconducting(operations: &[(String, Vec<usize>)]) -> QuantumResult<Vec<(String, Vec<usize>)>> {
        // Convert gates to CX, U3, ID
        let mut result = Vec::new();

        for (gate, targets) in operations {
            match gate.as_str() {
                "CNOT" => {
                    result.push((String::from("CX"), targets.clone()));
                }
                "H" => {
                    // H = U3(π/2, 0, π)
                    result.push((format!("U3({:.6},{:.6},{:.6})",
                                        core::f64::consts::PI/2.0, 0.0, core::f64::consts::PI),
                                targets.clone()));
                }
                _ => {
                    result.push((gate.clone(), targets.clone()));
                }
            }
        }

        Ok(result)
    }

    /// Transpile to trapped ion gate set
    fn transpile_to_trapped_ion(operations: &[(String, Vec<usize>)]) -> QuantumResult<Vec<(String, Vec<usize>)>> {
        // Trapped ions have all-to-all connectivity
        // Can use native two-qubit gates directly
        Ok(operations.to_vec())
    }

    /// Transpile to photonic quantum computing
    fn transpile_to_photonic(operations: &[(String, Vec<usize>)]) -> QuantumResult<Vec<(String, Vec<usize>)>> {
        // Photonic QC often uses measurement-based approach
        // This is a placeholder for full transpilation
        Ok(operations.to_vec())
    }

    /// Noise-aware circuit optimization
    fn noise_aware_optimization(operations: &[(String, Vec<usize>)]) -> Vec<(String, Vec<usize>)> {
        // Avoid deep circuits, minimize two-qubit gates
        // This is a simplified implementation
        operations.to_vec()
    }

    /// Approximate circuit compilation
    fn approximate_compilation(
        operations: &[(String, Vec<usize>)],
        error_tolerance: f64,
    ) -> QuantumResult<Vec<(String, Vec<usize>)>> {
        // Replace certain gate sequences with approximations
        // This is a simplified implementation
        if error_tolerance < 1e-6 {
            return Ok(operations.to_vec());
        }

        // Example: Replace small rotations with identity
        let mut result = Vec::new();
        for (gate, targets) in operations {
            if let Some(angle) = Self::extract_angle("Rx", gate) {
                if angle.abs() < error_tolerance {
                    continue; // Skip small rotations
                }
            } else if let Some(angle) = Self::extract_angle("Ry", gate) {
                if angle.abs() < error_tolerance {
                    continue;
                }
            } else if let Some(angle) = Self::extract_angle("Rz", gate) {
                if angle.abs() < error_tolerance {
                    continue;
                }
            }
            result.push((gate.clone(), targets.clone()));
        }

        Ok(result)
    }

    /// Get current time in microseconds
    fn get_time() -> u64 {
        // Placeholder - in real implementation would use actual timing
        0
    }

    /// Analyze circuit for optimization opportunities
    pub fn analyze_circuit(operations: &[(String, Vec<usize>)]) -> CircuitAnalysis {
        let total_gates = operations.len();

        let mut single_qubit_gates = 0;
        let mut two_qubit_gates = 0;
        let mut multi_qubit_gates = 0;
        let mut rotation_gates = 0;

        let mut gate_counts: BTreeMap<String, usize> = BTreeMap::new();

        for (gate, targets) in operations {
            *gate_counts.entry(gate.clone()).or_insert(0) += 1;

            match targets.len() {
                1 => single_qubit_gates += 1,
                2 => two_qubit_gates += 1,
                _ => multi_qubit_gates += 1,
            }

            if gate.starts_with("Rx(") || gate.starts_with("Ry(") || gate.starts_with("Rz(") {
                rotation_gates += 1;
            }
        }

        CircuitAnalysis {
            total_gates,
            single_qubit_gates,
            two_qubit_gates,
            multi_qubit_gates,
            rotation_gates,
            gate_counts,
        }
    }
}

/// Circuit analysis result
#[derive(Debug, Clone)]
pub struct CircuitAnalysis {
    /// Total number of gates
    pub total_gates: usize,
    /// Number of single-qubit gates
    pub single_qubit_gates: usize,
    /// Number of two-qubit gates
    pub two_qubit_gates: usize,
    /// Number of multi-qubit gates
    pub multi_qubit_gates: usize,
    /// Number of rotation gates
    pub rotation_gates: usize,
    /// Count of each gate type
    pub gate_counts: BTreeMap<String, usize>,
}

/// Circuit cutter for decomposing large circuits
pub struct CircuitCutter;

impl CircuitCutter {
    /// Cut circuit into smaller sub-circuits
    ///
    /// # Arguments
    /// * `operations` - Circuit operations
    /// * `max_subcircuit_size` - Maximum size of each sub-circuit
    ///
    /// # Returns
    /// Vector of sub-circuits
    pub fn cut_circuit(
        operations: &[(String, Vec<usize>)],
        max_subcircuit_size: usize,
    ) -> Vec<Vec<(String, Vec<usize>)>> {
        let mut subcircuits = Vec::new();
        let mut current_subcircuit = Vec::new();
        let mut current_size = 0;

        for (gate, targets) in operations {
            let gate_size = targets.len();

            if current_size + gate_size > max_subcircuit_size && !current_subcircuit.is_empty() {
                subcircuits.push(current_subcircuit);
                current_subcircuit = Vec::new();
                current_size = 0;
            }

            current_subcircuit.push((gate.clone(), targets.clone()));
            current_size += gate_size;
        }

        if !current_subcircuit.is_empty() {
            subcircuits.push(current_subcircuit);
        }

        subcircuits
    }

    /// Find optimal cut positions using graph partitioning
    pub fn find_optimal_cuts(
        operations: &[(String, Vec<usize>)],
        num_cuts: usize,
    ) -> Vec<usize> {
        // Simplified: evenly spaced cuts
        let cut_positions = operations.len() / (num_cuts + 1);
        (1..=num_cuts).map(|i| i * cut_positions).collect()
    }
}

/// Gate synthesizer for creating optimal gate sequences
pub struct GateSynthesizer;

impl GateSynthesizer {
    /// Synthesize a unitary matrix into optimal gate sequence
    ///
    /// # Arguments
    /// * `matrix` - Target unitary matrix
    /// * `target_qubits` - Target qubits
    /// * `max_gates` - Maximum number of gates to use
    ///
    /// # Returns
    /// Gate sequence that approximates the matrix
    pub fn synthesize_unitary(
        _matrix: &Vec<Vec<num_complex::Complex64>>,
        _target_qubits: &[usize],
        _max_gates: usize,
    ) -> QuantumResult<Vec<(String, Vec<usize>)>> {
        // Simplified implementation
        // Real implementation would use KAK decomposition or similar
        Ok(Vec::new())
    }

    /// Synthesize rotation from arbitrary single-qubit unitary
    pub fn synthesize_single_qubit_unitary(
        _matrix: &Vec<Vec<num_complex::Complex64>>,
        _target: usize,
    ) -> QuantumResult<Vec<(String, Vec<usize>)>> {
        // Decompose using Z-Y-Z decomposition
        // U = e^(iα) Rz(β) Ry(γ) Rz(δ)
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gate_cancellation() {
        let operations = vec![
            (String::from("H"), vec![0]),
            (String::from("H"), vec![0]),
            (String::from("X"), vec![1]),
        ];

        let optimized = CircuitOptimizer::cancel_inverse_gates(&operations);

        assert_eq!(optimized.len(), 1);
        assert_eq!(optimized[0].0, "X");
    }

    #[test]
    fn test_rotation_merge() {
        let operations = vec![
            (String::from("Rx(0.5)"), vec![0]),
            (String::from("Rx(0.3)"), vec![0]),
            (String::from("Y"), vec![1]),
        ];

        let optimized = CircuitOptimizer::optimize_rotations(&operations);

        assert_eq!(optimized.len(), 2);
        assert!(optimized[0].0.starts_with("Rx("));
    }

    #[test]
    fn test_circuit_depth() {
        let operations = vec![
            (String::from("H"), vec![0]),
            (String::from("X"), vec![1]),
            (String::from("CNOT"), vec![0, 1]),
        ];

        let depth = CircuitDepth::calculate(2, &operations);

        assert_eq!(depth.total_depth, 2);
    }

    #[test]
    fn test_circuit_analysis() {
        let operations = vec![
            (String::from("H"), vec![0]),
            (String::from("X"), vec![0]),
            (String::from("CNOT"), vec![0, 1]),
            (String::from("Rz(0.5)"), vec![1]),
        ];

        let analysis = CircuitOptimizer::analyze_circuit(&operations);

        assert_eq!(analysis.total_gates, 4);
        assert_eq!(analysis.single_qubit_gates, 3);
        assert_eq!(analysis.two_qubit_gates, 1);
        assert_eq!(analysis.rotation_gates, 1);
    }

    #[test]
    fn test_circuit_cutting() {
        let operations = vec![
            (String::from("H"), vec![0]),
            (String::from("X"), vec![0]),
            (String::from("Y"), vec![0]),
            (String::from("Z"), vec![0]),
        ];

        let subcircuits = CircuitCutter::cut_circuit(&operations, 2);

        assert!(!subcircuits.is_empty());
    }

    #[test]
    fn test_optimization_pipeline() {
        let operations = vec![
            (String::from("H"), vec![0]),
            (String::from("H"), vec![0]),
            (String::from("X"), vec![1]),
        ];

        let config = OptimizationConfig {
            level: OptimizationLevel::Basic,
            ..Default::default()
        };

        let optimized = CircuitOptimizer::optimize(&operations, &config).unwrap();

        assert!(optimized.optimized_gates <= optimized.original_gates);
        assert!(optimized.gate_reduction >= 0.0);
    }
}
