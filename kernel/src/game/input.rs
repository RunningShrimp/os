//! # Game Input System
//!
//! Comprehensive input handling with:
//! - Keyboard and mouse input
//! - Gamepad/controller support
//! - Touch input for mobile
//! - Input mapping and actions
//! - Virtual controls
//! - Input recording and replay

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::compat::Float;

/// Input action IDs
static NEXT_ACTION_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActionId {
    id: u32,
}

impl ActionId {
    #[inline]
    pub fn new() -> Self {
        Self {
            id: NEXT_ACTION_ID.fetch_add(1, Ordering::SeqCst),
        }
    }

    #[inline]
    pub fn from_raw(id: u32) -> Self {
        Self { id }
    }
}

impl Default for ActionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Keyboard key codes
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Unknown,
    Space,
    Apostrophe,
    Comma,
    Minus,
    Period,
    Slash,
    Alpha0,
    Alpha1,
    Alpha2,
    Alpha3,
    Alpha4,
    Alpha5,
    Alpha6,
    Alpha7,
    Alpha8,
    Alpha9,
    Semicolon,
    Equal,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    LeftBracket,
    Backslash,
    RightBracket,
    GraveAccent,
    Escape,
    Enter,
    Tab,
    Backspace,
    Insert,
    Delete,
    Right,
    Left,
    Down,
    Up,
    PageUp,
    PageDown,
    Home,
    End,
    CapsLock,
    ScrollLock,
    NumLock,
    PrintScreen,
    Pause,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    Keypad0,
    Keypad1,
    Keypad2,
    Keypad3,
    Keypad4,
    Keypad5,
    Keypad6,
    Keypad7,
    Keypad8,
    Keypad9,
    KeypadDecimal,
    KeypadDivide,
    KeypadMultiply,
    KeypadSubtract,
    KeypadAdd,
    KeypadEnter,
    KeypadEqual,
    LeftShift,
    LeftControl,
    LeftAlt,
    LeftSuper,
    RightShift,
    RightControl,
    RightAlt,
    RightSuper,
    Menu,
}

/// Mouse buttons
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Unknown,
    Left,
    Right,
    Middle,
    Button4,
    Button5,
    Button6,
    Button7,
    Button8,
}

/// Gamepad buttons
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    A,
    B,
    X,
    Y,
    LeftBumper,
    RightBumper,
    LeftTrigger,
    RightTrigger,
    Back,
    Start,
    Guide,
    LeftThumb,
    RightThumb,
    DPadUp,
    DPadRight,
    DPadDown,
    DPadLeft,
    Misc1,
    Paddle1,
    Paddle2,
    Paddle3,
    Paddle4,
    Touchpad,
}

/// Gamepad axes
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
    LeftTrigger,
    RightTrigger,
}

/// Touch input
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchPoint {
    pub id: u64,
    pub x: Float,
    pub y: Float,
    pub pressure: Float,
}

/// Input state
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputState {
    Released,
    Pressed,
    JustPressed,
    JustReleased,
}

impl InputState {
    #[inline]
    pub fn is_pressed(&self) -> bool {
        matches!(self, InputState::Pressed | InputState::JustPressed)
    }

    #[inline]
    pub fn is_released(&self) -> bool {
        matches!(self, InputState::Released | InputState::JustReleased)
    }

    #[inline]
    pub fn just_pressed(&self) -> bool {
        matches!(self, InputState::JustPressed)
    }

    #[inline]
    pub fn just_released(&self) -> bool {
        matches!(self, InputState::JustReleased)
    }
}

/// Input action
#[derive(Clone, Debug)]
pub struct InputAction {
    pub id: ActionId,
    pub name: alloc::string::String,
    pub bindings: Vec<InputBinding>,
    pub state: InputState,
    pub value: Float,
    pub deadzone: Float,
    pub sensitivity: Float,
}

impl InputAction {
    #[inline]
    pub fn new(name: alloc::string::String) -> Self {
        Self {
            id: ActionId::new(),
            name,
            bindings: Vec::new(),
            state: InputState::Released,
            value: 0.0,
            deadzone: 0.1,
            sensitivity: 1.0,
        }
    }

    #[inline]
    pub fn with_binding(mut self, binding: InputBinding) -> Self {
        self.bindings.push(binding);
        self
    }

    #[inline]
    pub fn with_deadzone(mut self, deadzone: Float) -> Self {
        self.deadzone = deadzone.max(0.0);
        self
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.state.is_pressed()
    }

    #[inline]
    pub fn get_value(&self) -> Float {
        self.value
    }

    #[inline]
    pub fn get_raw_value(&self) -> Float {
        self.value / self.sensitivity
    }
}

/// Input binding
#[derive(Clone, Debug)]
pub enum InputBinding {
    Key(KeyCode),
    Mouse(MouseButton),
    GamepadButton(GamepadButton),
    GamepadAxis(GamepadAxis),
    MouseAxis(MouseAxis),
}

/// Mouse axes
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseAxis {
    X,
    Y,
    ScrollX,
    ScrollY,
}

/// Input context (group of actions)
#[derive(Clone, Debug)]
pub struct InputContext {
    pub name: alloc::string::String,
    pub actions: BTreeMap<alloc::string::String, InputAction>,
    pub enabled: bool,
}

impl InputContext {
    #[inline]
    pub fn new(name: alloc::string::String) -> Self {
        Self {
            name,
            actions: BTreeMap::new(),
            enabled: true,
        }
    }

    #[inline]
    pub fn add_action(&mut self, action: InputAction) {
        let name = action.name.clone();
        self.actions.insert(name, action);
    }

    #[inline]
    pub fn get_action(&self, name: &str) -> Option<&InputAction> {
        self.actions.get(name)
    }

    #[inline]
    pub fn get_action_mut(&mut self, name: &str) -> Option<&mut InputAction> {
        self.actions.get_mut(name)
    }

    #[inline]
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    #[inline]
    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

/// Input manager
pub struct InputManager {
    contexts: Vec<InputContext>,
    active_context: usize,
    keyboard_state: BTreeMap<KeyCode, InputState>,
    mouse_state: BTreeMap<MouseButton, InputState>,
    mouse_position: (Float, Float),
    mouse_delta: (Float, Float),
    scroll_delta: (Float, Float),
    gamepads: Vec<GamepadState>,
    touches: Vec<TouchPoint>,
}

impl InputManager {
    #[inline]
    pub fn new() -> Self {
        Self {
            contexts: Vec::new(),
            active_context: 0,
            keyboard_state: BTreeMap::new(),
            mouse_state: BTreeMap::new(),
            mouse_position: (0.0, 0.0),
            mouse_delta: (0.0, 0.0),
            scroll_delta: (0.0, 0.0),
            gamepads: Vec::new(),
            touches: Vec::new(),
        }
    }

    #[inline]
    pub fn add_context(&mut self, context: InputContext) {
        self.contexts.push(context);
    }

    #[inline]
    pub fn set_active_context(&mut self, index: usize) {
        if index < self.contexts.len() {
            self.active_context = index;
        }
    }

    #[inline]
    pub fn get_active_context(&self) -> Option<&InputContext> {
        self.contexts.get(self.active_context)
    }

    #[inline]
    pub fn get_active_context_mut(&mut self) -> Option<&mut InputContext> {
        self.contexts.get_mut(self.active_context)
    }

    #[inline]
    pub fn get_action(&self, name: &str) -> Option<&InputAction> {
        if let Some(context) = self.get_active_context() {
            if context.enabled {
                return context.get_action(name);
            }
        }
        None
    }

    #[inline]
    pub fn is_action_pressed(&self, name: &str) -> bool {
        self.get_action(name).map_or(false, |a| a.is_active())
    }

    #[inline]
    pub fn is_action_just_pressed(&self, name: &str) -> bool {
        self.get_action(name).map_or(false, |a| a.state.just_pressed())
    }

    #[inline]
    pub fn get_action_value(&self, name: &str) -> Float {
        self.get_action(name).map_or(0.0, |a| a.get_value())
    }

    #[inline]
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keyboard_state
            .get(&key)
            .map_or(false, |s| s.is_pressed())
    }

    #[inline]
    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        self.keyboard_state
            .get(&key)
            .map_or(false, |s| s.just_pressed())
    }

    #[inline]
    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_state
            .get(&button)
            .map_or(false, |s| s.is_pressed())
    }

    #[inline]
    pub fn get_mouse_position(&self) -> (Float, Float) {
        self.mouse_position
    }

    #[inline]
    pub fn get_mouse_delta(&self) -> (Float, Float) {
        self.mouse_delta
    }

    #[inline]
    pub fn get_scroll_delta(&self) -> (Float, Float) {
        self.scroll_delta
    }

    #[inline]
    pub fn get_gamepad(&self, index: usize) -> Option<&GamepadState> {
        self.gamepads.get(index)
    }

    #[inline]
    pub fn get_touches(&self) -> &[TouchPoint] {
        &self.touches
    }

    #[inline]
    pub fn update(&mut self, _dt: Float) {
        // Reset "just" states
        for (_, state) in self.keyboard_state.iter_mut() {
            match state {
                InputState::JustPressed => *state = InputState::Pressed,
                InputState::JustReleased => *state = InputState::Released,
                _ => {}
            }
        }

        for (_, state) in self.mouse_state.iter_mut() {
            match state {
                InputState::JustPressed => *state = InputState::Pressed,
                InputState::JustReleased => *state = InputState::Released,
                _ => {}
            }
        }

        // Reset delta values
        self.mouse_delta = (0.0, 0.0);
        self.scroll_delta = (0.0, 0.0);

        // Update actions
        self.update_actions();
    }

    fn update_actions(&mut self) {
        if let Some(context) = self.get_active_context_mut() {
            if !context.enabled {
                return;
            }

            for action in context.actions.values_mut() {
                let mut new_value = 0.0;
                let mut new_state = InputState::Released;

                for binding in &action.bindings {
                    match binding {
                        InputBinding::Key(key) => {
                            if let Some(&state) = self.keyboard_state.get(key) {
                                if state.is_pressed() {
                                    new_state = InputState::Pressed;
                                    new_value = 1.0;
                                }
                            }
                        }
                        InputBinding::Mouse(button) => {
                            if let Some(&state) = self.mouse_state.get(button) {
                                if state.is_pressed() {
                                    new_state = InputState::Pressed;
                                    new_value = 1.0;
                                }
                            }
                        }
                        InputBinding::GamepadButton(button) => {
                            for gamepad in &self.gamepads {
                                if let Some(state) = gamepad.get_button_state(*button) {
                                    if state.is_pressed() {
                                        new_state = InputState::Pressed;
                                        new_value = 1.0;
                                    }
                                }
                            }
                        }
                        InputBinding::GamepadAxis(axis) => {
                            for gamepad in &self.gamepads {
                                let axis_value = gamepad.get_axis_value(*axis);
                                let abs_value = axis_value.abs();

                                if abs_value > action.deadzone {
                                    new_state = InputState::Pressed;
                                    // Normalize and apply sensitivity
                                    let normalized = (abs_value - action.deadzone)
                                        / (1.0 - action.deadzone);
                                    new_value = normalized * axis_value.signum() * action.sensitivity;
                                }
                            }
                        }
                        _ => {}
                    }
                }

                action.state = new_state;
                action.value = new_value;
            }
        }
    }
}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Gamepad state
#[derive(Clone, Debug)]
pub struct GamepadState {
    pub id: usize,
    pub name: alloc::string::String,
    pub buttons: BTreeMap<GamepadButton, InputState>,
    pub axes: BTreeMap<GamepadAxis, Float>,
    pub connected: bool,
}

impl GamepadState {
    #[inline]
    pub fn new(id: usize) -> Self {
        Self {
            id,
            name: alloc::string::String::from("Unknown"),
            buttons: BTreeMap::new(),
            axes: BTreeMap::new(),
            connected: true,
        }
    }

    #[inline]
    pub fn get_button_state(&self, button: GamepadButton) -> Option<InputState> {
        self.buttons.get(&button).copied()
    }

    #[inline]
    pub fn get_axis_value(&self, axis: GamepadAxis) -> Float {
        self.axes.get(&axis).copied().unwrap_or(0.0)
    }

    #[inline]
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Virtual button for touch controls
#[derive(Clone, Debug)]
pub struct VirtualButton {
    pub x: Float,
    pub y: Float,
    pub width: Float,
    pub height: Float,
    pub action: alloc::string::String,
}

impl VirtualButton {
    #[inline]
    pub fn new(x: Float, y: Float, width: Float, height: Float, action: alloc::string::String) -> Self {
        Self {
            x,
            y,
            width,
            height,
            action,
        }
    }

    #[inline]
    pub fn contains_point(&self, px: Float, py: Float) -> bool {
        px >= self.x
            && px <= self.x + self.width
            && py >= self.y
            && py <= self.y + self.height
    }
}

/// Virtual joystick for touch controls
#[derive(Clone, Debug)]
pub struct VirtualJoystick {
    pub center_x: Float,
    pub center_y: Float,
    pub radius: Float,
    pub action_x: alloc::string::String,
    pub action_y: alloc::string::String,
    pub current_x: Float,
    pub current_y: Float,
    pub active: bool,
}

impl VirtualJoystick {
    #[inline]
    pub fn new(
        center_x: Float,
        center_y: Float,
        radius: Float,
        action_x: alloc::string::String,
        action_y: alloc::string::String,
    ) -> Self {
        Self {
            center_x,
            center_y,
            radius,
            action_x,
            action_y,
            current_x: 0.0,
            current_y: 0.0,
            active: false,
        }
    }

    #[inline]
    pub fn update(&mut self, touch_x: Float, touch_y: Float) {
        let dx = touch_x - self.center_x;
        let dy = touch_y - self.center_y;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance > self.radius {
            let angle = dy.atan2(dx);
            self.current_x = angle.cos() * self.radius;
            self.current_y = angle.sin() * self.radius;
        } else {
            self.current_x = dx;
            self.current_y = dy;
        }

        self.active = true;
    }

    #[inline]
    pub fn reset(&mut self) {
        self.current_x = 0.0;
        self.current_y = 0.0;
        self.active = false;
    }

    #[inline]
    pub fn get_normalized_values(&self) -> (Float, Float) {
        if self.radius > 0.0 {
            (
                self.current_x / self.radius,
                self.current_y / self.radius,
            )
        } else {
            (0.0, 0.0)
        }
    }
}

// Re-export commonly used types
pub use super::physics::Vec3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_state() {
        let state = InputState::JustPressed;
        assert!(state.is_pressed());
        assert!(state.just_pressed());
    }

    #[test]
    fn test_virtual_button() {
        let button = VirtualButton::new(10.0, 10.0, 50.0, 50.0, alloc::string::String::from("test"));
        assert!(button.contains_point(20.0, 20.0));
        assert!(!button.contains_point(70.0, 70.0));
    }

    #[test]
    fn test_virtual_joystick() {
        let mut joystick = VirtualJoystick::new(
            100.0,
            100.0,
            50.0,
            alloc::string::String::from("x"),
            alloc::string::String::from("y"),
        );

        joystick.update(120.0, 100.0);
        let (nx, ny) = joystick.get_normalized_values();
        assert!(nx > 0.0);
        assert_eq!(ny, 0.0);
    }
}
