//! Screen Reader Interface and Text-to-Speech Integration
//!
//! This module provides comprehensive screen reader functionality including:
//! - Text-to-speech engine integration
//! - Screen content extraction and analysis
//! - Focus tracking and announcement
//! - Accessibility event handling
//! - Screen reader compatibility layer

use crate::subsystems::sync::spinlock::SpinLock;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;
use crate::HashMap;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Screen reader configuration
#[derive(Debug, Clone)]
pub struct ScreenReaderConfig {
    /// Speech rate (words per minute)
    pub speech_rate: u32,
    /// Pitch adjustment (0.0-2.0, 1.0 is normal)
    pub pitch: f32,
    /// Volume (0.0-1.0)
    pub volume: f32,
    /// Enable speech
    pub enabled: bool,
    /// Verbose mode (announces more details)
    pub verbose: bool,
    /// Echo input characters
    pub echo_characters: bool,
    /// Echo words
    pub echo_words: bool,
}

impl Default for ScreenReaderConfig {
    fn default() -> Self {
        Self {
            speech_rate: 180,
            pitch: 1.0,
            volume: 0.8,
            enabled: true,
            verbose: false,
            echo_characters: true,
            echo_words: false,
        }
    }
}

/// Screen content region
#[derive(Debug, Clone, Copy)]
pub struct ScreenRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl ScreenRegion {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.x && x < self.x + self.width &&
        y >= self.y && y < self.y + self.height
    }

    pub fn intersects(&self, other: &ScreenRegion) -> bool {
        !(self.x + self.width <= other.x ||
          other.x + other.width <= self.x ||
          self.y + self.height <= other.y ||
          other.y + other.height <= self.y)
    }
}

/// Accessible element on screen
#[derive(Debug, Clone)]
pub struct AccessibleElement {
    /// Unique identifier
    pub id: u64,
    /// Element type
    pub element_type: AccessibleType,
    /// Screen region
    pub region: ScreenRegion,
    /// Element label/name
    pub label: String,
    /// Detailed description
    pub description: String,
    /// Current value (for sliders, progress bars, etc.)
    pub value: Option<String>,
    /// State information
    pub state: AccessibleState,
    /// Parent element ID
    pub parent_id: Option<u64>,
    /// Child element IDs
    pub child_ids: Vec<u64>,
}

/// Type of accessible element
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibleType {
    /// Generic container
    Container,
    /// Text label
    Label,
    /// Button
    Button,
    /// Text input field
    TextBox,
    /// Checkbox
    CheckBox,
    /// Radio button
    RadioButton,
    /// Slider
    Slider,
    /// Progress bar
    ProgressBar,
    /// Menu
    Menu,
    /// Menu item
    MenuItem,
    /// List
    List,
    /// List item
    ListItem,
    /// Table
    Table,
    /// Table cell
    TableCell,
    /// Tab
    Tab,
    /// Combo box (dropdown)
    ComboBox,
    /// Tree view
    TreeView,
    /// Scroll bar
    ScrollBar,
    /// Pane
    Pane,
    /// Unknown element
    Unknown,
}

/// Accessible state flags
#[derive(Debug, Clone, Copy)]
pub struct AccessibleState {
    /// Element is visible
    pub visible: bool,
    /// Element is enabled
    pub enabled: bool,
    /// Element has focus
    pub focused: bool,
    /// Element is selected
    pub selected: bool,
    /// Element is checked (for checkboxes)
    pub checked: bool,
    /// Element is expanded (for trees, menus)
    pub expanded: bool,
    /// Element is editable
    pub editable: bool,
    /// Element is selectable
    pub selectable: bool,
    /// Element is a modal dialog
    pub modal: bool,
    /// Element is read-only
    pub read_only: bool,
    /// Element is required (form input)
    pub required: bool,
    /// Element is invalid
    pub invalid: bool,
}

impl Default for AccessibleState {
    fn default() -> Self {
        Self {
            visible: true,
            enabled: true,
            focused: false,
            selected: false,
            checked: false,
            expanded: false,
            editable: false,
            selectable: false,
            modal: false,
            read_only: false,
            required: false,
            invalid: false,
        }
    }
}

/// Accessibility event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityEvent {
    /// Element gained focus
    FocusGained,
    /// Element lost focus
    FocusLost,
    /// Element clicked/activated
    Activated,
    /// Element state changed
    StateChanged,
    /// Element value changed
    ValueChanged,
    /// Element text changed
    TextChanged,
    /// Element visibility changed
    VisibilityChanged,
    /// Element bounds changed
    BoundsChanged,
    /// Element added
    Added,
    /// Element removed
    Removed,
    /// Window activated
    WindowActivated,
    /// Window deactivated
    WindowDeactivated,
    /// Screen updated
    ScreenUpdated,
    /// Custom event
    Custom(u32),
}

/// Speech priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpeechPriority {
    /// Low priority (informational)
    Low = 0,
    /// Normal priority (default)
    Normal = 1,
    /// High priority (important messages)
    High = 2,
    /// Critical priority (alerts, errors)
    Critical = 3,
}

/// Speech request
#[derive(Debug, Clone)]
pub struct SpeechRequest {
    /// Text to speak
    pub text: String,
    /// Priority level
    pub priority: SpeechPriority,
    /// Interrupt current speech
    pub interrupt: bool,
    /// Unique request ID
    pub id: u64,
}

/// Text-to-speech engine interface
pub trait TextToSpeechEngine: Send + Sync {
    /// Speak text with given parameters
    fn speak(&self, text: &str, interrupt: bool) -> Result<(), TtsError>;

    /// Stop all speech
    fn stop(&self) -> Result<(), TtsError>;

    /// Pause speech
    fn pause(&self) -> Result<(), TtsError>;

    /// Resume speech
    fn resume(&self) -> Result<(), TtsError>;

    /// Check if currently speaking
    fn is_speaking(&self) -> bool;

    /// Set speech rate (words per minute)
    fn set_rate(&self, wpm: u32) -> Result<(), TtsError>;

    /// Set pitch (0.0-2.0)
    fn set_pitch(&self, pitch: f32) -> Result<(), TtsError>;

    /// Set volume (0.0-1.0)
    fn set_volume(&self, volume: f32) -> Result<(), TtsError>;

    /// Get available voices
    fn get_voices(&self) -> Vec<VoiceInfo>;

    /// Set voice
    fn set_voice(&self, voice_id: &str) -> Result<(), TtsError>;
}

/// TTS error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtsError {
    /// Engine not available
    EngineUnavailable,
    /// Invalid parameter
    InvalidParameter,
    /// Operation failed
    OperationFailed,
    /// Voice not found
    VoiceNotFound,
    /// Engine busy
    EngineBusy,
    /// Not supported
    NotSupported,
}

/// Voice information
#[derive(Debug, Clone)]
pub struct VoiceInfo {
    /// Voice ID
    pub id: String,
    /// Voice name
    pub name: String,
    /// Language code (e.g., "en-US")
    pub language: String,
    /// Gender
    pub gender: VoiceGender,
}

/// Voice gender
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceGender {
    Male,
    Female,
    Neutral,
    Unknown,
}

/// Default TTS engine implementation
pub struct DefaultTtsEngine {
    config: SpinLock<TtsEngineConfig>,
    speaking: AtomicBool,
    next_request_id: AtomicU64,
}

#[derive(Debug, Clone)]
struct TtsEngineConfig {
    rate: u32,
    pitch: f32,
    volume: f32,
    current_voice: Option<String>,
}

impl Default for DefaultTtsEngine {
    fn default() -> Self {
        Self {
            config: SpinLock::new(TtsEngineConfig {
                rate: 180,
                pitch: 1.0,
                volume: 0.8,
                current_voice: None,
            }),
            speaking: AtomicBool::new(false),
            next_request_id: AtomicU64::new(1),
        }
    }
}

impl TextToSpeechEngine for DefaultTtsEngine {
    fn speak(&self, text: &str, interrupt: bool) -> Result<(), TtsError> {
        if text.is_empty() {
            return Ok(());
        }

        if interrupt {
            self.stop()?;
        }

        self.speaking.store(true, Ordering::Release);

        // In a real implementation, this would interface with actual TTS hardware/software
        // For now, we simulate speech by storing the state
        log::debug!("TTS speaking: {}", text);

        Ok(())
    }

    fn stop(&self) -> Result<(), TtsError> {
        self.speaking.store(false, Ordering::Release);
        log::debug!("TTS stopped");
        Ok(())
    }

    fn pause(&self) -> Result<(), TtsError> {
        log::debug!("TTS paused");
        Ok(())
    }

    fn resume(&self) -> Result<(), TtsError> {
        log::debug!("TTS resumed");
        Ok(())
    }

    fn is_speaking(&self) -> bool {
        self.speaking.load(Ordering::Acquire)
    }

    fn set_rate(&self, wpm: u32) -> Result<(), TtsError> {
        if !(50..=500).contains(&wpm) {
            return Err(TtsError::InvalidParameter);
        }
        let mut config = self.config.lock();
        config.rate = wpm;
        Ok(())
    }

    fn set_pitch(&self, pitch: f32) -> Result<(), TtsError> {
        if !(0.0..=2.0).contains(&pitch) {
            return Err(TtsError::InvalidParameter);
        }
        let mut config = self.config.lock();
        config.pitch = pitch;
        Ok(())
    }

    fn set_volume(&self, volume: f32) -> Result<(), TtsError> {
        if !(0.0..=1.0).contains(&volume) {
            return Err(TtsError::InvalidParameter);
        }
        let mut config = self.config.lock();
        config.volume = volume;
        Ok(())
    }

    fn get_voices(&self) -> Vec<VoiceInfo> {
        vec![
            VoiceInfo {
                id: "default".into(),
                name: "Default Voice".into(),
                language: "en-US".into(),
                gender: VoiceGender::Neutral,
            },
        ]
    }

    fn set_voice(&self, voice_id: &str) -> Result<(), TtsError> {
        let mut config = self.config.lock();
        config.current_voice = Some(voice_id.into());
        Ok(())
    }
}

/// Screen reader instance
pub struct ScreenReader {
    config: SpinLock<ScreenReaderConfig>,
    tts: SpinLock<Box<dyn TextToSpeechEngine>>,
    elements: SpinLock<HashMap<u64, AccessibleElement>>,
    focused_element: SpinLock<Option<u64>>,
    next_element_id: AtomicU64,
    speech_queue: SpinLock<Vec<SpeechRequest>>,
    enabled: AtomicBool,
}

impl ScreenReader {
    /// Create a new screen reader instance
    pub fn new(tts: Box<dyn TextToSpeechEngine>) -> Self {
        Self {
            config: SpinLock::new(ScreenReaderConfig::default()),
            tts: SpinLock::new(tts),
            elements: SpinLock::new(HashMap::new()),
            focused_element: SpinLock::new(None),
            next_element_id: AtomicU64::new(1),
            speech_queue: SpinLock::new(Vec::new()),
            enabled: AtomicBool::new(true),
        }
    }

    /// Enable or disable the screen reader
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
        let mut config = self.config.lock();
        config.enabled = enabled;

        if !enabled {
            self.stop_speech();
        }
    }

    /// Check if screen reader is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    /// Update configuration
    pub fn update_config<F>(&self, f: F)
    where
        F: FnOnce(&mut ScreenReaderConfig),
    {
        let mut config = self.config.lock();
        f(&mut config);

        // Apply speech settings to TTS engine
        let tts = self.tts.lock();
        let _ = tts.set_rate(config.speech_rate);
        let _ = tts.set_pitch(config.pitch);
        let _ = tts.set_volume(config.volume);
    }

    /// Get current configuration
    pub fn get_config(&self) -> ScreenReaderConfig {
        self.config.lock().clone()
    }

    /// Register an accessible element
    pub fn register_element(&self, element: AccessibleElement) -> u64 {
        let id = element.id;
        let mut elements = self.elements.lock();
        elements.insert(id, element.clone());

        // Announce new element in verbose mode
        if self.config.lock().verbose {
            self.announce(&format!("New {}: {}",
                self.element_type_name(element.element_type),
                element.label),
                SpeechPriority::Low,
                false
            );
        }

        id
    }

    /// Update an accessible element
    pub fn update_element(&self, element: AccessibleElement) {
        let mut elements = self.elements.lock();
        elements.insert(element.id, element);
    }

    /// Unregister an accessible element
    pub fn unregister_element(&self, id: u64) {
        let mut elements = self.elements.lock();
        elements.remove(&id);
    }

    /// Get element by ID
    pub fn get_element(&self, id: u64) -> Option<AccessibleElement> {
        self.elements.lock().get(&id).cloned()
    }

    /// Set focus to an element
    pub fn set_focus(&self, id: u64) {
        let mut focused = self.focused_element.lock();

        // Clear previous focus
        if let Some(prev_id) = *focused {
            if let Some(element) = self.elements.lock().get_mut(&prev_id) {
                element.state.focused = false;
            }
        }

        // Set new focus
        *focused = Some(id);
        if let Some(element) = self.elements.lock().get_mut(&id) {
            element.state.focused = true;

            // Announce focused element
            let announcement = self.build_focus_announcement(&element);
            drop(element);
            drop(focused);

            self.announce(&announcement, SpeechPriority::High, true);
        }
    }

    /// Get currently focused element
    pub fn get_focused_element(&self) -> Option<AccessibleElement> {
        let focused = self.focused_element.lock();
        if let Some(id) = *focused {
            self.elements.lock().get(&id).cloned()
        } else {
            None
        }
    }

    /// Handle accessibility event
    pub fn handle_event(&self, event: AccessibilityEvent, element_id: u64) {
        if !self.is_enabled() {
            return;
        }

        match event {
            AccessibilityEvent::FocusGained => {
                self.set_focus(element_id);
            }
            AccessibilityEvent::FocusLost => {
                let mut focused = self.focused_element.lock();
                if *focused == Some(element_id) {
                    *focused = None;
                }
            }
            AccessibilityEvent::ValueChanged |
            AccessibilityEvent::StateChanged => {
                if let Some(element) = self.get_element(element_id) {
                    let announcement = self.build_state_announcement(&element);
                    self.announce(&announcement, SpeechPriority::Normal, false);
                }
            }
            AccessibilityEvent::TextChanged => {
                if let Some(element) = self.get_element(element_id) {
                    if element.state.focused {
                        let announcement = self.build_text_announcement(&element);
                        self.announce(&announcement, SpeechPriority::Normal, false);
                    }
                }
            }
            AccessibilityEvent::Activated => {
                if let Some(element) = self.get_element(element_id) {
                    let announcement = format!("Activated {}", element.label);
                    self.announce(&announcement, SpeechPriority::High, true);
                }
            }
            _ => {
                // Log other events but don't announce
                log::debug!("Accessibility event: {:?}", event);
            }
        }
    }

    /// Speak text immediately
    pub fn speak(&self, text: &str, interrupt: bool) {
        if !self.is_enabled() {
            return;
        }

        if interrupt {
            self.stop_speech();
        }

        let tts = self.tts.lock();
        let _ = tts.speak(text, interrupt);
    }

    /// Announce with priority
    pub fn announce(&self, text: &str, priority: SpeechPriority, interrupt: bool) {
        if !self.is_enabled() || text.is_empty() {
            return;
        }

        // Critical and high priority announcements interrupt
        let should_interrupt = interrupt || priority >= SpeechPriority::High;

        let tts = self.tts.lock();
        let _ = tts.speak(text, should_interrupt);
    }

    /// Stop all speech
    pub fn stop_speech(&self) {
        let tts = self.tts.lock();
        let _ = tts.stop();
    }

    /// Read entire screen
    pub fn read_screen(&self) {
        if !self.is_enabled() {
            return;
        }

        let elements = self.elements.lock();
        let mut text = String::from("Screen contents: ");

        // Collect visible elements in reading order
        let mut visible_elements: Vec<_> = elements.values()
            .filter(|e| e.state.visible)
            .collect();

        // Sort by position (top to bottom, left to right)
        visible_elements.sort_by(|a, b| {
            a.region.y.cmp(&b.region.y)
                .then(a.region.x.cmp(&b.region.x))
        });

        for element in visible_elements {
            if !element.label.is_empty() {
                text.push_str(&element.label);
                text.push_str(". ");
            }
        }

        drop(elements);
        self.announce(&text, SpeechPriority::Normal, true);
    }

    /// Read focused element
    pub fn read_focused_element(&self) {
        if !self.is_enabled() {
            return;
        }

        if let Some(element) = self.get_focused_element() {
            let announcement = self.build_full_announcement(&element);
            self.announce(&announcement, SpeechPriority::Normal, true);
        }
    }

    /// Read from current position to end
    pub fn read_from_position(&self, element_id: u64) {
        if !self.is_enabled() {
            return;
        }

        let elements = self.elements.lock();
        let start_element = elements.get(&element_id);

        if let Some(start) = start_element {
            let mut text = String::new();

            // Collect elements after the starting position
            let mut following_elements: Vec<_> = elements.values()
                .filter(|e| e.state.visible &&
                    (e.region.y > start.region.y ||
                     (e.region.y == start.region.y && e.region.x >= start.region.x)))
                .collect();

            following_elements.sort_by(|a, b| {
                a.region.y.cmp(&b.region.y)
                    .then(a.region.x.cmp(&b.region.x))
            });

            for element in following_elements {
                if !element.label.is_empty() {
                    text.push_str(&element.label);
                    text.push_str(" ");
                }
            }

            drop(elements);

            if !text.is_empty() {
                self.announce(&text, SpeechPriority::Normal, true);
            }
        }
    }

    /// Extract text from screen region
    pub fn extract_text_from_region(&self, region: &ScreenRegion) -> String {
        let elements = self.elements.lock();
        let mut text = String::new();

        let mut matching_elements: Vec<_> = elements.values()
            .filter(|e| e.state.visible && e.region.intersects(region))
            .collect();

        matching_elements.sort_by(|a, b| {
            a.region.y.cmp(&b.region.y)
                .then(a.region.x.cmp(&b.region.x))
        });

        for element in matching_elements {
            if !element.label.is_empty() {
                text.push_str(&element.label);
                text.push(' ');
            }
        }

        text
    }

    /// Build focus announcement
    fn build_focus_announcement(&self, element: &AccessibleElement) -> String {
        let mut announcement = String::new();

        // Element type
        announcement.push_str(self.element_type_name(element.element_type));
        announcement.push_str(": ");

        // Label
        if !element.label.is_empty() {
            announcement.push_str(&element.label);
        }

        // State information
        if element.state.checked {
            announcement.push_str(", checked");
        } else if element.element_type == AccessibleType::CheckBox {
            announcement.push_str(", not checked");
        }

        if element.state.selected {
            announcement.push_str(", selected");
        }

        if element.state.expanded {
            announcement.push_str(", expanded");
        } else if matches!(element.element_type,
            AccessibleType::TreeView | AccessibleType::Menu | AccessibleType::ComboBox) {
            announcement.push_str(", collapsed");
        }

        // Value
        if let Some(value) = &element.value {
            announcement.push_str(&format!(" {}", value));
        }

        // Description in verbose mode
        if self.config.lock().verbose && !element.description.is_empty() {
            announcement.push_str(&format!(". {}", element.description));
        }

        announcement
    }

    /// Build state change announcement
    fn build_state_announcement(&self, element: &AccessibleElement) -> String {
        if let Some(value) = &element.value {
            format!("{}: {}", element.label, value)
        } else if element.state.checked {
            format!("{} checked", element.label)
        } else if element.element_type == AccessibleType::CheckBox {
            format!("{} unchecked", element.label)
        } else if element.state.selected {
            format!("{} selected", element.label)
        } else if element.state.expanded {
            format!("{} expanded", element.label)
        } else {
            format!("{} changed", element.label)
        }
    }

    /// Build text change announcement
    fn build_text_announcement(&self, element: &AccessibleElement) -> String {
        if let Some(value) = &element.value {
            value.clone()
        } else {
            element.label.clone()
        }
    }

    /// Build full announcement (all details)
    fn build_full_announcement(&self, element: &AccessibleElement) -> String {
        let mut announcement = String::new();

        announcement.push_str(self.element_type_name(element.element_type));
        announcement.push_str(": ");
        announcement.push_str(&element.label);

        if !element.description.is_empty() {
            announcement.push_str(&format!(". {}", element.description));
        }

        if let Some(value) = &element.value {
            announcement.push_str(&format!(". Value: {}", value));
        }

        if element.state.read_only {
            announcement.push_str(". Read-only");
        }

        if element.state.required {
            announcement.push_str(". Required");
        }

        if element.state.invalid {
            announcement.push_str(". Invalid");
        }

        announcement
    }

    /// Get human-readable element type name
    fn element_type_name(&self, element_type: AccessibleType) -> &'static str {
        match element_type {
            AccessibleType::Container => "container",
            AccessibleType::Label => "label",
            AccessibleType::Button => "button",
            AccessibleType::TextBox => "text box",
            AccessibleType::CheckBox => "check box",
            AccessibleType::RadioButton => "radio button",
            AccessibleType::Slider => "slider",
            AccessibleType::ProgressBar => "progress bar",
            AccessibleType::Menu => "menu",
            AccessibleType::MenuItem => "menu item",
            AccessibleType::List => "list",
            AccessibleType::ListItem => "list item",
            AccessibleType::Table => "table",
            AccessibleType::TableCell => "table cell",
            AccessibleType::Tab => "tab",
            AccessibleType::ComboBox => "combo box",
            AccessibleType::TreeView => "tree view",
            AccessibleType::ScrollBar => "scroll bar",
            AccessibleType::Pane => "pane",
            AccessibleType::Unknown => "element",
        }
    }
}

/// Helper function to create a new accessible element
pub fn create_accessible_element(
    element_type: AccessibleType,
    region: ScreenRegion,
    label: &str,
    description: &str,
) -> AccessibleElement {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);

    AccessibleElement {
        id: NEXT_ID.fetch_add(1, Ordering::SeqCst),
        element_type,
        region,
        label: String::from(label),
        description: String::from(description),
        value: None,
        state: AccessibleState::default(),
        parent_id: None,
        child_ids: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_region_contains() {
        let region = ScreenRegion::new(10, 10, 100, 100);

        assert!(region.contains(10, 10));
        assert!(region.contains(50, 50));
        assert!(region.contains(109, 109));
        assert!(!region.contains(110, 110));
        assert!(!region.contains(5, 5));
    }

    #[test]
    fn test_screen_region_intersects() {
        let region1 = ScreenRegion::new(0, 0, 100, 100);
        let region2 = ScreenRegion::new(50, 50, 100, 100);
        let region3 = ScreenRegion::new(100, 100, 100, 100);

        assert!(region1.intersects(&region2));
        assert!(!region1.intersects(&region3));
    }

    #[test]
    fn test_create_accessible_element() {
        let region = ScreenRegion::new(0, 0, 100, 50);
        let element = create_accessible_element(
            AccessibleType::Button,
            region,
            "Click me",
            "A button"
        );

        assert_eq!(element.element_type, AccessibleType::Button);
        assert_eq!(element.label, "Click me");
        assert_eq!(element.description, "A button");
        assert!(element.state.enabled);
        assert!(element.state.visible);
        assert!(!element.state.focused);
    }

    #[test]
    fn test_screen_reader_config() {
        let config = ScreenReaderConfig::default();
        assert_eq!(config.speech_rate, 180);
        assert_eq!(config.pitch, 1.0);
        assert_eq!(config.volume, 0.8);
        assert!(config.enabled);
    }
}
