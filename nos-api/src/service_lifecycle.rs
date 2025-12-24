//! 服务生命周期管理模块
//!
//! 本模块提供服务生命周期管理功能，用于替代全局静态变量。

use crate::error::Result;
use crate::interfaces::{InterfaceService as Service, InterfaceServiceManager as ServiceManager, InterfaceServiceInfo as ServiceInfo, InterfaceServiceStatus as ServiceStatus, InterfaceServiceStats as ServiceStats};
use alloc::{
    string::{String, ToString},
    collections::BTreeMap,
    sync::Arc,
    vec::Vec,
    format,
};
use spin::Mutex;

/// 服务生命周期管理器
pub struct ServiceLifecycleManager {
    services: Mutex<BTreeMap<String, Arc<dyn Service>>>,
    service_info: Mutex<BTreeMap<String, ServiceInfo>>,
    next_service_id: Mutex<u32>,
}

impl Default for ServiceLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceLifecycleManager {
    /// 创建新的服务生命周期管理器
    pub fn new() -> Self {
        Self {
            services: Mutex::new(BTreeMap::new()),
            service_info: Mutex::new(BTreeMap::new()),
            next_service_id: Mutex::new(1),
        }
    }
}

impl ServiceManager for ServiceLifecycleManager {
    fn register_service(&mut self, service: Arc<dyn Service>) -> Result<()> {
        let mut services = self.services.lock();
        let mut service_info = self.service_info.lock();
        let mut next_id = self.next_service_id.lock();
        
        let service_id_str = service.service_id();
        let service_name_str = service.name();
        let service_version_str = service.version();
        
        let service_id: &'static str = unsafe { core::mem::transmute(service_id_str) };
        let service_name: &'static str = unsafe { core::mem::transmute(service_name_str) };
        let service_version: &'static str = unsafe { core::mem::transmute(service_version_str) };
        
        let info = ServiceInfo {
            id: service_id.to_string(),
            name: service_name.to_string(),
            version: service_version.to_string(),
            status: ServiceStatus::Running,
            description: "".to_string(),
            dependencies: Vec::new(),
            registration_time: crate::event::get_time_ns(),
        };
        
        services.insert(service_id.to_string(), service);
        service_info.insert(service_id.to_string(), info);
        
        *next_id += 1;
        Ok(())
    }
    
    fn unregister_service(&mut self, service_id: &str) -> Result<()> {
        let mut services = self.services.lock();
        let mut service_info = self.service_info.lock();
        
        services.remove(service_id);
        service_info.remove(service_id);
        
        Ok(())
    }
    
    fn get_service(&self, service_id: &str) -> Option<Arc<dyn Service>> {
        let services = self.services.lock();
        services.get(service_id).cloned()
    }
    
    fn list_services(&self) -> Vec<ServiceInfo> {
        let service_info = self.service_info.lock();
        service_info.values().cloned().collect()
    }
    
    fn get_stats(&self) -> ServiceStats {
        let service_info = self.service_info.lock();
        let total_services = service_info.len();
        let running_services = service_info.values()
            .filter(|info| matches!(info.status, ServiceStatus::Running))
            .count();
        
        ServiceStats {
            registered_services: total_services,
            running_services,
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_response_time_ns: 0,
        }
    }
}

/// 服务生命周期事件
#[derive(Debug, Clone)]
pub enum ServiceLifecycleEvent {
    /// 服务注册
    ServiceRegistered {
        service_id: String,
        service_name: String,
    },
    /// 服务启动
    ServiceStarted {
        service_id: String,
        service_name: String,
    },
    /// 服务停止
    ServiceStopped {
        service_id: String,
        service_name: String,
    },
    ///服务错误
    ServiceError {
        service_id: String,
        service_name: String,
        error: String,
    },
    ///服务注销
    ServiceUnregistered {
        service_id: String,
    },
}

/// 服务生命周期监听器
pub trait ServiceLifecycleListener: Send + Sync {
    /// 处理服务生命周期事件
    fn on_event(&self, event: &ServiceLifecycleEvent);
    
    /// 获取监听器名称
    fn name(&self) -> &str;
}

/// 带生命周期管理的服务管理器
pub struct LifecycleAwareServiceManager {
    inner: ServiceLifecycleManager,
    listeners: Mutex<Vec<Arc<dyn ServiceLifecycleListener>>>
}

impl Default for LifecycleAwareServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LifecycleAwareServiceManager {
    /// 创建新的带生命周期管理的服务管理器
    pub fn new() -> Self {
        Self {
            inner: ServiceLifecycleManager::new(),
            listeners: Mutex::new(Vec::new()),
        }
    }
    
    /// 添加生命周期监听器
    pub fn add_listener(&self, listener: Arc<dyn ServiceLifecycleListener>) {
        let mut listeners = self.listeners.lock();
        listeners.push(listener);
    }
    
    /// 移除生命周期监听器
    pub fn remove_listener(&self, listener_name: &str) {
        let mut listeners = self.listeners.lock();
        listeners.retain(|l: &Arc<dyn ServiceLifecycleListener>| l.name() != listener_name);
    }
    
    /// 触发生命周期事件
    fn notify_listeners(&self, event: &ServiceLifecycleEvent) {
        let listeners: Vec<Arc<dyn ServiceLifecycleListener>> = self.listeners.lock().clone();
        for listener in listeners.iter() {
            listener.on_event(event);
        }
    }
}

impl LifecycleAwareServiceManager {
    /// 启动服务
    pub fn start_service(&mut self, service_id: &str) -> Result<()> {
        // 检查服务当前状态
        {
            let service_info = self.inner.service_info.lock();
            let info = service_info.get(service_id);
            
            if let Some(info) = info {
                match info.status {
                    ServiceStatus::Running => {
                        // 服务已经在运行
                        return Ok(());
                    }
                    ServiceStatus::Stopped => {
                        // 可以启动
                    }
                    _ => {
                        return Err(crate::error::service_error("Service is not in a startable state"));
                    }
                }
            } else {
                return Err(crate::error::not_found(&format!("Service '{}' not found", service_id)));
            }
        } // 释放锁
        
        // 先获取服务实例，避免在持有锁时调用服务方法
        let service_instance: Option<Arc<dyn Service>> = {
            let services = self.inner.services.lock();
            services.get(service_id).cloned()
        };
        
        if let Some(service) = service_instance {
            // 更新状态为正在启动
            {
                let mut service_info = self.inner.service_info.lock();
                if let Some(info) = service_info.get_mut(service_id) {
                    info.status = ServiceStatus::Starting;
                }
            } // 释放锁
            
            // 通知监听器服务正在启动
            self.notify_listeners(&ServiceLifecycleEvent::ServiceStarted {
                service_id: service_id.to_string(),
                service_name: service.name().to_string(),
            });
            
            // 启动服务
            service.start()?;
            
            // 更新服务状态为运行中
            {
                let mut service_info = self.inner.service_info.lock();
                let service_id_owned = service_id.to_string();
                if let Some(info) = service_info.get_mut(&service_id_owned) {
                    info.status = ServiceStatus::Running;
                }
            } // 释放锁
            
            Ok(())
        } else {
            // 回滚状态
            {
                let mut service_info = self.inner.service_info.lock();
                let service_id_owned = service_id.to_string();
                if let Some(info) = service_info.get_mut(&service_id_owned)
                    && info.status == ServiceStatus::Starting {
                    info.status = ServiceStatus::Stopped;
                }
            } // 释放锁
            
            Err(crate::error::not_found(&format!("Service '{}' not found", service_id)))
        }
    }
    
    /// 停止服务
    pub fn stop_service(&mut self, service_id: &str) -> Result<()> {
        // 检查服务当前状态
        {
            let service_info = self.inner.service_info.lock();
            let info_exists = service_info.get(service_id).is_some();
            
            if info_exists {
                let status = service_info.get(service_id).map(|info| info.status);
                
                if let Some(status) = status {
                    match status {
                        ServiceStatus::Stopped => {
                            // 服务已经停止
                            return Ok(());
                        }
                        ServiceStatus::Running => {
                            // 可以停止
                        }
                        _ => {
                            return Err(crate::error::service_error("Service is not in a stoppable state"));
                        }
                    }
                }
            } else {
            return Err(crate::error::not_found(&format!("Service '{}' not found", service_id)));
        }
        } // 释放锁
        
        // 先获取服务实例，避免在持有锁时调用服务方法
        let service_instance: Option<Arc<dyn Service>> = {
            let services = self.inner.services.lock();
            let service_id_owned = service_id.to_string();
            services.get(&service_id_owned).cloned()
        };
        
        if let Some(service) = service_instance {
            // 更新状态为正在停止
            {
                let mut service_info = self.inner.service_info.lock();
                let service_id_owned = service_id.to_string();
                if let Some(info) = service_info.get_mut(&service_id_owned) {
                    info.status = ServiceStatus::Stopping;
                }
            } // 释放锁
            
            // 通知监听器服务正在停止
            self.notify_listeners(&ServiceLifecycleEvent::ServiceStopped {
                service_id: service_id.to_string(),
                service_name: service.name().to_string(),
            });
            
            // 停止服务
            service.stop()?;
            
            // 更新服务状态为已停止
            {
                let mut service_info = self.inner.service_info.lock();
                let service_id_owned = service_id.to_string();
                if let Some(info) = service_info.get_mut(&service_id_owned) {
                    info.status = ServiceStatus::Stopped;
                }
            } // 释放锁
            
            Ok(())
        } else {
            // 回滚状态
            {
                let mut service_info = self.inner.service_info.lock();
                let service_id_owned = service_id.to_string();
                if let Some(info) = service_info.get_mut(&service_id_owned)
                    && info.status == ServiceStatus::Stopping {
                    info.status = ServiceStatus::Running;
                }
            } // 释放锁
            
            Err(crate::error::not_found(&format!("Service '{}' not found", service_id)))
        }
    }
}

impl ServiceManager for LifecycleAwareServiceManager {
    fn register_service(&mut self, service: Arc<dyn Service>) -> Result<()> {
        // 先提取服务信息，确保在service被移动前获取所有需要的数据
        let service_id_str = service.service_id();
        let service_name_str = service.name();
        
        // 将服务ID和名称转换为String以确保内存由内核管理
        let service_id = service_id_str.to_string();
        let service_name = service_name_str.to_string();
        
        // 注册服务
        self.inner.register_service(service)?;
        
        // 通知监听器
        self.notify_listeners(&ServiceLifecycleEvent::ServiceRegistered {
            service_id,
            service_name,
        });
        
        Ok(())
    }
    
    fn unregister_service(&mut self, service_id: &str) -> Result<()> {
        // 检查服务是否存在
        {
            let services = self.inner.services.lock();
            let service_id_owned = service_id.to_string();
            if !services.contains_key(&service_id_owned) {
                return Err(crate::error::service_error("Service not found"));
            }
        } // 释放锁
        
        // 注销服务
        self.inner.unregister_service(service_id)?;
        
        // 通知监听器
        self.notify_listeners(&ServiceLifecycleEvent::ServiceUnregistered {
            service_id: service_id.to_string(),
        });
        
        Ok(())
    }
    
    fn get_service(&self, service_id: &str) -> Option<Arc<dyn Service>> {
        self.inner.get_service(service_id)
    }
    
    fn list_services(&self) -> Vec<ServiceInfo> {
        self.inner.list_services()
    }
    
    fn get_stats(&self) -> ServiceStats {
        self.inner.get_stats()
    }
}

/// 全局服务生命周期管理器
static GLOBAL_SERVICE_MANAGER: Mutex<Option<Arc<Mutex<LifecycleAwareServiceManager>>>> = Mutex::new(None);
static SERVICE_MANAGER_INIT: Mutex<bool> = Mutex::new(false);

/// 初始化全局服务生命周期管理器
pub fn init_service_lifecycle_manager() -> Result<()> {
    let mut is_init = SERVICE_MANAGER_INIT.lock();
    if *is_init {
        return Ok(());
    }
    
    {
        let mut manager = GLOBAL_SERVICE_MANAGER.lock();
        *manager = Some(Arc::new(Mutex::new(LifecycleAwareServiceManager::new())));
    }
    *is_init = true;
    Ok(())
}

/// 获取全局服务生命周期管理器
pub fn get_service_lifecycle_manager() -> Arc<Mutex<LifecycleAwareServiceManager>> {
    let manager = GLOBAL_SERVICE_MANAGER.lock();
    manager
        .as_ref()
        .expect("Service lifecycle manager not initialized")
        .clone()
}

/// 注册服务
pub fn register_service(service: Arc<dyn Service>) -> Result<()> {
    let manager = get_service_lifecycle_manager();
    let mut manager = manager.lock();
    manager.register_service(service)
}

/// 注销服务
pub fn unregister_service(service_id: &str) -> Result<()> {
    let manager = get_service_lifecycle_manager();
    let mut manager = manager.lock();
    manager.unregister_service(service_id)
}

/// 获取服务
pub fn get_service(service_id: &str) -> Option<Arc<dyn Service>> {
    let manager = get_service_lifecycle_manager();
    let manager = manager.lock();
    manager.get_service(service_id)
}

/// 列出所有服务
pub fn list_services() -> Vec<ServiceInfo> {
    let manager = get_service_lifecycle_manager();
    let manager = manager.lock();
    manager.list_services()
}

/// 获取服务统计信息
pub fn get_service_stats() -> ServiceStats {
    let manager = get_service_lifecycle_manager();
    let manager = manager.lock();
    manager.get_stats()
}

/// 启动服务
pub fn start_service(service_id: &str) -> Result<()> {
    let manager = get_service_lifecycle_manager();
    let mut manager = manager.lock();
    manager.start_service(service_id)
}

/// 停止服务
pub fn stop_service(service_id: &str) -> Result<()> {
    let manager = get_service_lifecycle_manager();
    let mut manager = manager.lock();
    manager.stop_service(service_id)
}

/// 添加服务生命周期监听器
pub fn add_lifecycle_listener(listener: Arc<dyn ServiceLifecycleListener>) -> Result<()> {
    let manager = get_service_lifecycle_manager();
    let manager = manager.lock();
    manager.add_listener(listener);
    Ok(())
}

/// 移除服务生命周期监听器
pub fn remove_lifecycle_listener(listener_name: &str) -> Result<()> {
    let manager = get_service_lifecycle_manager();
    let manager = manager.lock();
    manager.remove_listener(listener_name);
    Ok(())
}

