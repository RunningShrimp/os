//! Function-as-a-Service (FaaS) Framework
//!
//! Event-driven serverless function execution with cold start optimization
//! and resource management for edge computing.

use alloc::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use core::time::Duration;

use crate::prelude::*;
use crate::container::ContainerConfig;
use crate::edge::microvm::{MicroVM, MicroVMConfig, MicroVMState};
use crate::subsystems::sync::Mutex;

/// Unique function identifier
pub type FunctionId = u64;

/// Unique instance identifier
pub type InstanceId = u64;

/// Function state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionState {
    /// Function is being created
    Creating,
    /// Function is ready
    Ready,
    /// Function is active (has instances)
    Active,
    /// Function is paused
    Paused,
    /// Function is being deleted
    Deleting,
    /// Function has failed
    Failed,
}

/// Function instance state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceState {
    /// Instance is starting
    Starting,
    /// Instance is idle (ready to invoke)
    Idle,
    /// Instance is running
    Running,
    /// Instance is terminating
    Terminating,
    /// Instance has terminated
    Terminated,
    /// Instance has failed
    Failed,
}

/// Function configuration
#[derive(Debug, Clone)]
pub struct FunctionConfig {
    /// Function ID (0 for auto-assign)
    pub function_id: FunctionId,
    /// Function name
    pub name: String,
    /// Function description
    pub description: String,
    /// Function runtime (e.g., "nodejs", "python", "rust")
    pub runtime: String,
    /// Function handler
    pub handler: String,
    /// Function code (in-memory or path)
    pub code: FunctionCode,
    /// Resource requirements
    pub resources: ResourceQuota,
    /// Environment variables
    pub environment: BTreeMap<String, String>,
    /// Timeout in seconds
    pub timeout_secs: u64,
    /// Maximum instances
    pub max_instances: usize,
    /// Minimum instances (for warm start)
    pub min_instances: usize,
    /// Enable cold start optimization
    pub cold_start_optimization: bool,
    /// Function tags
    pub tags: Vec<String>,
}

/// Function code source
#[derive(Debug, Clone)]
pub enum FunctionCode {
    /// Inline code
    Inline { code: String },
    /// Code from path
    Path { path: String },
    /// Code from container image
    ContainerImage { image: String },
    /// Code from URL
    URL { url: String },
}

/// Resource quota for functions
#[derive(Debug, Clone)]
pub struct ResourceQuota {
    /// CPU limit (milliCPUs, 1000 = 1 CPU)
    pub cpu_milli: u64,
    /// Memory limit in bytes
    pub memory_bytes: u64,
    /// Storage limit in bytes
    pub storage_bytes: u64,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Max concurrent invocations
    pub max_concurrent: usize,
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            cpu_milli: 500,    // 0.5 CPU
            memory_bytes: 128 * 1024 * 1024, // 128MB
            storage_bytes: 512 * 1024 * 1024, // 512MB
            timeout_ms: 30_000, // 30 seconds
            max_concurrent: 10,
        }
    }
}

/// Function instance
#[derive(Debug, Clone)]
pub struct FunctionInstance {
    /// Instance ID
    pub instance_id: InstanceId,
    /// Function ID
    pub function_id: FunctionId,
    /// Instance state
    pub state: InstanceState,
    /// MicroVM (if using MicroVM isolation)
    pub microvm: Option<MicroVM>,
    /// Container ID (if using container isolation)
    pub container_id: Option<u64>,
    /// Creation timestamp
    pub created_at: u64,
    /// Last invocation timestamp
    pub last_invocation: Option<u64>,
    /// Invocation count
    pub invocation_count: AtomicU64,
    /// Cold start timestamp
    pub cold_start_time: Option<u64>,
}

/// Function invocation request
#[derive(Debug, Clone)]
pub struct FunctionInvocation {
    /// Invocation ID
    pub invocation_id: u64,
    /// Function ID
    pub function_id: FunctionId,
    /// Invocation payload
    pub payload: Vec<u8>,
    /// Invocation context
    pub context: InvocationContext,
    /// Invocation timestamp
    pub timestamp: u64,
}

/// Invocation context
#[derive(Debug, Clone)]
pub struct InvocationContext {
    /// Request ID
    pub request_id: String,
    /// Client ID
    pub client_id: Option<String>,
    /// Trace ID
    pub trace_id: Option<String>,
    /// Custom metadata
    pub metadata: BTreeMap<String, String>,
}

/// Function invocation result
#[derive(Debug, Clone)]
pub struct InvocationResult {
    /// Invocation ID
    pub invocation_id: u64,
    /// Function ID
    pub function_id: FunctionId,
    /// Instance ID
    pub instance_id: InstanceId,
    /// Result payload
    pub payload: Vec<u8>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Cold start time in milliseconds
    pub cold_start_time_ms: Option<u64>,
    /// Success flag
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Memory usage in bytes
    pub memory_used: u64,
    /// CPU time in milliseconds
    pub cpu_time_ms: u64,
}

/// Function chain (workflow)
#[derive(Debug, Clone)]
pub struct FunctionChain {
    /// Chain ID
    pub chain_id: u64,
    /// Chain name
    pub name: String,
    /// Chain steps
    pub steps: Vec<ChainStep>,
    /// Chain execution policy
    pub policy: ChainExecutionPolicy,
}

/// Chain step
#[derive(Debug, Clone)]
pub struct ChainStep {
    /// Step ID
    pub step_id: u64,
    /// Function ID
    pub function_id: FunctionId,
    /// Step name
    pub name: String,
    /// Step arguments
    pub arguments: BTreeMap<String, String>,
    /// Continue on error
    pub continue_on_error: bool,
    /// Retry policy
    pub retry_policy: Option<RetryPolicy>,
}

/// Chain execution policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainExecutionPolicy {
    /// Sequential execution
    Sequential,
    /// Parallel execution
    Parallel,
    /// Custom execution
    Custom,
}

/// Retry policy
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum retry attempts
    pub max_attempts: usize,
    /// Initial delay in milliseconds
    pub initial_delay_ms: u64,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,
}

/// Autoscaling policy
#[derive(Debug, Clone)]
pub struct AutoscalingPolicy {
    /// Policy name
    pub name: String,
    /// Minimum instances
    pub min_instances: usize,
    /// Maximum instances
    pub max_instances: usize,
    /// Target CPU utilization percentage
    pub target_cpu_percent: u64,
    /// Target memory utilization percentage
    pub target_memory_percent: u64,
    /// Scale up cooldown in seconds
    pub scale_up_cooldown_secs: u64,
    /// Scale down cooldown in seconds
    pub scale_down_cooldown_secs: u64,
    /// Metrics collection interval in seconds
    pub metrics_interval_secs: u64,
}

/// Function service
pub struct FunctionService {
    /// Registered functions
    functions: Mutex<BTreeMap<FunctionId, FunctionConfig>>,
    /// Active instances
    instances: Mutex<BTreeMap<InstanceId, FunctionInstance>>,
    /// Function instances mapping (function_id -> [instance_ids])
    function_instances: Mutex<BTreeMap<FunctionId, BTreeSet<InstanceId>>>,
    /// Pending invocations
    pending_invocations: Mutex<VecDeque<FunctionInvocation>>,
    /// Active invocations
    active_invocations: Mutex<BTreeMap<u64, FunctionInvocation>>,
    /// Completed invocations
    completed_invocations: Mutex<VecDeque<InvocationResult>>,
    /// Function chains
    chains: Mutex<BTreeMap<u64, FunctionChain>>,
    /// Autoscaling policies
    autoscaling_policies: Mutex<BTreeMap<FunctionId, AutoscalingPolicy>>,
    /// Next function ID
    next_function_id: AtomicU64,
    /// Next instance ID
    next_instance_id: AtomicU64,
    /// Next invocation ID
    next_invocation_id: AtomicU64,
    /// Next chain ID
    next_chain_id: AtomicU64,
    /// Running flag
    running: AtomicBool,
}

impl FunctionService {
    /// Create a new FaaS service
    pub fn new() -> Self {
        Self {
            functions: Mutex::new(BTreeMap::new()),
            instances: Mutex::new(BTreeMap::new()),
            function_instances: Mutex::new(BTreeMap::new()),
            pending_invocations: Mutex::new(VecDeque::new()),
            active_invocations: Mutex::new(BTreeMap::new()),
            completed_invocations: Mutex::new(VecDeque::new()),
            chains: Mutex::new(BTreeMap::new()),
            autoscaling_policies: Mutex::new(BTreeMap::new()),
            next_function_id: AtomicU64::new(1),
            next_instance_id: AtomicU64::new(1),
            next_invocation_id: AtomicU64::new(1),
            next_chain_id: AtomicU64::new(1),
            running: AtomicBool::new(false),
        }
    }

    /// Deploy a new function
    pub fn deploy_function(&self, config: FunctionConfig) -> Result<FunctionId> {
        crate::println!("[faas] Deploying function: {}", config.name);

        let function_id = if config.function_id == 0 {
            self.next_function_id.fetch_add(1, Ordering::SeqCst)
        } else {
            config.function_id
        };

        let mut function_config = config.clone();
        function_config.function_id = function_id;

        // Store function configuration
        {
            let mut functions = self.functions.lock();
            functions.insert(function_id, function_config.clone());
        }

        // Create minimum instances if specified
        if function_config.min_instances > 0 {
            for _ in 0..function_config.min_instances {
                let _instance = self.create_instance(function_id)?;
            }
        }

        crate::println!("[faas] Function {} deployed with ID: {}", function_config.name, function_id);

        Ok(function_id)
    }

    /// Create function instance
    fn create_instance(&self, function_id: FunctionId) -> Result<FunctionInstance> {
        let functions = self.functions.lock();
        let function_config = functions.get(&function_id)
            .ok_or(nos_api::Error::NotFound)?;

        let instance_id = self.next_instance_id.fetch_add(1, Ordering::SeqCst);

        // Create MicroVM for isolation
        let microvm_config = MicroVMConfig {
            vm_id: instance_id,
            name: format!("{}-instance-{}", function_config.name, instance_id),
            vcpu_count: ((function_config.resources.cpu_milli + 999) / 1000).max(1),
            memory_size: function_config.resources.memory_bytes,
            kernel_path: None,
            kernel_args: String::new(),
            rootfs_path: None,
            unikernel: false,
            network_interfaces: Vec::new(),
            mount_points: Vec::new(),
            resource_limits: Default::default(),
            security: Default::default(),
        };

        let mut microvm = MicroVM::new(microvm_config)?;
        microvm.boot()?;

        let instance = FunctionInstance {
            instance_id,
            function_id,
            state: InstanceState::Idle,
            microvm: Some(microvm),
            container_id: None,
            created_at: nos_api::event::get_time_ns(),
            last_invocation: None,
            invocation_count: AtomicU64::new(0),
            cold_start_time: Some(nos_api::event::get_time_ns()),
        };

        // Store instance
        {
            let mut instances = self.instances.lock();
            instances.insert(instance_id, instance.clone());
        }

        // Update function instances mapping
        {
            let mut function_instances = self.function_instances.lock();
            function_instances
                .entry(function_id)
                .or_insert_with(BTreeSet::new)
                .insert(instance_id);
        }

        crate::println!("[faas] Instance {} created for function {}", instance_id, function_id);

        Ok(instance)
    }

    /// Invoke a function
    pub fn invoke_function(&self, function_id: FunctionId, payload: Vec<u8>) -> Result<InvocationResult> {
        crate::println!("[faas] Invoking function {}", function_id);

        let invocation_id = self.next_invocation_id.fetch_add(1, Ordering::SeqCst);

        let invocation = FunctionInvocation {
            invocation_id,
            function_id,
            payload,
            context: InvocationContext {
                request_id: format!("req-{}", invocation_id),
                client_id: None,
                trace_id: None,
                metadata: BTreeMap::new(),
            },
            timestamp: nos_api::event::get_time_ns(),
        };

        // Get or create instance
        let instance_id = self.get_or_create_instance(function_id)?;

        // Execute invocation
        let result = self.execute_invocation(invocation, instance_id)?;

        // Store result
        {
            let mut completed = self.completed_invocations.lock();
            completed.push_back(result.clone());
            // Keep only last 1000 results
            if completed.len() > 1000 {
                completed.pop_front();
            }
        }

        crate::println!("[faas] Invocation {} completed in {}ms",
            invocation_id, result.execution_time_ms);

        Ok(result)
    }

    /// Get or create instance for function
    fn get_or_create_instance(&self, function_id: FunctionId) -> Result<InstanceId> {
        // Try to get an idle instance
        {
            let function_instances = self.function_instances.lock();
            if let Some(instance_ids) = function_instances.get(&function_id) {
                let instances = self.instances.lock();
                for &instance_id in instance_ids {
                    if let Some(instance) = instances.get(&instance_id) {
                        if instance.state == InstanceState::Idle {
                            return Ok(instance_id);
                        }
                    }
                }
            }
        }

        // No idle instance, create new one
        let instance = self.create_instance(function_id)?;
        Ok(instance.instance_id)
    }

    /// Execute invocation on instance
    fn execute_invocation(&self, invocation: FunctionInvocation, instance_id: InstanceId) -> Result<InvocationResult> {
        let start_time = nos_api::event::get_time_ns();

        // Get instance
        let mut instances = self.instances.lock();
        let instance = instances.get_mut(&instance_id)
            .ok_or(nos_api::Error::NotFound)?;

        // Get cold start time
        let cold_start_time_ms = instance.cold_start_time.map(|t| {
            (start_time - t) / 1_000_000
        });

        // Mark instance as running
        instance.state = InstanceState::Running;
        instance.invocation_count.fetch_add(1, Ordering::Relaxed);
        instance.last_invocation = Some(start_time);

        // Execute function (placeholder)
        // In real implementation, this would:
        // 1. Send invocation to MicroVM
        // 2. Wait for result
        // 3. Get metrics

        let execution_time_ms = 10; // Placeholder: 10ms execution

        // Mark instance as idle again
        instance.state = InstanceState::Idle;

        let end_time = nos_api::event::get_time_ns();

        Ok(InvocationResult {
            invocation_id: invocation.invocation_id,
            function_id: invocation.function_id,
            instance_id,
            payload: Vec::new(), // Placeholder result
            execution_time_ms,
            cold_start_time_ms,
            success: true,
            error: None,
            memory_used: 0,
            cpu_time_ms: execution_time_ms,
        })
    }

    /// Invoke function chain
    pub fn invoke_chain(&self, chain_id: u64, initial_payload: Vec<u8>) -> Result<Vec<InvocationResult>> {
        crate::println!("[faas] Invoking chain {}", chain_id);

        let chains = self.chains.lock();
        let chain = chains.get(&chain_id)
            .ok_or(nos_api::Error::NotFound)?;

        let mut results = Vec::new();
        let mut payload = initial_payload;

        for step in &chain.steps {
            let result = self.invoke_function(step.function_id, payload)?;
            payload = result.payload.clone();
            results.push(result);

            if !result.success && !step.continue_on_error {
                break;
            }
        }

        Ok(results)
    }

    /// Create function chain
    pub fn create_chain(&self, chain: FunctionChain) -> Result<u64> {
        let chain_id = self.next_chain_id.fetch_add(1, Ordering::SeqCst);

        let mut chain = chain.clone();
        chain.chain_id = chain_id;

        let mut chains = self.chains.lock();
        chains.insert(chain_id, chain);

        crate::println!("[faas] Chain {} created", chain_id);

        Ok(chain_id)
    }

    /// Set autoscaling policy
    pub fn set_autoscaling_policy(&self, function_id: FunctionId, policy: AutoscalingPolicy) {
        let mut policies = self.autoscaling_policies.lock();
        policies.insert(function_id, policy);
    }

    /// Get function statistics
    pub fn get_function_stats(&self, function_id: FunctionId) -> Result<FunctionStats> {
        let function_instances = self.function_instances.lock();
        let instance_ids = function_instances.get(&function_id)
            .ok_or(nos_api::Error::NotFound)?;

        let instances = self.instances.lock();
        let mut total_invocations = 0;
        let mut active_instances = 0;

        for &instance_id in instance_ids {
            if let Some(instance) = instances.get(&instance_id) {
                total_invocations += instance.invocation_count.load(Ordering::Relaxed) as usize;
                if instance.state == InstanceState::Running {
                    active_instances += 1;
                }
            }
        }

        Ok(FunctionStats {
            function_id,
            total_instances: instance_ids.len(),
            active_instances,
            total_invocations,
        })
    }

    /// List all functions
    pub fn list_functions(&self) -> Vec<FunctionConfig> {
        let functions = self.functions.lock();
        functions.values().cloned().collect()
    }

    /// Remove function
    pub fn remove_function(&self, function_id: FunctionId) -> Result<()> {
        crate::println!("[faas] Removing function {}", function_id);

        // Remove function configuration
        {
            let mut functions = self.functions.lock();
            functions.remove(&function_id);
        }

        // Remove all instances
        let instance_ids = {
            let mut function_instances = self.function_instances.lock();
            function_instances.remove(&function_id)
                .unwrap_or_default()
        };

        for instance_id in instance_ids {
            let mut instances = self.instances.lock();
            instances.remove(&instance_id);
        }

        crate::println!("[faas] Function {} removed", function_id);

        Ok(())
    }
}

impl Default for FunctionService {
    fn default() -> Self {
        Self::new()
    }
}

/// Function statistics
#[derive(Debug, Clone)]
pub struct FunctionStats {
    /// Function ID
    pub function_id: FunctionId,
    /// Total instances
    pub total_instances: usize,
    /// Active instances
    pub active_instances: usize,
    /// Total invocations
    pub total_invocations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_faas_creation() {
        let service = FunctionService::new();
        assert!(!service.running.load(Ordering::Relaxed));
    }

    #[test]
    fn test_function_deployment() {
        let service = FunctionService::new();

        let config = FunctionConfig {
            function_id: 0,
            name: "test-function".to_string(),
            description: "Test function".to_string(),
            runtime: "rust".to_string(),
            handler: "handle".to_string(),
            code: FunctionCode::Inline {
                code: "fn main() {}".to_string()
            },
            resources: ResourceQuota::default(),
            environment: BTreeMap::new(),
            timeout_secs: 30,
            max_instances: 10,
            min_instances: 0,
            cold_start_optimization: true,
            tags: Vec::new(),
        };

        let result = service.deploy_function(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_quota_default() {
        let quota = ResourceQuota::default();
        assert_eq!(quota.cpu_milli, 500);
        assert_eq!(quota.memory_bytes, 128 * 1024 * 1024);
    }
}
