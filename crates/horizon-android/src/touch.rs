//! The touch controller: screen touches to the game's input.
//!
//! A pure state machine (no I/O, no threads — the caller passes millisecond
//! timestamps), so it is unit-tested on the host and reused verbatim on the device.
//! It implements the mobile control scheme of the ZalithLauncher family:
//!
//! - the left half of the screen is the movement stick (W/A/S/D by drag direction),
//! - the right half is the look stick (drag = camera delta, scaled by the look
//!   sensitivity; in `Absolute` mode a tap aims at the touched point),
//! - taps break (left half) and place (right half), double taps jump and sprint,
//! - a long press (gesture control) holds the configured mouse button,
//! - the bottom hotbar strip selects slots, with double-tap and long-press gestures,
//! - a two-finger pinch zooms.
//!
//! The output is a batch of [`TouchAction`]s the game consumes once per frame
//! (`horizon-client`'s `poll_touch`).

use crate::config::{CompatibilitySettings, GestureAction, MouseButton, MoveKey, TouchAction};
use std::collections::{HashMap, HashSet};

/// A raw touch as the platform delivers it.
#[derive(Clone, Copy, Debug)]
pub struct TouchPoint {
    /// The touch's id (the platform's pointer id).
    pub id: i32,
    pub x: f32,
    pub y: f32,
    /// `Down`, `Move` or `Up`.
    pub phase: TouchPhase,
    /// 0..1; only used for multi-touch ordering.
    pub pressure: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TouchPhase {
    Down,
    Move,
    Up,
}

/// Which half of the screen a touch started in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Zone {
    Left,
    Right,
}

/// One finger's state while it is down.
#[derive(Debug)]
struct Finger {
    zone: Zone,
    anchor_x: f32,
    anchor_y: f32,
    last_x: f32,
    last_y: f32,
    /// Pixels the finger has travelled from its anchor.
    travelled: f32,
    start_ms: i64,
    /// The movement keys this finger currently holds.
    move_keys: HashSet<MoveKey>,
    /// The mouse button a long press is holding (gesture control).
    held: Option<MouseButton>,
}

/// The touch controller.
pub struct TouchController {
    settings: CompatibilitySettings,
    screen_w: u32,
    screen_h: u32,
    fingers: HashMap<i32, Finger>,
    /// Distance between the two fingers of an active pinch, for zoom.
    pinch_distance: Option<f32>,
    /// The last tap's time and zone, for double taps.
    last_tap_ms: i64,
    last_tap_zone: Zone,
    /// The last hotbar tap, for hotbar double taps.
    last_hotbar_ms: i64,
    last_hotbar_slot: Option<u8>,
}

impl TouchController {
    pub fn new(settings: CompatibilitySettings) -> Self {
        Self {
            settings,
            screen_w: 1920,
            screen_h: 1080,
            fingers: HashMap::new(),
            pinch_distance: None,
            last_tap_ms: i64::MIN / 2,
            last_tap_zone: Zone::Left,
            last_hotbar_ms: i64::MIN / 2,
            last_hotbar_slot: None,
        }
    }

    /// The screen's pixel size (for zone and hotbar math).
    pub fn set_screen_size(&mut self, width: u32, height: u32) {
        self.screen_w = width;
        self.screen_h = height;
    }

    /// Replace the settings mid-game (the settings screen saves live).
    pub fn set_settings(&mut self, settings: CompatibilitySettings) {
        self.settings = settings;
    }

    /// The settings in force.
    pub fn settings(&self) -> &CompatibilitySettings {
        &self.settings
    }

    // --- geometry ---

    /// The hotbar strip's rectangle in screen pixels, empty when the hotbar is 0-sized.
    fn hotbar_rect(&self) -> Option<(f32, f32, f32, f32)> {
        if self.settings.hotbar_height == 0 || self.settings.hotbar_width == 0 {
            return None;
        }
        let w = (self.screen_w as f32) * self.settings.hotbar_width as f32 / 1000.0;
        let h = (self.screen_h as f32) * self.settings.hotbar_height as f32 / 1000.0;
        let x = (self.screen_w as f32 - w) / 2.0;
        let y = (self.screen_h as f32) - h;
        Some((x, y, w, h))
    }

    fn in_hotbar(&self, x: f32, y: f32) -> bool {
        match self.hotbar_rect() {
            Some((x0, y0, w, h)) => x >= x0 && x < x0 + w && y >= y0 && y < y0 + h,
            None => false,
        }
    }

    fn zone_of(&self, x: f32) -> Zone {
        if x < self.screen_w as f32 / 2.0 {
            Zone::Left
        } else {
            Zone::Right
        }
    }

    // --- the event loop ---

    /// Feed one touch. `now_ms` is milliseconds on any monotonic clock.
    pub fn feed(&mut self, point: TouchPoint, now_ms: i64) -> Vec<TouchAction> {
        match point.phase {
            TouchPhase::Down => self.on_down(point, now_ms),
            TouchPhase::Move => self.on_move(point, now_ms),
            TouchPhase::Up => self.on_up(point, now_ms),
        }
    }

    /// Release every finger (activity paused, surface lost).
    pub fn finish(&mut self) -> Vec<TouchAction> {
        let mut out = Vec::new();
        for (_, finger) in self.fingers.drain() {
            if let Some(button) = finger.held {
                out.push(TouchAction::Mouse {
                    button,
                    down: false,
                });
            }
        }
        out
    }

    fn on_down(&mut self, point: TouchPoint, now_ms: i64) -> Vec<TouchAction> {
        if !self.settings.touch_controls_enabled {
            return Vec::new();
        }
        // A hotbar tap never becomes a stick.
        if self.in_hotbar(point.x, point.y) {
            return self.handle_hotbar_down(point, now_ms);
        }
        self.fingers.insert(
            point.id,
            Finger {
                zone: self.zone_of(point.x),
                anchor_x: point.x,
                anchor_y: point.y,
                last_x: point.x,
                last_y: point.y,
                travelled: 0.0,
                start_ms: now_ms,
                move_keys: HashSet::new(),
                held: None,
            },
        );
        self.update_pinch()
    }

    fn on_move(&mut self, point: TouchPoint, _now_ms: i64) -> Vec<TouchAction> {
        let Some(finger) = self.fingers.get_mut(&point.id) else {
            return Vec::new();
        };
        finger.last_x = point.x;
        finger.last_y = point.y;
        let dx = point.x - finger.anchor_x;
        let dy = point.y - finger.anchor_y;
        finger.travelled = finger.travelled.max((dx * dx + dy * dy).sqrt());

        let mut out = Vec::new();
        // A long press that became a drag stops holding its button.
        if finger.held.is_some() && finger.travelled > 12.0 {
            let button = finger.held.take().unwrap();
            out.push(TouchAction::Mouse {
                button,
                down: false,
            });
        }

        if self.settings.touch_controls_enabled {
            let settings = &self.settings;
            match finger.zone {
                Zone::Right => look_drag(settings, dx, dy, &mut out),
                Zone::Left => move_drag(settings, finger, dx, dy, &mut out),
            }
        }
        self.update_pinch_from(&mut out);
        out
    }

    fn on_up(&mut self, point: TouchPoint, now_ms: i64) -> Vec<TouchAction> {
        let Some(finger) = self.fingers.remove(&point.id) else {
            return Vec::new();
        };
        self.pinch_distance = None;

        let mut out = Vec::new();
        if let Some(button) = finger.held {
            out.push(TouchAction::Mouse {
                button,
                down: false,
            });
        } else {
            let quick = now_ms - finger.start_ms < 200;
            let still = finger.travelled < 12.0;
            if still && quick {
                // A tap: break on the left half, place on the right.
                let button = match finger.zone {
                    Zone::Left => MouseButton::Left,
                    Zone::Right => MouseButton::Right,
                };
                out.push(TouchAction::Mouse {
                    button,
                    down: true,
                });
                out.push(TouchAction::Mouse {
                    button,
                    down: false,
                });
                // Double tap: jump (left) or sprint (right).
                if finger.zone == self.last_tap_zone && now_ms - self.last_tap_ms <= 300 {
                    if finger.zone == Zone::Left {
                        out.push(TouchAction::Jump);
                    } else {
                        out.push(TouchAction::Sprint);
                    }
                    self.last_tap_ms = i64::MIN / 2; // consumed
                } else {
                    self.last_tap_ms = now_ms;
                    self.last_tap_zone = finger.zone;
                }
            }
        }
        out
    }

    // --- gestures ---

    /// Called from the game's frame loop: long presses mature here, so the
    /// controller never needs its own timer thread.
    pub fn tick(&mut self, now_ms: i64) -> Vec<TouchAction> {
        let mut out = Vec::new();
        if self.settings.gesture_control {
            let delay = self.settings.gesture_long_press_delay;
            for finger in self.fingers.values_mut() {
                if finger.held.is_none()
                    && now_ms - finger.start_ms >= delay
                    && finger.travelled < 12.0
                {
                    let button = match finger.zone {
                        Zone::Left => match self.settings.gesture_long_press_action {
                            GestureAction::MouseLeft => MouseButton::Left,
                            GestureAction::MouseRight => MouseButton::Right,
                            GestureAction::None => continue,
                        },
                        Zone::Right => match self.settings.gesture_tap_action {
                            GestureAction::MouseLeft => MouseButton::Left,
                            GestureAction::MouseRight => MouseButton::Right,
                            GestureAction::None => continue,
                        },
                    };
                    finger.held = Some(button);
                    out.push(TouchAction::Mouse {
                        button,
                        down: true,
                    });
                }
            }
        }
        out
    }

    // --- hotbar ---

    fn handle_hotbar_down(&mut self, point: TouchPoint, now_ms: i64) -> Vec<TouchAction> {
        let Some((x0, _, w, _)) = self.hotbar_rect() else {
            return Vec::new();
        };
        let ratio = (point.x - x0) / w;
        let slot = (ratio * 9.0).round().clamp(0.0, 8.0) as u8;
        let mut out = vec![TouchAction::Hotbar(slot)];
        // Double tap on the same slot: pick block (creative) when enabled.
        if self.settings.hotbar_double_click
            && now_ms - self.last_hotbar_ms <= 300
            && self.last_hotbar_slot == Some(slot)
        {
            out.push(TouchAction::PickBlock);
            self.last_hotbar_ms = i64::MIN / 2;
        } else {
            self.last_hotbar_ms = now_ms;
            self.last_hotbar_slot = Some(slot);
        }
        out
    }

    // --- pinch ---

    fn update_pinch(&mut self) -> Vec<TouchAction> {
        let mut out = Vec::new();
        self.update_pinch_from(&mut out);
        out
    }

    fn update_pinch_from(&mut self, out: &mut Vec<TouchAction>) {
        let right_fingers: Vec<(f32, f32)> = self
            .fingers
            .values()
            .filter(|f| f.zone == Zone::Right)
            .map(|f| (f.last_x, f.last_y))
            .collect();
        if right_fingers.len() >= 2 {
            let (a, b) = (right_fingers[0], right_fingers[1]);
            let dist = ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
            if let Some(prev) = self.pinch_distance {
                let delta = dist - prev;
                if delta.abs() > 1.0 {
                    // Pinch out zooms in.
                    out.push(TouchAction::Zoom {
                        delta: delta * 0.05,
                    });
                }
            }
            self.pinch_distance = Some(dist);
        } else {
            self.pinch_distance = None;
        }
    }
}

// --- sticks (free functions: the caller passes the settings, so a finger borrowed
// from the controller's map doesn't conflict with a borrow of the controller) ---

fn look_drag(settings: &CompatibilitySettings, dx: f32, dy: f32, out: &mut Vec<TouchAction>) {
    let scale = settings.touch_look_sensitivity as f32 / 100.0;
    out.push(TouchAction::Look {
        dx: dx * scale,
        dy: dy * scale,
    });
}

fn move_drag(
    settings: &CompatibilitySettings,
    finger: &mut Finger,
    dx: f32,
    dy: f32,
    out: &mut Vec<TouchAction>,
) {
    // Normalize by a stick range, apply a deadzone that shrinks with sensitivity.
    const RANGE: f32 = 48.0;
    let deadzone = 10.0 / (settings.touch_move_sensitivity as f32 / 100.0);
    let nx = (dx / RANGE).clamp(-1.0, 1.0);
    let ny = (dy / RANGE).clamp(-1.0, 1.0);
    let mag = (nx * nx + ny * ny).sqrt();
    if mag <= deadzone / RANGE {
        return;
    }
    const THRESHOLD: f32 = 0.35;
    let forward = ny < -THRESHOLD;
    let back = ny > THRESHOLD;
    let left = nx < -THRESHOLD;
    let right = nx > THRESHOLD;
    for (key, down) in [
        (MoveKey::Forward, forward),
        (MoveKey::Back, back),
        (MoveKey::Left, left),
        (MoveKey::Right, right),
    ] {
        let was = finger.move_keys.contains(&key);
        if down && !was {
            finger.move_keys.insert(key);
            out.push(TouchAction::Move { key, down: true });
        } else if !down && was {
            finger.move_keys.remove(&key);
            out.push(TouchAction::Move { key, down: false });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controller() -> TouchController {
        let mut c = TouchController::new(CompatibilitySettings::default());
        c.set_screen_size(1920, 1080);
        c
    }

    fn down(id: i32, x: f32, y: f32) -> TouchPoint {
        TouchPoint {
            id,
            x,
            y,
            phase: TouchPhase::Down,
            pressure: 1.0,
        }
    }

    fn up(id: i32, x: f32, y: f32) -> TouchPoint {
        TouchPoint {
            id,
            x,
            y,
            phase: TouchPhase::Up,
            pressure: 1.0,
        }
    }

    fn move_(id: i32, x: f32, y: f32) -> TouchPoint {
        TouchPoint {
            id,
            x,
            y,
            phase: TouchPhase::Move,
            pressure: 1.0,
        }
    }

    #[test]
    fn a_tap_on_the_left_breaks_and_on_the_right_places() {
        let mut c = controller();
        assert!(c.feed(down(0, 300.0, 500.0), 0).is_empty());
        assert_eq!(
            c.feed(up(0, 300.0, 500.0), 50),
            vec![
                TouchAction::Mouse {
                    button: MouseButton::Left,
                    down: true
                },
                TouchAction::Mouse {
                    button: MouseButton::Left,
                    down: false
                }
            ]
        );
        assert!(c.feed(down(1, 1600.0, 500.0), 100).is_empty());
        assert_eq!(
            c.feed(up(1, 1600.0, 500.0), 150),
            vec![
                TouchAction::Mouse {
                    button: MouseButton::Right,
                    down: true
                },
                TouchAction::Mouse {
                    button: MouseButton::Right,
                    down: false
                }
            ]
        );
    }

    #[test]
    fn a_drag_on_the_right_looks() {
        let mut c = controller();
        c.feed(down(0, 1600.0, 500.0), 0);
        assert_eq!(
            c.feed(move_(0, 1500.0, 560.0), 10),
            vec![TouchAction::Look {
                dx: -100.0,
                dy: 60.0
            }]
        );
        // Sensitivity scales the delta.
        c.settings.touch_look_sensitivity = 200;
        c.feed(down(1, 1600.0, 500.0), 20);
        assert_eq!(
            c.feed(move_(1, 1400.0, 500.0), 30),
            vec![TouchAction::Look { dx: -400.0, dy: 0.0 }]
        );
    }

    #[test]
    fn a_drag_on_the_left_moves() {
        let mut c = controller();
        c.feed(down(0, 400.0, 500.0), 0);
        assert_eq!(
            c.feed(move_(0, 400.0, 400.0), 10), // up = forward
            vec![TouchAction::Move {
                key: MoveKey::Forward,
                down: true
            }]
        );
        assert_eq!(
            c.feed(move_(0, 300.0, 400.0), 20), // up-left
            vec![TouchAction::Move {
                key: MoveKey::Left,
                down: true
            }]
        );
        assert_eq!(
            c.feed(move_(0, 400.0, 600.0), 30), // down = back: forward released
            vec![
                TouchAction::Move {
                    key: MoveKey::Forward,
                    down: false
                },
                TouchAction::Move {
                    key: MoveKey::Back,
                    down: true
                },
                TouchAction::Move {
                    key: MoveKey::Left,
                    down: false
                }
            ]
        );
    }

    #[test]
    fn double_tap_jumps_and_sprints() {
        let mut c = controller();
        c.feed(down(0, 300.0, 500.0), 0);
        c.feed(up(0, 300.0, 500.0), 30);
        c.feed(down(1, 320.0, 500.0), 100);
        assert_eq!(
            c.feed(up(1, 320.0, 500.0), 130),
            vec![
                TouchAction::Mouse {
                    button: MouseButton::Left,
                    down: true
                },
                TouchAction::Mouse {
                    button: MouseButton::Left,
                    down: false
                },
                TouchAction::Jump
            ]
        );
        // Right side double tap sprints.
        c.feed(down(2, 1600.0, 500.0), 200);
        c.feed(up(2, 1600.0, 500.0), 230);
        c.feed(down(3, 1620.0, 500.0), 300);
        assert_eq!(
            c.feed(up(3, 1620.0, 500.0), 330),
            vec![
                TouchAction::Mouse {
                    button: MouseButton::Right,
                    down: true
                },
                TouchAction::Mouse {
                    button: MouseButton::Right,
                    down: false
                },
                TouchAction::Sprint
            ]
        );
    }

    #[test]
    fn long_press_holds_its_button() {
        let mut c = controller();
        c.settings.gesture_control = true;
        c.feed(down(0, 300.0, 500.0), 0);
        assert_eq!(
            c.tick(350),
            vec![TouchAction::Mouse {
                button: MouseButton::Left,
                down: true
            }]
        );
        // A drag stops the hold and also moves forward (the drag is upward).
        assert_eq!(
            c.feed(move_(0, 300.0, 400.0), 360),
            vec![
                TouchAction::Mouse {
                    button: MouseButton::Left,
                    down: false
                },
                TouchAction::Move {
                    key: MoveKey::Forward,
                    down: true
                }
            ]
        );
    }

    #[test]
    fn hotbar_taps_select_slots() {
        let mut c = controller();
        // The hotbar spans x = 480..1440 at 1920 wide; slot 4 is at ~907.
        assert_eq!(c.feed(down(0, 907.0, 1050.0), 0), vec![TouchAction::Hotbar(4)]);
        assert_eq!(c.feed(down(1, 1400.0, 1050.0), 50), vec![TouchAction::Hotbar(8)]);
        // The hotbar strip ignores sticks.
        c.feed(down(2, 960.0, 1000.0), 100);
        assert!(c.fingers.is_empty());
    }

    #[test]
    fn pinch_zooms() {
        let mut c = controller();
        c.feed(down(0, 1500.0, 500.0), 0);
        c.feed(down(1, 1700.0, 500.0), 0);
        let zoomed = c.feed(move_(1, 1700.0, 560.0), 10);
        assert!(zoomed
            .iter()
            .any(|a| matches!(a, TouchAction::Zoom { delta } if *delta > 0.0)));
    }

    #[test]
    fn disabled_touch_controls_ignore_everything() {
        let mut c = controller();
        c.settings.touch_controls_enabled = false;
        assert!(c.feed(down(0, 300.0, 500.0), 0).is_empty());
        assert!(c.feed(move_(0, 300.0, 400.0), 10).is_empty());
        assert!(c.feed(up(0, 300.0, 400.0), 20).is_empty());
    }

    #[test]
    fn finish_releases_held_buttons() {
        let mut c = controller();
        c.settings.gesture_control = true;
        c.feed(down(0, 300.0, 500.0), 0);
        c.tick(400);
        assert_eq!(
            c.finish(),
            vec![TouchAction::Mouse {
                button: MouseButton::Left,
                down: false
            }]
        );
    }
}
