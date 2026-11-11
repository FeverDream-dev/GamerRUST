//! Unified input: keyboard, mouse, and gamepad behind one API.
//!
//! winit events feed [`Input::handle_window_event`]; gilrs gamepads are
//! polled once per frame via [`Input::poll_gamepads`]. Edge semantics live
//! in the pure [`ButtonTracker`], so "just pressed" is unit-testable
//! without a window.

use glam::Vec2;
use std::collections::HashMap;
use winit::event::WindowEvent;

/// A button's edge state on top of its held state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ButtonState {
    pub held: bool,
    pub just_pressed: bool,
    pub just_released: bool,
}

/// Pure edge-detection for one button: feed the raw held state each frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ButtonTracker {
    state: ButtonState,
}

impl ButtonTracker {
    /// Feed this frame's raw held state; updates just_* edges.
    pub fn update(&mut self, held: bool) {
        let was = self.state.held;
        self.state = ButtonState {
            held,
            just_pressed: held && !was,
            just_released: !held && was,
        };
    }

    pub fn state(&self) -> ButtonState {
        self.state
    }

    /// Drop edge flags without changing held.
    pub fn reset_edges(&mut self) {
        self.state.just_pressed = false;
        self.state.just_released = false;
    }
}

/// Keyboard key — engine-local enum so games never import winit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
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
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Space,
    Enter,
    Escape,
    Up,
    Down,
    Left,
    Right,
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    Tab,
    Backspace,
}

/// Mouse buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Gamepad buttons — engine-local enum so games never import gilrs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    South, // Cross / A
    East,  // Circle / B
    North, // Triangle / Y
    West,  // Square / X
    Start,
    Select,
    L1,
    R1,
    L3,
    R3,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

/// Gamepad analog axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
    LeftTrigger,
    RightTrigger,
}

impl From<winit::keyboard::KeyCode> for Key {
    fn from(k: winit::keyboard::KeyCode) -> Self {
        use winit::keyboard::KeyCode as K;
        match k {
            K::KeyA => Key::A,
            K::KeyB => Key::B,
            K::KeyC => Key::C,
            K::KeyD => Key::D,
            K::KeyE => Key::E,
            K::KeyF => Key::F,
            K::KeyG => Key::G,
            K::KeyH => Key::H,
            K::KeyI => Key::I,
            K::KeyJ => Key::J,
            K::KeyK => Key::K,
            K::KeyL => Key::L,
            K::KeyM => Key::M,
            K::KeyN => Key::N,
            K::KeyO => Key::O,
            K::KeyP => Key::P,
            K::KeyQ => Key::Q,
            K::KeyR => Key::R,
            K::KeyS => Key::S,
            K::KeyT => Key::T,
            K::KeyU => Key::U,
            K::KeyV => Key::V,
            K::KeyW => Key::W,
            K::KeyX => Key::X,
            K::KeyY => Key::Y,
            K::KeyZ => Key::Z,
            K::Digit0 => Key::Digit0,
            K::Digit1 => Key::Digit1,
            K::Digit2 => Key::Digit2,
            K::Digit3 => Key::Digit3,
            K::Digit4 => Key::Digit4,
            K::Digit5 => Key::Digit5,
            K::Digit6 => Key::Digit6,
            K::Digit7 => Key::Digit7,
            K::Digit8 => Key::Digit8,
            K::Digit9 => Key::Digit9,
            K::Space => Key::Space,
            K::Enter => Key::Enter,
            K::Escape => Key::Escape,
            K::ArrowUp => Key::Up,
            K::ArrowDown => Key::Down,
            K::ArrowLeft => Key::Left,
            K::ArrowRight => Key::Right,
            K::ShiftLeft => Key::ShiftLeft,
            K::ShiftRight => Key::ShiftRight,
            K::ControlLeft => Key::ControlLeft,
            K::ControlRight => Key::ControlRight,
            K::Tab => Key::Tab,
            K::Backspace => Key::Backspace,
            // ponytail: unmapped physical keys map to Space; grow the enum
            // when a game needs more keys.
            _ => Key::Space,
        }
    }
}

impl From<winit::event::MouseButton> for MouseButton {
    fn from(b: winit::event::MouseButton) -> Self {
        match b {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            _ => MouseButton::Left,
        }
    }
}

impl From<gilrs::Button> for GamepadButton {
    fn from(b: gilrs::Button) -> Self {
        use gilrs::Button as B;
        match b {
            B::South => GamepadButton::South,
            B::East => GamepadButton::East,
            B::North => GamepadButton::North,
            B::West => GamepadButton::West,
            B::Start => GamepadButton::Start,
            B::Select => GamepadButton::Select,
            B::LeftTrigger => GamepadButton::L1,
            B::RightTrigger => GamepadButton::R1,
            B::LeftThumb => GamepadButton::L3,
            B::RightThumb => GamepadButton::R3,
            B::DPadUp => GamepadButton::DPadUp,
            B::DPadDown => GamepadButton::DPadDown,
            B::DPadLeft => GamepadButton::DPadLeft,
            B::DPadRight => GamepadButton::DPadRight,
            _ => GamepadButton::South,
        }
    }
}

impl From<gilrs::Axis> for GamepadAxis {
    fn from(a: gilrs::Axis) -> Self {
        use gilrs::Axis as A;
        match a {
            A::LeftStickX => GamepadAxis::LeftStickX,
            A::LeftStickY => GamepadAxis::LeftStickY,
            A::RightStickX => GamepadAxis::RightStickX,
            A::RightStickY => GamepadAxis::RightStickY,
            A::LeftZ => GamepadAxis::LeftTrigger,
            A::RightZ => GamepadAxis::RightTrigger,
            _ => GamepadAxis::LeftStickX,
        }
    }
}

/// Per-gamepad state.
#[derive(Debug, Default)]
struct GamepadState {
    buttons: HashMap<GamepadButton, ButtonTracker>,
    axes: HashMap<GamepadAxis, f32>,
}

/// Unified keyboard/mouse/gamepad input.
#[derive(Debug)]
pub struct Input {
    keys: HashMap<Key, ButtonTracker>,
    mouse_buttons: HashMap<MouseButton, ButtonTracker>,
    mouse_pos: Vec2,
    mouse_delta: Vec2,
    gamepads: HashMap<gilrs::GamepadId, GamepadState>,
    gilrs: Option<gilrs::Gilrs>,
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

impl Input {
    pub fn new() -> Self {
        // ponytail: gilrs init failure = no gamepad support, keep going.
        let gilrs = gilrs::Gilrs::new().ok();
        Self {
            keys: HashMap::new(),
            mouse_buttons: HashMap::new(),
            mouse_pos: Vec2::ZERO,
            mouse_delta: Vec2::ZERO,
            gamepads: HashMap::new(),
            gilrs,
        }
    }

    /// Feed a winit window event (keyboard + mouse).
    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        use winit::event::ElementState;
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;
                    self.keys.entry(code.into()).or_default().update(pressed);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let new_pos = Vec2::new(position.x as f32, position.y as f32);
                self.mouse_delta += new_pos - self.mouse_pos;
                self.mouse_pos = new_pos;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = *state == ElementState::Pressed;
                self.mouse_buttons
                    .entry((*button).into())
                    .or_default()
                    .update(pressed);
            }
            _ => {}
        }
    }

    /// Poll gilrs for gamepad state; call once per frame.
    pub fn poll_gamepads(&mut self) {
        let Some(gilrs) = self.gilrs.as_mut() else {
            return;
        };
        // Drain connect/disconnect, then snapshot live state.
        while let Some(ev) = gilrs.next_event() {
            match ev.event {
                gilrs::EventType::Connected => {
                    self.gamepads.entry(ev.id).or_default();
                }
                gilrs::EventType::Disconnected => {
                    self.gamepads.remove(&ev.id);
                }
                _ => {}
            }
        }
        for (id, pad) in gilrs.gamepads() {
            let state = self.gamepads.entry(id).or_default();
            for axis in [
                gilrs::Axis::LeftStickX,
                gilrs::Axis::LeftStickY,
                gilrs::Axis::RightStickX,
                gilrs::Axis::RightStickY,
                gilrs::Axis::LeftZ,
                gilrs::Axis::RightZ,
            ] {
                let v = pad.value(axis).clamp(-1.0, 1.0);
                state.axes.insert(axis.into(), v);
            }
            for btn in [
                gilrs::Button::South,
                gilrs::Button::East,
                gilrs::Button::North,
                gilrs::Button::West,
                gilrs::Button::Start,
                gilrs::Button::Select,
                gilrs::Button::LeftTrigger,
                gilrs::Button::RightTrigger,
                gilrs::Button::LeftThumb,
                gilrs::Button::RightThumb,
                gilrs::Button::DPadUp,
                gilrs::Button::DPadDown,
                gilrs::Button::DPadLeft,
                gilrs::Button::DPadRight,
            ] {
                state
                    .buttons
                    .entry(btn.into())
                    .or_default()
                    .update(pad.is_pressed(btn));
            }
        }
    }

    /// Clear per-frame edges. The engine calls this after draw.
    pub fn end_frame(&mut self) {
        for t in self.keys.values_mut() {
            t.reset_edges();
        }
        for t in self.mouse_buttons.values_mut() {
            t.reset_edges();
        }
        self.mouse_delta = Vec2::ZERO;
        for pad in self.gamepads.values_mut() {
            for t in pad.buttons.values_mut() {
                t.reset_edges();
            }
        }
    }

    // ---- keyboard ----

    fn key_state(&self, key: Key) -> ButtonState {
        self.keys
            .get(&key)
            .map_or(ButtonState::default(), |t| t.state())
    }

    pub fn pressed(&self, key: Key) -> bool {
        self.key_state(key).held
    }

    pub fn just_pressed(&self, key: Key) -> bool {
        self.key_state(key).just_pressed
    }

    pub fn just_released(&self, key: Key) -> bool {
        self.key_state(key).just_released
    }

    // ---- mouse ----

    pub fn mouse_pos(&self) -> Vec2 {
        self.mouse_pos
    }

    pub fn mouse_delta(&self) -> Vec2 {
        self.mouse_delta
    }

    pub fn mouse_button(&self, button: MouseButton) -> ButtonState {
        self.mouse_buttons
            .get(&button)
            .map_or(ButtonState::default(), |t| t.state())
    }

    // ---- gamepad ----

    pub fn gamepad_connected(&self, id: gilrs::GamepadId) -> bool {
        self.gamepads.contains_key(&id)
    }

    /// All currently connected gamepad ids.
    pub fn gamepad_ids(&self) -> Vec<gilrs::GamepadId> {
        self.gamepads.keys().copied().collect()
    }

    pub fn gamepad_axis(&self, id: gilrs::GamepadId, axis: GamepadAxis) -> f32 {
        self.gamepads
            .get(&id)
            .and_then(|p| p.axes.get(&axis))
            .copied()
            .unwrap_or(0.0)
    }

    pub fn gamepad_button(&self, id: gilrs::GamepadId, button: GamepadButton) -> ButtonState {
        self.gamepads
            .get(&id)
            .and_then(|p| p.buttons.get(&button))
            .map_or(ButtonState::default(), |t| t.state())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_press_hold_release() {
        let mut t = ButtonTracker::default();
        t.update(true); // press
        assert!(t.state().just_pressed && t.state().held);
        t.update(true); // hold
        assert!(!t.state().just_pressed && t.state().held);
        t.update(false); // release
        assert!(t.state().just_released && !t.state().held);
        t.update(false); // stay released
        assert!(!t.state().just_released && !t.state().held);
    }

    #[test]
    fn tracker_release_repress() {
        let mut t = ButtonTracker::default();
        t.update(true);
        t.update(false);
        t.update(true); // repress
        assert!(t.state().just_pressed && t.state().held);
    }

    #[test]
    fn tracker_reset_edges_clears_but_keeps_held() {
        let mut t = ButtonTracker::default();
        t.update(true);
        t.reset_edges();
        assert!(t.state().held);
        assert!(!t.state().just_pressed);
    }
}
