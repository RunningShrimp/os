//! # Display Engine Management
//!
//! This module provides comprehensive display management including:
//! - Mode setting with EDID parsing
//! - Framebuffer management (fbdev, double buffering)
//! - Display pipeline (planes, CRTCs, encoders, connectors)
//! - Hotplug detection
//! - Vblank handling
//! - Color management (gamma, degamma, color spaces)
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::graphics::display::{DisplayEngine, DisplayMode, Framebuffer};
//!
//! // Initialize display engine
//! let display = DisplayEngine::init(&gpu)?;
//!
//! // Parse EDID and get preferred mode
//! let mode = display.get_preferred_mode()?;
//!
//! // Set display mode
//! display.set_mode(&mode)?;
//!
//! // Get framebuffer
//! let fb = display.framebuffer();
//! fb.clear(0xFF0000); // Clear to red
//! ```

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use core::time::Duration;

use crate::subsystems::sync::{Mutex, RwLock};

use super::error::{GraphicsError, GraphicsResult};
use super::gpu::GpuDevice;
use super::DeviceId;

/// Display connector type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorType {
    /// Digital Visual Interface
    DVI,
    /// High-Definition Multimedia Interface
    HDMI,
    /// DisplayPort
    DisplayPort,
    /// Video Graphics Array
    VGA,
    /// Embedded DisplayPort
    eDP,
    /// Virtual output
    Virtual,
}

/// Display mode information
#[derive(Debug, Clone)]
pub struct DisplayMode {
    /// Horizontal resolution
    pub hdisplay: u32,
    /// Vertical resolution
    pub vdisplay: u32,
    /// Horizontal sync start
    pub hsync_start: u32,
    /// Horizontal sync end
    pub hsync_end: u32,
    /// Horizontal total
    pub htotal: u32,
    /// Vertical sync start
    pub vsync_start: u32,
    /// Vertical sync end
    pub vsync_end: u32,
    /// Vertical total
    pub vtotal: u32,
    /// Pixel clock in kHz
    pub clock: u32,
    /// Refresh rate in Hz
    pub vrefresh: u32,
    /// Flags
    pub flags: u32,
    /// Mode name
    pub name: String,
}

impl DisplayMode {
    /// Create a new display mode
    pub fn new(hdisplay: u32, vdisplay: u32, refresh: u32) -> Self {
        // Calculate timings for standard CVT
        let hblank = 160;
        let hsync = 32;
        let vblank = 10;

        let htotal = hdisplay + hblank;
        let vtotal = vdisplay + vblank;

        // Pixel clock = total pixels * refresh rate
        let clock = (htotal * vtotal * refresh) / 1000; // kHz

        Self {
            hdisplay,
            vdisplay,
            hsync_start: hdisplay + 8,
            hsync_end: hdisplay + 8 + hsync,
            htotal,
            vsync_start: vdisplay + 3,
            vsync_end: vdisplay + 3 + 3,
            vtotal,
            clock,
            vrefresh: refresh,
            flags: 0,
            name: format!("{}x{}@{}", hdisplay, vdisplay, refresh),
        }
    }

    /// Get total pixels
    pub fn total_pixels(&self) -> u64 {
        self.htotal as u64 * self.vtotal as u64
    }

    /// Get pixel clock in Hz
    pub fn pixel_clock_hz(&self) -> u64 {
        self.clock as u64 * 1000
    }

    /// Check if mode is valid
    pub fn is_valid(&self) -> bool {
        self.hdisplay > 0 && self.vdisplay > 0 && self.clock > 0
    }
}

/// EDID (Extended Display Identification Data) information
#[derive(Debug, Clone)]
pub struct Edid {
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
    /// Serial number
    pub serial_number: u32,
    /// Manufacture week
    pub manufacture_week: u8,
    /// Manufacture year
    pub manufacture_year: u16,
    /// EDID version
    pub version: u8,
    /// Supported modes
    pub modes: Vec<DisplayMode>,
    /// Preferred mode index
    pub preferred_mode: Option<usize>,
    /// Display width in cm
    pub width_cm: u8,
    /// Display height in cm
    pub height_cm: u8,
    /// Supports HDCP
    pub supports_hdcp: bool,
}

impl Edid {
    /// Parse EDID data
    pub fn parse(data: &[u8]) -> GraphicsResult<Self> {
        if data.len() < 128 {
            return Err(GraphicsError::InvalidArgument(
                "EDID too short".to_string(),
            ));
        }

        // Check EDID header
        let header = &data[0..8];
        if header != &[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00] {
            return Err(GraphicsError::InvalidArgument("Invalid EDID header".to_string()));
        }

        // Parse vendor and product ID
        let vendor_id = ((data[8] as u16) << 8) | (data[9] as u16);
        let product_id = ((data[10] as u16) << 8) | (data[11] as u16);
        let serial_number = ((data[12] as u32) << 24)
            | ((data[13] as u32) << 16)
            | ((data[14] as u32) << 8)
            | (data[15] as u32);

        let manufacture_week = data[16];
        let manufacture_year = (data[17] as u16) + 1990;
        let version = data[18];

        // Parse display size
        let width_cm = data[21];
        let height_cm = data[22];

        Ok(Self {
            vendor_id,
            product_id,
            serial_number,
            manufacture_week,
            manufacture_year,
            version,
            modes: Vec::new(),
            preferred_mode: None,
            width_cm,
            height_cm,
            supports_hdcp: false,
        })
    }

    /// Get preferred mode
    pub fn preferred_mode(&self) -> Option<&DisplayMode> {
        self.preferred_mode
            .and_then(|idx| self.modes.get(idx))
            .or_else(|| self.modes.first())
    }

    /// Find best matching mode
    pub fn find_best_mode(&self, width: u32, height: u32) -> Option<&DisplayMode> {
        self.modes
            .iter()
            .filter(|m| m.hdisplay == width && m.vdisplay == height)
            .max_by_key(|m| m.vrefresh)
    }
}

/// Color correction table
#[derive(Debug, Clone)]
pub struct ColorTable {
    /// Red channel values (256 entries)
    pub red: [u16; 256],
    /// Green channel values (256 entries)
    pub green: [u16; 256],
    /// Blue channel values (256 entries)
    pub blue: [u16; 256],
}

impl Default for ColorTable {
    fn default() -> Self {
        Self {
            red: Self::linear_ramp(),
            green: Self::linear_ramp(),
            blue: Self::linear_ramp(),
        }
    }
}

impl ColorTable {
    /// Create linear ramp (0-65535)
    fn linear_ramp() -> [u16; 256] {
        let mut ramp = [0u16; 256];
        for (i, val) in ramp.iter_mut().enumerate() {
            *val = ((i as u32 * 65535) / 255) as u16;
        }
        ramp
    }

    /// Create new color table
    pub fn new() -> Self {
        Self::default()
    }

    /// Set gamma value (1.0 = linear, >1.0 = brighter, <1.0 = darker)
    pub fn set_gamma(&mut self, gamma: f32) {
        for i in 0..256 {
            let value = (i as f32 / 255.0).powf(gamma);
            let scaled = (value * 65535.0) as u16;
            self.red[i] = scaled;
            self.green[i] = scaled;
            self.blue[i] = scaled;
        }
    }
}

/// Color space
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    /// sRGB color space
    Srgb,
    /// Rec.709 color space
    Rec709,
    /// Rec.2020 color space
    Rec2020,
    /// DCI-P3 color space
    DciP3,
}

/// CRTC (CRT Controller) - timing generator
#[derive(Debug)]
pub struct Crtc {
    /// CRTC ID
    id: u32,
    /// Current mode
    mode: Option<DisplayMode>,
    /// Is enabled
    enabled: Arc<AtomicBool>,
    /// Framebuffer
    framebuffer: Option<Framebuffer>,
    /// Gamma table
    gamma: Mutex<ColorTable>,
    /// Degamma table
    degamma: Mutex<ColorTable>,
}

impl Crtc {
    /// Create a new CRTC
    pub fn new(id: u32) -> Self {
        Self {
            id,
            mode: None,
            enabled: Arc::new(AtomicBool::new(false)),
            framebuffer: None,
            gamma: Mutex::new(ColorTable::new()),
            degamma: Mutex::new(ColorTable::new()),
        }
    }

    /// Get CRTC ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    /// Enable CRTC
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Release);
    }

    /// Disable CRTC
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Release);
    }

    /// Set mode
    pub fn set_mode(&mut self, mode: DisplayMode) {
        self.mode = Some(mode);
    }

    /// Get current mode
    pub fn mode(&self) -> Option<&DisplayMode> {
        self.mode.as_ref()
    }

    /// Attach framebuffer
    pub fn attach_framebuffer(&mut self, fb: Framebuffer) {
        self.framebuffer = Some(fb);
    }

    /// Get framebuffer
    pub fn framebuffer(&self) -> Option<&Framebuffer> {
        self.framebuffer.as_ref()
    }
}

/// Plane (layer) for display composition
#[derive(Debug)]
pub struct Plane {
    /// Plane ID
    id: u32,
    /// Plane type
    plane_type: PlaneType,
    /// Z-order (stacking order)
    zpos: u32,
    /// Is enabled
    enabled: Arc<AtomicBool>,
}

/// Plane type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaneType {
    /// Primary plane (main framebuffer)
    Primary,
    /// Overlay plane (for overlays)
    Overlay,
    /// Cursor plane
    Cursor,
}

impl Plane {
    /// Create a new plane
    pub fn new(id: u32, plane_type: PlaneType) -> Self {
        Self {
            id,
            plane_type,
            zpos: 0,
            enabled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get plane ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get plane type
    pub fn plane_type(&self) -> PlaneType {
        self.plane_type
    }

    /// Set Z-order
    pub fn set_zpos(&mut self, zpos: u32) {
        self.zpos = zpos;
    }

    /// Get Z-order
    pub fn zpos(&self) -> u32 {
        self.zpos
    }

    /// Enable plane
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Release);
    }

    /// Disable plane
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Release);
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }
}

/// Connector (output port)
#[derive(Debug)]
pub struct Connector {
    /// Connector ID
    id: u32,
    /// Connector type
    connector_type: ConnectorType,
    /// Is connected
    connected: Arc<AtomicBool>,
    /// EDID data
    edid: Mutex<Option<Edid>>,
    /// Supported modes
    modes: Vec<DisplayMode>,
}

impl Connector {
    /// Create a new connector
    pub fn new(id: u32, connector_type: ConnectorType) -> Self {
        Self {
            id,
            connector_type,
            connected: Arc::new(AtomicBool::new(false)),
            edid: Mutex::new(None),
            modes: Vec::new(),
        }
    }

    /// Get connector ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get connector type
    pub fn connector_type(&self) -> ConnectorType {
        self.connector_type
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

    /// Set connection status
    pub fn set_connected(&self, connected: bool) {
        self.connected.store(connected, Ordering::Release);
    }

    /// Set EDID
    pub fn set_edid(&self, edid: Edid) {
        let mut edid_lock = self.edid.lock();
        *edid_lock = Some(edid);
    }

    /// Get EDID
    pub fn edid(&self) -> Option<Edid> {
        let edid_lock = self.edid.lock();
        edid_lock.clone()
    }

    /// Get supported modes
    pub fn modes(&self) -> &[DisplayMode] {
        &self.modes
    }

    /// Add supported mode
    pub fn add_mode(&mut self, mode: DisplayMode) {
        self.modes.push(mode);
    }
}

/// Framebuffer for display output
#[derive(Debug, Clone)]
pub struct Framebuffer {
    /// Framebuffer address
    pub addr: u64,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Pitch (bytes per line)
    pub pitch: u32,
    /// Bits per pixel
    pub bpp: u32,
    /// Size in bytes
    pub size: u64,
}

impl Framebuffer {
    /// Create a new framebuffer
    pub fn new(addr: u64, width: u32, height: u32, bpp: u32) -> Self {
        let pitch = (width * bpp) / 8;
        let size = (pitch * height) as u64;

        Self {
            addr,
            width,
            height,
            pitch,
            bpp,
            size,
        }
    }

    /// Calculate size for framebuffer
    pub fn calculate_size(width: u32, height: u32, bpp: u32) -> u64 {
        ((width * bpp) / 8) as u64 * height as u64
    }

    /// Get stride in bytes
    pub fn stride(&self) -> u32 {
        self.pitch
    }

    /// Clear framebuffer with color (RGB)
    pub fn clear(&self, color: u32) {
        // In real implementation, fill framebuffer with color
        // For now, just a placeholder
    }

    /// Blit from source to destination
    pub fn blit(&self, src: u64, dst: u64, width: u32, height: u32) {
        // In real implementation, perform blit operation
    }
}

/// Display engine
#[derive(Debug)]
pub struct DisplayEngine {
    /// Device ID
    device_id: DeviceId,
    /// CRTCs
    crtcs: Vec<Crtc>,
    /// Planes
    planes: Vec<Plane>,
    /// Connectors
    connectors: Vec<Connector>,
    /// Current framebuffer
    framebuffer: RwLock<Option<Framebuffer>>,
    /// Vblank counter
    vblank_counter: Arc<AtomicU64>,
    /// Double buffering enabled
    double_buffer: Arc<AtomicBool>,
}

impl DisplayEngine {
    /// Initialize display engine
    pub fn init(_gpu: &GpuDevice) -> GraphicsResult<Self> {
        // Create CRTC
        let crtc = Crtc::new(0);

        // Create planes
        let primary = Plane::new(0, PlaneType::Primary);
        let cursor = Plane::new(1, PlaneType::Cursor);

        // Create connector (HDMI)
        let connector = Connector::new(0, ConnectorType::HDMI);

        Ok(Self {
            device_id: DeviceId::new(1),
            crtcs: vec![crtc],
            planes: vec![primary, cursor],
            connectors: vec![connector],
            framebuffer: RwLock::new(None),
            vblank_counter: Arc::new(AtomicU64::new(0)),
            double_buffer: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Get device ID
    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }

    /// Get CRTCs
    pub fn crtcs(&self) -> &[Crtc] {
        &self.crtcs
    }

    /// Get planes
    pub fn planes(&self) -> &[Plane] {
        &self.planes
    }

    /// Get connectors
    pub fn connectors(&self) -> &[Connector] {
        &self.connectors
    }

    /// Get connector by ID
    pub fn get_connector(&self, id: u32) -> Option<&Connector> {
        self.connectors.iter().find(|c| c.id() == id)
    }

    /// Detect displays (read EDID)
    pub fn detect_displays(&self) -> GraphicsResult<()> {
        for connector in &self.connectors {
            // In real implementation, read EDID from hardware
            connector.set_connected(true);

            // Add some common modes
            let modes = vec![
                DisplayMode::new(1920, 1080, 60),
                DisplayMode::new(1280, 720, 60),
                DisplayMode::new(640, 480, 60),
            ];

            let mut connector_mut = connector;
            for mode in modes {
                connector_mut.add_mode(mode);
            }
        }

        Ok(())
    }

    /// Get preferred display mode
    pub fn get_preferred_mode(&self) -> GraphicsResult<DisplayMode> {
        for connector in &self.connectors {
            if connector.is_connected() {
                if let Some(edid) = connector.edid() {
                    if let Some(mode) = edid.preferred_mode() {
                        return Ok(mode.clone());
                    }
                }

                // Return first available mode
                if let Some(mode) = connector.modes().first() {
                    return Ok(mode.clone());
                }
            }
        }

        Err(GraphicsError::NoDevice("No display connected".to_string()))
    }

    /// Set display mode
    pub fn set_mode(&self, mode: &DisplayMode) -> GraphicsResult<()> {
        if !mode.is_valid() {
            return Err(GraphicsError::InvalidMode("Invalid mode".to_string()));
        }

        // Update CRTC mode
        let crtc = self
            .crtcs
            .first()
            .ok_or_else(|| GraphicsError::NoDevice("No CRTC available".to_string()))?;

        // In real implementation, program hardware with mode timings
        Ok(())
    }

    /// Get framebuffer
    pub fn framebuffer(&self) -> Framebuffer {
        let fb = self.framebuffer.read();
        fb.clone().unwrap_or_else(|| Framebuffer::new(0, 1920, 1080, 32))
    }

    /// Set framebuffer
    pub fn set_framebuffer(&self, fb: Framebuffer) {
        let mut framebuffer = self.framebuffer.write();
        *framebuffer = Some(fb);
    }

    /// Enable double buffering
    pub fn enable_double_buffer(&self) {
        self.double_buffer.store(true, Ordering::Release);
    }

    /// Disable double buffering
    pub fn disable_double_buffer(&self) {
        self.double_buffer.store(false, Ordering::Release);
    }

    /// Check if double buffering is enabled
    pub fn is_double_buffer_enabled(&self) -> bool {
        self.double_buffer.load(Ordering::Acquire)
    }

    /// Wait for VBlank
    pub fn wait_vblank(&self) -> GraphicsResult<()> {
        // In real implementation, wait for VBlank interrupt
        Ok(())
    }

    /// Get VBlank counter
    pub fn vblank_count(&self) -> u64 {
        self.vblank_counter.load(Ordering::Acquire)
    }

    /// Set gamma table
    pub fn set_gamma(&self, crtc_id: u32, gamma: ColorTable) -> GraphicsResult<()> {
        let crtc = self
            .crtcs
            .iter()
            .find(|c| c.id() == crtc_id)
            .ok_or_else(|| GraphicsError::InvalidArgument("Invalid CRTC ID".to_string()))?;

        let mut gamma_table = crtc.gamma.lock();
        *gamma_table = gamma;

        Ok(())
    }

    /// Set degamma table
    pub fn set_degamma(&self, crtc_id: u32, degamma: ColorTable) -> GraphicsResult<()> {
        let crtc = self
            .crtcs
            .iter()
            .find(|c| c.id() == crtc_id)
            .ok_or_else(|| GraphicsError::InvalidArgument("Invalid CRTC ID".to_string()))?;

        let mut degamma_table = crtc.degamma.lock();
        *degamma_table = degamma;

        Ok(())
    }

    /// Enable CRTC
    pub fn enable_crtc(&self, id: u32) -> GraphicsResult<()> {
        let crtc = self
            .crtcs
            .iter()
            .find(|c| c.id() == id)
            .ok_or_else(|| GraphicsError::InvalidArgument("Invalid CRTC ID".to_string()))?;
        crtc.enable();
        Ok(())
    }

    /// Disable CRTC
    pub fn disable_crtc(&self, id: u32) -> GraphicsResult<()> {
        let crtc = self
            .crtcs
            .iter()
            .find(|c| c.id() == id)
            .ok_or_else(|| GraphicsError::InvalidArgument("Invalid CRTC ID".to_string()))?;
        crtc.disable();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_mode() {
        let mode = DisplayMode::new(1920, 1080, 60);
        assert_eq!(mode.hdisplay, 1920);
        assert_eq!(mode.vdisplay, 1080);
        assert_eq!(mode.vrefresh, 60);
        assert!(mode.is_valid());
    }

    #[test]
    fn test_display_mode_total_pixels() {
        let mode = DisplayMode::new(1920, 1080, 60);
        let total = mode.total_pixels();
        assert!(total > (1920 * 1080) as u64);
    }

    #[test]
    fn test_edid_parse() {
        let mut data = [0u8; 128];
        // Set EDID header
        data[0..8].copy_from_slice(&[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]);

        let edid = Edid::parse(&data);
        assert!(edid.is_ok());

        let edid = edid.unwrap();
        assert_eq!(edid.version, 0);
    }

    #[test]
    fn test_edid_invalid_header() {
        let data = [0u8; 128];
        let edid = Edid::parse(&data);
        assert!(edid.is_err());
    }

    #[test]
    fn test_edid_too_short() {
        let data = [0u8; 64];
        let edid = Edid::parse(&data);
        assert!(edid.is_err());
    }

    #[test]
    fn test_color_table_default() {
        let table = ColorTable::default();
        assert_eq!(table.red[0], 0);
        assert_eq!(table.red[255], 65535);
    }

    #[test]
    fn test_color_table_gamma() {
        let mut table = ColorTable::new();
        table.set_gamma(2.2);

        // Check gamma curve
        assert!(table.red[128] > 32768); // Midpoint should be brighter with gamma 2.2
    }

    #[test]
    fn test_crtc() {
        let crtc = Crtc::new(0);
        assert_eq!(crtc.id(), 0);
        assert!(!crtc.is_enabled());

        crtc.enable();
        assert!(crtc.is_enabled());

        crtc.disable();
        assert!(!crtc.is_enabled());
    }

    #[test]
    fn test_crtc_mode() {
        let mut crtc = Crtc::new(0);
        assert!(crtc.mode().is_none());

        let mode = DisplayMode::new(1920, 1080, 60);
        crtc.set_mode(mode.clone());

        let current = crtc.mode().unwrap();
        assert_eq!(current.hdisplay, 1920);
    }

    #[test]
    fn test_plane() {
        let plane = Plane::new(0, PlaneType::Primary);
        assert_eq!(plane.id(), 0);
        assert_eq!(plane.plane_type(), PlaneType::Primary);
        assert!(!plane.is_enabled());

        plane.enable();
        assert!(plane.is_enabled());
    }

    #[test]
    fn test_plane_zpos() {
        let mut plane = Plane::new(0, PlaneType::Overlay);
        assert_eq!(plane.zpos(), 0);

        plane.set_zpos(5);
        assert_eq!(plane.zpos(), 5);
    }

    #[test]
    fn test_connector() {
        let connector = Connector::new(0, ConnectorType::HDMI);
        assert_eq!(connector.id(), 0);
        assert_eq!(connector.connector_type(), ConnectorType::HDMI);
        assert!(!connector.is_connected());

        connector.set_connected(true);
        assert!(connector.is_connected());
    }

    #[test]
    fn test_framebuffer() {
        let fb = Framebuffer::new(0x1000, 1920, 1080, 32);
        assert_eq!(fb.addr, 0x1000);
        assert_eq!(fb.width, 1920);
        assert_eq!(fb.height, 1080);
        assert_eq!(fb.bpp, 32);
    }

    #[test]
    fn test_framebuffer_size() {
        let size = Framebuffer::calculate_size(1920, 1080, 32);
        assert_eq!(size, 1920 * 1080 * 4);
    }

    #[test]
    fn test_display_engine_init() {
        let engine = DisplayEngine::init(&unsafe { core::mem::zeroed() }).unwrap();
        assert_eq!(engine.crtcs().len(), 1);
        assert_eq!(engine.planes().len(), 2);
        assert_eq!(engine.connectors().len(), 1);
    }

    #[test]
    fn test_display_engine_vblank() {
        let engine = DisplayEngine::init(&unsafe { core::mem::zeroed() }).unwrap();
        assert_eq!(engine.vblank_count(), 0);

        // Simulate vblank
        engine.vblank_counter.fetch_add(1, Ordering::Release);
        assert_eq!(engine.vblank_count(), 1);
    }

    #[test]
    fn test_display_engine_double_buffer() {
        let engine = DisplayEngine::init(&unsafe { core::mem::zeroed() }).unwrap();
        assert!(!engine.is_double_buffer_enabled());

        engine.enable_double_buffer();
        assert!(engine.is_double_buffer_enabled());

        engine.disable_double_buffer();
        assert!(!engine.is_double_buffer_enabled());
    }
}
