//! 信号系统调用服务实现
//! 
//! 本模块提供信号系统调用服务的实现，包括：
//! - 信号发送和处理
//! - 信号处理程序管理
//! - 信号掩码操作
//! - 信号集操作

use nos_nos_error_handling::unified::KernelError;
use crate::syscalls::services::{BaseService, ServiceStatus, SyscallService};
use crate::syscalls::signal_service::handlers::*;
use crate::process::ProcessId;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// 信号系统调用服务
/// 
/// 实现SyscallService trait，提供信号相关的系统调用处理
/// 在新的模块化服务架构中处理信号操作。
#[derive(Debug)]
pub struct SignalService {
    /// 服务名称
    name: String,
    /// 服务版本
    version: String,
    /// 服务描述
    description: String,
    /// 当前服务状态
    status: ServiceStatus,
    /// 支持的系统调用号
    supported_syscalls: Vec<u32>,
    /// 信号统计
    stats: SignalStats,
}

impl SignalService {
    /// 创建新的信号服务实例
    pub fn new() -> Self {
        Self {
            name: String::from("signal"),
            version: String::from("1.0.0"),
            description: String::from("Signal syscall service for managing process signals"),
            status: ServiceStatus::Uninitialized,
            supported_syscalls: get_supported_syscalls(),
            stats: SignalStats::default(),
        }
    }

    /// 获取信号统计
    pub fn get_stats(&self) -> &SignalStats {
        &self.stats
    }

    /// 更新操作统计
    fn update_stats(&mut self, operation: SignalOperation) {
        match operation {
            SignalOperation::Kill => {
                self.stats.kill_calls += 1;
            }
            SignalOperation::SigAction => {
                self.stats.sigaction_calls += 1;
            }
            SignalOperation::SigProcMask => {
                self.stats.sigprocmask_calls += 1;
            }
            SignalOperation::SigPending => {
                self.stats.sigpending_calls += 1;
            }
            SignalOperation::SigSuspend => {
                self.stats.sigsuspend_calls += 1;
            }
            SignalOperation::SigReturn => {
                self.stats.sigreturn_calls += 1;
            }
            _ => self.stats.other_calls += 1,
        }
        self.stats.total_calls += 1;
    }

    /// 重置信号统计
    pub fn reset_stats(&mut self) {
        self.stats = SignalStats::default();
    }

    /// 发送信号到进程
    pub fn kill_process(&mut self, pid: ProcessId, sig: SignalNumber) -> Result<(), KernelError> {
        crate::log_debug!("Sending signal {} to process {}", sig, pid);
        
        // 更新统计
        self.update_stats(SignalOperation::Kill);
        
        // 调用处理程序
        send_signal(pid, sig)
    }

    /// 设置信号处理程序
    pub fn set_sigaction(
        &mut self,
        pid: ProcessId,
        sig: SignalNumber,
        action: SignalAction,
    ) -> Result<Option<SignalAction>, KernelError> {
        crate::log_debug!("Setting signal handler for signal {} in process {}", sig, pid);
        
        // 更新统计
        self.update_stats(SignalOperation::SigAction);
        
        // 获取全局处理程序管理器
        let handler_manager = get_global_handler_manager();
        handler_manager.set_process_handler(pid, sig, action)
    }

    /// 设置进程信号掩码
    pub fn set_process_sigmask(
        &mut self,
        pid: ProcessId,
        how: u32,
        new_mask: SignalSet,
        old_mask: Option<&mut SignalSet>,
    ) -> Result<(), KernelError> {
        crate::log_debug!("Setting signal mask for process {} with operation {}", pid, how);
        
        // 更新统计
        self.update_stats(SignalOperation::SigProcMask);
        
        // 调用处理程序
        set_signal_mask(pid, how, new_mask, old_mask)
    }

    /// 获取进程挂起的信号
    pub fn get_pending_signals(&mut self, pid: ProcessId) -> Result<SignalSet, KernelError> {
        crate::log_debug!("Getting pending signals for process {}", pid);
        
        // 更新统计
        self.update_stats(SignalOperation::SigPending);
        
        // TODO: 实现获取挂起信号的逻辑
        Ok(SignalSet::empty())
    }

    /// 挂起进程直到信号到达
    pub fn suspend_process(&mut self, pid: ProcessId) -> Result<(), KernelError> {
        crate::log_debug!("Suspending process {} until signal arrives", pid);
        
        // 更新统计
        self.update_stats(SignalOperation::SigSuspend);
        
        // TODO: 实现进程挂起的逻辑
        Ok(())
    }
}

impl Default for SignalService {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseService for SignalService {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn initialize(&mut self) -> Result<(), KernelError> {
        crate::log_info!("Initializing SignalService");
        self.status = ServiceStatus::Initializing;
        
        // TODO: 初始化信号处理子系统
        
        self.status = ServiceStatus::Initialized;
        crate::log_info!("SignalService initialized successfully");
        Ok(())
    }

    fn start(&mut self) -> Result<(), KernelError> {
        crate::log_info!("Starting SignalService");
        self.status = ServiceStatus::Starting;
        
        // TODO: 启动信号处理子系统
        
        self.status = ServiceStatus::Running;
        crate::log_info!("SignalService started successfully");
        Ok(())
    }

    fn stop(&mut self) -> Result<(), KernelError> {
        crate::log_info!("Stopping SignalService");
        self.status = ServiceStatus::Stopping;
        
        // TODO: 停止信号处理子系统
        
        self.status = ServiceStatus::Stopped;
        crate::log_info!("SignalService stopped successfully");
        Ok(())
    }

    fn destroy(&mut self) -> Result<(), KernelError> {
        crate::log_info!("Destroying SignalService");
        
        // 执行最终清理
        self.reset_stats();
        
        self.status = ServiceStatus::Uninitialized;
        crate::log_info!("SignalService destroyed successfully");
        Ok(())
    }

    fn status(&self) -> ServiceStatus {
        self.status
    }

    fn dependencies(&self) -> Vec<&str> {
        vec!["process"]
    }

    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

impl SyscallService for SignalService {
    fn supported_syscalls(&self) -> Vec<u32> {
        self.supported_syscalls.clone()
    }

    fn handle_syscall(&mut self, syscall_number: u32, args: &[u64]) -> Result<u64, KernelError> {
        crate::log_debug!("Handling signal syscall: {} with {} args", syscall_number, args.len());
        
        // 更新统计
        let operation = match syscall_number {
            0x2000 => SignalOperation::Kill,
            0x2001 => SignalOperation::SigAction,
            0x2002 => SignalOperation::SigProcMask,
            0x2003 => SignalOperation::SigPending,
            0x2004 => SignalOperation::SigSuspend,
            0x2005 => SignalOperation::SigReturn,
            _ => SignalOperation::Other,
        };
        self.update_stats(operation);
        
        // 分发到适当的处理程序
        match syscall_number {
            0x2000 => { // kill
                let pid = ProcessId::new(args.get(0).copied().unwrap_or(0) as u32);
                let sig = args.get(1).copied().unwrap_or(0) as SignalNumber;
                self.kill_process(pid, sig)?;
                Ok(0)
            }
            0x2001 => { // sigaction
                let pid = ProcessId::new(args.get(0).copied().unwrap_or(0) as u32);
                let sig = args.get(1).copied().unwrap_or(0) as SignalNumber;
                let action_ptr = args.get(2).copied().unwrap_or(0) as *const SignalAction;
                let old_action_ptr = args.get(3).copied().unwrap_or(0) as *mut SignalAction;
                
                // TODO: 安全地从用户空间读取信号处理程序
                let action = unsafe { action_ptr.read() };
                let old_action = self.set_sigaction(pid, sig, action)?;
                
                // TODO: 安全地向用户空间写入旧的信号处理程序
                if !old_action_ptr.is_null() {
                    unsafe { old_action_ptr.write(old_action.unwrap_or(SignalAction::Default)) };
                }
                
                Ok(0)
            }
            0x2002 => { // sigprocmask
                let pid = ProcessId::new(args.get(0).copied().unwrap_or(0) as u32);
                let how = args.get(1).copied().unwrap_or(0) as u32;
                let new_mask_ptr = args.get(2).copied().unwrap_or(0) as *const SignalSet;
                let old_mask_ptr = args.get(3).copied().unwrap_or(0) as *mut SignalSet;
                
                // TODO: 安全地从用户空间读取信号集
                let new_mask = unsafe { new_mask_ptr.read() };
                let mut old_mask = SignalSet::empty();
                
                self.set_process_sigmask(pid, how, new_mask, Some(&mut old_mask))?;
                
                // TODO: 安全地向用户空间写入旧的信号集
                if !old_mask_ptr.is_null() {
                    unsafe { old_mask_ptr.write(old_mask) };
                }
                
                Ok(0)
            }
            0x2003 => { // sigpending
                let pid = ProcessId::new(args.get(0).copied().unwrap_or(0) as u32);
                let set_ptr = args.get(1).copied().unwrap_or(0) as *mut SignalSet;
                
                let pending = self.get_pending_signals(pid)?;
                
                // TODO: 安全地向用户空间写入挂起的信号集
                if !set_ptr.is_null() {
                    unsafe { set_ptr.write(pending) };
                }
                
                Ok(0)
            }
            0x2004 => { // sigsuspend
                let pid = ProcessId::new(args.get(0).copied().unwrap_or(0) as u32);
                let mask_ptr = args.get(1).copied().unwrap_or(0) as *const SignalSet;
                
                // TODO: 安全地从用户空间读取信号集
                let mask = unsafe { mask_ptr.read() };
                
                // 设置新的信号掩码并挂起进程
                self.set_process_sigmask(pid, 2, mask, None)?; // 2 = SIG_SETMASK
                self.suspend_process(pid)?;
                
                Ok(0)
            }
            0x2005 => { // sigreturn
                // TODO: 实现信号返回处理
                crate::log_warn!("sigreturn syscall not implemented yet");
                Err(KernelError::Syscall(crate::syscalls::types::SyscallError::ENOSYS))
            }
            _ => {
                crate::log_warn!("Unsupported signal syscall: {}", syscall_number);
                Err(KernelError::Syscall(crate::syscalls::types::SyscallError::ENOSYS))
            }
        }
    }

    fn priority(&self) -> u32 {
        20 // 信号操作是高优先级的
    }
}

/// 信号操作类型，用于统计
#[derive(Debug, Clone, Copy)]
pub enum SignalOperation {
    Kill,
    SigAction,
    SigProcMask,
    SigPending,
    SigSuspend,
    SigReturn,
    Other,
}

/// 信号操作计数器，用于统计
#[derive(Debug, Clone, Default)]
pub struct SignalStats {
    /// 处理的信号系统调用总数
    pub total_calls: u64,
    /// kill系统调用次数
    pub kill_calls: u64,
    /// sigaction系统调用次数
    pub sigaction_calls: u64,
    /// sigprocmask系统调用次数
    pub sigprocmask_calls: u64,
    /// sigpending系统调用次数
    pub sigpending_calls: u64,
    /// sigsuspend系统调用次数
    pub sigsuspend_calls: u64,
    /// sigreturn系统调用次数
    pub sigreturn_calls: u64,
    /// 其他信号系统调用次数
    pub other_calls: u64,
}

/// 获取支持的信号系统调用号列表
pub fn get_supported_syscalls() -> Vec<u32> {
    vec![
        0x2000, // kill
        0x2001, // sigaction
        0x2002, // sigprocmask
        0x2003, // sigpending
        0x2004, // sigsuspend
        0x2005, // sigreturn
    ]
}

/// 信号服务工厂
/// 
/// 用于创建信号服务实例的工厂
pub struct SignalServiceFactory;

impl SignalServiceFactory {
    /// 创建新的信号服务实例
    pub fn create() -> Box<dyn SyscallService> {
        Box::new(SignalService::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_service_creation() {
        let service = SignalService::new();
        assert_eq!(service.name(), "signal");
        assert_eq!(service.version(), "1.0.0");
        assert_eq!(service.status(), ServiceStatus::Uninitialized);
        assert!(!service.supported_syscalls().is_empty());
    }

    #[test]
    fn test_signal_service_lifecycle() {
        let mut service = SignalService::new();

        // 测试初始化
        assert!(service.initialize().is_ok());
        assert_eq!(service.status(), ServiceStatus::Initialized);

        // 测试启动
        assert!(service.start().is_ok());
        assert_eq!(service.status(), ServiceStatus::Running);

        // 测试停止
        assert!(service.stop().is_ok());
        assert_eq!(service.status(), ServiceStatus::Stopped);
    }

    #[test]
    fn test_signal_operations() {
        let mut service = SignalService::new();
        let pid = ProcessId::new(123);
        let sig = 9;
        
        // 测试信号发送
        assert!(service.kill_process(pid, sig).is_ok());
        
        // 测试信号处理程序设置
        let action = SignalAction::Ignore;
        assert!(service.set_sigaction(pid, sig, action).is_ok());
        
        // 测试信号掩码设置
        let mask = SignalSet::empty();
        assert!(service.set_process_sigmask(pid, 0, mask, None).is_ok());
        
        // 测试获取挂起信号
        let pending = service.get_pending_signals(pid).unwrap();
        assert_eq!(pending, SignalSet::empty());
    }
}