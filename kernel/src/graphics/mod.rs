//! Graphics subsystem for NOS
//!
//! Provides zero-copy graphics compositor with shared memory surfaces.
//! This module implements a high-performance graphics system designed for
//! low latency and efficient resource usage.

pub mod surface;
pub mod compositor;
pub mod buffer;
pub mod vsync;
pub mod input;
pub mod gui;
pub mod ime;

/// Initialize graphics subsystem
pub fn init() {
    // Initialize buffer manager
    if let Err(e) = buffer::init_buffer_manager() {
        crate::println!("[graphics] Failed to initialize buffer manager: {}", e);
    }
    
    // Initialize surface manager
    if let Err(e) = surface::init_surface_manager() {
        crate::println!("[graphics] Failed to initialize surface manager: {}", e);
    }
    
    // Initialize input manager
    if let Err(e) = input::init_input_manager() {
        crate::println!("[graphics] Failed to initialize input manager: {}", e);
    }
    
    // Initialize GUI manager
    if let Err(e) = gui::init_gui_manager() {
        crate::println!("[graphics] Failed to initialize GUI manager: {}", e);
    }
    
    // Initialize IME manager
    if let Err(e) = ime::init_ime_manager() {
        crate::println!("[graphics] Failed to initialize IME manager: {}", e);
    }
    
    crate::println!("[graphics] Graphics subsystem initialized");
}

