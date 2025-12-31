//! # Output Management
//!
//! This module provides comprehensive display output management including:
//! - Multi-display support (multi-head, multi-monitor)
//! - Output configuration (mirroring, extending)
//! - Output hotplug (dynamic add/remove)
//! - Display resolution and refresh rate
//! - Audio over HDMI/DisplayPort
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::graphics::output::{OutputManager, Output, OutputConfig};
//!
//! // Initialize output manager
//! let manager = OutputManager::init(&gpu, &display)?;
//!
//! // Get all outputs
//! let outputs = manager.outputs();
//!
//! // Configure output
//! let config = OutputConfig::new(1920, 1080, 60);
//! manager.configure_output(output.id(), &config)?;
//! ```

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::subsystems::sync::{Mutex, RwLock};

use super::error::{GraphicsError, GraphicsResult};
use super::display::{Connector, ConnectorType, DisplayMode};
use super::gpu::GpuDevice;
use super::DeviceId;

/// Output ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutputId(u32);

impl OutputId {
    /// Create a new output ID
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub fn value(&self) -> u32 {
        self.0
    }
}

/// Output configuration mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Extended desktop (each output shows different content)
    Extended,
    /// Mirrored (all outputs show same content)
    Mirrored,
    /// Independent (each output is independent)
    Independent,
}

/// Output configuration
#[derive(Debug, Clone)]
pub struct OutputConfig {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Refresh rate in Hz
    pub refresh_rate: u32,
    /// X position (for extended mode)
    pub x: i32,
    /// Y position (for extended mode)
    pub y: i32,
    /// Primary output
    pub primary: bool,
    /// Enabled
    pub enabled: bool,
    /// Rotation angle
    pub rotation: u32,
}

impl OutputConfig {
    /// Create a new output configuration
    pub fn new(width: u32, height: u32, refresh_rate: u32) -> Self {
        Self {
            width,
            height,
            refresh_rate,
            x: 0,
            y: 0,
            primary: false,
            enabled: true,
            rotation: 0,
        }
    }

    /// Set position
    pub fn set_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    /// Set as primary
    pub fn set_primary(&mut self, primary: bool) {
        self.primary = primary;
    }

    /// Set rotation
    pub fn set_rotation(&mut self, rotation: u32) {
        self.rotation = rotation;
    }
}

/// Audio format for HDMI/DisplayPort
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    /// PCM audio
    Pcm,
    /// AC3 audio
    Ac3,
    /// DTS audio
    Dts,
}

/// Audio configuration
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u32,
    /// Audio format
    pub format: AudioFormat,
    /// Bit depth
    pub bit_depth: u32,
}

impl AudioConfig {
    /// Create a new audio configuration
    pub fn new(sample_rate: u32, channels: u32, format: AudioFormat) -> Self {
        Self {
            sample_rate,
            channels,
            format,
            bit_depth: 16,
        }
    }

    /// Create standard stereo PCM configuration
    pub fn stereo_pcm() -> Self {
        Self::new(48000, 2, AudioFormat::Pcm)
    }

    /// Create 5.1 surround configuration
    pub fn surround_51() -> Self {
        Self::new(48000, 6, AudioFormat::Pcm)
    }
}

/// Output device (display)
#[derive(Debug)]
pub struct Output {
    /// Output ID
    id: OutputId,
    /// Connector
    connector: Arc<Connector>,
    /// Output configuration
    config: RwLock<OutputConfig>,
    /// Current mode
    current_mode: RwLock<Option<DisplayMode>>,
    /// Is connected
    connected: Arc<AtomicBool>,
    /// Is primary
    primary: Arc<AtomicBool>,
    /// Audio enabled
    audio_enabled: Arc<AtomicBool>,
    /// Audio configuration
    audio_config: Mutex<Option<AudioConfig>>,
}

impl Output {
    /// Create a new output
    pub fn new(id: OutputId, connector: Connector) -> Self {
        let config = OutputConfig::new(1920, 1080, 60);
        let connected = Arc::new(AtomicBool::new(false));

        Self {
            id,
            connector: Arc::new(connector),
            config: RwLock::new(config),
            current_mode: RwLock::new(None),
            connected: Arc::clone(&connected),
            primary: Arc::new(AtomicBool::new(false)),
            audio_enabled: Arc::new(AtomicBool::new(false)),
            audio_config: Mutex::new(None),
        }
    }

    /// Get output ID
    pub fn id(&self) -> OutputId {
        self.id
    }

    /// Get connector
    pub fn connector(&self) -> &Connector {
        &self.connector
    }

    /// Get connector type
    pub fn connector_type(&self) -> ConnectorType {
        self.connector.connector_type()
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

    /// Set connection status
    pub fn set_connected(&self, connected: bool) {
        self.connector.set_connected(connected);
        self.connected.store(connected, Ordering::Release);
    }

    /// Get configuration
    pub fn config(&self) -> OutputConfig {
        let config = self.config.read();
        config.clone()
    }

    /// Set configuration
    pub fn set_config(&self, config: OutputConfig) {
        let mut cfg = self.config.write();
        *cfg = config;
    }

    /// Get current mode
    pub fn current_mode(&self) -> Option<DisplayMode> {
        let mode = self.current_mode.read();
        mode.clone()
    }

    /// Set current mode
    pub fn set_mode(&self, mode: DisplayMode) {
        let mut current = self.current_mode.write();
        *current = Some(mode);
    }

    /// Check if primary
    pub fn is_primary(&self) -> bool {
        self.primary.load(Ordering::Acquire)
    }

    /// Set as primary
    pub fn set_primary(&self, primary: bool) {
        self.primary.store(primary, Ordering::Release);
        let mut config = self.config.write();
        config.primary = primary;
    }

    /// Check if audio is enabled
    pub fn is_audio_enabled(&self) -> bool {
        self.audio_enabled.load(Ordering::Acquire)
    }

    /// Enable audio
    pub fn enable_audio(&self) {
        self.audio_enabled.store(true, Ordering::Release);
    }

    /// Disable audio
    pub fn disable_audio(&self) {
        self.audio_enabled.store(false, Ordering::Release);
    }

    /// Get audio configuration
    pub fn audio_config(&self) -> Option<AudioConfig> {
        let config = self.audio_config.lock();
        config.clone()
    }

    /// Set audio configuration
    pub fn set_audio_config(&self, config: AudioConfig) {
        let mut audio = self.audio_config.lock();
        *audio = Some(config);
        self.enable_audio();
    }

    /// Get supported modes
    pub fn supported_modes(&self) -> Vec<DisplayMode> {
        self.connector.modes().to_vec()
    }

    /// Get preferred mode
    pub fn preferred_mode(&self) -> Option<DisplayMode> {
        self.connector
            .edid()
            .and_then(|edid| edid.preferred_mode().cloned())
            .or_else(|| self.connector.modes().first().cloned())
    }

    /// Enable output
    pub fn enable(&self) {
        let mut config = self.config.write();
        config.enabled = true;
    }

    /// Disable output
    pub fn disable(&self) {
        let mut config = self.config.write();
        config.enabled = false;
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        let config = self.config.read();
        config.enabled
    }
}

/// Output manager
#[derive(Debug)]
pub struct OutputManager {
    /// Device ID
    device_id: DeviceId,
    /// Outputs
    outputs: Mutex<BTreeMap<OutputId, Arc<Output>>>,
    /// Next output ID
    next_output_id: Arc<AtomicU32>,
    /// Output mode (extended/mirrored)
    output_mode: Mutex<OutputMode>,
    /// Hotplug detection enabled
    hotplug_enabled: Arc<AtomicBool>,
}

impl OutputManager {
    /// Initialize output manager
    pub fn init(_gpu: &GpuDevice, _display: &super::display::DisplayEngine) -> GraphicsResult<Self> {
        Ok(Self {
            device_id: DeviceId::new(1),
            outputs: Mutex::new(BTreeMap::new()),
            next_output_id: Arc::new(AtomicU32::new(1)),
            output_mode: Mutex::new(OutputMode::Extended),
            hotplug_enabled: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Get device ID
    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }

    /// Get all outputs
    pub fn outputs(&self) -> Vec<Arc<Output>> {
        let outputs = self.outputs.lock();
        outputs.values().cloned().collect()
    }

    /// Get output by ID
    pub fn get_output(&self, id: OutputId) -> GraphicsResult<Arc<Output>> {
        let outputs = self.outputs.lock();
        outputs
            .get(&id)
            .cloned()
            .ok_or_else(|| GraphicsError::InvalidArgument("Invalid output ID".to_string()))
    }

    /// Add output
    pub fn add_output(&self, connector: Connector) -> GraphicsResult<OutputId> {
        let id = OutputId::new(self.next_output_id.fetch_add(1, Ordering::SeqCst));
        let output = Arc::new(Output::new(id, connector));

        let mut outputs = self.outputs.lock();
        outputs.insert(id, output);

        Ok(id)
    }

    /// Remove output
    pub fn remove_output(&self, id: OutputId) -> GraphicsResult<()> {
        let mut outputs = self.outputs.lock();
        outputs
            .remove(&id)
            .ok_or_else(|| GraphicsError::InvalidArgument("Invalid output ID".to_string()))?;
        Ok(())
    }

    /// Configure output
    pub fn configure_output(&self, id: OutputId, config: &OutputConfig) -> GraphicsResult<()> {
        let output = self.get_output(id)?;
        output.set_config(config.clone());

        // Set mode if configured
        if config.enabled {
            let mode = DisplayMode::new(config.width, config.height, config.refresh_rate);
            output.set_mode(mode);
        }

        Ok(())
    }

    /// Enable output
    pub fn enable_output(&self, id: OutputId) -> GraphicsResult<()> {
        let output = self.get_output(id)?;
        output.enable();
        Ok(())
    }

    /// Disable output
    pub fn disable_output(&self, id: OutputId) -> GraphicsResult<()> {
        let output = self.get_output(id)?;
        output.disable();
        Ok(())
    }

    /// Set primary output
    pub fn set_primary_output(&self, id: OutputId) -> GraphicsResult<()> {
        // Clear primary from all outputs
        let outputs = self.outputs.lock();
        for output in outputs.values() {
            output.set_primary(false);
        }

        // Set new primary
        let output = outputs
            .get(&id)
            .ok_or_else(|| GraphicsError::InvalidArgument("Invalid output ID".to_string()))?;
        output.set_primary(true);

        Ok(())
    }

    /// Get primary output
    pub fn primary_output(&self) -> Option<Arc<Output>> {
        let outputs = self.outputs.lock();
        outputs.values().find(|o| o.is_primary()).cloned()
    }

    /// Set output mode (extended/mirrored)
    pub fn set_output_mode(&self, mode: OutputMode) {
        let mut output_mode = self.output_mode.lock();
        *output_mode = mode;
    }

    /// Get output mode
    pub fn output_mode(&self) -> OutputMode {
        let output_mode = self.output_mode.lock();
        *output_mode
    }

    /// Check if multi-display is supported
    pub fn supports_multi_display(&self) -> bool {
        let outputs = self.outputs.lock();
        outputs.len() > 1
    }

    /// Get connected outputs
    pub fn connected_outputs(&self) -> Vec<Arc<Output>> {
        let outputs = self.outputs.lock();
        outputs
            .values()
            .filter(|o| o.is_connected())
            .cloned()
            .collect()
    }

    /// Get enabled outputs
    pub fn enabled_outputs(&self) -> Vec<Arc<Output>> {
        let outputs = self.outputs.lock();
        outputs
            .values()
            .filter(|o| o.is_enabled())
            .cloned()
            .collect()
    }

    /// Enable hotplug detection
    pub fn enable_hotplug(&self) {
        self.hotplug_enabled.store(true, Ordering::Release);
    }

    /// Disable hotplug detection
    pub fn disable_hotplug(&self) {
        self.hotplug_enabled.store(false, Ordering::Release);
    }

    /// Check if hotplug is enabled
    pub fn is_hotplug_enabled(&self) -> bool {
        self.hotplug_enabled.load(Ordering::Acquire)
    }

    /// Handle hotplug event
    pub fn handle_hotplug(&self, output_id: OutputId, connected: bool) -> GraphicsResult<()> {
        let output = self.get_output(output_id)?;
        output.set_connected(connected);

        if connected {
            // Output was connected
            if let Some(preferred) = output.preferred_mode() {
                output.set_mode(preferred);
            }
        } else {
            // Output was disconnected
            output.disable();
        }

        Ok(())
    }

    /// Get total desktop size (for extended mode)
    pub fn desktop_size(&self) -> (u32, u32) {
        let outputs = self.enabled_outputs();
        if outputs.is_empty() {
            return (0, 0);
        }

        let mut max_width = 0u32;
        let mut max_height = 0u32;

        for output in &outputs {
            let config = output.config();
            let right = (config.x as u32) + config.width;
            let bottom = (config.y as u32) + config.height;

            max_width = max_width.max(right);
            max_height = max_height.max(bottom);
        }

        (max_width, max_height)
    }

    /// Set audio configuration for output
    pub fn set_audio_config(&self, id: OutputId, config: AudioConfig) -> GraphicsResult<()> {
        let output = self.get_output(id)?;
        output.set_audio_config(config);
        Ok(())
    }

    /// Enable audio for output
    pub fn enable_audio(&self, id: OutputId) -> GraphicsResult<()> {
        let output = self.get_output(id)?;
        output.enable_audio();
        Ok(())
    }

    /// Disable audio for output
    pub fn disable_audio(&self, id: OutputId) -> GraphicsResult<()> {
        let output = self.get_output(id)?;
        output.disable_audio();
        Ok(())
    }

    /// Get outputs with audio
    pub fn audio_outputs(&self) -> Vec<Arc<Output>> {
        let outputs = self.outputs.lock();
        outputs
            .values()
            .filter(|o| o.is_audio_enabled())
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_id() {
        let id = OutputId::new(42);
        assert_eq!(id.value(), 42);
    }

    #[test]
    fn test_output_config() {
        let config = OutputConfig::new(1920, 1080, 60);
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert_eq!(config.refresh_rate, 60);
        assert!(config.enabled);
        assert!(!config.primary);
    }

    #[test]
    fn test_output_config_position() {
        let mut config = OutputConfig::new(1920, 1080, 60);
        config.set_position(1920, 0);
        assert_eq!(config.x, 1920);
        assert_eq!(config.y, 0);
    }

    #[test]
    fn test_output_config_primary() {
        let mut config = OutputConfig::new(1920, 1080, 60);
        config.set_primary(true);
        assert!(config.primary);
    }

    #[test]
    fn test_audio_config() {
        let config = AudioConfig::new(48000, 2, AudioFormat::Pcm);
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 2);
        assert_eq!(config.format, AudioFormat::Pcm);
    }

    #[test]
    fn test_audio_config_stereo() {
        let config = AudioConfig::stereo_pcm();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 2);
    }

    #[test]
    fn test_audio_config_surround() {
        let config = AudioConfig::surround_51();
        assert_eq!(config.channels, 6);
    }

    #[test]
    fn test_output() {
        let connector = Connector::new(0, ConnectorType::HDMI);
        let output = Output::new(OutputId::new(1), connector);

        assert_eq!(output.id(), OutputId::new(1));
        assert_eq!(output.connector_type(), ConnectorType::HDMI);
        assert!(!output.is_connected());
    }

    #[test]
    fn test_output_connection() {
        let connector = Connector::new(0, ConnectorType::DisplayPort);
        let output = Output::new(OutputId::new(1), connector);

        output.set_connected(true);
        assert!(output.is_connected());
    }

    #[test]
    fn test_output_primary() {
        let connector = Connector::new(0, ConnectorType::HDMI);
        let output = Output::new(OutputId::new(1), connector);

        assert!(!output.is_primary());

        output.set_primary(true);
        assert!(output.is_primary());
    }

    #[test]
    fn test_output_audio() {
        let connector = Connector::new(0, ConnectorType::HDMI);
        let output = Output::new(OutputId::new(1), connector);

        assert!(!output.is_audio_enabled());

        output.enable_audio();
        assert!(output.is_audio_enabled());

        let config = AudioConfig::stereo_pcm();
        output.set_audio_config(config);
        assert!(output.is_audio_enabled());
    }

    #[test]
    fn test_output_manager_init() {
        let manager = OutputManager::init(&unsafe { core::mem::zeroed() }, &unsafe { core::mem::zeroed() }).unwrap();
        assert_eq!(manager.outputs().len(), 0);
    }

    #[test]
    fn test_output_manager_add() {
        let manager = OutputManager::init(&unsafe { core::mem::zeroed() }, &unsafe { core::mem::zeroed() }).unwrap();
        let connector = Connector::new(0, ConnectorType::HDMI);

        let id = manager.add_output(connector).unwrap();
        assert_eq!(id.value(), 1);
        assert_eq!(manager.outputs().len(), 1);
    }

    #[test]
    fn test_output_manager_remove() {
        let manager = OutputManager::init(&unsafe { core::mem::zeroed() }, &unsafe { core::mem::zeroed() }).unwrap();
        let connector = Connector::new(0, ConnectorType::HDMI);

        let id = manager.add_output(connector).unwrap();
        assert!(manager.remove_output(id).is_ok());
        assert_eq!(manager.outputs().len(), 0);
    }

    #[test]
    fn test_output_manager_configure() {
        let manager = OutputManager::init(&unsafe { core::mem::zeroed() }, &unsafe { core::mem::zeroed() }).unwrap();
        let connector = Connector::new(0, ConnectorType::HDMI);
        let id = manager.add_output(connector).unwrap();

        let config = OutputConfig::new(1920, 1080, 60);
        assert!(manager.configure_output(id, &config).is_ok());

        let output = manager.get_output(id).unwrap();
        let output_config = output.config();
        assert_eq!(output_config.width, 1920);
    }

    #[test]
    fn test_output_manager_primary() {
        let manager = OutputManager::init(&unsafe { core::mem::zeroed() }, &unsafe { core::mem::zeroed() }).unwrap();
        let connector1 = Connector::new(0, ConnectorType::HDMI);
        let connector2 = Connector::new(1, ConnectorType::DisplayPort);

        let id1 = manager.add_output(connector1).unwrap();
        let id2 = manager.add_output(connector2).unwrap();

        assert!(manager.set_primary_output(id1).is_ok());

        let primary = manager.primary_output().unwrap();
        assert_eq!(primary.id(), id1);
    }

    #[test]
    fn test_output_manager_mode() {
        let manager = OutputManager::init(&unsafe { core::mem::zeroed() }, &unsafe { core::mem::zeroed() }).unwrap();

        manager.set_output_mode(OutputMode::Mirrored);
        assert_eq!(manager.output_mode(), OutputMode::Mirrored);

        manager.set_output_mode(OutputMode::Extended);
        assert_eq!(manager.output_mode(), OutputMode::Extended);
    }

    #[test]
    fn test_output_manager_hotplug() {
        let manager = OutputManager::init(&unsafe { core::mem::zeroed() }, &unsafe { core::mem::zeroed() }).unwrap();
        let connector = Connector::new(0, ConnectorType::HDMI);
        let id = manager.add_output(connector).unwrap();

        assert!(!manager.is_hotplug_enabled());

        manager.enable_hotplug();
        assert!(manager.is_hotplug_enabled());

        // Handle hotplug event
        assert!(manager.handle_hotplug(id, true).is_ok());

        let output = manager.get_output(id).unwrap();
        assert!(output.is_connected());
    }

    #[test]
    fn test_output_manager_desktop_size() {
        let manager = OutputManager::init(&unsafe { core::mem::zeroed() }, &unsafe { core::mem::zeroed() }).unwrap();

        let connector1 = Connector::new(0, ConnectorType::HDMI);
        let id1 = manager.add_output(connector1).unwrap();

        let connector2 = Connector::new(1, ConnectorType::DisplayPort);
        let id2 = manager.add_output(connector2).unwrap();

        // Configure first output at (0, 0)
        let config1 = OutputConfig::new(1920, 1080, 60);
        config1.set_position(0, 0);
        manager.configure_output(id1, &config1).unwrap();

        // Configure second output at (1920, 0)
        let config2 = OutputConfig::new(1920, 1080, 60);
        config2.set_position(1920, 0);
        manager.configure_output(id2, &config2).unwrap();

        let (width, height) = manager.desktop_size();
        assert_eq!(width, 3840);
        assert_eq!(height, 1080);
    }
}
