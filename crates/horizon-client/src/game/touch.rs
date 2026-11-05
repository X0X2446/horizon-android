//! The Android port's touch controller (`horizon-android`): the actions its
//! controller decides become the game's own input here. The controller's logic
//! lives in the port crate; this module only translates its actions into window
//! events and mouse motion, through the same input path a keyboard or a mouse
//! takes.
//!
//! A desktop build installs no [`TouchSource`] and every method returns without
//! doing anything, so the game's behaviour is unchanged there.

use super::event_loop::EventLoopCtl;
use super::input_trace::InputOrigin;
use super::{Game, Screen};
use horizon_client_input::touch::{
    touch_source, gyro_source, GyroSample, PointerPhase, TouchAction, TouchMove, TouchMouse,
};
use winit::{
    DeviceId, ElementState, MouseButton, MouseScrollDelta, PhysicalKey, WindowEvent, WindowId,
    event::KeyEvent,
    keyboard::KeyCode,
};

/// The keyboard keys the movement stick drives (vanilla's WASD).
const MOVE_KEYS: [(TouchMove, KeyCode); 4] = [
    (TouchMove::Forward, KeyCode::KeyW),
    (TouchMove::Back, KeyCode::KeyS),
    (TouchMove::Left, KeyCode::KeyA),
    (TouchMove::Right, KeyCode::KeyD),
];

/// The gyroscope's rotation-vector units to screen-pixel look degrees per step.
const GYRO_SENSITIVITY: f32 = 3.0;

/// The touch source's actions: gestures matured this frame, then the ones decided
/// since the last poll.
pub(super) fn poll_touch(&mut self, event_loop: &dyn EventLoopCtl) {
    let Some(source) = touch_source() else {
        return;
    };
    for action in source.tick() {
        self.apply_touch_action(event_loop, action);
    }
    for action in source.take_actions() {
        self.apply_touch_action(event_loop, action);
    }
}

/// The gyroscope's latest sample: a rotation vector becomes look motion.
pub(super) fn poll_gyro(&mut self) {
    let Some(source) = gyro_source() else {
        return;
    };
    let Some(GyroSample { x, y, .. }) = source.sample() else {
        return;
    };
    // The device's y axis is yaw, x is pitch. Look is positive x to the right,
    // positive y down; the gyro reports the opposite signs.
    if self.screen == Screen::Playing && self.player.is_some() {
        self.look_turn(f64::from(-y * GYRO_SENSITIVITY), f64::from(x * GYRO_SENSITIVITY));
    }
}

/// One touch action, as the game's own input.
fn apply_touch_action(&mut self, event_loop: &dyn EventLoopCtl, action: TouchAction) {
    match action {
        TouchAction::Look { dx, dy } => {
            if self.screen == Screen::Playing && self.player.is_some() {
                self.look_turn(f64::from(dx), f64::from(dy));
            }
        }
        TouchAction::Move { key, down } => {
            let Some((_, code)) = MOVE_KEYS.iter().find(|(k, _)| *k == key) else {
                return;
            };
            self.synth_key(event_loop, *code, down);
        }
        TouchAction::Mouse { button, down } => {
            let button = match button {
                TouchMouse::Left => MouseButton::Left,
                TouchMouse::Right => MouseButton::Right,
            };
            self.synth_mouse(event_loop, button, down);
        }
        TouchAction::Hotbar(slot) => {
            // Vanilla's hotbar keys: Digit1 to Digit9.
            if let Some(code) = hotbar_key(slot) {
                self.synth_key(event_loop, code, true);
                self.synth_key(event_loop, code, false);
            }
        }
        TouchAction::Scroll { delta } => {
            self.synth_wheel(event_loop, delta);
        }
        TouchAction::Zoom { delta } => {
            // A pinch zoom scrolls the hotbar: positive (zoom in) scrolls down.
            self.synth_wheel(event_loop, -delta);
        }
        TouchAction::Jump => {
            self.synth_key(event_loop, KeyCode::Space, true);
            self.synth_key(event_loop, KeyCode::Space, false);
        }
        TouchAction::Sprint => {
            self.synth_key(event_loop, KeyCode::ControlLeft, true);
            self.synth_key(event_loop, KeyCode::ControlLeft, false);
        }
        TouchAction::PickBlock => {
            // Vanilla: the middle mouse button picks the pointed block.
            self.synth_mouse(event_loop, MouseButton::Middle, true);
            self.synth_mouse(event_loop, MouseButton::Middle, false);
        }
    }
}

/// Vanilla's hotbar key for a slot (0..9 → Digit1..Digit9).
fn hotbar_key(slot: u8) -> Option<KeyCode> {
    match slot {
        0 => Some(KeyCode::Digit1),
        1 => Some(KeyCode::Digit2),
        2 => Some(KeyCode::Digit3),
        3 => Some(KeyCode::Digit4),
        4 => Some(KeyCode::Digit5),
        5 => Some(KeyCode::Digit6),
        6 => Some(KeyCode::Digit7),
        7 => Some(KeyCode::Digit8),
        8 => Some(KeyCode::Digit9),
        _ => None,
    }
}

/// A key press or release, as the window would send it.
fn synth_key(&mut self, event_loop: &dyn EventLoopCtl, code: KeyCode, down: bool) {
    // `KeyEvent::new` takes a character for the text the key types; the keys the
    // touch controller drives type nothing (they are not character input).
    let mut event = KeyEvent::new(PhysicalKey::Code(code), '\0');
    event.text = None;
    event.state = if down {
        ElementState::Pressed
    } else {
        ElementState::Released
    };
    let window_event = WindowEvent::KeyboardInput {
        event,
        device_id: DeviceId::dummy(),
        is_synthetic: false,
        platform_modifiers: Default::default(),
    };
    self.handle_window_event(event_loop, self.window_id(), window_event, InputOrigin::User);
}

/// A mouse button press or release, as the window would send it.
fn synth_mouse(&mut self, event_loop: &dyn EventLoopCtl, button: MouseButton, down: bool) {
    let window_event = WindowEvent::MouseInput {
        state: if down {
            ElementState::Pressed
        } else {
            ElementState::Released
        },
        button,
        device_id: DeviceId::dummy(),
        modifiers: Default::default(),
    };
    self.handle_window_event(event_loop, self.window_id(), window_event, InputOrigin::User);
}

/// A scroll wheel step (a touch gesture's zoom or hotbar cycling).
fn synth_wheel(&mut self, event_loop: &dyn EventLoopCtl, delta: f32) {
    let event = WindowEvent::MouseWheel {
        delta: MouseScrollDelta::LineDelta(0.0, delta),
    };
    self.handle_window_event(event_loop, self.window_id(), event, InputOrigin::User);
}

/// The game's window (the events' target), or the dummy id headless runs use.
fn window_id(&self) -> WindowId {
    self.window.as_ref().map(|w| w.id()).unwrap_or_else(WindowId::dummy)
}

/// Pointer events go to the touch controller, which decides what they mean.
/// They are the only pointer events the game itself ignores, so the rest of the
/// dispatch continues as it would.
pub(super) fn forward_pointer(&self, x: f32, y: f32, pointer_id: i32, phase: PointerPhase) {
    if let Some(source) = touch_source() {
        source.on_pointer(x, y, pointer_id, phase);
    }
}

#[cfg(test)]
mod tests {
    use super::hotbar_key;
    use horizon_client_input::touch::TouchMove;
    use winit::keyboard::KeyCode;

    #[test]
    fn the_hotbar_keys_are_vanilla_s_digit_keys() {
        assert_eq!(hotbar_key(0), Some(KeyCode::Digit1));
        assert_eq!(hotbar_key(8), Some(KeyCode::Digit9));
        assert_eq!(hotbar_key(9), None);
    }

    #[test]
    fn the_movement_sticks_map_to_wasd() {
        let keys = [
            (TouchMove::Forward, KeyCode::KeyW),
            (TouchMove::Back, KeyCode::KeyS),
            (TouchMove::Left, KeyCode::KeyA),
            (TouchMove::Right, KeyCode::KeyD),
        ];
        for (move_key, code) in keys {
            assert!(MOVE_KEYS.contains(&(move_key, code)));
        }
    }
}
