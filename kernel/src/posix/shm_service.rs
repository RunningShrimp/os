//! Shared Memory Service
//!
//! This module provides the Shared Memory Service for dependency injection.
//! It wraps the global SHM_SEGMENTS registry and provides a clean interface
//! for managing shared memory segments.

use alloc::sync::Arc;
use core::sync::atomic;
use spin::Mutex;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use alloc::boxed::Box;
use core::sync::atomic;
use alloc::collections::BTreeMap;
use core::sync::atomic;
use nos_api::di::{ServiceFactory, ServiceMetadata, ServiceScope};
use core::sync::atomic;

/// Shared Memory Service
pub struct SharedMemoryService {
    /// Shared memory segments registry
    segments: Mutex<BTreeMap<i32, Arc<Mutex<super::SharedMemorySegment>>>>,
    /// Next shared memory ID
    next_shm_id: core::sync::atomic::AtomicI32,
}

impl SharedMemoryService {
    /// Create a new Shared Memory Service
    pub fn new() -> Self {
        Self {
            segments: Mutex::new(BTreeMap::new()),
            next_shm_id: core::sync::atomic::AtomicI32::new(1),
        }
    }

    /// Get or create a shared memory segment
    pub fn get_or_create_segment(
        &self,
        key: i32,
        size: usize,
        mode: super::Mode,
        create_flag: bool,
        excl_flag: bool,
    ) -> Option<i32> {
        let mut segments = self.segments.lock();

        // Look for existing segment
        if let Some(_segment) = segments.get(&key) {
            if excl_flag {
                return None;
            }
            return Some(_segment.lock().id);
        }

        // Create new segment if requested
        if create_flag {
            // Allocate segment ID
            let id = self.next_shm_id.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
            if id <= 0 {
                return None;
            }

            // Create segment (simplified - full implementation in shm.rs)
            let segment = super::SharedMemorySegment {
                id,
                key,
                size,
                pages: alloc::vec::Vec::new(),
                perm: super::IpcPerm {
                    uid: crate::process::getuid(),
                    gid: crate::process::getgid(),
                    cuid: crate::process::getuid(),
                    cgid: crate::process::getgid(),
                    mode,
                    seq: 0,
                    key,
                },
                nattch: 0,
                creator_pid: crate::process::getpid(),
                last_attach_pid: 0,
                last_detach_time: 0,
                creation_time: crate::subsystems::time::timestamp(),
                remove_pending: false,
            };

            let segment = Arc::new(Mutex::new(segment));
            segments.insert(key, segment.clone());
            Some(id)
        } else {
            None
        }
    }

    /// Find a segment by ID
    pub fn find_by_id(&self, id: i32) -> Option<Arc<Mutex<super::SharedMemorySegment>>> {
        let segments = self.segments.lock();
        for segment in segments.values() {
            if segment.lock().id == id {
                return Some(segment.clone());
            }
        }
        None
    }

    /// Remove a segment by key
    pub fn remove_by_key(&self, key: i32) -> bool {
        let mut segments = self.segments.lock();
        segments.remove(&key).is_some()
    }

    /// Get all segments
    pub fn get_all_keys(&self) -> alloc::vec::Vec<i32> {
        let segments = self.segments.lock();
        segments.keys().cloned().collect()
    }
}

impl Default for SharedMemoryService {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared Memory Service Factory
pub struct SharedMemoryServiceFactory;

impl SharedMemoryServiceFactory {
    /// Create a new factory
    pub fn new() -> Self {
        Self
    }
}

impl ServiceFactory for SharedMemoryServiceFactory {
    fn create(&self, _container: &nos_api::di::Container) -> nos_api::error::Result<alloc::boxed::Box<dyn core::any::Any + Send + Sync>> {
        Ok(alloc::boxed::Box::new(SharedMemoryService::new()))
    }

    fn type_id(&self) -> core::any::TypeId {
        core::any::TypeId::of::<SharedMemoryService>()
    }

    fn metadata(&self) -> &ServiceMetadata {
        static METADATA: ServiceMetadata = ServiceMetadata {
            name: "SharedMemoryService",
            version: "1.0.0",
            description: "Manages System V shared memory segments",
            dependencies: alloc::Vec::new(),
            scope: ServiceScope::Singleton,
            lazy: false,
        };
        &METADATA
    }
}
