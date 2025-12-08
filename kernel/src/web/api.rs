//! Web system APIs
//!
//! Provides Web APIs for file access, notifications, and other system features.

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;
use crate::reliability::errno::{EINVAL, EPERM, ENOENT};

/// Web file API - provides secure file access for web applications
pub struct WebFileApi {
    /// Allowed directories
    allowed_directories: Vec<String>,
}

impl WebFileApi {
    /// Create a new web file API
    pub fn new() -> Self {
        Self {
            allowed_directories: Vec::new(),
        }
    }
    
    /// Read file (with permission check)
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, i32> {
        // Check if path is allowed
        if !self.is_path_allowed(path) {
            return Err(EPERM);
        }
        
        // In real implementation, this would:
        // 1. Validate path
        // 2. Check permissions
        // 3. Read file via VFS
        // 4. Return file contents
        
        Err(ENOENT) // Placeholder
    }
    
    /// Write file (with permission check)
    pub fn write_file(&self, path: &str, data: &[u8]) -> Result<(), i32> {
        // Check if path is allowed
        if !self.is_path_allowed(path) {
            return Err(EPERM);
        }
        
        // In real implementation, this would write file via VFS
        Err(ENOENT) // Placeholder
    }
    
    /// Check if path is allowed
    fn is_path_allowed(&self, path: &str) -> bool {
        // Check against allowed directories
        for allowed in &self.allowed_directories {
            if path.starts_with(allowed) {
                return true;
            }
        }
        false
    }
}

/// Web notification API
pub struct WebNotificationApi {
    /// Active notifications
    notifications: Vec<Notification>,
}

/// Notification
#[derive(Debug, Clone)]
pub struct Notification {
    /// Notification ID
    pub id: String,
    /// Title
    pub title: String,
    /// Body
    pub body: String,
    /// Icon URL
    pub icon: Option<String>,
    /// Tag
    pub tag: Option<String>,
}

impl WebNotificationApi {
    /// Create a new web notification API
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
        }
    }
    
    /// Show notification
    pub fn show(&mut self, title: &str, body: &str, icon: Option<&str>) -> Result<String, i32> {
        let id = alloc::format!("notif-{}", crate::time::hrtime_nanos());
        let notification = Notification {
            id: id.clone(),
            title: title.to_string(),
            body: body.to_string(),
            icon: icon.map(|s| s.to_string()),
            tag: None,
        };
        
        self.notifications.push(notification);
        crate::println!("[web] Notification: {} - {}", title, body);
        
        Ok(id)
    }
    
    /// Close notification
    pub fn close(&mut self, id: &str) -> Result<(), i32> {
        self.notifications.retain(|n| n.id != id);
        Ok(())
    }
}

/// Web system API manager
pub struct WebSystemApi {
    /// File API
    file_api: WebFileApi,
    /// Notification API
    notification_api: WebNotificationApi,
}

impl WebSystemApi {
    /// Create a new web system API manager
    pub fn new() -> Self {
        Self {
            file_api: WebFileApi::new(),
            notification_api: WebNotificationApi::new(),
        }
    }
    
    /// Get file API
    pub fn get_file_api(&self) -> &WebFileApi {
        &self.file_api
    }
    
    /// Get notification API
    pub fn get_notification_api(&mut self) -> &mut WebNotificationApi {
        &mut self.notification_api
    }
}

/// Global web system API instance
static WEB_SYSTEM_API: crate::sync::Mutex<Option<WebSystemApi>> = crate::sync::Mutex::new(None);

/// Initialize web system API
pub fn init_web_system_api() -> Result<(), i32> {
    let mut api = WEB_SYSTEM_API.lock();
    if api.is_none() {
        *api = Some(WebSystemApi::new());
        crate::println!("[web] Web system API initialized");
    }
    Ok(())
}

/// Get web system API
pub fn get_web_system_api() -> &'static crate::sync::Mutex<WebSystemApi> {
    static INIT_ONCE: crate::sync::Once = crate::sync::Once::new();
    INIT_ONCE.call_once(|| {
        let mut api = WEB_SYSTEM_API.lock();
        if api.is_none() {
            *api = Some(WebSystemApi::new());
        }
    });
    
    unsafe {
        &*(WEB_SYSTEM_API.lock().as_ref().unwrap() as *const WebSystemApi as *const crate::sync::Mutex<WebSystemApi>)
    }
}

