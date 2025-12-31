//! Health aggregation for the NOS kernel.
//!
//! This module provides multi-component health aggregation:
//! - Aggregation strategies (all, any, majority)
//! - Component dependencies
//! - Cascading failure detection
//! - Graceful degradation
//! - Partial health status reporting

#![no_std]

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::hash::Hash;
use spin::RwLock;

use crate::subsystems::time::Timestamp;
use super::check::{HealthStatus, CheckResult};

/// Health aggregation strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AggregationStrategy {
    /// All components must be healthy
    All,
    /// At least one component must be healthy
    Any,
    /// Majority of components must be healthy
    Majority,
    /// Weighted aggregation based on component importance
    Weighted,
    /// Custom aggregation logic
    Custom,
}

impl AggregationStrategy {
    /// Get strategy name
    pub fn name(&self) -> &str {
        match self {
            AggregationStrategy::All => "all",
            AggregationStrategy::Any => "any",
            AggregationStrategy::Majority => "majority",
            AggregationStrategy::Weighted => "weighted",
            AggregationStrategy::Custom => "custom",
        }
    }
}

/// Component health information
#[derive(Debug, Clone)]
pub struct ComponentHealth {
    /// Component identifier
    pub id: String,
    /// Current health status
    pub status: HealthStatus,
    /// Last check timestamp
    pub last_check: Timestamp,
    /// Component weight (for weighted aggregation)
    pub weight: f64,
    /// Optional health check result
    pub last_result: Option<CheckResult>,
    /// Dependencies (component IDs this component depends on)
    pub dependencies: Vec<String>,
    /// Whether this component is critical
    pub critical: bool,
}

impl ComponentHealth {
    /// Create new component health
    pub fn new(id: String) -> Self {
        ComponentHealth {
            id,
            status: HealthStatus::Unknown,
            last_check: Timestamp::now(),
            weight: 1.0,
            last_result: None,
            dependencies: Vec::new(),
            critical: false,
        }
    }

    /// Set health status
    pub fn with_status(mut self, status: HealthStatus) -> Self {
        self.status = status;
        self.last_check = Timestamp::now();
        self
    }

    /// Set weight
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// Set last result
    pub fn with_last_result(mut self, result: CheckResult) -> Self {
        self.last_result = Some(result);
        self
    }

    /// Add dependency
    pub fn with_dependency(mut self, dep: String) -> Self {
        self.dependencies.push(dep);
        self
    }

    /// Set critical flag
    pub fn with_critical(mut self, critical: bool) -> Self {
        self.critical = critical;
        self
    }

    /// Check if component is healthy
    pub fn is_healthy(&self) -> bool {
        self.status.is_healthy()
    }
}

/// Aggregation result
#[derive(Debug, Clone)]
pub struct AggregationResult {
    /// Overall health status
    pub status: HealthStatus,
    /// Number of healthy components
    pub healthy_count: usize,
    /// Number of unhealthy components
    pub unhealthy_count: usize,
    /// Number of components with unknown status
    pub unknown_count: usize,
    /// Total number of components
    pub total_count: usize,
    /// Health percentage (0.0 to 1.0)
    pub health_percentage: f64,
    /// Timestamp of aggregation
    pub timestamp: Timestamp,
    /// Details about unhealthy components
    pub unhealthy_components: Vec<String>,
    /// Details about unknown components
    pub unknown_components: Vec<String>,
}

impl AggregationResult {
    /// Create aggregation result from components
    pub fn from_components(components: &[ComponentHealth]) -> Self {
        let total_count = components.len();
        let mut healthy_count = 0;
        let mut unhealthy_count = 0;
        let mut unknown_count = 0;
        let mut unhealthy_components = Vec::new();
        let mut unknown_components = Vec::new();

        for component in components {
            match component.status {
                HealthStatus::Healthy => healthy_count += 1,
                HealthStatus::Unhealthy => {
                    unhealthy_count += 1;
                    unhealthy_components.push(component.id.clone());
                }
                HealthStatus::Unknown => {
                    unknown_count += 1;
                    unknown_components.push(component.id.clone());
                }
            }
        }

        let health_percentage = if total_count > 0 {
            (healthy_count as f64) / (total_count as f64)
        } else {
            0.0
        };

        // Determine overall status
        let status = if unhealthy_count > 0 {
            HealthStatus::Unhealthy
        } else if unknown_count > 0 {
            HealthStatus::Unknown
        } else {
            HealthStatus::Healthy
        };

        AggregationResult {
            status,
            healthy_count,
            unhealthy_count,
            unknown_count,
            total_count,
            health_percentage,
            timestamp: Timestamp::now(),
            unhealthy_components,
            unknown_components,
        }
    }
}

/// Dependency graph for components
#[derive(Debug)]
pub struct DependencyGraph {
    /// Adjacency list: component -> dependencies
    dependencies: BTreeMap<String, BTreeSet<String>>,
    /// Reverse adjacency list: component -> dependents
    reverse_dependencies: BTreeMap<String, BTreeSet<String>>,
}

impl DependencyGraph {
    /// Create new dependency graph
    pub fn new() -> Self {
        DependencyGraph {
            dependencies: BTreeMap::new(),
            reverse_dependencies: BTreeMap::new(),
        }
    }

    /// Add dependency
    pub fn add_dependency(&mut self, component: String, depends_on: String) {
        self.dependencies
            .entry(component.clone())
            .or_insert_with(BTreeSet::new)
            .insert(depends_on.clone());

        self.reverse_dependencies
            .entry(depends_on)
            .or_insert_with(BTreeSet::new)
            .insert(component);
    }

    /// Get components that would be affected by a component failure
    pub fn affected_components(&self, component: &str) -> Vec<String> {
        let mut affected = Vec::new();
        let mut to_visit = Vec::new();

        if let Some(transitive) = self.reverse_dependencies.get(component) {
            for dependent in transitive {
                to_visit.push(dependent.clone());
            }
        }

        while let Some(current) = to_visit.pop() {
            if !affected.contains(&current) {
                affected.push(current.clone());

                if let Some(transitive) = self.reverse_dependencies.get(&current) {
                    for dependent in transitive {
                        to_visit.push(dependent.clone());
                    }
                }
            }
        }

        affected
    }

    /// Check if there are circular dependencies
    pub fn has_cycles(&self) -> bool {
        // Use DFS to detect cycles
        let mut visited = BTreeSet::new();
        let mut rec_stack = BTreeSet::new();

        for node in self.dependencies.keys() {
            if self.has_cycle_dfs(node, &mut visited, &mut rec_stack) {
                return true;
            }
        }

        false
    }

    /// DFS helper for cycle detection
    fn has_cycle_dfs(
        &self,
        node: &str,
        visited: &mut BTreeSet<String>,
        rec_stack: &mut BTreeSet<String>,
    ) -> bool {
        if rec_stack.contains(node) {
            return true;
        }

        if visited.contains(node) {
            return false;
        }

        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(deps) = self.dependencies.get(node) {
            for dep in deps {
                if self.has_cycle_dfs(dep, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    /// Get topological order (or None if cycles exist)
    pub fn topological_order(&self) -> Option<Vec<String>> {
        if self.has_cycles() {
            return None;
        }

        let mut order = Vec::new();
        let mut visited = BTreeSet::new();

        for node in self.dependencies.keys() {
            self.topological_dfs(node, &mut visited, &mut order);
        }

        Some(order)
    }

    /// DFS helper for topological sort
    fn topological_dfs(&self, node: &str, visited: &mut BTreeSet<String>, order: &mut Vec<String>) {
        if visited.contains(node) {
            return;
        }

        visited.insert(node.to_string());

        if let Some(deps) = self.dependencies.get(node) {
            for dep in deps {
                self.topological_dfs(dep, visited, order);
            }
        }

        order.push(node.to_string());
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Health aggregator
#[derive(Debug)]
pub struct HealthAggregator {
    /// Component health states
    components: RwLock<BTreeMap<String, ComponentHealth>>,
    /// Dependency graph
    dependencies: RwLock<DependencyGraph>,
    /// Aggregation strategy
    strategy: AggregationStrategy,
    /// Cascading failure detection enabled
    cascading_failure: bool,
}

impl HealthAggregator {
    /// Create new health aggregator
    pub fn new(strategy: AggregationStrategy) -> Self {
        HealthAggregator {
            components: RwLock::new(BTreeMap::new()),
            dependencies: RwLock::new(DependencyGraph::new()),
            strategy,
            cascading_failure: true,
        }
    }

    /// Add or update component
    pub fn set_component(&self, component: ComponentHealth) {
        let id = component.id.clone();
        let deps = component.dependencies.clone();

        let mut components = self.components.write();
        components.insert(id.clone(), component);

        // Update dependency graph
        let mut dependencies = self.dependencies.write();
        for dep in &deps {
            dependencies.add_dependency(id.clone(), dep.clone());
        }
    }

    /// Remove component
    pub fn remove_component(&self, id: &str) -> bool {
        let mut components = self.components.write();
        components.remove(id).is_some()
    }

    /// Get component health
    pub fn get_component(&self, id: &str) -> Option<ComponentHealth> {
        let components = self.components.read();
        components.get(id).cloned()
    }

    /// Update component status
    pub fn update_status(&self, id: &str, status: HealthStatus) {
        let mut components = self.components.write();
        if let Some(component) = components.get_mut(id) {
            component.status = status;
            component.last_check = Timestamp::now();
        }
    }

    /// Set aggregation strategy
    pub fn set_strategy(&mut self, strategy: AggregationStrategy) {
        self.strategy = strategy;
    }

    /// Enable/disable cascading failure detection
    pub fn set_cascading_failure(&mut self, enabled: bool) {
        self.cascading_failure = enabled;
    }

    /// Aggregate health status
    pub fn aggregate(&self) -> AggregationResult {
        let components = self.components.read();
        let component_list: Vec<ComponentHealth> = components.values().cloned().collect();

        let result = match self.strategy {
            AggregationStrategy::All => self.aggregate_all(&component_list),
            AggregationStrategy::Any => self.aggregate_any(&component_list),
            AggregationStrategy::Majority => self.aggregate_majority(&component_list),
            AggregationStrategy::Weighted => self.aggregate_weighted(&component_list),
            AggregationStrategy::Custom => self.aggregate_custom(&component_list),
        };

        // Apply cascading failure detection if enabled
        if self.cascading_failure {
            self.apply_cascading_failure(result)
        } else {
            result
        }
    }

    /// All strategy: all components must be healthy
    fn aggregate_all(&self, components: &[ComponentHealth]) -> AggregationResult {
        let mut result = AggregationResult::from_components(components);

        if result.unhealthy_count > 0 {
            result.status = HealthStatus::Unhealthy;
        } else if result.unknown_count > 0 {
            result.status = HealthStatus::Unknown;
        } else {
            result.status = HealthStatus::Healthy;
        }

        result
    }

    /// Any strategy: at least one component must be healthy
    fn aggregate_any(&self, components: &[ComponentHealth]) -> AggregationResult {
        let mut result = AggregationResult::from_components(components);

        if result.healthy_count > 0 {
            result.status = HealthStatus::Healthy;
        } else if result.unknown_count > 0 {
            result.status = HealthStatus::Unknown;
        } else {
            result.status = HealthStatus::Unhealthy;
        }

        result
    }

    /// Majority strategy: most components must be healthy
    fn aggregate_majority(&self, components: &[ComponentHealth]) -> AggregationResult {
        let mut result = AggregationResult::from_components(components);

        let healthy_threshold = (components.len() as f64) / 2.0;

        if result.healthy_count as f64 > healthy_threshold {
            result.status = HealthStatus::Healthy;
        } else if result.unhealthy_count as f64 > healthy_threshold {
            result.status = HealthStatus::Unhealthy;
        } else {
            result.status = HealthStatus::Unknown;
        }

        result
    }

    /// Weighted strategy: weighted sum of component health
    fn aggregate_weighted(&self, components: &[ComponentHealth]) -> AggregationResult {
        let mut result = AggregationResult::from_components(components);

        let total_weight: f64 = components.iter().map(|c| c.weight).sum();
        let healthy_weight: f64 = components
            .iter()
            .filter(|c| c.is_healthy())
            .map(|c| c.weight)
            .sum();

        let weight_percentage = if total_weight > 0.0 {
            healthy_weight / total_weight
        } else {
            0.0
        };

        if weight_percentage >= 0.8 {
            result.status = HealthStatus::Healthy;
        } else if weight_percentage >= 0.5 {
            result.status = HealthStatus::Unknown;
        } else {
            result.status = HealthStatus::Unhealthy;
        }

        result
    }

    /// Custom aggregation logic
    fn aggregate_custom(&self, components: &[ComponentHealth]) -> AggregationResult {
        let mut result = AggregationResult::from_components(components);

        // Check if any critical components are unhealthy
        let critical_unhealthy = components
            .iter()
            .any(|c| c.critical && !c.is_healthy());

        if critical_unhealthy {
            result.status = HealthStatus::Unhealthy;
        } else if result.health_percentage >= 0.8 {
            result.status = HealthStatus::Healthy;
        } else if result.health_percentage >= 0.5 {
            result.status = HealthStatus::Unknown;
        } else {
            result.status = HealthStatus::Unhealthy;
        }

        result
    }

    /// Apply cascading failure detection
    fn apply_cascading_failure(&self, mut result: AggregationResult) -> AggregationResult {
        let components = self.components.read();
        let dependencies = self.dependencies.read();

        // Find all unhealthy components
        let unhealthy: Vec<String> = components
            .iter()
            .filter(|(_, c)| !c.is_healthy())
            .map(|(id, _)| id.clone())
            .collect();

        // Find components affected by unhealthy ones
        let mut affected = Vec::new();
        for unhealthy_id in &unhealthy {
            let mut affected_by_this = dependencies.affected_components(unhealthy_id);
            affected.append(&mut affected_by_this);
        }

        // Mark affected components as unhealthy
        for affected_id in &affected {
            if let Some(component) = components.get(affected_id.as_str()) {
                // Only mark if not already unhealthy
                if component.is_healthy() {
                    result.unhealthy_components.push(affected_id.clone());
                    result.unhealthy_count += 1;
                    result.healthy_count -= 1;
                }
            }
        }

        // Recalculate status if any components were affected
        if !affected.is_empty() {
            result.health_percentage = if result.total_count > 0 {
                (result.healthy_count as f64) / (result.total_count as f64)
            } else {
                0.0
            };

            if result.unhealthy_count > 0 {
                result.status = HealthStatus::Unhealthy;
            }
        }

        result
    }

    /// Get all component IDs
    pub fn component_ids(&self) -> Vec<String> {
        let components = self.components.read();
        components.keys().cloned().collect()
    }

    /// Get component count
    pub fn component_count(&self) -> usize {
        let components = self.components.read();
        components.len()
    }

    /// Check for circular dependencies
    pub fn has_circular_dependencies(&self) -> bool {
        let dependencies = self.dependencies.read();
        dependencies.has_cycles()
    }

    /// Get dependency order (topological sort)
    pub fn dependency_order(&self) -> Option<Vec<String>> {
        let dependencies = self.dependencies.read();
        dependencies.topological_order()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregation_strategy_names() {
        assert_eq!(AggregationStrategy::All.name(), "all");
        assert_eq!(AggregationStrategy::Any.name(), "any");
        assert_eq!(AggregationStrategy::Majority.name(), "majority");
        assert_eq!(AggregationStrategy::Weighted.name(), "weighted");
        assert_eq!(AggregationStrategy::Custom.name(), "custom");
    }

    #[test]
    fn test_component_health() {
        let component = ComponentHealth::new("test".to_string())
            .with_status(HealthStatus::Healthy)
            .with_weight(2.0)
            .with_critical(true);

        assert_eq!(component.id, "test");
        assert!(component.is_healthy());
        assert_eq!(component.weight, 2.0);
        assert!(component.critical);
    }

    #[test]
    fn test_aggregation_result() {
        let components = vec![
            ComponentHealth::new("c1".to_string()).with_status(HealthStatus::Healthy),
            ComponentHealth::new("c2".to_string()).with_status(HealthStatus::Unhealthy),
            ComponentHealth::new("c3".to_string()).with_status(HealthStatus::Unknown),
        ];

        let result = AggregationResult::from_components(&components);

        assert_eq!(result.total_count, 3);
        assert_eq!(result.healthy_count, 1);
        assert_eq!(result.unhealthy_count, 1);
        assert_eq!(result.unknown_count, 1);
        assert_eq!(result.health_percentage, 1.0 / 3.0);
        assert_eq!(result.status, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();

        graph.add_dependency("c1".to_string(), "c2".to_string());
        graph.add_dependency("c2".to_string(), "c3".to_string());

        let affected = graph.affected_components("c3");
        assert!(affected.contains(&"c2".to_string()));
        assert!(affected.contains(&"c1".to_string()));
    }

    #[test]
    fn test_dependency_graph_cycles() {
        let mut graph = DependencyGraph::new();

        graph.add_dependency("c1".to_string(), "c2".to_string());
        graph.add_dependency("c2".to_string(), "c1".to_string());

        assert!(graph.has_cycles());
    }

    #[test]
    fn test_dependency_graph_topological() {
        let mut graph = DependencyGraph::new();

        graph.add_dependency("c1".to_string(), "c2".to_string());
        graph.add_dependency("c2".to_string(), "c3".to_string());

        let order = graph.topological_order();
        assert!(order.is_some());

        let order = order.unwrap();
        // c3 should come before c2, and c2 before c1
        let pos_c1 = order.iter().position(|x| x == "c1").unwrap();
        let pos_c2 = order.iter().position(|x| x == "c2").unwrap();
        let pos_c3 = order.iter().position(|x| x == "c3").unwrap();

        assert!(pos_c3 < pos_c2);
        assert!(pos_c2 < pos_c1);
    }

    #[test]
    fn test_health_aggregator_all_strategy() {
        let aggregator = HealthAggregator::new(AggregationStrategy::All);

        aggregator.set_component(
            ComponentHealth::new("c1".to_string()).with_status(HealthStatus::Healthy),
        );
        aggregator.set_component(
            ComponentHealth::new("c2".to_string()).with_status(HealthStatus::Healthy),
        );

        let result = aggregator.aggregate();
        assert_eq!(result.status, HealthStatus::Healthy);
        assert_eq!(result.healthy_count, 2);

        // Add unhealthy component
        aggregator.set_component(
            ComponentHealth::new("c3".to_string()).with_status(HealthStatus::Unhealthy),
        );

        let result = aggregator.aggregate();
        assert_eq!(result.status, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_health_aggregator_any_strategy() {
        let aggregator = HealthAggregator::new(AggregationStrategy::Any);

        aggregator.set_component(
            ComponentHealth::new("c1".to_string()).with_status(HealthStatus::Unhealthy),
        );
        aggregator.set_component(
            ComponentHealth::new("c2".to_string()).with_status(HealthStatus::Healthy),
        );

        let result = aggregator.aggregate();
        assert_eq!(result.status, HealthStatus::Healthy);
    }

    #[test]
    fn test_health_aggregator_majority_strategy() {
        let aggregator = HealthAggregator::new(AggregationStrategy::Majority);

        // 2 healthy, 1 unhealthy = majority healthy
        aggregator.set_component(
            ComponentHealth::new("c1".to_string()).with_status(HealthStatus::Healthy),
        );
        aggregator.set_component(
            ComponentHealth::new("c2".to_string()).with_status(HealthStatus::Healthy),
        );
        aggregator.set_component(
            ComponentHealth::new("c3".to_string()).with_status(HealthStatus::Unhealthy),
        );

        let result = aggregator.aggregate();
        assert_eq!(result.status, HealthStatus::Healthy);
    }

    #[test]
    fn test_health_aggregator_weighted_strategy() {
        let aggregator = HealthAggregator::new(AggregationStrategy::Weighted);

        // Heavy component healthy, light component unhealthy
        aggregator.set_component(
            ComponentHealth::new("c1".to_string())
                .with_status(HealthStatus::Healthy)
                .with_weight(10.0),
        );
        aggregator.set_component(
            ComponentHealth::new("c2".to_string())
                .with_status(HealthStatus::Unhealthy)
                .with_weight(1.0),
        );

        let result = aggregator.aggregate();
        // 10/11 = 91% > 80% threshold
        assert_eq!(result.status, HealthStatus::Healthy);
    }

    #[test]
    fn test_health_aggregator_custom_strategy() {
        let aggregator = HealthAggregator::new(AggregationStrategy::Custom);

        // Critical component unhealthy
        aggregator.set_component(
            ComponentHealth::new("c1".to_string())
                .with_status(HealthStatus::Healthy)
                .with_critical(false),
        );
        aggregator.set_component(
            ComponentHealth::new("c2".to_string())
                .with_status(HealthStatus::Unhealthy)
                .with_critical(true),
        );

        let result = aggregator.aggregate();
        // Should be unhealthy because critical component is down
        assert_eq!(result.status, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_health_aggregator_cascading_failure() {
        let aggregator = HealthAggregator::new(AggregationStrategy::All);

        // c1 depends on c2, c2 depends on c3
        aggregator.set_component(
            ComponentHealth::new("c1".to_string())
                .with_status(HealthStatus::Healthy)
                .with_dependency("c2".to_string()),
        );
        aggregator.set_component(
            ComponentHealth::new("c2".to_string())
                .with_status(HealthStatus::Healthy)
                .with_dependency("c3".to_string()),
        );
        aggregator.set_component(
            ComponentHealth::new("c3".to_string()).with_status(HealthStatus::Healthy),
        );

        let result = aggregator.aggregate();
        assert_eq!(result.status, HealthStatus::Healthy);

        // c3 becomes unhealthy
        aggregator.update_status("c3", HealthStatus::Unhealthy);

        let result = aggregator.aggregate();
        // Cascading failure should affect c1 and c2
        assert_eq!(result.status, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_health_aggregator_operations() {
        let aggregator = HealthAggregator::new(AggregationStrategy::All);

        aggregator.set_component(
            ComponentHealth::new("c1".to_string()).with_status(HealthStatus::Healthy),
        );

        assert_eq!(aggregator.component_count(), 1);
        assert!(aggregator.get_component("c1").is_some());
        assert!(aggregator.get_component("c2").is_none());

        aggregator.update_status("c1", HealthStatus::Unknown);
        assert_eq!(
            aggregator.get_component("c1").unwrap().status,
            HealthStatus::Unknown
        );

        assert!(aggregator.remove_component("c1"));
        assert_eq!(aggregator.component_count(), 0);
    }
}
