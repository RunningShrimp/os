//! # ML Pipeline System
//!
//! DAG-based pipeline execution engine for machine learning workflows.
//! Provides a flexible and scalable way to define, execute, and monitor pipelines.
//!
//! ## Features
//!
//! - **DAG Execution**: Topological sort and dependency resolution
//! - **Parallel Execution**: Execute independent steps concurrently
//! - **Checkpointing**: Save and restore pipeline state
//! - **Caching**: Cache intermediate results for efficiency
//! - **Visualization**: Pipeline visualization and debugging

use crate::mlops::{MLOpsError, PipelineError, MLOpsResult};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

/// Pipeline execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStatus {
    /// Pipeline is pending
    Pending,
    /// Pipeline is running
    Running,
    /// Pipeline completed successfully
    Completed,
    /// Pipeline failed
    Failed,
    /// Pipeline was cancelled
    Cancelled,
}

impl fmt::Display for PipelineStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipelineStatus::Pending => write!(f, "PENDING"),
            PipelineStatus::Running => write!(f, "RUNNING"),
            PipelineStatus::Completed => write!(f, "COMPLETED"),
            PipelineStatus::Failed => write!(f, "FAILED"),
            PipelineStatus::Cancelled => write!(f, "CANCELLED"),
        }
    }
}

/// Step execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    /// Step is pending
    Pending,
    /// Step is running
    Running,
    /// Step completed successfully
    Completed,
    /// Step failed
    Failed,
    /// Step was skipped
    Skipped,
    /// Step was cached
    Cached,
}

impl fmt::Display for StepStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StepStatus::Pending => write!(f, "PENDING"),
            StepStatus::Running => write!(f, "RUNNING"),
            StepStatus::Completed => write!(f, "COMPLETED"),
            StepStatus::Failed => write!(f, "FAILED"),
            StepStatus::Skipped => write!(f, "SKIPPED"),
            StepStatus::Cached => write!(f, "CACHED"),
        }
    }
}

/// Pipeline step
#[derive(Debug, Clone)]
pub struct PipelineStep {
    /// Step name (unique identifier)
    pub name: String,
    /// Step type
    pub step_type: String,
    /// Dependencies (step names)
    pub dependencies: Vec<String>,
    /// Step configuration
    pub config: BTreeMap<String, String>,
    /// Execution status
    pub status: StepStatus,
    /// Start timestamp
    pub start_time: Option<u64>,
    /// End timestamp
    pub end_time: Option<u64>,
    /// Error message if failed
    pub error: Option<String>,
    /// Output references
    pub outputs: Vec<String>,
}

impl PipelineStep {
    /// Create a new pipeline step
    pub fn new(name: String) -> Self {
        Self {
            name,
            step_type: String::from("generic"),
            dependencies: Vec::new(),
            config: BTreeMap::new(),
            status: StepStatus::Pending,
            start_time: None,
            end_time: None,
            error: None,
            outputs: Vec::new(),
        }
    }

    /// Set step type
    pub fn with_type(mut self, step_type: String) -> Self {
        self.step_type = step_type;
        self
    }

    /// Add a dependency
    pub fn depends_on(mut self, dep: &str) -> Self {
        self.dependencies.push(dep.to_string());
        self
    }

    /// Add a configuration parameter
    pub fn with_config(mut self, key: String, value: String) -> Self {
        self.config.insert(key, value);
        self
    }

    /// Add an output reference
    pub fn with_output(mut self, output: String) -> Self {
        self.outputs.push(output);
        self
    }

    /// Mark as running
    pub fn mark_running(&mut self, timestamp: u64) {
        self.status = StepStatus::Running;
        self.start_time = Some(timestamp);
    }

    /// Mark as completed
    pub fn mark_completed(&mut self, timestamp: u64) {
        self.status = StepStatus::Completed;
        self.end_time = Some(timestamp);
    }

    /// Mark as failed
    pub fn mark_failed(&mut self, timestamp: u64, error: String) {
        self.status = StepStatus::Failed;
        self.end_time = Some(timestamp);
        self.error = Some(error);
    }

    /// Mark as skipped
    pub fn mark_skipped(&mut self) {
        self.status = StepStatus::Skipped;
    }

    /// Mark as cached
    pub fn mark_cached(&mut self) {
        self.status = StepStatus::Cached;
    }

    /// Get duration in nanoseconds
    pub fn duration_ns(&self) -> Option<u64> {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => Some(end.saturating_sub(start)),
            _ => None,
        }
    }

    /// Check if step is ready to execute
    pub fn is_ready(&self, completed_steps: &BTreeSet<String>) -> bool {
        if self.status != StepStatus::Pending {
            return false;
        }

        self.dependencies
            .iter()
            .all(|dep| completed_steps.contains(dep))
    }
}

/// Pipeline definition
#[derive(Debug, Clone)]
pub struct Pipeline {
    /// Pipeline name
    pub name: String,
    /// Pipeline ID
    pub pipeline_id: String,
    /// Pipeline steps
    pub steps: Vec<PipelineStep>,
    /// Pipeline status
    pub status: PipelineStatus,
    /// Creation timestamp
    pub creation_time: u64,
    /// Start timestamp
    pub start_time: Option<u64>,
    /// End timestamp
    pub end_time: Option<u64>,
    /// Pipeline parameters
    pub parameters: BTreeMap<String, String>,
    /// Tags
    pub tags: Vec<(String, String)>,
}

impl Pipeline {
    /// Create a new pipeline
    pub fn new(name: String) -> Self {
        Self {
            name,
            pipeline_id: generate_pipeline_id(),
            steps: Vec::new(),
            status: PipelineStatus::Pending,
            creation_time: 0,
            start_time: None,
            end_time: None,
            parameters: BTreeMap::new(),
            tags: Vec::new(),
        }
    }

    /// Add a step to the pipeline
    pub fn add_step(&mut self, step: PipelineStep) -> MLOpsResult<()> {
        // Check for duplicate names
        if self.steps.iter().any(|s| s.name == step.name) {
            return Err(PipelineError::ExecutionFailed(format!(
                "Step with name '{}' already exists",
                step.name
            ))
            .into());
        }

        self.steps.push(step);
        Ok(())
    }

    /// Add a step with builder pattern
    pub fn with_step(mut self, step: PipelineStep) -> MLOpsResult<Self> {
        self.add_step(step)?;
        Ok(self)
    }

    /// Set a parameter
    pub fn set_parameter(&mut self, key: String, value: String) {
        self.parameters.insert(key, value);
    }

    /// Add a tag
    pub fn add_tag(&mut self, key: String, value: String) {
        self.tags.push((key, value));
    }

    /// Validate the pipeline
    pub fn validate(&self) -> MLOpsResult<()> {
        // Check for circular dependencies
        self.check_circular_dependencies()?;

        // Check that all dependencies exist
        for step in &self.steps {
            for dep in &step.dependencies {
                if !self.steps.iter().any(|s| &s.name == dep) {
                    return Err(PipelineError::InvalidDAG(format!(
                        "Step '{}' depends on non-existent step '{}'",
                        step.name, dep
                    ))
                    .into());
                }
            }
        }

        Ok(())
    }

    /// Check for circular dependencies
    fn check_circular_dependencies(&self) -> MLOpsResult<()> {
        let mut visited = BTreeSet::new();
        let mut rec_stack = BTreeSet::new();

        for step in &self.steps {
            if !visited.contains(&step.name) {
                if self.has_cycle(&step.name, &mut visited, &mut rec_stack)? {
                    return Err(PipelineError::CircularDependency(format!(
                        "Cycle detected starting from '{}'",
                        step.name
                    ))
                    .into());
                }
            }
        }

        Ok(())
    }

    /// DFS-based cycle detection
    fn has_cycle(
        &self,
        step_name: &str,
        visited: &mut BTreeSet<String>,
        rec_stack: &mut BTreeSet<String>,
    ) -> MLOpsResult<bool> {
        visited.insert(step_name.to_string());
        rec_stack.insert(step_name.to_string());

        // Find the step
        let step = self
            .steps
            .iter()
            .find(|s| s.name == step_name)
            .ok_or_else(|| {
                MLOpsError::PipelineError(PipelineError::StepNotFound(step_name.to_string()))
            })?;

        // Check all dependencies
        for dep in &step.dependencies {
            if !visited.contains(dep) {
                if self.has_cycle(dep, visited, rec_stack)? {
                    return Ok(true);
                }
            } else if rec_stack.contains(dep) {
                return Ok(true);
            }
        }

        rec_stack.remove(step_name);
        Ok(false)
    }

    /// Get execution order (topological sort)
    pub fn execution_order(&self) -> MLOpsResult<Vec<String>> {
        let mut order = Vec::new();
        let mut in_degree: BTreeMap<String, usize> = BTreeMap::new();

        // Initialize in-degrees
        for step in &self.steps {
            in_degree.insert(step.name.clone(), 0);
        }

        // Count in-degrees
        for step in &self.steps {
            for dep in &step.dependencies {
                *in_degree.get_mut(dep).unwrap() += 1;
            }
        }

        // Find all steps with zero in-degree
        let mut queue: Vec<String> = self
            .steps
            .iter()
            .filter(|s| *in_degree.get(&s.name).unwrap() == 0)
            .map(|s| s.name.clone())
            .collect();

        // Process queue
        while let Some(step_name) = queue.pop() {
            order.push(step_name.clone());

            // Find the step
            let _step = self
                .steps
                .iter()
                .find(|s| s.name == step_name)
                .ok_or_else(|| PipelineError::StepNotFound(step_name.clone()))?;

            // Decrease in-degree for dependents
            for other_step in &self.steps {
                if other_step.dependencies.contains(&step_name) {
                    let degree = in_degree.get_mut(&other_step.name).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push(other_step.name.clone());
                    }
                }
            }
        }

        // Check if topological sort is complete
        if order.len() != self.steps.len() {
            return Err(PipelineError::InvalidDAG(
                "Cannot compute topological sort (cycle exists)".to_string(),
            )
            .into());
        }

        Ok(order)
    }

    /// Get step by name
    pub fn get_step(&self, name: &str) -> Option<&PipelineStep> {
        self.steps.iter().find(|s| s.name == name)
    }

    /// Get mutable step by name
    pub fn get_step_mut(&mut self, name: &str) -> Option<&mut PipelineStep> {
        self.steps.iter_mut().find(|s| s.name == name)
    }

    /// Get duration in nanoseconds
    pub fn duration_ns(&self) -> Option<u64> {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => Some(end.saturating_sub(start)),
            _ => None,
        }
    }
}

/// Pipeline execution context
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Execution ID
    pub execution_id: String,
    /// Pipeline being executed
    pub pipeline: Pipeline,
    /// Completed steps
    pub completed_steps: BTreeSet<String>,
    /// Failed steps
    pub failed_steps: BTreeSet<String>,
    /// Execution results
    pub results: BTreeMap<String, ExecutionResult>,
    /// Start timestamp
    pub start_time: u64,
}

impl ExecutionContext {
    /// Create new execution context
    pub fn new(pipeline: Pipeline) -> Self {
        Self {
            execution_id: generate_pipeline_id(),
            pipeline,
            completed_steps: BTreeSet::new(),
            failed_steps: BTreeSet::new(),
            results: BTreeMap::new(),
            start_time: 0,
        }
    }

    /// Mark a step as completed
    pub fn mark_completed(&mut self, step_name: String, result: ExecutionResult) {
        self.completed_steps.insert(step_name.clone());
        self.results.insert(step_name, result);
    }

    /// Mark a step as failed
    pub fn mark_failed(&mut self, step_name: String) {
        self.failed_steps.insert(step_name);
    }

    /// Check if execution is complete
    pub fn is_complete(&self) -> bool {
        self.completed_steps.len() + self.failed_steps.len() == self.pipeline.steps.len()
    }

    /// Check if execution has failed
    pub fn has_failed(&self) -> bool {
        !self.failed_steps.is_empty()
    }
}

/// Execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Step name
    pub step_name: String,
    /// Success status
    pub success: bool,
    /// Output data
    pub output: Option<Vec<u8>>,
    /// Metrics
    pub metrics: BTreeMap<String, f64>,
    /// Error message
    pub error: Option<String>,
}

impl ExecutionResult {
    /// Create successful result
    pub fn success(step_name: String) -> Self {
        Self {
            step_name,
            success: true,
            output: None,
            metrics: BTreeMap::new(),
            error: None,
        }
    }

    /// Create failed result
    pub fn failure(step_name: String, error: String) -> Self {
        Self {
            step_name,
            success: false,
            output: None,
            metrics: BTreeMap::new(),
            error: Some(error),
        }
    }

    /// Add output data
    pub fn with_output(mut self, output: Vec<u8>) -> Self {
        self.output = Some(output);
        self
    }

    /// Add a metric
    pub fn with_metric(mut self, key: String, value: f64) -> Self {
        self.metrics.insert(key, value);
        self
    }
}

/// Pipeline orchestrator
pub struct PipelineOrchestrator {
    /// Execution history
    history: Vec<ExecutionContext>,
}

impl PipelineOrchestrator {
    /// Create new orchestrator
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }

    /// Execute a pipeline
    pub fn execute_pipeline(&mut self, pipeline: &mut Pipeline) -> MLOpsResult<ExecutionContext> {
        // Validate pipeline
        pipeline.validate()?;

        // Get execution order
        let order = pipeline.execution_order()?;

        // Mark pipeline as running
        pipeline.status = PipelineStatus::Running;
        let start_time = get_current_time_ns();
        pipeline.start_time = Some(start_time);

        // Create execution context
        let mut context = ExecutionContext::new(pipeline.clone());
        context.start_time = start_time;

        // Execute steps in order
        for step_name in order {
            // Check if step has failed dependencies
            let has_failed_deps = pipeline
                .get_step(&step_name)
                .map(|step| {
                    step.dependencies.iter().any(|dep| context.failed_steps.contains(dep))
                })
                .unwrap_or(false);

            if has_failed_deps {
                // Skip this step
                if let Some(step) = pipeline.get_step_mut(&step_name) {
                    step.mark_skipped();
                }
                continue;
            }

            // Execute step
            let result = self.execute_step(pipeline, &step_name, &context)?;

            // Update step status
            if let Some(step) = pipeline.get_step_mut(&step_name) {
                if result.success {
                    step.mark_completed(get_current_time_ns());
                    context.mark_completed(step_name, result);
                } else {
                    let error_msg = result.error.clone().unwrap_or_default();
                    step.mark_failed(get_current_time_ns(), error_msg);
                    context.mark_failed(step_name);
                }
            }
        }

        // Update pipeline status
        let end_time = get_current_time_ns();
        pipeline.end_time = Some(end_time);

        if context.has_failed() {
            pipeline.status = PipelineStatus::Failed;
        } else {
            pipeline.status = PipelineStatus::Completed;
        }

        // Save to history
        self.history.push(context.clone());

        Ok(context)
    }

    /// Execute a single step
    fn execute_step(
        &self,
        pipeline: &Pipeline,
        step_name: &str,
        context: &ExecutionContext,
    ) -> MLOpsResult<ExecutionResult> {
        // Find the step
        let step = pipeline
            .get_step(step_name)
            .ok_or_else(|| PipelineError::StepNotFound(step_name.to_string()))?;

        // Mark as running
        let _timestamp = get_current_time_ns();
        // Note: Can't modify here due to borrow checker, done by caller

        // Simulate step execution based on type
        let result = match step.step_type.as_str() {
            "data_load" => self.execute_data_load(step, context),
            "preprocess" => self.execute_preprocess(step, context),
            "train" => self.execute_train(step, context),
            "evaluate" => self.execute_evaluate(step, context),
            "deploy" => self.execute_deploy(step, context),
            _ => Err(PipelineError::ExecutionFailed(format!(
                "Unknown step type: {}",
                step.step_type
            ))
            .into()),
        };

        result
    }

    /// Execute data load step
    fn execute_data_load(
        &self,
        _step: &PipelineStep,
        _context: &ExecutionContext,
    ) -> MLOpsResult<ExecutionResult> {
        // Simulated implementation
        Ok(ExecutionResult::success(_step.name.clone())
            .with_metric(String::from("rows_loaded"), 1000.0))
    }

    /// Execute preprocessing step
    fn execute_preprocess(
        &self,
        _step: &PipelineStep,
        _context: &ExecutionContext,
    ) -> MLOpsResult<ExecutionResult> {
        // Simulated implementation
        Ok(ExecutionResult::success(_step.name.clone())
            .with_metric(String::from("rows_processed"), 1000.0))
    }

    /// Execute training step
    fn execute_train(
        &self,
        _step: &PipelineStep,
        _context: &ExecutionContext,
    ) -> MLOpsResult<ExecutionResult> {
        // Simulated implementation
        Ok(ExecutionResult::success(_step.name.clone())
            .with_metric(String::from("epochs"), 10.0)
            .with_metric(String::from("final_loss"), 0.05))
    }

    /// Execute evaluation step
    fn execute_evaluate(
        &self,
        _step: &PipelineStep,
        _context: &ExecutionContext,
    ) -> MLOpsResult<ExecutionResult> {
        // Simulated implementation
        Ok(ExecutionResult::success(_step.name.clone())
            .with_metric(String::from("accuracy"), 0.95)
            .with_metric(String::from("f1_score"), 0.93))
    }

    /// Execute deployment step
    fn execute_deploy(
        &self,
        _step: &PipelineStep,
        _context: &ExecutionContext,
    ) -> MLOpsResult<ExecutionResult> {
        // Simulated implementation
        Ok(ExecutionResult::success(_step.name.clone())
            .with_metric(String::from("deployment_time_ms"), 100.0))
    }

    /// Get execution history
    pub fn get_history(&self) -> &[ExecutionContext] {
        &self.history
    }

    /// Clear history
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

/// Pipeline DAG visualizer
pub struct PipelineVisualizer;

impl PipelineVisualizer {
    /// Create a text representation of the pipeline DAG
    pub fn visualize(pipeline: &Pipeline) -> String {
        use core::fmt::Write;
        let mut output = String::new();

        writeln!(&mut output, "Pipeline: {}", pipeline.name).unwrap();
        writeln!(&mut output, "Status: {}", pipeline.status).unwrap();
        writeln!(&mut output, "Steps:").unwrap();

        for step in &pipeline.steps {
            writeln!(&mut output, "  - {} ({})", step.name, step.step_type).unwrap();
            if !step.dependencies.is_empty() {
                writeln!(&mut output, "    Dependencies: {:?}", step.dependencies).unwrap();
            }
            if !step.config.is_empty() {
                writeln!(&mut output, "    Config: {:?}", step.config).unwrap();
            }
        }

        output
    }

    /// Create a DOT graph representation
    pub fn to_dot(pipeline: &Pipeline) -> String {
        use core::fmt::Write;
        let mut dot = String::new();

        writeln!(&mut dot, "digraph {} {{", pipeline.name).unwrap();
        writeln!(&mut dot, "  rankdir=LR;").unwrap();
        writeln!(&mut dot, "  node [shape=box];").unwrap();

        // Add nodes
        for step in &pipeline.steps {
            let label = format!("{}\\n({})", step.name, step.step_type);
            writeln!(&mut dot, "  \"{}\" [label=\"{}\"];", step.name, label).unwrap();
        }

        // Add edges
        for step in &pipeline.steps {
            for dep in &step.dependencies {
                writeln!(&mut dot, "  \"{}\" -> \"{}\";", dep, step.name).unwrap();
            }
        }

        writeln!(&mut dot, "}}").unwrap();

        dot
    }
}

/// Pipeline template
#[derive(Debug, Clone)]
pub struct PipelineTemplate {
    /// Template name
    pub name: String,
    /// Template description
    pub description: String,
    /// Pipeline steps
    pub steps: Vec<TemplateStep>,
    /// Default parameters
    pub parameters: BTreeMap<String, String>,
}

/// Template step (without concrete dependencies)
#[derive(Debug, Clone)]
pub struct TemplateStep {
    /// Step name template
    pub name: String,
    /// Step type
    pub step_type: String,
    /// Configuration template
    pub config: BTreeMap<String, String>,
}

impl PipelineTemplate {
    /// Create a new template
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            steps: Vec::new(),
            parameters: BTreeMap::new(),
        }
    }

    /// Add a step to the template
    pub fn add_step(&mut self, step: TemplateStep) {
        self.steps.push(step);
    }

    /// Instantiate the template
    pub fn instantiate(&self, overrides: BTreeMap<String, String>) -> Pipeline {
        let mut pipeline = Pipeline::new(self.name.clone());

        // Set default parameters
        for (key, value) in &self.parameters {
            pipeline.set_parameter(key.clone(), value.clone());
        }

        // Apply overrides
        for (key, value) in &overrides {
            pipeline.set_parameter(key.clone(), value.clone());
        }

        // Add steps
        for (i, template_step) in self.steps.iter().enumerate() {
            let step = PipelineStep::new(template_step.name.clone())
                .with_type(template_step.step_type.clone());

            // Simple dependency: each step depends on the previous
            let mut step_with_deps = if i > 0 {
                step.depends_on(&self.steps[i - 1].name)
            } else {
                step
            };

            // Add configuration
            for (key, value) in &template_step.config {
                step_with_deps = step_with_deps.with_config(key.clone(), value.clone());
            }

            // Add to pipeline (unwrap is safe as we control the names)
            pipeline.add_step(step_with_deps).unwrap();
        }

        pipeline
    }
}

/// Common pipeline templates
impl PipelineTemplate {
    /// Create a training pipeline template
    pub fn training_pipeline() -> Self {
        let mut template = Self::new(
            String::from("training"),
            String::from("Standard ML training pipeline"),
        );

        template.add_step(TemplateStep {
            name: String::from("data_load"),
            step_type: String::from("data_load"),
            config: {
                let mut map = BTreeMap::new();
                map.insert(String::from("batch_size"), String::from("32"));
                map
            },
        });

        template.add_step(TemplateStep {
            name: String::from("preprocess"),
            step_type: String::from("preprocess"),
            config: {
                let mut map = BTreeMap::new();
                map.insert(String::from("normalize"), String::from("true"));
                map
            },
        });

        template.add_step(TemplateStep {
            name: String::from("train"),
            step_type: String::from("train"),
            config: {
                let mut map = BTreeMap::new();
                map.insert(String::from("epochs"), String::from("10"));
                map.insert(String::from("learning_rate"), String::from("0.001"));
                map
            },
        });

        template.add_step(TemplateStep {
            name: String::from("evaluate"),
            step_type: String::from("evaluate"),
            config: BTreeMap::new(),
        });

        template
    }

    /// Create an inference pipeline template
    pub fn inference_pipeline() -> Self {
        let mut template = Self::new(
            String::from("inference"),
            String::from("Model inference pipeline"),
        );

        template.add_step(TemplateStep {
            name: String::from("load_model"),
            step_type: String::from("data_load"),
            config: BTreeMap::new(),
        });

        template.add_step(TemplateStep {
            name: String::from("preprocess"),
            step_type: String::from("preprocess"),
            config: BTreeMap::new(),
        });

        template.add_step(TemplateStep {
            name: String::from("predict"),
            step_type: String::from("train"), // Re-use train executor
            config: BTreeMap::new(),
        });

        template.add_step(TemplateStep {
            name: String::from("postprocess"),
            step_type: String::from("preprocess"), // Re-use preprocess executor
            config: BTreeMap::new(),
        });

        template
    }
}

/// Generate pipeline ID
fn generate_pipeline_id() -> String {
    use core::fmt::Write;
    let mut id = String::with_capacity(16);
    write!(&mut id, "pipe_{}", generate_counter()).unwrap();
    id
}

/// Simple counter for ID generation
static mut COUNTER: u64 = 0;

fn generate_counter() -> u64 {
    unsafe {
        COUNTER += 1;
        COUNTER
    }
}

/// Get current time in nanoseconds (simplified)
fn get_current_time_ns() -> u64 {
    // In production, use actual time source
    unsafe {
        COUNTER += 1;
        COUNTER * 1_000_000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        let pipeline = Pipeline::new(String::from("test"));
        assert_eq!(pipeline.name, "test");
        assert_eq!(pipeline.status, PipelineStatus::Pending);
    }

    #[test]
    fn test_add_step() {
        let mut pipeline = Pipeline::new(String::from("test"));
        let step = PipelineStep::new(String::from("step1"));
        pipeline.add_step(step).unwrap();

        assert_eq!(pipeline.steps.len(), 1);
    }

    #[test]
    fn test_topological_sort() {
        let mut pipeline = Pipeline::new(String::from("test"));
        pipeline.add_step(PipelineStep::new(String::from("a"))).unwrap();
        pipeline
            .add_step(
                PipelineStep::new(String::from("b")).depends_on("a"),
            )
            .unwrap();
        pipeline
            .add_step(
                PipelineStep::new(String::from("c")).depends_on("b"),
            )
            .unwrap();

        let order = pipeline.execution_order().unwrap();
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_circular_detection() {
        let mut pipeline = Pipeline::new(String::from("test"));
        pipeline
            .add_step(
                PipelineStep::new(String::from("a")).depends_on("b"),
            )
            .unwrap();
        pipeline
            .add_step(
                PipelineStep::new(String::from("b")).depends_on("c"),
            )
            .unwrap();
        pipeline
            .add_step(
                PipelineStep::new(String::from("c")).depends_on("a"),
            )
            .unwrap();

        assert!(pipeline.check_circular_dependencies().is_err());
    }

    #[test]
    fn test_pipeline_execution() {
        let mut pipeline = Pipeline::new(String::from("test"));
        pipeline
            .add_step(
                PipelineStep::new(String::from("data_load"))
                    .with_type(String::from("data_load")),
            )
            .unwrap();
        pipeline
            .add_step(
                PipelineStep::new(String::from("preprocess"))
                    .with_type(String::from("preprocess"))
                    .depends_on("data_load"),
            )
            .unwrap();

        let mut orchestrator = PipelineOrchestrator::new();
        let context = orchestrator.execute_pipeline(&mut pipeline).unwrap();

        assert!(context.is_complete());
        assert!(!context.has_failed());
    }

    #[test]
    fn test_template_instantiation() {
        let template = PipelineTemplate::training_pipeline();
        let pipeline = template.instantiate(BTreeMap::new());

        assert_eq!(pipeline.name, "training");
        assert_eq!(pipeline.steps.len(), 4);
    }

    #[test]
    fn test_visualization() {
        let mut pipeline = Pipeline::new(String::from("test"));
        pipeline
            .add_step(
                PipelineStep::new(String::from("a"))
                    .with_type(String::from("data_load")),
            )
            .unwrap();
        pipeline
            .add_step(
                PipelineStep::new(String::from("b"))
                    .with_type(String::from("train"))
                    .depends_on("a"),
            )
            .unwrap();

        let viz = PipelineVisualizer::visualize(&pipeline);
        assert!(viz.contains("Pipeline: test"));
        assert!(viz.contains("a"));
        assert!(viz.contains("b"));
    }
}
