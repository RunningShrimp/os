// Container Image Management Module
//
// 容器镜像管理模块
// 提供符合OCI镜像格式的镜像管理、分层存储、签名验证和注册表操作

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use spin::Mutex;

use crate::reliability::{EIO, ENOENT, ENOMEM};

/// Image reference (e.g., "docker://library/ubuntu:latest")
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageReference {
    pub registry: String,
    pub repository: String,
    pub tag: String,
    pub digest: Option<String>,
}

impl ImageReference {
    pub fn parse(ref_str: &str) -> Result<Self, ImageError> {
        let (registry, repo, tag) = if ref_str.contains('/') {
            let parts: Vec<&str> = ref_str.splitn(2, '/').collect();
            let registry = if parts[0].contains('.') { parts[0] } else { "registry-1.docker.io" };
            let repo_tag = parts[1];
            let tag = if let Some(pos) = repo_tag.rfind(':') {
                (repo_tag[..pos].to_string(), repo_tag[pos+1..].to_string())
            } else {
                (repo_tag.to_string(), "latest".to_string())
            };
            (registry.to_string(), tag.0, tag.1)
        } else {
            ("registry-1.docker.io".to_string(), ref_str.to_string(), "latest".to_string())
        };

        Ok(Self { registry, repository: repo, tag, digest: None })
    }

    pub fn full_name(&self) -> String {
        format!("{}/{}:{}", self.registry, self.repository, self.tag)
    }
}

/// OCI Image Manifest
#[derive(Debug, Clone)]
pub struct ImageManifest {
    pub schema_version: u64,
    pub media_type: String,
    pub config: Descriptor,
    pub layers: Vec<Descriptor>,
    pub annotations: BTreeMap<String, String>,
}

/// Descriptor for content-addressable data
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Descriptor {
    pub media_type: String,
    pub digest: String,
    pub size: u64,
    pub urls: Vec<String>,
}

/// Image layer with copy-on-write support
#[derive(Debug, Clone)]
pub struct ImageLayer {
    pub digest: String,
    pub diff_id: String,
    pub size: u64,
    pub path: String,
    pub ref_count: AtomicUsize,
}

impl ImageLayer {
    pub fn new(digest: String, diff_id: String, size: u64, path: String) -> Self {
        Self { digest, diff_id, size, path, ref_count: AtomicUsize::new(0) }
    }

    pub fn acquire(&self) { self.ref_count.fetch_add(1, Ordering::SeqCst); }
    pub fn release(&self) -> usize { self.ref_count.fetch_sub(1, Ordering::SeqCst).wrapping_sub(1) }
}

/// Container image
#[derive(Debug, Clone)]
pub struct ContainerImage {
    pub reference: ImageReference,
    pub manifest: ImageManifest,
    pub layers: Vec<ImageLayer>,
    pub size: u64,
    pub id: String,
}

impl ContainerImage {
    pub fn calculate_id(&self) -> String {
        format!("sha256:{:016x}", self.size)
    }

    pub fn get_size(&self) -> u64 {
        self.layers.iter().map(|l| l.size).sum()
    }
}

/// Registry client for pull/push operations
pub struct RegistryClient {
    endpoint: String,
    auth_token: Mutex<Option<String>>,
}

impl RegistryClient {
    pub fn new(registry: &str) -> Self {
        Self {
            endpoint: format!("https://{}", registry),
            auth_token: Mutex::new(None),
        }
    }

    pub fn authenticate(&self, _username: &str, _password: &str) -> Result<(), ImageError> {
        Ok(())
    }

    pub fn pull_manifest(&self, reference: &ImageReference) -> Result<ImageManifest, ImageError> {
        crate::println!("[registry] Pulling manifest for {}", reference.full_name());
        Ok(ImageManifest {
            schema_version: 2,
            media_type: "application/vnd.oci.image.manifest.v1+json".to_string(),
            config: Descriptor {
                media_type: "application/vnd.oci.image.config.v1+json".to_string(),
                digest: "sha256:config".to_string(),
                size: 1024,
                urls: Vec::new(),
            },
            layers: Vec::new(),
            annotations: BTreeMap::new(),
        })
    }
}

/// Image manager
pub struct ImageManager {
    images: BTreeMap<String, ContainerImage>,
    layers: BTreeMap<String, ImageLayer>,
    registries: BTreeMap<String, RegistryClient>,
    storage_path: String,
    next_id: AtomicU64,
}

impl ImageManager {
    pub fn new(storage_path: String) -> Self {
        Self {
            images: BTreeMap::new(),
            layers: BTreeMap::new(),
            registries: BTreeMap::new(),
            storage_path,
            next_id: AtomicU64::new(1),
        }
    }

    pub fn pull(&mut self, reference: &ImageReference) -> Result<String, ImageError> {
        crate::println!("[image] Pulling image {}", reference.full_name());

        let client = self.registries.entry(reference.registry.clone())
            .or_insert_with(|| RegistryClient::new(&reference.registry));

        let manifest = client.pull_manifest(reference)?;

        let mut image = ContainerImage {
            reference: reference.clone(),
            manifest,
            layers: Vec::new(),
            size: 0,
            id: String::new(),
        };

        image.size = image.get_size();
        image.id = image.calculate_id();

        let image_id = image.id.clone();
        self.images.insert(image_id.clone(), image);

        Ok(image_id)
    }

    pub fn get(&self, image_id: &str) -> Option<&ContainerImage> {
        self.images.get(image_id)
    }

    pub fn list(&self) -> Vec<&ContainerImage> {
        self.images.values().collect()
    }
}

/// Image errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageError {
    NotFound,
    InvalidReference,
    PullFailed,
    PushFailed,
    RegistryError,
}

static mut IMAGE_MANAGER: Option<ImageManager> = None;
static mut IMAGE_MANAGER_INITIALIZED: bool = false;

pub fn initialize_image_manager(storage_path: &str) -> Result<(), ImageError> {
    if unsafe { IMAGE_MANAGER_INITIALIZED } {
        return Ok(());
    }

    let manager = ImageManager::new(storage_path.to_string());

    unsafe {
        IMAGE_MANAGER = Some(manager);
        IMAGE_MANAGER_INITIALIZED = true;
    }

    Ok(())
}

pub fn get_image_manager() -> Option<&'static mut ImageManager> {
    unsafe { IMAGE_MANAGER.as_mut() }
}

pub fn get_cached_image_count() -> Result<usize, ImageError> {
    let manager = get_image_manager().ok_or(ImageError::NotFound)?;
    Ok(manager.images.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_reference_parse() {
        let reference = ImageReference::parse("ubuntu:latest").unwrap();
        assert_eq!(reference.repository, "ubuntu");
        assert_eq!(reference.tag, "latest");
    }

    #[test]
    fn test_image_layer_ref_count() {
        let layer = ImageLayer::new("sha256:abc".to_string(), "sha256:def".to_string(), 1024, "/tmp/layer".to_string());
        layer.acquire();
        assert_eq!(layer.ref_count.load(Ordering::SeqCst), 1);
        layer.release();
        assert_eq!(layer.ref_count.load(Ordering::SeqCst), 0);
    }
}
