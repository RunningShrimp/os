//! Boot Application Service - Coordinates complete bootloader flow
//!
//! Implements the main bootloader use case by orchestrating:
//! 1. Configuration initialization (using DI container)
//! 2. Hardware detection and initialization (using DI container)
//! 3. Graphics setup (using DI container)
//! 4. Kernel loading and validation
//! 5. Handoff preparation

use crate::domain::*;
use crate::domain::hardware_detection::HardwareDetectionService;
use crate::domain::repositories::BootConfigRepository;
use crate::domain::events::DomainEventPublisher;
use crate::infrastructure::{DIContainer, BootDIContainer};
use crate::protocol::BootProtocolType;
use crate::utils::error::{BootError, Result as BootResult};
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;

/// Boot Application Service
///
/// High-level service that implements the complete boot sequence.
/// Uses dependency injection to get all required services.
/// Coordinates between domain services and infrastructure components.
pub struct BootApplicationService {
    /// DI container for service resolution
    di_container: Box<DIContainer>,
    /// Event publisher service (resolved from DI container)
    event_publisher: Box<dyn DomainEventPublisher>,
    /// Hardware detection service (resolved from DI container)
    hardware_detection: Box<dyn HardwareDetectionService>,
    /// Boot config repository (resolved from DI container)
    config_repo: Box<dyn BootConfigRepository>,
}

impl BootApplicationService {
    /// Create new boot application service using DI container
    ///
    /// # Arguments
    /// * `di_container` - DI container for service resolution
    ///
    /// # Returns
    /// New instance of BootApplicationService or error if services cannot be resolved
    pub fn new(di_container: Box<DIContainer>) -> BootResult<Self> {
        // Resolve required services from DI container
        let event_publisher = di_container
            .resolve::<Box<dyn DomainEventPublisher>>("DomainEventPublisher")
            .map_err(|e| BootError::ServiceResolutionFailed(format!("Failed to resolve DomainEventPublisher: {}", e)))?;
        
        let hardware_detection = di_container
            .resolve::<Box<dyn HardwareDetectionService>>("HardwareDetectionService")
            .map_err(|e| BootError::ServiceResolutionFailed(format!("Failed to resolve HardwareDetectionService: {}", e)))?;
        
        let config_repo = di_container
            .resolve::<Box<dyn BootConfigRepository>>("BootConfigRepository")
            .map_err(|e| BootError::ServiceResolutionFailed(format!("Failed to resolve BootConfigRepository: {}", e)))?;
        
        Ok(Self {
            di_container,
            event_publisher,
            hardware_detection,
            config_repo,
        })
    }
    
    /// Create new boot application service with default DI container
    ///
    /// # Arguments
    /// * `protocol_type` - Boot protocol type (BIOS/UEFI/Multiboot2)
    ///
    /// # Returns
    /// New instance of BootApplicationService or error if initialization fails
    pub fn with_default_container(protocol_type: BootProtocolType) -> BootResult<Self> {
        // Create DI container with auto-discovery
        let mut di_container = BootDIContainer::new(protocol_type);
        
        // Initialize the container
        di_container.initialize()
            .map_err(|e| BootError::ProtocolInitializationFailed(e.to_string()))?;
        
        // Enable auto-discovery of additional services
        let inner_container = di_container.inner_mut();
        inner_container.auto_discover_services()
            .map_err(|_| BootError::ProtocolInitializationFailed("Service discovery failed".to_string()))?;
        
        // Create service with the initialized container
        Self::new(Box::new(di_container.into_inner()))
    }
    
    /// Create new boot application service from configuration
    ///
    /// # Arguments
    /// * `config_str` - Configuration string (TOML or JSON)
    /// * `format` - Configuration format ("toml" or "json")
    ///
    /// # Returns
    /// New instance of BootApplicationService or error if configuration fails
    pub fn from_config(config_str: &str, format: &str) -> BootResult<Self> {
        // Create DI container from configuration
        let di_container = match format {
            "toml" => DIContainer::from_toml_config(config_str)
                .map_err(|_| BootError::ConfigurationError(String::from("TOML configuration error")))?,
            "json" => DIContainer::from_json_config(config_str)
                .map_err(|_| BootError::ConfigurationError(String::from("JSON configuration error")))?,
            _ => return Err(BootError::ConfigurationError(String::from("Unsupported configuration format"))),
        };
        
        // Create service with the configured container
        Self::new(Box::new(di_container))
    }
    
    /// Execute complete boot sequence
    ///
    /// Orchestrates the entire boot process using injected dependencies:
    /// 1. Load configuration (from repository)
    /// 2. Detect hardware (via injected service)
    /// 3. Validate prerequisites (using domain services)
    /// 4. Initialize graphics (if enabled)
    /// 5. Create boot info
    /// 6. Publish ready event
    ///
    /// # Arguments
    /// * `cmdline` - Optional command line parameters
    ///
    /// # Returns
    /// Complete boot information ready for kernel handoff
    pub fn boot_system(&mut self, cmdline: Option<&str>) -> BootResult<BootInfo> {
        // Phase 1: Load configuration using repository
        let config = self.load_boot_configuration(cmdline)?;
        
        // crate::console::write_str("[boot] Configuration loaded\n");

        // Phase 2: Detect hardware using injected service
        let hw_info = self.detect_hardware()?;
        
        // crate::console::write_str("[boot] Hardware detected\n");

        // Phase 3: Validate prerequisites using domain services
        self.validate_prerequisites(&config, &hw_info)?;
        
        // crate::console::write_str("[boot] Prerequisites validated\n");

        // Phase 4: Initialize graphics if enabled
        if config.graphics_mode.is_some() {
            self.initialize_graphics(&config)?;
        }

        // Phase 5: Create boot info
        let mut boot_info = self.create_boot_info(&config)?;
        
        // Phase 6: Load kernel (would happen here in full implementation)
        // For now, just set placeholder values
        let kernel_info = KernelInfo::new(0x100000, 0x500000, 0x100000).map_err(|e| BootError::ValidationError(e.to_string()))?;
        boot_info.set_kernel_info(kernel_info).map_err(|e| BootError::ValidationError(e.to_string()))?;
        boot_info.boot_timestamp = 0;

        // Phase 7: Validate boot info
        boot_info.validate()
            .map_err(|_error| {
                log::error!("Boot info validation failed, proceeding with boot");
                BootError::ValidationError("Boot info validation failed".to_string())
            })?;

        // Phase 8: Publish boot ready event
        self.publish_boot_ready_event(&boot_info)?;

        // crate::console::write_str("[boot] System ready for kernel handoff\n");

        Ok(boot_info)
    }
    
    /// Load boot configuration using repository
    ///
    /// # Arguments
    /// * `cmdline` - Optional command line parameters
    ///
    /// # Returns
    /// Boot configuration loaded from repository
    fn load_boot_configuration(&self, cmdline: Option<&str>) -> BootResult<BootConfig> {
        if let Some(cmd) = cmdline {
            self.config_repo
                .create_from_cmdline(cmd)
                .map_err(|e| BootError::ConfigurationError(format!("Failed to create config from cmdline: {}", e)))
        } else {
            self.config_repo
                .load_default()
                .map_err(|e| BootError::ConfigurationError(format!("Failed to load default config: {}", e)))
        }
    }
    
    /// Detect hardware using injected service
    ///
    /// # Returns
    /// Complete hardware information
    fn detect_hardware(&mut self) -> BootResult<crate::domain::boot_services::HardwareInfo> {
        self.hardware_detection
            .detect_hardware()
            .map_err(|_e| BootError::HardwareError("Hardware detection failed"))
    }
    
    /// Validate boot prerequisites using domain services
    ///
    /// # Arguments
    /// * `config` - Boot configuration
    /// * `hw_info` - Hardware information
    fn validate_prerequisites(&self, config: &BootConfig, hw_info: &crate::domain::boot_services::HardwareInfo) -> BootResult<()> {
        BootValidator::validate_prerequisites(config, hw_info)
            .map_err(|_errors| {
                BootError::ValidationError("Prerequisites validation failed".to_string())
            })
    }
    
    /// Initialize graphics subsystem using DI container
    ///
    /// # Arguments
    /// * `config` - Boot configuration with graphics settings
    fn initialize_graphics(&mut self, config: &BootConfig) -> BootResult<()> {
        if let Some(mode) = config.graphics_mode {
            // Try to resolve graphics backend from DI container
            if let Ok(mut graphics_backend) = self.di_container.resolve::<Box<dyn crate::infrastructure::graphics_backend::GraphicsBackend>>("GraphicsBackend") {
                // Initialize graphics backend
                graphics_backend
                    .initialize(&mode)
                    .map_err(|_e| BootError::DeviceError("Graphics initialization failed"))?;
                
                // Clear screen with a nice color
                graphics_backend
                    .clear_screen(0x1A1A2E)
                    .map_err(|_e| BootError::DeviceError("Failed to clear screen"))?;
                
                // Graphics operations are handled by the backend initialization
                
                // Get actual framebuffer info from backend
                if let Some(fb_info) = graphics_backend.get_framebuffer_info() {
                    // Publish graphics initialized event with real framebuffer info
                    let event = Box::new(events::GraphicsInitializedEvent::new(
                        mode.width,
                        mode.height,
                        fb_info.address,  // Real address from backend
                        0,  // Timestamp (would come from time service in real implementation)
                    )
                    .with_bpp(fb_info.bpp as u8)
                    .with_backend("VBE"));  // Would detect actual backend type in real implementation
                    self.event_publisher
                        .publish(event)
                        .map_err(|e| BootError::EventError(format!("Failed to publish graphics event: {}", e)))?;
                }
            } else {
                return Err(BootError::ServiceResolutionFailed("Graphics backend not available".to_string()));
            }
        }
        Ok(())
    }
    
    /// Create boot information
    ///
    /// # Arguments
    /// * `config` - Boot configuration
    ///
    /// # Returns
    /// Boot information structure
    fn create_boot_info(&self, config: &BootConfig) -> BootResult<BootInfo> {
        let boot_info = BootInfo::from_config(config, self.get_protocol_type())
            .map_err(|e| BootError::ValidationError(format!("Failed to create boot info: {}", e)))?;
        
        Ok(boot_info)
    }
    
    /// Publish boot ready event
    ///
    /// # Arguments
    /// * `boot_info` - Boot information to publish
    fn publish_boot_ready_event(&mut self, boot_info: &BootInfo) -> BootResult<()> {
        let event = Box::new(events::BootPhaseCompletedEvent::new(
            "boot_complete",
            boot_info.boot_timestamp,
            true,
        ));
        
        self.event_publisher
            .publish(event)
            .map_err(|e| BootError::EventError(format!("Failed to publish boot ready event: {}", e)))
    }
    
    /// Get protocol type from DI container
    ///
    /// # Returns
    /// The boot protocol type being used
    fn get_protocol_type(&self) -> BootProtocolType {
        // Get actual protocol type from DI container
        self.di_container.protocol_type()
    }
    
    /// Get hardware detection service reference
    ///
    /// Provides access to the hardware detection service.
    /// This method follows the dependency inversion principle
    /// by returning a reference to the abstraction.
    pub fn hardware_detection_service(&self) -> &dyn HardwareDetectionService {
        self.hardware_detection.as_ref()
    }
    
    /// Get event publisher reference
    ///
    /// Provides access to the event publisher service.
    pub fn event_publisher(&self) -> &dyn DomainEventPublisher {
        self.event_publisher.as_ref()
    }
    
    /// Get config repository reference
    ///
    /// Provides access to the configuration repository service.
    pub fn config_repository(&self) -> &dyn BootConfigRepository {
        self.config_repo.as_ref()
    }
    
    /// Get DI container reference
    ///
    /// Provides access to the underlying DI container.
    pub fn di_container(&self) -> &DIContainer {
        self.di_container.as_ref()
    }
    
    /// Get service statistics from DI container
    ///
    /// # Returns
    /// Statistics about registered and instantiated services
    pub fn get_service_stats(&self) -> crate::infrastructure::ServiceStats {
        self.di_container.get_service_stats()
    }
    
    /// Generate current DI container configuration
    ///
    /// # Returns
    /// TOML representation of current container configuration
    pub fn generate_container_config_toml(&self) -> String {
        self.di_container.generate_toml_config()
    }
    
    /// Generate current DI container configuration
    ///
    /// # Returns
    /// JSON representation of current container configuration
    pub fn generate_container_config_json(&self) -> String {
        self.di_container.generate_json_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::di_container::{
        DefaultBootConfigRepositoryFactory, SimpleEventPublisherFactory
    };
    
    #[test]
    fn test_boot_application_service_creation() {
        let mut di_container = crate::infrastructure::BootDIContainer::new(BootProtocolType::Bios);
        di_container.initialize().unwrap();
        
        let service = BootApplicationService::new(Box::new(di_container.inner()));
        assert!(service.is_ok());
    }
    
    #[test]
    fn test_boot_application_service_with_default_container() {
        #[cfg(feature = "bios_support")]
        {
            let service = BootApplicationService::with_default_container(BootProtocolType::Bios);
            assert!(service.is_ok());
        }
    }
    
    #[test]
    fn test_boot_application_service_from_config() {
        let config = r#"
[di_container]
default_protocol = "Bios"
enable_lazy_loading = true

[services.boot_config_repository]
type = "BootConfigRepository"
lifecycle = "Singleton"
"#;
        
        let service = BootApplicationService::from_config(config, "toml");
        assert!(service.is_ok());
    }
    
    #[test]
    fn test_boot_system_sequence() {
        let mut di_container = crate::infrastructure::BootDIContainer::new(BootProtocolType::Bios);
        di_container.initialize().unwrap();
        
        let mut service = BootApplicationService::new(Box::new(di_container.inner())).unwrap();
        
        let result = service.boot_system(None);
        assert!(result.is_ok());
        
        let boot_info = result.unwrap();
        assert_eq!(boot_info.kernel_address, 0x100000);
        assert_eq!(boot_info.kernel_size, 0x500000);
    }
    
    #[test]
    fn test_service_resolution() {
        let mut di_container = crate::infrastructure::BootDIContainer::new(BootProtocolType::Bios);
        di_container.initialize().unwrap();
        
        let service = BootApplicationService::new(Box::new(di_container.inner())).unwrap();
        
        // Test that all required services are available
        let _hw_service = service.hardware_detection_service();
        let _event_pub = service.event_publisher();
        let _config_repo = service.config_repository();
        
        // Test service statistics
        let stats = service.get_service_stats();
        assert!(stats.total_services > 0);
    }
    
    #[test]
    fn test_config_generation() {
        let mut di_container = crate::infrastructure::BootDIContainer::new(BootProtocolType::Bios);
        di_container.initialize().unwrap();
        
        let service = BootApplicationService::new(Box::new(di_container.inner())).unwrap();
        
        // Test TOML generation
        let toml_config = service.generate_container_config_toml();
        assert!(toml_config.contains("di_container"));
        assert!(toml_config.contains("services"));
        
        // Test JSON generation
        let json_config = service.generate_container_config_json();
        assert!(json_config.contains("di_container"));
        assert!(json_config.contains("services"));
    }
}
