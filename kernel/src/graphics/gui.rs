//! GUI framework integration
//!
//! Provides backend adapters for Rust GUI frameworks (Slint/Iced) to work with NOS graphics system.

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;
use crate::graphics::surface::{SurfaceId, SurfaceFormat, get_surface_manager};
use crate::graphics::input::{InputEvent, InputEventHandler, get_input_manager};
use crate::graphics::compositor::{get_compositor, composite_frame};
use crate::reliability::errno::{EINVAL, ENOMEM};

/// GUI framework backend trait
pub trait GuiBackend: Send + Sync {
    /// Initialize the backend
    fn init(&mut self, width: u32, height: u32) -> Result<(), i32>;
    
    /// Create a window/surface
    fn create_window(&mut self, title: &str, width: u32, height: u32) -> Result<SurfaceId, i32>;
    
    /// Get surface buffer address for rendering
    fn get_buffer_addr(&self, surface_id: SurfaceId) -> Option<usize>;
    
    /// Swap buffers (present frame)
    fn swap_buffers(&mut self, surface_id: SurfaceId) -> Result<(), i32>;
    
    /// Process input events
    fn process_events(&mut self) -> Result<(), i32>;
    
    /// Run main loop
    fn run(&mut self) -> Result<(), i32>;
}

/// Slint backend adapter
pub struct SlintBackend {
    /// Main surface ID
    main_surface: Option<SurfaceId>,
    /// Surface width
    width: u32,
    /// Surface height
    height: u32,
}

impl SlintBackend {
    /// Create a new Slint backend
    pub fn new() -> Self {
        Self {
            main_surface: None,
            width: 0,
            height: 0,
        }
    }
}

impl GuiBackend for SlintBackend {
    fn init(&mut self, width: u32, height: u32) -> Result<(), i32> {
        self.width = width;
        self.height = height;
        crate::println!("[gui] Slint backend initialized ({}x{})", width, height);
        Ok(())
    }
    
    fn create_window(&mut self, title: &str, width: u32, height: u32) -> Result<SurfaceId, i32> {
        let surface_manager = crate::graphics::surface::get_surface_manager();
        let surface_id = surface_manager.create_surface(width, height, SurfaceFormat::ARGB8888, 0)?;
        self.main_surface = Some(surface_id);
        crate::println!("[gui] Created Slint window '{}' (surface {})", title, surface_id);
        Ok(surface_id)
    }
    
    fn get_buffer_addr(&self, surface_id: SurfaceId) -> Option<usize> {
        let surface_manager = crate::graphics::surface::get_surface_manager();
        // Get back buffer address for rendering
        // In real implementation, this would return the actual buffer address
        None // Placeholder
    }
    
    fn swap_buffers(&mut self, surface_id: SurfaceId) -> Result<(), i32> {
        let surface_manager = crate::graphics::surface::get_surface_manager();
        // Swap buffers and trigger compositor
        // In real implementation, this would swap and mark dirty
        composite_frame()?;
        Ok(())
    }
    
    fn process_events(&mut self) -> Result<(), i32> {
        // Process input events
        // In real implementation, this would poll input devices
        Ok(())
    }
    
    fn run(&mut self) -> Result<(), i32> {
        // Main event loop
        // In real implementation, this would run until window is closed
        loop {
            self.process_events()?;
            // Render and composite
            if let Some(surface_id) = self.main_surface {
                self.swap_buffers(surface_id)?;
            }
            // Sleep to avoid busy-waiting
            crate::subsystems::time::sleep_ms(16); // ~60 FPS
        }
    }
}

/// Iced backend adapter
pub struct IcedBackend {
    /// Main surface ID
    main_surface: Option<SurfaceId>,
    /// Surface width
    width: u32,
    /// Surface height
    height: u32,
}

impl IcedBackend {
    /// Create a new Iced backend
    pub fn new() -> Self {
        Self {
            main_surface: None,
            width: 0,
            height: 0,
        }
    }
}

impl GuiBackend for IcedBackend {
    fn init(&mut self, width: u32, height: u32) -> Result<(), i32> {
        self.width = width;
        self.height = height;
        crate::println!("[gui] Iced backend initialized ({}x{})", width, height);
        Ok(())
    }
    
    fn create_window(&mut self, title: &str, width: u32, height: u32) -> Result<SurfaceId, i32> {
        let surface_manager = crate::graphics::surface::get_surface_manager();
        let surface_id = surface_manager.create_surface(width, height, SurfaceFormat::ARGB8888, 0)?;
        self.main_surface = Some(surface_id);
        crate::println!("[gui] Created Iced window '{}' (surface {})", title, surface_id);
        Ok(surface_id)
    }
    
    fn get_buffer_addr(&self, surface_id: SurfaceId) -> Option<usize> {
        // Get back buffer address for rendering
        None // Placeholder
    }
    
    fn swap_buffers(&mut self, surface_id: SurfaceId) -> Result<(), i32> {
        // Swap buffers and trigger compositor
        composite_frame()?;
        Ok(())
    }
    
    fn process_events(&mut self) -> Result<(), i32> {
        // Process input events
        Ok(())
    }
    
    fn run(&mut self) -> Result<(), i32> {
        // Main event loop
        loop {
            self.process_events()?;
            if let Some(surface_id) = self.main_surface {
                self.swap_buffers(surface_id)?;
            }
            crate::subsystems::time::sleep_ms(16); // ~60 FPS
        }
    }
}

/// GUI framework type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiFramework {
    Slint,
    Iced,
}

/// GUI manager - manages GUI framework backends
pub struct GuiManager {
    /// Active backend
    backend: Option<alloc::boxed::Box<dyn GuiBackend>>,
    /// Framework type
    framework: Option<GuiFramework>,
}

impl GuiManager {
    /// Create a new GUI manager
    pub fn new() -> Self {
        Self {
            backend: None,
            framework: None,
        }
    }
    
    /// Initialize GUI framework backend
    pub fn init_backend(&mut self, framework: GuiFramework, width: u32, height: u32) -> Result<(), i32> {
        let mut backend: alloc::boxed::Box<dyn GuiBackend> = match framework {
            GuiFramework::Slint => alloc::boxed::Box::new(SlintBackend::new()),
            GuiFramework::Iced => alloc::boxed::Box::new(IcedBackend::new()),
        };
        
        backend.init(width, height)?;
        self.backend = Some(backend);
        self.framework = Some(framework);
        
        crate::println!("[gui] Initialized {} backend", match framework {
            GuiFramework::Slint => "Slint",
            GuiFramework::Iced => "Iced",
        });
        
        Ok(())
    }
    
    /// Get backend
    pub fn get_backend(&mut self) -> Option<&mut dyn GuiBackend> {
        self.backend.as_mut().map(|b| b.as_mut())
    }
}

/// Global GUI manager instance
static GUI_MANAGER: crate::subsystems::sync::Mutex<Option<GuiManager>> = crate::subsystems::sync::Mutex::new(None);

/// Initialize GUI manager
pub fn init_gui_manager() -> Result<(), i32> {
    let mut manager = GUI_MANAGER.lock();
    if manager.is_none() {
        *manager = Some(GuiManager::new());
        crate::println!("[gui] GUI manager initialized");
    }
    Ok(())
}

/// Get GUI manager
pub fn get_gui_manager() -> &'static crate::subsystems::sync::Mutex<GuiManager> {
    static INIT_ONCE: crate::subsystems::sync::Once = crate::subsystems::sync::Once::new();
    INIT_ONCE.call_once(|| {
        let mut manager = GUI_MANAGER.lock();
        if manager.is_none() {
            *manager = Some(GuiManager::new());
        }
    });
    
    unsafe {
        &*(GUI_MANAGER.lock().as_ref().unwrap() as *const GuiManager as *const crate::subsystems::sync::Mutex<GuiManager>)
    }
}

