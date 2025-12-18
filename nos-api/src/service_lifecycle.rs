//! 服务生命周期管理模块
//!
//! 本模块提供服务生命周期管理功能，用于替代全局静态变量。

use crate::error::Result;
use crate::interfaces::{InterfaceService as Service, InterfaceServiceManager as ServiceManager, InterfaceServiceInfo as ServiceInfo, InterfaceServiceStatus as ServiceStatus, InterfaceServiceStats as ServiceStats};
#[cfg(feature = "alloc")]
use alloc::{
    string::{String, ToString},
    collections::BTreeMap,
    sync::Arc,
    vec::Vec,
    format,
};

// Import types from interfaces module for no-alloc mode
#[cfg(not(feature = "alloc"))]
use crate::interfaces::{Arc, Vec};
#[cfg(not(feature = "alloc"))]
use crate::collections::BTreeMap;
use spin::Mutex;

/// 服务生命周期管理器
pub struct ServiceLifecycleManager {
    #[cfg(feature = "alloc")]
    services: Mutex<BTreeMap<String, Arc<dyn Service>>>,
    #[cfg(not(feature = "alloc"))]
    services: Mutex<BTreeMap<&'static str, Arc<dyn Service>>>,
    #[cfg(feature = "alloc")]
    service_info: Mutex<BTreeMap<String, ServiceInfo>>,
    #[cfg(not(feature = "alloc"))]
    service_info: Mutex<BTreeMap<&'static str, ServiceInfo>>,
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
        
        #[cfg(feature = "alloc")] {
            let service_id = service.service_id().to_string();
            let info = ServiceInfo {
                id: service_id.clone(),
                name: service.name().to_string(),
                version: service.version().to_string(),
                status: ServiceStatus::Initialized,
                description: String::new(),
                dependencies: Vec::new(),
                registration_time: crate::event::get_time_ns(),
            };
            
            services.insert(service_id.clone(), service.clone());
            service_info.insert(service_id, info);
        }
        
        #[cfg(not(feature = "alloc"))] {
            // 先获取所有需要的服务信息
            let service_id_str = service.service_id();
            let service_name_str = service.name();
            let service_version_str = service.version();
            
            // 使用unsafe将临时引用转换为'static
            // 这是安全的，因为service会被注册并长期存在
            let service_id: &'static str = unsafe { core::mem::transmute(service_id_str) };
            let service_name: &'static str = unsafe { core::mem::transmute(service_name_str) };
            let service_version: &'static str = unsafe { core::mem::transmute(service_version_str) };
            
            let info = ServiceInfo {
                id: service_id,
                name: service_name,
                version: service_version,
                status: ServiceStatus::Initialized,
                description: "",
                dependencies: &[],
                registration_time: crate::event::get_time_ns(),
            };
            
            services.insert(service_id, service);
            service_info.insert(service_id, info);
        }
        
        *next_id += 1;
        Ok(())
    }
    
    fn unregister_service(&mut self, service_id: &str) -> Result<()> {
        #[cfg(not(feature = "alloc"))]
        let service_id: &'static str = unsafe { core::mem::transmute(service_id) };
        
        let mut services = self.services.lock();
        let mut service_info = self.service_info.lock();
        
        #[cfg(feature = "alloc")] {
            services.remove(service_id);
            service_info.remove(service_id);
        }
        #[cfg(not(feature = "alloc"))] {
            services.remove(&service_id);
            service_info.remove(&service_id);
        }
        
        Ok(())
    }
    
    fn get_service(&self, service_id: &str) -> Option<Arc<dyn Service>> {
        #[cfg(not(feature = "alloc"))]
        let service_id: &'static str = unsafe { core::mem::transmute(service_id) };
        
        let services = self.services.lock();
        #[cfg(feature = "alloc")]
        return services.get(service_id).cloned();
        #[cfg(not(feature = "alloc"))]
        return services.get(&service_id).cloned();
    }
    
    fn list_services(&self) -> Vec<ServiceInfo> {
        #[cfg(feature = "alloc")]
        {
            let service_info = self.service_info.lock();
            return service_info.values().cloned().collect();
        }
        #[cfg(not(feature = "alloc"))] {
            // In no-alloc mode, we can't dynamically allocate a Vec
            // This is a limitation of the no-alloc environment
            // For now, return an empty slice
            &[]
        }
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
    #[cfg(feature = "alloc")]
    ServiceRegistered {
        service_id: String,
        service_name: String,
    },
    #[cfg(not(feature = "alloc"))]
    ServiceRegistered {
        service_id: &'static str,
        service_name: &'static str,
    },
    /// 服务启动
    #[cfg(feature = "alloc")]
    ServiceStarted {
        service_id: String,
        service_name: String,
    },
    #[cfg(not(feature = "alloc"))]
    ServiceStarted {
        service_id: &'static str,
        service_name: &'static str,
    },
    /// 服务停止
    #[cfg(feature = "alloc")]
    ServiceStopped {
        service_id: String,
        service_name: String,
    },
    #[cfg(not(feature = "alloc"))]
    ServiceStopped {
        service_id: &'static str,
        service_name: &'static str,
    },
    ///服务错误
    #[cfg(feature = "alloc")]
    ServiceError {
        service_id: String,
        service_name: String,
        error: String,
    },
    #[cfg(not(feature = "alloc"))]
    ServiceError {
        service_id: &'static str,
        service_name: &'static str,
        error: &'static str,
    },
    ///服务注销
    #[cfg(feature = "alloc")]
    ServiceUnregistered {
        service_id: String,
    },
    #[cfg(not(feature = "alloc"))]
    ServiceUnregistered {
        service_id: &'static str,
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
    #[cfg(feature = "alloc")]
    listeners: Mutex<Vec<Arc<dyn ServiceLifecycleListener>>>,
    // 在no-alloc模式下，不支持监听器功能
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
            #[cfg(feature = "alloc")]
            listeners: Mutex::new(Vec::new()),
        }
    }
    
    /// 添加生命周期监听器
    #[cfg(feature = "alloc")]
    pub fn add_listener(&self, listener: Arc<dyn ServiceLifecycleListener>) {
        let mut listeners = self.listeners.lock();
        listeners.push(listener);
    }
    #[cfg(not(feature = "alloc"))]
    pub fn add_listener(&self, _listener: Arc<dyn ServiceLifecycleListener>) {
        // 在no-alloc模式下，不支持监听器功能
    }
    
    /// 移除生命周期监听器
    #[cfg(feature = "alloc")]
    pub fn remove_listener(&self, listener_name: &str) {
        let mut listeners = self.listeners.lock();
        listeners.retain(|l: &Arc<dyn ServiceLifecycleListener>| l.name() != listener_name);
    }
    #[cfg(not(feature = "alloc"))]
    pub fn remove_listener(&self, _listener_name: &str) {
        // 在no-alloc模式下，不支持监听器功能
    }
    
    /// 触发生命周期事件
    fn notify_listeners(&self, event: &ServiceLifecycleEvent) {
        #[cfg(feature = "alloc")] {
            let listeners: Vec<Arc<dyn ServiceLifecycleListener>> = self.listeners.lock().clone();
            for listener in listeners.iter() {
                listener.on_event(event);
            }
        }
        #[cfg(not(feature = "alloc"))] {
            // 在no-alloc模式下，不支持监听器功能
            // Use the event parameter to avoid unused variable warning
            let _ = event;
        }
    }
}

impl LifecycleAwareServiceManager {
    /// 启动服务
    pub fn start_service(&mut self, service_id: &str) -> Result<()> {
        #[cfg(not(feature = "alloc"))]
        let service_id: &'static str = unsafe { core::mem::transmute(service_id) };
        
        // 检查服务当前状态
        {
            let service_info = self.inner.service_info.lock();
            #[cfg(feature = "alloc")]
            let info = service_info.get(service_id);
            #[cfg(not(feature = "alloc"))]
            let info = service_info.get(&service_id);
            
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
                #[cfg(feature = "alloc")]
                return Err(crate::error::not_found(&format!("Service '{}' not found", service_id)));
                #[cfg(not(feature = "alloc"))]
                return Err(crate::error::not_found("Service not found"));
            }
        } // 释放锁
        
        // 先获取服务实例，避免在持有锁时调用服务方法
        let service_instance: Option<Arc<dyn Service>> = {
            let services = self.inner.services.lock();
            #[cfg(feature = "alloc")]
            {
                services.get(service_id).cloned()
            }
            #[cfg(not(feature = "alloc"))]
            {
                services.get(&service_id).cloned()
            }
        };
        
        if let Some(service) = service_instance {
            // 更新状态为正在启动
            {
                let mut service_info = self.inner.service_info.lock();
                #[cfg(feature = "alloc")]
                if let Some(info) = service_info.get_mut(service_id) {
                    info.status = ServiceStatus::Starting;
                }
                #[cfg(not(feature = "alloc"))]
                if let Some(info) = service_info.get_mut(&service_id) {
                    info.status = ServiceStatus::Starting;
                }
            } // 释放锁
            
            // 通知监听器服务正在启动
            #[cfg(feature = "alloc")] {
                let service_id = service_id.to_string();
                let service_name = service.name().to_string();
                self.notify_listeners(&ServiceLifecycleEvent::ServiceStarted {
                    service_id,
                    service_name,
                });
            }
            #[cfg(not(feature = "alloc"))] {
                // 在no-alloc模式下，我们假设service_id是&'static str
                // 这里我们使用unsafe将&str转换为&'static str
                // 这是安全的，因为我们只在内部使用这个引用，并且在事件处理完成后就不再需要它
                let service_id_static: &'static str = unsafe {
                    core::mem::transmute(service_id)
                };
                // 获取service.name()并转换为'static str
                let service_name_str: &str = service.name();
                let service_name_static: &'static str = unsafe {
                    core::mem::transmute(service_name_str)
                };
                self.notify_listeners(&ServiceLifecycleEvent::ServiceStarted {
                    service_id: service_id_static,
                    service_name: service_name_static,
                });
            }
            
            // 启动服务
            service.start()?;
            
            // 更新服务状态为运行中
            {
                let mut service_info = self.inner.service_info.lock();
                #[cfg(feature = "alloc")]
                if let Some(info) = service_info.get_mut(service_id) {
                    info.status = ServiceStatus::Running;
                }
                #[cfg(not(feature = "alloc"))]
                if let Some(info) = service_info.get_mut(&service_id) {
                    info.status = ServiceStatus::Running;
                }
            } // 释放锁
            
            Ok(())
        } else {
            // 回滚状态
            {
                let mut service_info = self.inner.service_info.lock();
                #[cfg(feature = "alloc")]
                if let Some(info) = service_info.get_mut(service_id) {
                    if info.status == ServiceStatus::Starting {
                        info.status = ServiceStatus::Stopped;
                    }
                }
                #[cfg(not(feature = "alloc"))]
                if let Some(info) = service_info.get_mut(&service_id) {
                    if info.status == ServiceStatus::Starting {
                        info.status = ServiceStatus::Stopped;
                    }
                }
            } // 释放锁
            
            #[cfg(feature = "alloc")]
            return Err(crate::error::not_found(&format!("Service '{}' not found", service_id)));
            #[cfg(not(feature = "alloc"))]
            return Err(crate::error::not_found("Service not found"));
        }
    }
    
    /// 停止服务
    pub fn stop_service(&mut self, service_id: &str) -> Result<()> {
        #[cfg(not(feature = "alloc"))]
        let service_id: &'static str = unsafe { core::mem::transmute(service_id) };
        
        // 检查服务当前状态
        {
            let service_info = self.inner.service_info.lock();
            #[cfg(feature = "alloc")]
            let info_exists = service_info.get(service_id).is_some();
            #[cfg(not(feature = "alloc"))]
            let info_exists = service_info.get(&service_id).is_some();
            
            if info_exists {
                #[cfg(feature = "alloc")]
                let status = service_info.get(service_id).map(|info| info.status);
                #[cfg(not(feature = "alloc"))]
                let status = service_info.get(&service_id).map(|info| info.status);
                
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
                #[cfg(feature = "alloc")]
                return Err(crate::error::not_found(&format!("Service '{}' not found", service_id)));
                #[cfg(not(feature = "alloc"))]
                return Err(crate::error::not_found("Service not found"));
            }
        } // 释放锁
        
        // 先获取服务实例，避免在持有锁时调用服务方法
        let service_instance: Option<Arc<dyn Service>> = {
            let services = self.inner.services.lock();
            #[cfg(feature = "alloc")]
            {
                services.get(service_id).cloned()
            }
            #[cfg(not(feature = "alloc"))]
            {
                services.get(&service_id).cloned()
            }
        };
        
        if let Some(service) = service_instance {
            // 更新状态为正在停止
            {
                let mut service_info = self.inner.service_info.lock();
                #[cfg(feature = "alloc")]
                if let Some(info) = service_info.get_mut(service_id) {
                    info.status = ServiceStatus::Stopping;
                }
                #[cfg(not(feature = "alloc"))]
                if let Some(info) = service_info.get_mut(&service_id) {
                    info.status = ServiceStatus::Stopping;
                }
            } // 释放锁
            
            // 通知监听器服务正在停止
            #[cfg(feature = "alloc")] {
                let service_id = service_id.to_string();
                let service_name = service.name().to_string();
                self.notify_listeners(&ServiceLifecycleEvent::ServiceStopped {
                    service_id,
                    service_name,
                });
            }
            #[cfg(not(feature = "alloc"))] {
                // 在no-alloc模式下，我们假设service_id是&'static str
                // 这里我们使用unsafe将&str转换为&'static str
                // 这是安全的，因为我们只在内部使用这个引用，并且在事件处理完成后就不再需要它
                let service_id_static: &'static str = unsafe {
                    core::mem::transmute(service_id)
                };
                // 获取service.name()并转换为'static str
                let service_name_str: &str = service.name();
                let service_name_static: &'static str = unsafe {
                    core::mem::transmute(service_name_str)
                };
                self.notify_listeners(&ServiceLifecycleEvent::ServiceStopped {
                    service_id: service_id_static,
                    service_name: service_name_static,
                });
            }
            
            // 停止服务
            service.stop()?;
            
            // 更新服务状态为已停止
            {
                let mut service_info = self.inner.service_info.lock();
                #[cfg(feature = "alloc")]
                if let Some(info) = service_info.get_mut(service_id) {
                    info.status = ServiceStatus::Stopped;
                }
                #[cfg(not(feature = "alloc"))]
                if let Some(info) = service_info.get_mut(&service_id) {
                    info.status = ServiceStatus::Stopped;
                }
            } // 释放锁
            
            Ok(())
        } else {
            // 回滚状态
            {
                let mut service_info = self.inner.service_info.lock();
                #[cfg(feature = "alloc")]
                if let Some(info) = service_info.get_mut(service_id) {
                    if info.status == ServiceStatus::Stopping {
                        info.status = ServiceStatus::Running;
                    }
                }
                #[cfg(not(feature = "alloc"))]
                if let Some(info) = service_info.get_mut(&service_id) {
                    if info.status == ServiceStatus::Stopping {
                        info.status = ServiceStatus::Running;
                    }
                }
            } // 释放锁
            
            #[cfg(feature = "alloc")]
            return Err(crate::error::not_found(&format!("Service '{}' not found", service_id)));
            #[cfg(not(feature = "alloc"))]
            return Err(crate::error::not_found("Service not found"));
        }
    }
}

impl ServiceManager for LifecycleAwareServiceManager {
    fn register_service(&mut self, service: Arc<dyn Service>) -> Result<()> {
        // 先提取服务信息，确保在service被移动前获取所有需要的数据
        let service_id_str = service.service_id();
        let service_name_str = service.name();
        
        // 在no-alloc模式下，我们需要将引用转换为'static
        #[cfg(not(feature = "alloc"))] {
            // 使用unsafe将临时引用转换为'static
            // 注意：这是安全的，因为service会被注册并长期存在
            let service_id: &'static str = unsafe { core::mem::transmute(service_id_str) };
            let service_name: &'static str = unsafe { core::mem::transmute(service_name_str) };
            
            // 注册服务
            self.inner.register_service(service)?;
            
            // 通知监听器
            self.notify_listeners(&ServiceLifecycleEvent::ServiceRegistered {
                service_id,
                service_name,
            });
        }
        
        #[cfg(feature = "alloc")] {
            // 注册服务
            self.inner.register_service(service.clone())?;
            
            // 通知监听器
            let service_id = service_id_str.to_string();
            let service_name = service_name_str.to_string();
            self.notify_listeners(&ServiceLifecycleEvent::ServiceRegistered {
                service_id,
                service_name,
            });
        }
        
        Ok(())
    }
    
    fn unregister_service(&mut self, service_id: &str) -> Result<()> {
        // 检查服务是否存在
        {
            let services = self.inner.services.lock();
            #[cfg(feature = "alloc")]
            if !services.contains_key(service_id) {
                return Err(crate::error::service_error("Service not found"));
            }
            #[cfg(not(feature = "alloc"))]
            if !services.contains_key(&service_id) {
                return Err(crate::error::service_error("Service not found"));
            }
        } // 释放锁
        
        // 注销服务
        self.inner.unregister_service(service_id)?;
        
        // 通知监听器
        #[cfg(feature = "alloc")] {
            let service_id = service_id.to_string();
            self.notify_listeners(&ServiceLifecycleEvent::ServiceUnregistered {
                service_id,
            });
        }
        #[cfg(not(feature = "alloc"))] {
            // 在no-alloc模式下，我们假设service_id是&'static str
            // 这里我们使用unsafe将&str转换为&'static str
            // 这是安全的，因为我们只在内部使用这个引用，并且在事件处理完成后就不再需要它
            let service_id_static: &'static str = unsafe {
                core::mem::transmute(service_id)
            };
            self.notify_listeners(&ServiceLifecycleEvent::ServiceUnregistered {
                service_id: service_id_static,
            });
        }
        
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
    
    #[cfg(feature = "alloc")]
    {
        let mut manager = GLOBAL_SERVICE_MANAGER.lock();
        *manager = Some(Arc::new(Mutex::new(LifecycleAwareServiceManager::new())));
    }
    #[cfg(not(feature = "alloc"))]
    {
        // In no-alloc mode, we can't use Arc
        // This would need a different approach for a real implementation
        let _manager = GLOBAL_SERVICE_MANAGER.lock();
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

