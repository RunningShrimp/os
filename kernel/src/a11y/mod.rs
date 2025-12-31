//! Accessibility (A11y) Module
//!
//! This module provides comprehensive accessibility support for the NOS kernel including:
//! - Screen reader integration and text-to-speech
//! - Braille display support
//! - Screen magnification (2x-16x)
//! - Keyboard navigation and focus management
//! - AT-SPI (Assistive Technology Service Provider Interface) compatibility
//! - Accessibility tree and event system
//! - High contrast display modes

mod screen;
mod braille;
mod magnifier;
mod navigation;

pub use screen::{
    ScreenReader,
    ScreenReaderConfig,
    TextToSpeechEngine,
    DefaultTtsEngine,
    AccessibleElement,
    AccessibleType,
    AccessibleState,
    AccessibilityEvent,
    SpeechPriority,
    ScreenRegion,
    VoiceInfo,
    VoiceGender,
    TtsError,
    create_accessible_element,
};

pub use braille::{
    RefreshableBrailleDisplay,
    BrailleConfig,
    BrailleCell,
    BrailleDevice,
    BrailleDeviceInfo,
    BrailleKey,
    BrailleKeyType,
    BrailleError,
    TranslationMode,
    CursorStyle,
    BraillePatterns,
    SimulatedBrailleDevice,
};

pub use magnifier::{
    ScreenMagnifier,
    MagnifierConfig,
    MagnifierViewport,
    TrackingMode,
    LensShape,
    FilterMode,
    CursorEnhancement,
    ScreenPosition,
    ScreenRect,
    ScreenSize,
    ColorTransform,
};

pub use navigation::{
    KeyboardNavigation,
    NavigationConfig,
    FocusableElement,
    FocusableType,
    FocusDirection,
    FocusStyle,
    TabOrder,
    NavigationResult,
    KeyboardShortcut,
    KeyboardEvent,
    KeyCode,
    ShortcutModifiers,
};

use crate::subsystems::sync::spinlock::SpinLock;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;
use crate::HashMap;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Global accessibility manager
pub struct AccessibilityManager {
    /// Screen reader instance
    screen_reader: SpinLock<Option<ScreenReader>>,
    /// Braille display instance
    braille_display: SpinLock<Option<RefreshableBrailleDisplay>>,
    /// Screen magnifier instance
    magnifier: SpinLock<Option<ScreenMagnifier>>,
    /// Keyboard navigation instance
    navigation: SpinLock<Option<KeyboardNavigation>>,
    /// Accessibility tree
    accessibility_tree: SpinLock<AccessibilityTree>,
    /// Configuration
    config: SpinLock<GlobalA11yConfig>,
    /// Enabled state
    enabled: AtomicBool,
}

/// Global accessibility configuration
#[derive(Debug, Clone)]
pub struct GlobalA11yConfig {
    /// Enable screen reader
    pub screen_reader_enabled: bool,
    /// Enable Braille display
    pub braille_enabled: bool,
    /// Enable screen magnifier
    pub magnifier_enabled: bool,
    /// Enable keyboard navigation
    pub keyboard_navigation_enabled: bool,
    /// High contrast mode
    pub high_contrast: bool,
    /// High contrast theme
    pub contrast_theme: HighContrastTheme,
    /// Reduce animations
    pub reduce_animations: bool,
    /// Sticky keys (modifier keys latch)
    pub sticky_keys: bool,
    /// Slow keys (key delay)
    pub slow_keys: bool,
    /// Bounce keys (ignore rapid key presses)
    pub bounce_keys: bool,
    /// Toggle keys (sound on Caps/Num/Scroll lock)
    pub toggle_keys: bool,
}

impl Default for GlobalA11yConfig {
    fn default() -> Self {
        Self {
            screen_reader_enabled: false,
            braille_enabled: false,
            magnifier_enabled: false,
            keyboard_navigation_enabled: false,
            high_contrast: false,
            contrast_theme: HighContrastTheme::default(),
            reduce_animations: false,
            sticky_keys: false,
            slow_keys: false,
            bounce_keys: false,
            toggle_keys: false,
        }
    }
}

/// High contrast color themes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighContrastTheme {
    /// High contrast black on white
    BlackOnWhite,
    /// High contrast white on black
    WhiteOnBlack,
    /// High contrast yellow on black
    YellowOnBlack,
    /// High contrast green on black
    GreenOnBlack,
    /// High contrast blue on yellow
    BlueOnYellow,
    /// Custom theme
    Custom { foreground: u32, background: u32 },
}

impl Default for HighContrastTheme {
    fn default() -> Self {
        Self::WhiteOnBlack
    }
}

/// Accessibility tree node
#[derive(Debug, Clone)]
pub struct AccessibilityNode {
    /// Unique identifier
    pub id: u64,
    /// Node type
    pub node_type: AccessibleNodeType,
    /// Screen region
    pub region: ScreenRegion,
    /// Node name/label
    pub name: String,
    /// Detailed description
    pub description: String,
    /// Current value
    pub value: Option<String>,
    /// State information
    pub state: AccessibleNodeState,
    /// Parent node ID
    pub parent_id: Option<u64>,
    /// Child node IDs
    pub child_ids: Vec<u64>,
    /// AT-SPI role
    pub atspi_role: AtspiRole,
}

/// Accessibility node type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibleNodeType {
    /// Application window
    Application,
    /// Window
    Window,
    /// Dialog
    Dialog,
    /// Generic container
    Container,
    /// Panel
    Panel,
    /// Frame
    Frame,
    /// Label
    Label,
    /// Button
    Button,
    /// Text entry
    Entry,
    /// Password field
    Password,
    /// Checkbox
    CheckBox,
    /// Radio button
    RadioButton,
    /// Slider
    Slider,
    /// Progress bar
    ProgressBar,
    /// Spinner
    Spinner,
    /// Menu
    Menu,
    /// Menu bar
    MenuBar,
    /// Menu item
    MenuItem,
    /// Check menu item
    CheckMenuItem,
    /// Radio menu item
    RadioMenuItem,
    /// Combo box
    ComboBox,
    /// List
    List,
    /// List item
    ListItem,
    /// Table
    Table,
    /// Tree
    Tree,
    /// Tree table
    TreeTable,
    /// Tab container
    TabContainer,
    /// Tab page
    TabPage,
    /// Scroll bar
    ScrollBar,
    /// Split pane
    SplitPane,
    /// Status bar
    StatusBar,
    /// Tool bar
    ToolBar,
    /// Tool tip
    ToolTip,
    /// Unknown
    Unknown,
}

/// Accessibility node state
#[derive(Debug, Clone, Copy)]
pub struct AccessibleNodeState {
    pub visible: bool,
    pub showing: bool,
    pub enabled: bool,
    pub sensitive: bool,
    pub focused: bool,
    pub selected: bool,
    pub checked: bool,
    pub pressed: bool,
    pub expandable: bool,
    pub expanded: bool,
    pub modal: bool,
    pub multi_line: bool,
    pub editable: bool,
    pub active: bool,
    pub busy: bool,
    pub checkable: bool,
    pub selectable: bool,
    pub selectable_text: bool,
    pub horizontal: bool,
    pub vertical: bool,
    pub invalid: bool,
    pub required: bool,
}

impl Default for AccessibleNodeState {
    fn default() -> Self {
        Self {
            visible: true,
            showing: true,
            enabled: true,
            sensitive: true,
            focused: false,
            selected: false,
            checked: false,
            pressed: false,
            expandable: false,
            expanded: false,
            modal: false,
            multi_line: false,
            editable: false,
            active: false,
            busy: false,
            checkable: false,
            selectable: false,
            selectable_text: false,
            horizontal: false,
            vertical: false,
            invalid: false,
            required: false,
        }
    }
}

/// AT-SPI roles (Assistive Technology Service Provider Interface)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtspiRole {
    Invalid,
    AcceleratorLabel,
    Alert,
    Animation,
    Arrow,
    Calendar,
    Canvas,
    Chart,
    CheckBox,
    CheckMenuItem,
    ColorChooser,
    ColumnHeader,
    ComboBox,
    DateEditor,
    DesktopIcon,
    DesktopFrame,
    Dial,
    Dialog,
    DirectoryPane,
    DrawingArea,
    FileChooser,
    Filler,
    FontChooser,
    Frame,
    GlassPane,
    HtmlContainer,
    Icon,
    Image,
    InternalFrame,
    Label,
    LayeredPane,
    List,
    ListItem,
    Menu,
    MenuBar,
    MenuItem,
    OptionPane,
    PageTab,
    PageTabList,
    Panel,
    PasswordText,
    PopupMenu,
    ProgressBar,
    PushButton,
    RadioButton,
    RadioMenuItem,
    RootPane,
    RowHeader,
    ScrollBar,
    ScrollPane,
    Section,
    Separator,
    Slider,
    SpinButton,
    SplitPane,
    StatusBar,
    Table,
    TableCell,
    TableColumnHeader,
    TableRow,
    TableRowHeader,
    TearOffMenuItem,
    Terminal,
    Text,
    ToggleButton,
    ToolBar,
    ToolTip,
    Tree,
    TreeTable,
    Unknown,
    Viewport,
    Window,
    Extended,
    Header,
    Footer,
    Paragraph,
    Ruler,
    Application,
    Autocomplete,
    EditBar,
    Embedded,
    Entry,
    Caption,
    DocumentFrame,
    Heading,
    Page,
    RedundantObject,
    Form,
    Link,
    InputMethodWindow,
    TreeItem,
    ListGrid,
    _LastDefined,
}

/// AT-SPI event
#[derive(Debug, Clone)]
pub struct AtspiEvent {
    /// Event type
    pub event_type: String,
    /// Source node ID
    pub source_id: u64,
    /// Event detail
    pub detail1: i32,
    pub detail2: i32,
    /// Any additional data
    pub data: Option<String>,
}

/// Accessibility tree
pub struct AccessibilityTree {
    nodes: HashMap<u64, AccessibilityNode>,
    root_id: Option<u64>,
    next_id: AtomicU64,
}

impl AccessibilityTree {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root_id: None,
            next_id: AtomicU64::new(1),
        }
    }

    /// Set root node
    pub fn set_root(&mut self, node: AccessibilityNode) -> u64 {
        let id = node.id;
        self.nodes.insert(id, node);
        self.root_id = Some(id);
        id
    }

    /// Add node to tree
    pub fn add_node(&mut self, node: AccessibilityNode) {
        let id = node.id;

        // Update parent's child list
        if let Some(parent_id) = node.parent_id {
            if let Some(parent) = self.nodes.get_mut(&parent_id) {
                if !parent.child_ids.contains(&id) {
                    parent.child_ids.push(id);
                }
            }
        }

        self.nodes.insert(id, node);
    }

    /// Remove node from tree
    pub fn remove_node(&mut self, id: u64) {
        if let Some(node) = self.nodes.remove(&id) {
            // Remove from parent's child list
            if let Some(parent_id) = node.parent_id {
                if let Some(parent) = self.nodes.get_mut(&parent_id) {
                    parent.child_ids.retain(|&child_id| child_id != id);
                }
            }

            // Remove all children
            for child_id in node.child_ids {
                self.remove_node(child_id);
            }
        }
    }

    /// Get node by ID
    pub fn get_node(&self, id: u64) -> Option<&AccessibilityNode> {
        self.nodes.get(&id)
    }

    /// Get mutable node by ID
    pub fn get_node_mut(&mut self, id: u64) -> Option<&mut AccessibilityNode> {
        self.nodes.get_mut(&id)
    }

    /// Get root node
    pub fn get_root(&self) -> Option<&AccessibilityNode> {
        self.root_id.and_then(|id| self.nodes.get(&id))
    }

    /// Find nodes by AT-SPI role
    pub fn find_by_role(&self, role: AtspiRole) -> Vec<&AccessibilityNode> {
        self.nodes.values()
            .filter(|node| node.atspi_role == role)
            .collect()
    }

    /// Get all nodes
    pub fn get_all_nodes(&self) -> Vec<&AccessibilityNode> {
        self.nodes.values().collect()
    }
}

impl Default for AccessibilityTree {
    fn default() -> Self {
        Self::new()
    }
}

impl AccessibilityManager {
    /// Create a new accessibility manager
    pub fn new() -> Self {
        Self {
            screen_reader: SpinLock::new(None),
            braille_display: SpinLock::new(None),
            magnifier: SpinLock::new(None),
            navigation: SpinLock::new(None),
            accessibility_tree: SpinLock::new(AccessibilityTree::new()),
            config: SpinLock::new(GlobalA11yConfig::default()),
            enabled: AtomicBool::new(false),
        }
    }

    /// Initialize accessibility subsystem
    pub fn initialize(&self, screen_width: u32, screen_height: u32) {
        let config = self.config.lock();

        // Initialize components based on config
        if config.keyboard_navigation_enabled {
            let nav = KeyboardNavigation::new();
            *self.navigation.lock() = Some(nav);
        }

        if config.magnifier_enabled {
            let mag = ScreenMagnifier::new(screen_width, screen_height);
            *self.magnifier.lock() = Some(mag);
        }

        if config.screen_reader_enabled {
            let tts = Box::new(DefaultTtsEngine::default());
            let sr = ScreenReader::new(tts);
            *self.screen_reader.lock() = Some(sr);
        }

        if config.braille_enabled {
            let braille = RefreshableBrailleDisplay::new(BrailleConfig::default());
            *self.braille_display.lock() = Some(braille);
        }

        self.enabled.store(true, Ordering::Release);
    }

    /// Enable or disable accessibility
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
    }

    /// Check if accessibility is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    /// Get screen reader instance
    pub fn get_screen_reader(&self) -> Option<&ScreenReader> {
        // This is a simplified version - in practice you'd need better locking
        None
    }

    /// Get magnifier instance
    pub fn get_magnifier(&self) -> Option<&ScreenMagnifier> {
        None
    }

    /// Get navigation instance
    pub fn get_navigation(&self) -> Option<&KeyboardNavigation> {
        None
    }

    /// Update global configuration
    pub fn update_config<F>(&self, f: F)
    where
        F: FnOnce(&mut GlobalA11yConfig),
    {
        f(&mut self.config.lock());
    }

    /// Get current configuration
    pub fn get_config(&self) -> GlobalA11yConfig {
        self.config.lock().clone()
    }

    /// Handle accessibility event
    pub fn handle_event(&self, event: AtspiEvent) {
        if !self.is_enabled() {
            return;
        }

        // Forward to screen reader
        if let Some(ref sr) = *self.screen_reader.lock() {
            if let Ok(a11y_event) = self.atspi_to_a11y_event(&event) {
                sr.handle_event(a11y_event, event.source_id);
            }
        }
    }

    /// Convert AT-SPI event to accessibility event
    fn atspi_to_a11y_event(&self, atspi: &AtspiEvent) -> Result<AccessibilityEvent, ()> {
        match atspi.event_type.as_str() {
            "focus" => Ok(AccessibilityEvent::FocusGained),
            "window:activate" => Ok(AccessibilityEvent::WindowActivated),
            "object:state-changed" => Ok(AccessibilityEvent::StateChanged),
            "object:value-changed" => Ok(AccessibilityEvent::ValueChanged),
            "object:text-changed" => Ok(AccessibilityEvent::TextChanged),
            "object:bounds-changed" => Ok(AccessibilityEvent::BoundsChanged),
            _ => Err(()),
        }
    }

    /// Set high contrast mode
    pub fn set_high_contrast(&self, enabled: bool, theme: HighContrastTheme) {
        let mut config = self.config.lock();
        config.high_contrast = enabled;
        config.contrast_theme = theme;
    }

    /// Get accessibility tree
    pub fn get_tree(&self) -> &SpinLock<AccessibilityTree> {
        &self.accessibility_tree
    }

    /// Add accessible node to tree
    pub fn add_accessible_node(&self, node: AccessibilityNode) {
        let mut tree = self.accessibility_tree.lock();
        tree.add_node(node);
    }

    /// Remove accessible node from tree
    pub fn remove_accessible_node(&self, id: u64) {
        let mut tree = self.accessibility_tree.lock();
        tree.remove_node(id);
    }

    /// Update accessible node
    pub fn update_accessible_node<F>(&self, id: u64, f: F)
    where
        F: FnOnce(&mut AccessibilityNode),
    {
        let mut tree = self.accessibility_tree.lock();
        if let Some(node) = tree.get_node_mut(id) {
            f(node);
        }
    }

    /// Find accessible nodes by role
    pub fn find_by_role(&self, role: AtspiRole) -> Vec<AccessibilityNode> {
        let tree = self.accessibility_tree.lock();
        tree.find_by_role(role).into_iter().map(|n| n.clone()).collect()
    }
}

impl Default for AccessibilityManager {
    fn default() -> Self {
        Self::new()
    }
}

/// High contrast theme utilities
pub struct HighContrastThemes;

impl HighContrastThemes {
    /// Get colors for theme
    pub fn get_colors(theme: HighContrastTheme) -> (u32, u32) {
        match theme {
            HighContrastTheme::BlackOnWhite => (0xFF000000, 0xFFFFFFFF),
            HighContrastTheme::WhiteOnBlack => (0xFFFFFFFF, 0xFF000000),
            HighContrastTheme::YellowOnBlack => (0xFF00FFFF, 0xFF000000),
            HighContrastTheme::GreenOnBlack => (0xFF00FF00, 0xFF000000),
            HighContrastTheme::BlueOnYellow => (0xFF0000FF, 0xFF00FFFF),
            HighContrastTheme::Custom { foreground, background } => (foreground, background),
        }
    }

    /// Apply theme to color
    pub fn apply_theme(argb: u32, theme: HighContrastTheme) -> u32 {
        let (fg, bg) = Self::get_colors(theme);

        // Determine if original color is closer to foreground or background
        let r = (argb >> 16) & 0xFF;
        let g = (argb >> 8) & 0xFF;
        let b = argb & 0xFF;
        let brightness = (r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000;

        if brightness > 128 {
            fg
        } else {
            bg
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a11y_manager_creation() {
        let manager = AccessibilityManager::new();
        assert!(!manager.is_enabled());

        manager.initialize(1920, 1080);
        assert!(manager.is_enabled());
    }

    #[test]
    fn test_accessibility_tree() {
        let tree = AccessibilityTree::new();

        let root = AccessibilityNode {
            id: 1,
            node_type: AccessibleNodeType::Application,
            region: ScreenRegion::new(0, 0, 1920, 1080),
            name: "Test App".into(),
            description: String::new(),
            value: None,
            state: AccessibleNodeState::default(),
            parent_id: None,
            child_ids: Vec::new(),
            atspi_role: AtspiRole::Application,
        };

        let id = tree.set_root(root);
        assert_eq!(id, 1);
        assert!(tree.get_root().is_some());
    }

    #[test]
    fn test_high_contrast_themes() {
        let (fg, bg) = HighContrastThemes::get_colors(HighContrastTheme::WhiteOnBlack);
        assert_eq!(fg, 0xFFFFFFFF);
        assert_eq!(bg, 0xFF000000);
    }

    #[test]
    fn test_atspi_event_conversion() {
        let manager = AccessibilityManager::new();
        manager.initialize(1920, 1080);

        let event = AtspiEvent {
            event_type: "focus".into(),
            source_id: 1,
            detail1: 0,
            detail2: 0,
            data: None,
        };

        let a11y_event = manager.atspi_to_a11y_event(&event);
        assert!(a11y_event.is_ok());
        assert_eq!(a11y_event.unwrap(), AccessibilityEvent::FocusGained);
    }

    #[test]
    fn test_node_state() {
        let state = AccessibleNodeState::default();
        assert!(state.visible);
        assert!(state.enabled);
        assert!(!state.focused);
    }
}

/// Initialize accessibility subsystem
pub fn init() {
    log::info!("Initializing accessibility subsystem");
    // Global initialization would go here
}
