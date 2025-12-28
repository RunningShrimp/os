//! Container Image Management
//!
//! This module implements container image support for NOS:
//! - Image layers
//! - Image storage
//! - Image pull/push
//! - Image building
//!
//! Features:
//! - Docker/OCI-compliant image format
//! - Multi-layer images
//! - Image signing and verification
//! - Image caching

use spin::Mutex;
use core::sync::atomic;
use alloc::collections::BTreeMap;
use core::sync::atomic;
use alloc::string::String;
use core::sync::atomic;
use alloc::collections::BTreeSet;
use core::sync::atomic;
use alloc::sync::Arc;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use alloc::string::{String, ToString};
use core::sync::atomic;

// ============================================================================
// Container Image Constants
// ============================================================================

/// Maximum number of images
pub const MAX_IMAGES: usize = 1 << 10; // 1024 images

/// Maximum image name length
pub const MAX_IMAGE_NAME_LENGTH: usize = 256;

/// Default image layer size (bytes)
pub const DEFAULT_LAYER_SIZE: usize = 64 * 1024 * 1024; // 64MB

// ============================================================================
// Image Layer
// ============================================================================

/// Container image layer
#[derive(Debug, Clone)]
pub struct ImageLayer {
    /// Layer ID
    pub layer_id: u32,
    
    /// Layer digest (SHA256)
    pub digest: String,
    
    /// Layer size (bytes)
    pub size: usize,
    
    /// Layer data (compressed)
    pub data: Vec<u8>,
    
    /// Layer media type (tar, tar+gzip)
    pub media_type: String,
    
    /// Layer command
    pub command: Option<String>,
    
    /// Creation time
    pub created_at: u64,
    
    /// Parent layer (for chained layers)
    pub parent_layer: Option<u32>,
    
    /// Layer is cached
    pub cached: AtomicBool,
}

impl ImageLayer {
    /// Create new image layer
    pub fn new(layer_id: u32, digest: String, media_type: String,
               data: Vec<u8>, parent: Option<u32>) -> Self {
        
        Self {
            layer_id,
            digest,
            size: data.len(),
            data,
            media_type,
            command: None,
            created_at: crate::subsystems::time::timestamp_nanos(),
            parent_layer: parent,
            cached: AtomicBool::new(false),
        }
    }
    
    /// Set as cached
    pub fn set_cached(&self) {
        self.cached.store(true, Ordering::Release);
    }
    
    /// Check if cached
    pub fn is_cached(&self) -> bool {
        self.cached.load(Ordering::Acquire)
    }
}

// ============================================================================
// Container Image
// ============================================================================

/// Container image
#[derive(Debug, Clone)]
pub struct ContainerImage {
    /// Image ID
    pub image_id: u32,
    
    /// Image name
    pub name: String,
    
    /// Image tag
    pub tag: String,
    
    /// Image digest (manifest digest)
    pub digest: String,
    
    /// Image layers (bottom to top)
    pub layers: Vec<ImageLayer>,
    
    /// Image size (total)
    pub size: usize,
    
    /// Image architecture
    pub architecture: String,
    
    /// Image OS
    pub os: String,
    
    /// Config (OCI image config)
    pub config: Option<ImageConfig>,
    
    /// Creation time
    pub created_at: u64,
    
    /// Last pull time
    pub last_pulled_at: AtomicU64,
    
    /// Image is signed
    pub signed: bool,
    
    /// Image signature
    pub signature: Option<String>,
    
    /// Image statistics
    pub stats: Mutex<ImageStats>,
}

/// Image configuration (OCI spec)
#[derive(Debug, Clone)]
pub struct ImageConfig {
    /// Entrypoint
    pub entrypoint: Option<String>,
    
    /// Command
    pub cmd: Vec<String>,
    
    /// Working directory
    pub workdir: Option<String>,
    
    /// Environment variables
    pub env: Vec<String>,
    
    /// Exposed ports
    pub exposed_ports: BTreeSet<u16>,
    
    /// Volumes
    pub volumes: BTreeSet<String>,
    
    /// Labels
    pub labels: BTreeMap<String, String>,
    
    /// Stop signal
    pub stop_signal: String,
    
    /// Stop timeout (seconds)
    pub stop_timeout: u32,
}

/// Image statistics
#[derive(Debug, Clone, Copy)]
pub struct ImageStats {
    /// Number of layers
    pub num_layers: usize,
    
    /// Total size (bytes)
    pub total_size: usize,
    
    /// Number of pulls
    pub num_pulls: u64,
    
    /// Number of uses (containers started)
    pub num_uses: u64,
    
    /// Cache hit rate
    pub cache_hit_rate: f64,
}

impl Default for ImageStats {
    fn default() -> Self {
        Self {
            num_layers: 0,
            total_size: 0,
            num_pulls: 0,
            num_uses: 0,
            cache_hit_rate: 0.0,
        }
    }
}

impl ContainerImage {
    /// Create new container image
    pub fn new(image_id: u32, name: String, tag: String, digest: String,
               layers: Vec<ImageLayer>, config: Option<ImageConfig>) -> Self {
        
        let total_size: usize = layers.iter().map(|l| l.size).sum();
        
        Self {
            image_id,
            name,
            tag,
            digest,
            layers,
            size: total_size,
            architecture: String::from("amd64"),
            os: String::from("linux"),
            config,
            created_at: crate::subsystems::time::timestamp_nanos(),
            last_pulled_at: AtomicU64::new(0),
            signed: false,
            signature: None,
            stats: Mutex::new(ImageStats {
                num_layers: layers.len(),
                total_size,
                num_pulls: 0,
                num_uses: 0,
                cache_hit_rate: 0.0,
            }),
        }
    }
    
    /// Record pull
    pub fn record_pull(&self) {
        self.last_pulled_at.store(crate::subsystems::time::timestamp_nanos(), 
                                  Ordering::Release);
        
        let mut stats = self.stats.lock();
        stats.num_pulls += 1;
        
        crate::println!("[image] Recorded pull for image {}", self.image_id);
    }
    
    /// Record use
    pub fn record_use(&self) {
        let mut stats = self.stats.lock();
        stats.num_uses += 1;
        
        crate::println!("[image] Recorded use for image {}", self.image_id);
    }
    
    /// Get image statistics
    pub fn get_stats(&self) -> ImageStats {
        let mut stats = self.stats.lock();
        
        // Calculate cache hit rate
        let cached_layers = self.layers.iter().filter(|l| l.is_cached()).count();
        stats.cache_hit_rate = if self.layers.len() > 0 {
            cached_layers as f64 / self.layers.len() as f64
        } else {
            0.0
        };
        
        *stats
    }
}

/// Image manager
pub struct ImageManager {
    /// All images
    pub images: Mutex<BTreeMap<u32, Arc<ContainerImage>>>,
    
    /// Images by name:tag
    pub images_by_name_tag: Mutex<BTreeMap<String, BTreeSet<u32>>>,
    
    /// Layers (deduplicated)
    pub layers: Mutex<BTreeMap<u32, Arc<ImageLayer>>>,
    
    /// Next image ID
    pub next_image_id: AtomicU32,
    
    /// Next layer ID
    pub next_layer_id: AtomicU32,
    
    /// Total images
    pub total_images: AtomicUsize,
    
    /// Manager statistics
    pub stats: Mutex<ImageManagerStats>,
}

/// Image manager statistics
#[derive(Debug, Clone, Copy)]
pub struct ImageManagerStats {
    pub total_images: usize,
    pub total_layers: usize,
    pub total_size: usize,
    pub cached_images: usize,
    pub signed_images: usize,
}

impl Default for ImageManagerStats {
    fn default() -> Self {
        Self {
            total_images: 0,
            total_layers: 0,
            total_size: 0,
            cached_images: 0,
            signed_images: 0,
        }
    }
}

impl ImageManager {
    /// Create new image manager
    pub fn new() -> Self {
        Self {
            images: Mutex::new(BTreeMap::new()),
            images_by_name_tag: Mutex::new(BTreeMap::new()),
            layers: Mutex::new(BTreeMap::new()),
            next_image_id: AtomicU32::new(1),
            next_layer_id: AtomicU32::new(1),
            total_images: AtomicUsize::new(0),
            stats: Mutex::new(ImageManagerStats::default()),
        }
    }
    
    /// Pull image
    pub fn pull_image(&self, name: String, tag: String) 
        -> Result<u32, ImageError> {
        
        // In real implementation, would:
        // 1. Fetch image manifest from registry
        // 2. Download layers
        // 3. Verify signatures
        // 4. Cache layers
        
        let image_id = self.next_image_id.fetch_add(1, Ordering::Relaxed);
        
        // Create image (simplified)
        let digest = String::from("sha256:abc123");
        let layers = Vec::new();
        let config = None;
        
        let image = Arc::new(ContainerImage::new(image_id, name, tag, digest, 
                                                    layers, config));
        
        let mut images = self.images.lock();
        images.insert(image_id, image);
        self.total_images.fetch_add(1, Ordering::Relaxed);
        
        // Add to name:tag index
        let name_tag = { let mut s = alloc::string::String::from("{}:"); s.push_str(&name, tag.to_string()); s };
        let mut by_name_tag = self.images_by_name_tag.lock();
        by_name_tag.entry(name_tag).or_insert_with(BTreeSet::new).insert(image_id);
        
        // Record pull
        image.record_pull();
        
        crate::println!("[image] Pulled image {}:{} (ID: {})",
                        name, tag, image_id);
        
        Ok(image_id)
    }
    
    /// Get image by ID
    pub fn get_image(&self, image_id: u32) -> Option<Arc<ContainerImage>> {
        let images = self.images.lock();
        images.get(&image_id).cloned()
    }
    
    /// Get image by name:tag
    pub fn get_image_by_name_tag(&self, name: String, tag: String) 
        -> Option<Arc<ContainerImage>> {
        
        let name_tag = { let mut s = alloc::string::String::from("{}:"); s.push_str(&name, tag.to_string()); s };
        let by_name_tag = self.images_by_name_tag.lock();
        let images = self.images.lock();
        
        by_name_tag.get(&name_tag)
            .and_then(|ids| ids.iter().next())
            .and_then(|id| images.get(id).cloned())
    }
    
    /// Delete image
    pub fn delete_image(&self, image_id: u32) -> Result<(), ImageError> {
        let mut images = self.images.lock();
        
        if let Some(image) = images.remove(&image_id) {
            // Remove from name:tag index
            let name_tag = { let mut s = alloc::string::String::from("{}:"); s.push_str(&image.name, image.tag.to_string()); s };
            let mut by_name_tag = self.images_by_name_tag.lock();
            if let Some(ids) = by_name_tag.get_mut(&name_tag) {
                ids.remove(&image_id);
            }
            
            crate::println!("[image] Deleted image {} ({}:{})",
                            image_id, image.name, image.tag);
            
            Ok(())
        } else {
            Err(ImageError::ImageNotFound { image_id })
        }
    }
    
    /// Get manager statistics
    pub fn get_stats(&self) -> ImageManagerStats {
        let mut stats = self.stats.lock();
        
        stats.total_images = self.total_images.load(Ordering::Relaxed);
        
        let images = self.images.lock();
        let layers = self.layers.lock();
        
        stats.total_layers = layers.len();
        
        for image in images.values() {
            stats.total_size += image.size;
            
            if image.signed {
                stats.signed_images += 1;
            }
            
            for layer in &image.layers {
                if layer.is_cached() {
                    stats.cached_images += 1;
                }
            }
        }
        
        *stats
    }
}

/// Image error
#[derive(Debug, Clone)]
pub enum ImageError {
    /// Image not found
    ImageNotFound {
        image_id: u32,
    },
    
    /// Pull failed
    PullFailed {
        reason: String,
    },
    
    /// Invalid manifest
    InvalidManifest,
    
    /// Layer verification failed
    LayerVerificationFailed {
        layer_id: u32,
    },
    
    /// Storage error
    StorageError {
        reason: String,
    },
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_layer() {
        let layer = ImageLayer::new(
            1,
            String::from("sha256:abc123"),
            String::from("application/vnd.oci.image.layer.v1.tar+gzip"),
            {
    let mut v = alloc::vec::Vec::new();
    v.push(0x01);
    v.push(0x02);
    v.push(0x03);
    v
},
            None
        );
        
        assert_eq!(layer.layer_id, 1);
        assert_eq!(layer.size, 3);
        assert!(!layer.is_cached());
        
        layer.set_cached();
        assert!(layer.is_cached());
    }

    #[test]
    fn test_container_image() {
        let image = ContainerImage::new(
            1,
            String::from("ubuntu"),
            String::from("latest"),
            String::from("sha256:manifest123"),
            Vec::new(),
            None
        );
        
        assert_eq!(image.image_id, 1);
        assert_eq!(image.name, "ubuntu");
        assert_eq!(image.tag, "latest");
    }

    #[test]
    fn test_image_manager() {
        let manager = ImageManager::new();
        
        let image_id = manager.pull_image(
            String::from("ubuntu"),
            String::from("latest")
        ).unwrap();
        
        let image = manager.get_image(image_id).unwrap();
        assert_eq!(image.image_id, image_id);
        
        let image_by_tag = manager.get_image_by_name_tag(
            String::from("ubuntu"),
            String::from("latest")
        ).unwrap();
        
        assert_eq!(image_by_tag.image_id, image_id);
    }
}
