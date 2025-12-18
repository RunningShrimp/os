// Boot configuration and runtime settings

use alloc::string::String;

pub struct BootConfig {
    pub verbosity: u32,
    pub enable_post: bool,
    pub enable_paging: bool,
    pub enable_memory_check: bool,
    pub enable_device_detect: bool,
    pub enable_graphics: bool,
    pub graphics_width: u16,
    pub graphics_height: u16,
    pub graphics_bpp: u8,
    pub kernel_path: [u8; 256],
    pub kernel_path_len: usize,
    pub cmdline_buffer: [u8; 512],
    pub cmdline_len: usize,
}

impl BootConfig {
    pub fn new() -> Self {
        Self {
            verbosity: 1, // INFO level
            enable_post: true,
            enable_paging: true,
            enable_memory_check: true,
            enable_device_detect: true,
            enable_graphics: true,
            graphics_width: 1024,
            graphics_height: 768,
            graphics_bpp: 32,
            kernel_path: [0; 256],
            kernel_path_len: 0,
            cmdline_buffer: [0; 512],
            cmdline_len: 0,
        }
    }

    pub fn set_verbosity(&mut self, level: u32) {
        self.verbosity = level;
    }

    pub fn is_verbose(&self) -> bool {
        self.verbosity > 1
    }

    pub fn is_debug(&self) -> bool {
        self.verbosity > 2
    }

    pub fn get_kernel_path(&self) -> &[u8] {
        &self.kernel_path[..self.kernel_path_len]
    }

    pub fn set_kernel_path(&mut self, path: &[u8]) {
        let len = core::cmp::min(path.len(), 256);
        self.kernel_path[..len].copy_from_slice(&path[..len]);
        self.kernel_path_len = len;
    }

    pub fn get_cmdline(&self) -> &[u8] {
        &self.cmdline_buffer[..self.cmdline_len]
    }

    pub fn set_cmdline(&mut self, cmdline: &[u8]) {
        let len = core::cmp::min(cmdline.len(), 512);
        self.cmdline_buffer[..len]
            .copy_from_slice(&cmdline[..len]);
        self.cmdline_len = len;
    }

    /// Parse command-line flags using single-pass state machine
    ///
    /// Supports flexible parsing with:
    /// - Unknown parameter skipping (error-tolerant)
    /// - Strict resolution validation (320-4096 x 200-2160)
    /// - Dynamic allocation for values
    ///
    /// # Example command lines
    /// - `verbose no-graphics vga`
    /// - `graphics=1024x768@32 no-post`
    pub fn apply_cmdline_flags(&mut self, cmdline: &str) {
        let mut chars = cmdline.chars();
        let mut current_param = String::new();
        let mut in_value = false;
        
        loop {
            match chars.next() {
                Some('=') => {
                    in_value = true;
                }
                Some(' ') | Some('\t') | None => {
                    // End of parameter or value
                    if !current_param.is_empty() {
                        self.apply_single_flag(&current_param, in_value);
                    }
                    current_param.clear();
                    in_value = false;
                    
                    if chars.as_str().is_empty() {
                        break;
                    }
                }
                Some(c) => {
                    current_param.push(c);
                }
            }
        }
    }
    
    /// Apply a single flag from command line
    fn apply_single_flag(&mut self, flag: &str, is_value: bool) {
        if flag.is_empty() {
            return;
        }
        
        // Simple boolean flags
        match flag {
            "verbose" => self.verbosity = 2,
            "debug" => self.verbosity = 3,
            "no-post" => self.enable_post = false,
            "no-paging" => self.enable_paging = false,
            "no-memory-check" => self.enable_memory_check = false,
            "no-device-detect" => self.enable_device_detect = false,
            "no-graphics" => self.enable_graphics = false,
            "vga" => self.enable_graphics = false,
            _ => {
                // Try to parse graphics mode: WIDTHxHEIGHT@BPP
                if is_value && flag.contains('x') {
                    self.try_parse_graphics_mode(flag);
                }
                // Unknown flags are silently ignored (error-tolerant)
            }
        }
    }
    
    /// Try to parse graphics mode string
    /// Format: WIDTHxHEIGHT@BPP (e.g., "1024x768@32")
    fn try_parse_graphics_mode(&mut self, mode_str: &str) {
        let mut width_str = String::new();
        let mut height_str = String::new();
        let mut bpp_str = String::new();
        let mut state = 0; // 0=width, 1=height, 2=bpp
        
        for c in mode_str.chars() {
            match c {
                'x' | 'X' if state == 0 => {
                    state = 1;
                }
                '@' if state == 1 => {
                    state = 2;
                }
                '0'..='9' => {
                    match state {
                        0 => width_str.push(c),
                        1 => height_str.push(c),
                        2 => bpp_str.push(c),
                        _ => {}
                    }
                }
                _ => {
                    // Invalid character, stop parsing
                    return;
                }
            }
        }
        
        // Validate and apply parsed values
        if let (Ok(w), Ok(h)) = (width_str.parse::<u16>(), height_str.parse::<u16>()) {
            // Strict resolution validation: 320-4096 x 200-2160
            if w >= 320 && w <= 4096 && h >= 200 && h <= 2160 {
                self.graphics_width = w;
                self.graphics_height = h;
                
                // Parse BPP if provided, default to 32
                if !bpp_str.is_empty() {
                    if let Ok(bpp) = bpp_str.parse::<u8>() {
                        if bpp == 16 || bpp == 24 || bpp == 32 {
                            self.graphics_bpp = bpp;
                        }
                    }
                }
            }
        }
    }

    pub fn print_config(&self) {
        crate::drivers::console::write_str("Boot Configuration:\n");
        crate::drivers::console::write_str("  Verbosity: ");
        crate::drivers::console::write_str(match self.verbosity {
            0 => "Silent",
            1 => "Info",
            2 => "Verbose",
            _ => "Debug",
        });
        crate::drivers::console::write_str("\n");
        crate::drivers::console::write_str("  POST: ");
        crate::drivers::console::write_str(if self.enable_post {
            "enabled\n"
        } else {
            "disabled\n"
        });
        crate::drivers::console::write_str("  Paging: ");
        crate::drivers::console::write_str(if self.enable_paging {
            "enabled\n"
        } else {
            "disabled\n"
        });
        crate::drivers::console::write_str("  Graphics: ");
        crate::drivers::console::write_str(if self.enable_graphics {
            "enabled\n"
        } else {
            "disabled\n"
        });
        if self.enable_graphics {
            crate::drivers::console::write_str("  Graphics Mode: ");
            crate::drivers::console::write_str("Graphics Mode: ");
            // Simple width display
            if self.graphics_width >= 1000 {
                crate::drivers::console::write_str("high");
            } else if self.graphics_width >= 800 {
                crate::drivers::console::write_str("medium");
            } else {
                crate::drivers::console::write_str("low");
            }
            crate::drivers::console::write_str(" resolution\n");
        }
    }
}

impl Default for BootConfig {
    fn default() -> Self {
        Self::new()
    }
}

pub static mut BOOT_CONFIG: Option<BootConfig> = None;

pub fn init_config() -> &'static mut BootConfig {
    unsafe {
        BOOT_CONFIG = Some(BootConfig::new());
        match &mut *(&raw mut BOOT_CONFIG) {
            Some(config) => config,
            None => panic!("Failed to initialize config"),
        }
    }
}

pub fn get_config() -> Option<&'static mut BootConfig> {
    unsafe { (*(&raw mut BOOT_CONFIG)).as_mut() }
}
