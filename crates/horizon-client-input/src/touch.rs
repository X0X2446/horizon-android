//! The touch controller's interface to the game.
//!
//! The port (`horizon-android`) owns the touch controller's logic; this module is the
//! contract between it and the game: a [`TouchSource`] the game polls each frame for
//! the actions the controller decided, and a [`GyroSource`] for the device's
//! orientation. The game turns the actions into its own input (synthetic window
//! events for keys and buttons, accumulated mouse motion for look).
//!
//! A desktop build installs no source and every call is a no-op.

use std::sync::{Arc, OnceLock, RwLock};

/// A pointer's phase, as the controller needs it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PointerPhase {
    Down,
    Move,
    Up,
}

/// The movement key a touch stick drives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TouchMove {
    Forward,
    Back,
    Left,
    Right,
}

/// The mouse button a touch gesture presses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TouchMouse {
    Left,
    Right,
}

/// One action the controller decided since the last poll.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TouchAction {
    /// A look delta in screen pixels (x right, y down), for the accumulated mouse
    /// motion the frame's `apply_mouse_look` turns into a turn.
    Look { dx: f32, dy: f32 },
    /// A movement key held or released.
    Move { key: TouchMove, down: bool },
    /// A mouse button pressed or released.
    Mouse { button: TouchMouse, down: bool },
    /// A hotbar slot selected, 0..9.
    Hotbar(u8),
    /// A scroll wheel step (hotbar cycling).
    Scroll { delta: f32 },
    /// A pinch zoom step (positive = zoom in).
    Zoom { delta: f32 },
    /// Jump (a double tap).
    Jump,
    /// Sprint (a double tap).
    Sprint,
    /// Pick the pointed block (a hotbar double tap, creative).
    PickBlock,
}

/// A gyroscope sample: a rotation vector in the device frame and a timestamp.
#[derive(Clone, Copy, Debug)]
pub struct GyroSample {
    /// Rotation around the device's x axis (pitch), degrees per second-ish units.
    pub x: f32,
    /// Rotation around the device's y axis (yaw).
    pub y: f32,
    /// Rotation around the device's z axis (roll); unused for look.
    pub z: f32,
    /// The sample's timestamp, nanoseconds.
    pub t_ns: i64,
}

/// The touch controller, as the game sees it.
pub trait TouchSource: Send + Sync {
    /// One pointer's phase and screen position.
    fn on_pointer(&self, x: f32, y: f32, pointer_id: i32, phase: PointerPhase);
    /// The gestures that matured this frame (long presses).
    fn tick(&self) -> Vec<TouchAction>;
    /// The actions decided since the last poll.
    fn take_actions(&self) -> Vec<TouchAction>;
}

/// The gyroscope, as the game sees it.
pub trait GyroSource: Send + Sync {
    /// The latest sample, if any.
    fn sample(&self) -> Option<GyroSample>;
}

static TOUCH_SOURCE: OnceLock<Arc<dyn TouchSource>> = OnceLock::new();
static GYRO_SOURCE: OnceLock<Arc<dyn GyroSource>> = OnceLock::new();

/// Install the touch source. Only the Android port does; desktop builds have none.
pub fn set_touch_source(source: Arc<dyn TouchSource>) {
    let _ = TOUCH_SOURCE.set(source);
}

/// The installed touch source.
pub fn touch_source() -> Option<Arc<dyn TouchSource>> {
    TOUCH_SOURCE.get().cloned()
}

/// Install the gyroscope source.
pub fn set_gyro_source(source: Arc<dyn GyroSource>) {
    let _ = GYRO_SOURCE.set(source);
}

/// The installed gyroscope source.
pub fn gyro_source() -> Option<Arc<dyn GyroSource>> {
    GYRO_SOURCE.get().cloned()
}

/// The current screen size in pixels (the controller's zones need it).
static SCREEN: RwLock<(u32, u32)> = RwLock::new((1280, 720));

/// Set the screen size the game renders at.
pub fn set_screen_size(width: u32, height: u32) {
    if width > 0 && height > 0 {
        *SCREEN.write().unwrap_or_else(|e| e.into_inner()) = (width, height);
    }
}

/// The screen size the game renders at.
pub fn screen_size() -> (u32, u32) {
    *SCREEN.read().unwrap_or_else(|e| e.into_inner())
}

/// A no-op source, for desktop builds and tests.
#[derive(Default)]
pub struct NullTouchSource;

impl TouchSource for NullTouchSource {
    fn on_pointer(&self, _x: f32, _y: f32, _pointer_id: i32, _phase: PointerPhase) {}
    fn tick(&self) -> Vec<TouchAction> {
        Vec::new()
    }
    fn take_actions(&self) -> Vec<TouchAction> {
        Vec::new()
    }
}

impl GyroSource for NullTouchSource {
    fn sample(&self) -> Option<GyroSample> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_null_source_is_quiet() {
        let source = NullTouchSource;
        assert!(source.tick().is_empty());
        assert!(source.take_actions().is_empty());
        assert!(source.sample().is_none());
        source.on_pointer(1.0, 2.0, 3, PointerPhase::Down);
    }
}
