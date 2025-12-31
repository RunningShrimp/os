//! Keyboard Navigation and Focus Management
//!
//! This module provides comprehensive keyboard navigation functionality including:
//! - Full keyboard navigation support
//! - Tab order management
//! - Focus navigation between elements
//! - Keyboard shortcuts and hotkeys
//! - Visual focus indicators

use crate::subsystems::sync::spinlock::SpinLock;
use alloc::string::String;
use alloc::vec::Vec;
use crate::HashMap;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Navigation manager configuration
#[derive(Debug, Clone)]
pub struct NavigationConfig {
    /// Enable tab navigation
    pub enable_tab_navigation: bool,
    /// Enable arrow key navigation
    pub enable_arrow_navigation: bool,
    /// Wrap focus when reaching edges
    pub wrap_around: bool,
    /// Auto-cycle through elements
    pub auto_cycle: bool,
    /// Show visual focus indicators
    pub show_focus_indicators: bool,
    /// Focus indicator animation
    pub animate_focus: bool,
    /// Focus indicator style
    pub focus_style: FocusStyle,
    /// Tab navigation order
    pub tab_order: TabOrder,
    /// Keyboard shortcut modifiers
    pub shortcut_modifiers: ShortcutModifiers,
}

impl Default for NavigationConfig {
    fn default() -> Self {
        Self {
            enable_tab_navigation: true,
            enable_arrow_navigation: true,
            wrap_around: true,
            auto_cycle: false,
            show_focus_indicators: true,
            animate_focus: true,
            focus_style: FocusStyle::SolidBorder,
            tab_order: TabOrder::Logical,
            shortcut_modifiers: ShortcutModifiers::default(),
        }
    }
}

/// Focus indicator style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusStyle {
    /// No visible indicator
    None,
    /// Solid border
    SolidBorder,
    /// Dashed border
    DashedBorder,
    /// Dotted border
    DottedBorder,
    /// Glow effect
    Glow,
    /// Underline
    Underline,
    /// Custom style
    Custom,
}

/// Tab navigation order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabOrder {
    /// Logical order (as specified by element order)
    Logical,
    /// Spatial order (left-to-right, top-to-bottom)
    Spatial,
    /// Alphabetical order
    Alphabetical,
    /// Custom order
    Custom,
}

/// Keyboard modifier keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShortcutModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool, // Windows/Command key
}

impl ShortcutModifiers {
    pub fn none() -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        }
    }

    pub fn is_empty(&self) -> bool {
        !self.ctrl && !self.alt && !self.shift && !self.meta
    }
}

/// Keyboard event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardEvent {
    /// Key code
    pub key_code: KeyCode,
    /// Modifier keys
    pub modifiers: ShortcutModifiers,
    /// Key was pressed (true) or released (false)
    pub pressed: bool,
    /// Character input (for printable keys)
    pub character: Option<char>,
}

/// Virtual key codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Alphanumeric
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9,

    // Control keys
    Escape, Tab, Enter, Backspace, Space,

    // Navigation
    Left, Up, Right, Down, PageUp, PageDown, Home, End,

    // Modifiers
    ShiftLeft, ShiftRight, ControlLeft, ControlRight,
    AltLeft, AltRight, MetaLeft, MetaRight,

    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,

    // Other
    Insert, Delete, CapsLock, NumLock, ScrollLock,
    PrintScreen, Pause, Menu,

    // Unknown
    Unknown(u16),
}

/// Keyboard shortcut
#[derive(Debug, Clone)]
pub struct KeyboardShortcut {
    /// Unique identifier
    pub id: u64,
    /// Key combination
    pub key_code: KeyCode,
    pub modifiers: ShortcutModifiers,
    /// Action description
    pub description: String,
    /// Enabled state
    pub enabled: bool,
}

/// Focusable element
#[derive(Debug, Clone)]
pub struct FocusableElement {
    /// Unique identifier
    pub id: u64,
    /// Element name/label
    pub label: String,
    /// Element type
    pub element_type: FocusableType,
    /// Screen position
    pub position: ScreenPosition,
    /// Can receive focus
    pub can_focus: bool,
    /// Tab index (for custom tab order)
    pub tab_index: Option<i32>,
    /// Parent element ID
    pub parent_id: Option<u64>,
    /// Child element IDs
    pub child_ids: Vec<u64>,
    /// Enabled state
    pub enabled: bool,
    /// Visible state
    pub visible: bool,
}

/// Type of focusable element
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusableType {
    /// Button
    Button,
    /// Text input
    TextInput,
    /// Checkbox
    Checkbox,
    /// Radio button
    RadioButton,
    /// Combo box (dropdown)
    ComboBox,
    /// List box
    ListBox,
    /// Slider
    Slider,
    /// Spin button
    SpinButton,
    /// Tab in a tab control
    Tab,
    /// Link
    Link,
    /// Menu item
    MenuItem,
    /// Menu bar
    MenuBar,
    /// Tree view
    TreeView,
    /// Generic container
    Container,
}

/// Screen position
#[derive(Debug, Clone, Copy, Default)]
pub struct ScreenPosition {
    pub x: u32,
    pub y: u32,
}

impl ScreenPosition {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    pub fn distance(&self, other: ScreenPosition) -> f32 {
        let dx = (self.x as i32 - other.x as i32) as f32;
        let dy = (self.y as i32 - other.y as i32) as f32;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn is_above(&self, other: ScreenPosition) -> bool {
        self.y < other.y
    }

    pub fn is_below(&self, other: ScreenPosition) -> bool {
        self.y > other.y
    }

    pub fn is_left_of(&self, other: ScreenPosition) -> bool {
        self.x < other.x
    }

    pub fn is_right_of(&self, other: ScreenPosition) -> bool {
        self.x > other.x
    }
}

/// Focus direction for navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusDirection {
    Forward,   // Tab
    Backward,  // Shift+Tab
    Up,
    Down,
    Left,
    Right,
    First,     // Home/Ctrl+Home
    Last,      // End/Ctrl+End
    NextPage,  // PageDown
    PrevPage,  // PageUp
}

/// Navigation action result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationResult {
    /// Navigation succeeded
    Success,
    /// No focusable elements
    NoElements,
    /// Navigation blocked (wrap disabled)
    Blocked,
    /// Element not focusable
    NotFocusable,
    /// Element not found
    NotFound,
}

/// Keyboard navigation manager
pub struct KeyboardNavigation {
    config: SpinLock<NavigationConfig>,
    elements: SpinLock<HashMap<u64, FocusableElement>>,
    focused_element: SpinLock<Option<u64>>,
    shortcuts: SpinLock<HashMap<u64, KeyboardShortcut>>,
    next_element_id: AtomicU64,
    next_shortcut_id: AtomicU64,
    focus_history: SpinLock<Vec<u64>>,
    enabled: AtomicBool,
}

impl KeyboardNavigation {
    /// Create a new navigation manager
    pub fn new() -> Self {
        Self {
            config: SpinLock::new(NavigationConfig::default()),
            elements: SpinLock::new(HashMap::new()),
            focused_element: SpinLock::new(None),
            shortcuts: SpinLock::new(HashMap::new()),
            next_element_id: AtomicU64::new(1),
            next_shortcut_id: AtomicU64::new(1),
            focus_history: SpinLock::new(Vec::new()),
            enabled: AtomicBool::new(true),
        }
    }

    /// Enable or disable navigation
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
    }

    /// Check if navigation is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    /// Register a focusable element
    pub fn register_element(&self, element: FocusableElement) -> u64 {
        let id = element.id;
        self.elements.lock().insert(id, element);
        id
    }

    /// Unregister an element
    pub fn unregister_element(&self, id: u64) {
        let mut elements = self.elements.lock();
        elements.remove(&id);

        // Remove from focus history
        let mut history = self.focus_history.lock();
        history.retain(|&e| e != id);
    }

    /// Get element by ID
    pub fn get_element(&self, id: u64) -> Option<FocusableElement> {
        self.elements.lock().get(&id).cloned()
    }

    /// Set focus to an element
    pub fn set_focus(&self, id: u64) -> NavigationResult {
        if !self.is_enabled() {
            return NavigationResult::Blocked;
        }

        let elements = self.elements.lock();
        let element = elements.get(&id);

        if element.is_none() {
            return NavigationResult::NotFound;
        }

        let element = element.unwrap();

        if !element.can_focus || !element.enabled || !element.visible {
            return NavigationResult::NotFocusable;
        }

        drop(elements);

        // Update focus
        let mut focused = self.focused_element.lock();

        // Record previous focus in history
        if let Some(prev_id) = *focused {
            let mut history = self.focus_history.lock();
            if history.last() != Some(&prev_id) {
                history.push(prev_id);
            }
        }

        *focused = Some(id);

        NavigationResult::Success
    }

    /// Get currently focused element
    pub fn get_focused_element(&self) -> Option<FocusableElement> {
        let focused = self.focused_element.lock();
        if let Some(id) = *focused {
            self.elements.lock().get(&id).cloned()
        } else {
            None
        }
    }

    /// Clear focus
    pub fn clear_focus(&self) {
        *self.focused_element.lock() = None;
    }

    /// Navigate focus
    pub fn navigate(&self, direction: FocusDirection) -> NavigationResult {
        if !self.is_enabled() {
            return NavigationResult::Blocked;
        }

        let config = self.config.lock();
        let elements = self.elements.lock();
        let current = *self.focused_element.lock();

        // Collect focusable element IDs to avoid holding elements lock
        let focusable_ids: Vec<u64> = elements.values()
            .filter(|e| e.can_focus && e.enabled && e.visible)
            .map(|e| e.id)
            .collect();

        // Clone needed data before dropping locks
        let wrap_around = config.wrap_around;
        let tab_order = config.tab_order.clone();

        drop(elements);
        drop(config);

        if focusable_ids.is_empty() {
            return NavigationResult::NoElements;
        }

        // Reacquire elements lock for finding operations
        // Clone the elements to own them and avoid lifetime issues
        let focusable_owned: Vec<FocusableElement> = {
            let elements = self.elements.lock();
            elements.values()
                .filter(|e| focusable_ids.contains(&e.id))
                .map(|e| e.clone())
                .collect()
        };

        // Create references to the owned elements
        let focusable: Vec<&FocusableElement> = focusable_owned.iter().collect();

        let next_id = match direction {
            FocusDirection::Forward => {
                self.find_next_element(&focusable, current, &tab_order, true)
            }
            FocusDirection::Backward => {
                self.find_next_element(&focusable, current, &tab_order, false)
            }
            FocusDirection::Up => {
                self.find_spatial_element(&focusable, current, |curr, elem| elem.is_above(*curr))
            }
            FocusDirection::Down => {
                self.find_spatial_element(&focusable, current, |curr, elem| elem.is_below(*curr))
            }
            FocusDirection::Left => {
                self.find_spatial_element(&focusable, current, |curr, elem| elem.is_left_of(*curr))
            }
            FocusDirection::Right => {
                self.find_spatial_element(&focusable, current, |curr, elem| elem.is_right_of(*curr))
            }
            FocusDirection::First => {
                focusable.first().map(|e| e.id)
            }
            FocusDirection::Last => {
                focusable.last().map(|e| e.id)
            }
            FocusDirection::NextPage | FocusDirection::PrevPage => {
                // Page navigation - skip multiple elements
                self.find_page_element(&focusable, current, direction == FocusDirection::NextPage)
            }
        };

        if let Some(id) = next_id {
            self.set_focus(id)
        } else if !wrap_around {
            NavigationResult::Blocked
        } else {
            // Wrap around
            match direction {
                FocusDirection::Forward | FocusDirection::NextPage => {
                    if let Some(first) = focusable.first() {
                        self.set_focus(first.id)
                    } else {
                        NavigationResult::NoElements
                    }
                }
                FocusDirection::Backward | FocusDirection::PrevPage => {
                    if let Some(last) = focusable.last() {
                        self.set_focus(last.id)
                    } else {
                        NavigationResult::NoElements
                    }
                }
                _ => NavigationResult::Blocked,
            }
        }
    }

    /// Find next element in tab order
    fn find_next_element(
        &self,
        elements: &[&FocusableElement],
        current: Option<u64>,
        order: &TabOrder,
        forward: bool,
    ) -> Option<u64> {
        if elements.is_empty() {
            return None;
        }

        let sorted = match order {
            TabOrder::Logical => {
                // Sort by tab index, then by insertion order
                let mut sorted: Vec<_> = elements.to_vec();
                sorted.sort_by(|a, b| {
                    match (a.tab_index, b.tab_index) {
                        (Some(ai), Some(bi)) => ai.cmp(&bi),
                        (Some(_), None) => core::cmp::Ordering::Less,
                        (None, Some(_)) => core::cmp::Ordering::Greater,
                        (None, None) => a.id.cmp(&b.id),
                    }
                });
                sorted
            }
            TabOrder::Spatial => {
                // Sort by position (top-to-bottom, left-to-right)
                let mut sorted: Vec<_> = elements.to_vec();
                sorted.sort_by(|a, b| {
                    a.position.y.cmp(&b.position.y)
                        .then(a.position.x.cmp(&b.position.x))
                });
                sorted
            }
            TabOrder::Alphabetical => {
                // Sort by label
                let mut sorted: Vec<_> = elements.to_vec();
                sorted.sort_by(|a, b| a.label.cmp(&b.label));
                sorted
            }
            TabOrder::Custom => {
                elements.to_vec()
            }
        };

        if forward {
            if let Some(current_id) = current {
                let pos = sorted.iter().position(|e| e.id == current_id);
                if let Some(pos) = pos {
                    if pos + 1 < sorted.len() {
                        return Some(sorted[pos + 1].id);
                    }
                }
            }
            sorted.first().map(|e| e.id)
        } else {
            if let Some(current_id) = current {
                let pos = sorted.iter().position(|e| e.id == current_id);
                if let Some(pos) = pos {
                    if pos > 0 {
                        return Some(sorted[pos - 1].id);
                    }
                }
            }
            sorted.last().map(|e| e.id)
        }
    }

    /// Find element in spatial direction
    fn find_spatial_element<F>(&self, elements: &[&FocusableElement], current: Option<u64>, direction: F) -> Option<u64>
    where
        F: Fn(&ScreenPosition, &ScreenPosition) -> bool,
    {
        let current_pos = if let Some(id) = current {
            self.elements.lock().get(&id).map(|e| e.position)
        } else {
            None
        };

        let current_pos = match current_pos {
            Some(pos) => pos,
            None => return elements.first().map(|e| e.id),
        };

        // Find closest element in the direction
        let mut closest: Option<(&FocusableElement, f32)> = None;

        for element in elements {
            if Some(element.id) == current {
                continue;
            }

            if direction(&element.position, &current_pos) {
                let dist = element.position.distance(current_pos);
                match closest {
                    None => closest = Some((element, dist)),
                    Some((_, closest_dist)) => {
                        if dist < closest_dist {
                            closest = Some((element, dist));
                        }
                    }
                }
            }
        }

        closest.map(|(e, _)| e.id)
    }

    /// Find element for page navigation
    fn find_page_element(&self, elements: &[&FocusableElement], current: Option<u64>, forward: bool) -> Option<u64> {
        // Skip roughly 10 elements for page navigation
        let page_size = 10.max(elements.len() / 10);

        if let Some(current_id) = current {
            let pos = elements.iter().position(|e| e.id == current_id);
            if let Some(pos) = pos {
                if forward {
                    let new_pos = (pos + page_size).min(elements.len() - 1);
                    return Some(elements[new_pos].id);
                } else {
                    let new_pos = pos.saturating_sub(page_size);
                    return Some(elements[new_pos].id);
                }
            }
        }

        if forward {
            elements.first().map(|e| e.id)
        } else {
            elements.last().map(|e| e.id)
        }
    }

    /// Register keyboard shortcut
    pub fn register_shortcut(&self, key_code: KeyCode, modifiers: ShortcutModifiers, description: String) -> u64 {
        let id = self.next_shortcut_id.fetch_add(1, Ordering::SeqCst);

        let shortcut = KeyboardShortcut {
            id,
            key_code,
            modifiers,
            description,
            enabled: true,
        };

        self.shortcuts.lock().insert(id, shortcut);
        id
    }

    /// Unregister shortcut
    pub fn unregister_shortcut(&self, id: u64) {
        self.shortcuts.lock().remove(&id);
    }

    /// Enable/disable shortcut
    pub fn set_shortcut_enabled(&self, id: u64, enabled: bool) {
        let mut shortcuts = self.shortcuts.lock();
        if let Some(shortcut) = shortcuts.get_mut(&id) {
            shortcut.enabled = enabled;
        }
    }

    /// Handle keyboard event
    pub fn handle_key_event(&self, event: KeyboardEvent) -> bool {
        if !self.is_enabled() || !event.pressed {
            return false;
        }

        // Handle navigation keys
        let handled = match event.key_code {
            KeyCode::Tab if !event.modifiers.shift => {
                self.navigate(FocusDirection::Forward);
                true
            }
            KeyCode::Tab if event.modifiers.shift => {
                self.navigate(FocusDirection::Backward);
                true
            }
            KeyCode::Up if event.modifiers.is_empty() => {
                self.navigate(FocusDirection::Up);
                true
            }
            KeyCode::Down if event.modifiers.is_empty() => {
                self.navigate(FocusDirection::Down);
                true
            }
            KeyCode::Left if event.modifiers.is_empty() => {
                self.navigate(FocusDirection::Left);
                true
            }
            KeyCode::Right if event.modifiers.is_empty() => {
                self.navigate(FocusDirection::Right);
                true
            }
            KeyCode::Home => {
                self.navigate(FocusDirection::First);
                true
            }
            KeyCode::End => {
                self.navigate(FocusDirection::Last);
                true
            }
            KeyCode::PageUp => {
                self.navigate(FocusDirection::PrevPage);
                true
            }
            KeyCode::PageDown => {
                self.navigate(FocusDirection::NextPage);
                true
            }
            _ => false,
        };

        if handled {
            return true;
        }

        // Check for keyboard shortcuts
        let shortcuts = self.shortcuts.lock();
        for shortcut in shortcuts.values() {
            if shortcut.enabled &&
               shortcut.key_code == event.key_code &&
               shortcut.modifiers.ctrl == event.modifiers.ctrl &&
               shortcut.modifiers.alt == event.modifiers.alt &&
               shortcut.modifiers.shift == event.modifiers.shift &&
               shortcut.modifiers.meta == event.modifiers.meta {
                return true;
            }
        }

        false
    }

    /// Get all registered shortcuts
    pub fn get_shortcuts(&self) -> Vec<KeyboardShortcut> {
        self.shortcuts.lock().values().cloned().collect()
    }

    /// Update configuration
    pub fn update_config<F>(&self, f: F)
    where
        F: FnOnce(&mut NavigationConfig),
    {
        f(&mut self.config.lock());
    }

    /// Get current configuration
    pub fn get_config(&self) -> NavigationConfig {
        self.config.lock().clone()
    }

    /// Get focus history (most recent first)
    pub fn get_focus_history(&self) -> Vec<u64> {
        let history = self.focus_history.lock();
        history.iter().rev().copied().collect()
    }

    /// Navigate back in focus history
    pub fn navigate_back(&self) -> NavigationResult {
        let mut history = self.focus_history.lock();

        if history.is_empty() {
            return NavigationResult::NotFound;
        }

        let prev_id = history.pop();

        drop(history);

        if let Some(id) = prev_id {
            self.set_focus(id)
        } else {
            NavigationResult::NotFound
        }
    }

    /// Clear focus history
    pub fn clear_history(&self) {
        self.focus_history.lock().clear();
    }
}

impl Default for KeyboardNavigation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_creation() {
        let nav = KeyboardNavigation::new();
        assert!(nav.is_enabled());
        assert!(nav.get_focused_element().is_none());
    }

    #[test]
    fn test_element_registration() {
        let nav = KeyboardNavigation::new();

        let element = FocusableElement {
            id: 1,
            label: "Button".into(),
            element_type: FocusableType::Button,
            position: ScreenPosition::new(100, 100),
            can_focus: true,
            tab_index: None,
            parent_id: None,
            child_ids: Vec::new(),
            enabled: true,
            visible: true,
        };

        nav.register_element(element);

        assert!(nav.get_element(1).is_some());
    }

    #[test]
    fn test_focus_navigation() {
        let nav = KeyboardNavigation::new();

        let mut elements = Vec::new();
        for i in 1..=3 {
            elements.push(FocusableElement {
                id: i,
                label: format!("Button {}", i),
                element_type: FocusableType::Button,
                position: ScreenPosition::new(i * 100, 100),
                can_focus: true,
                tab_index: None,
                parent_id: None,
                child_ids: Vec::new(),
                enabled: true,
                visible: true,
            });
        }

        for element in &elements {
            nav.register_element(element.clone());
        }

        assert_eq!(nav.navigate(FocusDirection::First), NavigationResult::Success);
        assert_eq!(nav.get_focused_element().unwrap().id, 1);

        assert_eq!(nav.navigate(FocusDirection::Forward), NavigationResult::Success);
        assert_eq!(nav.get_focused_element().unwrap().id, 2);
    }

    #[test]
    fn test_keyboard_shortcuts() {
        let nav = KeyboardNavigation::new();

        let id = nav.register_shortcut(
            KeyCode::Q,
            ShortcutModifiers { ctrl: true, ..Default::default() },
            "Quit".into()
        );

        let shortcuts = nav.get_shortcuts();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].key_code, KeyCode::Q);
    }

    #[test]
    fn test_screen_position() {
        let pos1 = ScreenPosition::new(100, 100);
        let pos2 = ScreenPosition::new(200, 100);

        assert!(pos1.is_left_of(pos2));
        assert!(pos2.is_right_of(pos1));
        assert!(!pos1.is_above(pos2));
        assert!(!pos1.is_below(pos2));
    }
}
