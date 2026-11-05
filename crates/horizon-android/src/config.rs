//! The compatibility settings the Android port runs with.
//!
//! The fields are the same family ZalithLauncher2 exposes to its players
//! (`AllSettings.kt` in that launcher): which graphics backend to render with, how
//! the resolution is scaled, how much memory the game may use, the touch controls and
//! their sensitivities, the gyroscope, the gamepad, the hotbar layout, version
//! isolation and the integrity check. The values are stored in one JSON object the
//! launcher writes next to the game home; [`CompatibilitySettings::load`] reads it at
//! start and unknown keys are kept so a newer launcher's file never loses data.
//!
//! Every field carries the same default and range ZalithLauncher2 uses, so a profile
//! exported from one launcher imports into the other unchanged.

use crate::json::Value;
use std::path::Path;

/// Which graphics API the game renders with. Mirrors ZalithLauncher2's `graphicsApi`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GraphicsApi {
    /// The game picks: Vulkan first, then GLES.
    #[default]
    Auto,
    Vulkan,
    /// OpenGL ES — the fallback for GPUs without a working Vulkan driver.
    Gles,
}

impl GraphicsApi {
    pub const ALL: [GraphicsApi; 3] = [GraphicsApi::Auto, GraphicsApi::Vulkan, GraphicsApi::Gles];
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "auto" | "" => Some(Self::Auto),
            "vulkan" | "vulkan-instance" => Some(Self::Vulkan),
            "gles" | "opengl" | "opengl-es" => Some(Self::Gles),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Vulkan => "vulkan",
            Self::Gles => "gles",
        }
    }
}

/// How the resolution scale setting applies. Mirrors ZalithLauncher2's `resolutionRule`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ResolutionRule {
    /// `resolution_ratio` percent of the window's native size.
    #[default]
    Percentage,
    /// The explicit `custom_resolution_width` x `custom_resolution_height`.
    Custom,
}

impl ResolutionRule {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "percentage" | "percent" | "" => Some(Self::Percentage),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Percentage => "percentage",
            Self::Custom => "custom",
        }
    }
}

/// The FPS overlay's shape. Mirrors ZalithLauncher2's `fpsDisplayMode`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FpsDisplayMode {
    Off,
    #[default]
    Number,
    Graph,
}

impl FpsDisplayMode {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "none" | "off" | "" => Some(Self::Off),
            "number" | "count" => Some(Self::Number),
            "graph" | "chart" => Some(Self::Graph),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "none",
            Self::Number => "number",
            Self::Graph => "graph",
        }
    }
}

/// How a look drag on the touchscreen maps to camera rotation. Mirrors ZalithLauncher2's
/// `mouseControlMode`: `Slide` drags the finger and the camera follows the delta,
/// `Absolute` treats the finger's position as the aim point.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TouchLookMode {
    #[default]
    Slide,
    Absolute,
}

impl TouchLookMode {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "slide" | "" => Some(Self::Slide),
            "absolute" | "tap" => Some(Self::Absolute),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Slide => "slide",
            Self::Absolute => "absolute",
        }
    }
}

/// What a gesture does. Mirrors ZalithLauncher2's `gestureTapMouseAction` /
/// `gestureLongPressMouseAction`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GestureAction {
    None,
    /// A left mouse button press (break / use).
    MouseLeft,
    /// A right mouse button press (place / interact).
    #[default]
    MouseRight,
}

impl GestureAction {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "none" | "off" | "" => Some(Self::None),
            "mouse_left" | "left" => Some(Self::MouseLeft),
            "mouse_right" | "right" => Some(Self::MouseRight),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::MouseLeft => "mouse_left",
            Self::MouseRight => "mouse_right",
        }
    }
}

/// The touch controller's output: one action per touch decision.
/// The game turns these into its own input (see `horizon-client`'s `poll_touch`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TouchAction {
    /// A look delta, in screen pixels (x right, y down).
    Look { dx: f32, dy: f32 },
    /// A movement key held or released.
    Move { key: MoveKey, down: bool },
    /// A mouse button.
    Mouse { button: MouseButton, down: bool },
    /// A hotbar slot selected, 0..9.
    Hotbar(u8),
    /// A scroll wheel step (hotbar cycling).
    Scroll { delta: f32 },
    /// A pinch zoom step (positive = zoom in).
    Zoom { delta: f32 },
    /// Jump (double tap on the left half).
    Jump,
    /// Sprint (double tap on the right half).
    Sprint,
    /// Pick the pointed block (hotbar double tap, creative).
    PickBlock,
}

/// The four movement keys the touch controller drives.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MoveKey {
    Forward,
    Back,
    Left,
    Right,
}

/// The mouse buttons the touch controller presses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MouseButton {
    Left,
    Right,
}

/// The full compatibility profile.
#[derive(Clone, Debug)]
pub struct CompatibilitySettings {
    // ---- Renderer / graphics (ZalithLauncher2's Renderer Settings) ----
    /// `graphicsApi`: which API Horizon renders with.
    pub graphics_api: GraphicsApi,
    /// `resolutionRatio`: percent of the native resolution, 25..300.
    pub resolution_ratio: u32,
    /// `resolutionRule`: how `resolution_ratio` applies.
    pub resolution_rule: ResolutionRule,
    /// `customResolutionWidth` / `customResolutionHeight`: the explicit size (0 = unused).
    pub custom_resolution_width: u32,
    pub custom_resolution_height: u32,
    /// `gameFullScreen`: the game opens immersive-fullscreen.
    pub game_full_screen: bool,
    /// `sustainedPerformance`: asks the system for a sustained-performance mode.
    pub sustained_performance: bool,
    /// `requestMaxRefreshRate`: the surface asks for the display's highest refresh rate.
    pub request_max_refresh_rate: bool,

    // ---- Version / game (ZalithLauncher2's Game Settings) ----
    /// `versionIsolation`: each game version keeps its own game folder.
    pub version_isolation: bool,
    /// `versionCustomInfo`: a free-form line shown on the version page.
    pub version_custom_info: String,
    /// `skipGameIntegrityCheck`: launch without verifying the prepared resources.
    pub skip_game_integrity_check: bool,
    /// `ramAllocation`: the memory budget in MiB (native games have no JVM heap; this
    /// caps what the allocator may reserve and what the launcher offers the process).
    pub ram_allocation_mb: i64,
    /// `showLogAutomatic`: the launcher shows the game log when it stops.
    pub show_log_automatic: bool,

    // ---- Display / game settings passthrough ----
    /// `showFPS` / `fpsDisplayMode`: the FPS overlay.
    pub show_fps: bool,
    pub fps_display_mode: FpsDisplayMode,
    /// The game's language (vanilla's `language`).
    pub language: String,
    /// Vanilla's GUI scale (0 = auto).
    pub gui_scale: u32,
    /// Vanilla's max frames per second (260 = unlimited).
    pub max_fps: u32,
    /// Vanilla's render distance, in chunks.
    pub render_distance: i32,
    /// Vanilla's simulation distance, in chunks.
    pub simulation_distance: i32,
    /// Vanilla's master volume, 0..1.
    pub master_volume: f32,

    // ---- Touch controls (ZalithLauncher2's Control Settings) ----
    /// `touchControlsEnabled`: the touch controller is on.
    pub touch_controls_enabled: bool,
    /// `mouseControlMode`: how look drags map to the camera.
    pub touch_look_mode: TouchLookMode,
    /// `cursorSensitivity`: look drag sensitivity, 25..300.
    pub touch_look_sensitivity: u32,
    /// Movement stick sensitivity, 25..300.
    pub touch_move_sensitivity: u32,
    /// `hotbarWidth` / `hotbarHeight`: the hotbar's size in percent of the screen.
    pub hotbar_width: u32,
    pub hotbar_height: u32,
    /// `hotbarDoubleClick` / `hotbarLongClick`: gestures on hotbar slots.
    pub hotbar_double_click: bool,
    pub hotbar_long_click: bool,
    /// `hotbarLongClickDelay`: ms.
    pub hotbar_long_click_delay: i64,

    // ---- Gestures (ZalithLauncher2's gesture settings) ----
    /// `gestureControl`: taps and long presses on the look area act as mouse buttons.
    pub gesture_control: bool,
    /// `gestureTapMouseAction`: a quick tap does this.
    pub gesture_tap_action: GestureAction,
    /// `gestureLongPressMouseAction`: a long press does this.
    pub gesture_long_press_action: GestureAction,
    /// `gestureLongPressDelay`: ms before a press becomes a long press.
    pub gesture_long_press_delay: i64,

    // ---- Gyroscope (ZalithLauncher2's gyroscope settings) ----
    /// `gyroscopeControl`: the device's orientation drives the camera.
    pub gyroscope_control: bool,
    /// `gyroscopeSensitivity`: 25..300.
    pub gyroscope_sensitivity: u32,
    /// `gyroscopeSampleRate`: sensor rate in Hz, 5..50.
    pub gyroscope_sample_rate: u32,
    /// `gyroscopeSmoothing` / `gyroscopeSmoothingWindow`.
    pub gyroscope_smoothing: bool,
    pub gyroscope_smoothing_window: u32,
    pub gyroscope_invert_x: bool,
    pub gyroscope_invert_y: bool,

    // ---- Gamepad (ZalithLauncher2's gamepad settings) ----
    /// `gamepadControl`: an Android gamepad drives the game.
    pub gamepad_control: bool,
    /// `gamepadInputMode`: `mapped` (native mapping) or `none`.
    pub gamepad_input_mode: String,
    /// `gamepadDeadZoneScale`: 50..200.
    pub gamepad_dead_zone_scale: u32,
    /// `gamepadMappingConfig`: the mapping file's name (`default`).
    pub gamepad_mapping_config: String,
}

impl Default for CompatibilitySettings {
    fn default() -> Self {
        Self {
            graphics_api: GraphicsApi::Auto,
            resolution_ratio: 100,
            resolution_rule: ResolutionRule::Percentage,
            custom_resolution_width: 0,
            custom_resolution_height: 0,
            game_full_screen: true,
            sustained_performance: false,
            request_max_refresh_rate: true,
            version_isolation: true,
            version_custom_info: String::new(),
            skip_game_integrity_check: false,
            ram_allocation_mb: 2048,
            show_log_automatic: false,
            show_fps: true,
            fps_display_mode: FpsDisplayMode::Number,
            language: "en".into(),
            gui_scale: 0,
            max_fps: 260,
            render_distance: 12,
            simulation_distance: 12,
            master_volume: 1.0,
            touch_controls_enabled: true,
            touch_look_mode: TouchLookMode::Slide,
            touch_look_sensitivity: 100,
            touch_move_sensitivity: 100,
            hotbar_width: 500,
            hotbar_height: 100,
            hotbar_double_click: true,
            hotbar_long_click: true,
            hotbar_long_click_delay: 300,
            gesture_control: false,
            gesture_tap_action: GestureAction::MouseRight,
            gesture_long_press_action: GestureAction::MouseLeft,
            gesture_long_press_delay: 300,
            gyroscope_control: false,
            gyroscope_sensitivity: 100,
            gyroscope_sample_rate: 16,
            gyroscope_smoothing: true,
            gyroscope_smoothing_window: 4,
            gyroscope_invert_x: false,
            gyroscope_invert_y: false,
            gamepad_control: true,
            gamepad_input_mode: "mapped".into(),
            gamepad_dead_zone_scale: 100,
            gamepad_mapping_config: "default".into(),
        }
    }
}

impl CompatibilitySettings {
    /// The file's name inside the game home.
    pub const FILE_NAME: &'static str = "horizon-android.json";

    pub fn load(path: &Path) -> Self {
        let mut settings = Self::default();
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(value) = Value::parse(&text) {
                settings.apply(&value);
            }
        }
        settings
    }

    /// Parse from JSON text directly (the settings screen's save button).
    pub fn from_json(text: &str) -> Self {
        let mut settings = Self::default();
        if let Ok(value) = Value::parse(text) {
            settings.apply(&value);
        }
        settings
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_json())
    }

    /// Compact JSON text of the whole profile.
    pub fn to_json(&self) -> String {
        self.to_value().to_string()
    }

    /// The profile as a JSON object, in the launcher's key order.
    pub fn to_value(&self) -> Value {
        let mut map = std::collections::BTreeMap::new();
        let insert = |map: &mut std::collections::BTreeMap<String, Value>, key: &str, value: Value| {
            map.insert(key.to_string(), value);
        };
        insert(&mut map, "graphics_api", Value::String(self.graphics_api.as_str().into()));
        insert(&mut map, "resolution_ratio", Value::Number(self.resolution_ratio as f64));
        insert(&mut map, "resolution_rule", Value::String(self.resolution_rule.as_str().into()));
        insert(&mut map, "custom_resolution_width", Value::Number(self.custom_resolution_width as f64));
        insert(&mut map, "custom_resolution_height", Value::Number(self.custom_resolution_height as f64));
        insert(&mut map, "game_full_screen", Value::Bool(self.game_full_screen));
        insert(&mut map, "sustained_performance", Value::Bool(self.sustained_performance));
        insert(&mut map, "request_max_refresh_rate", Value::Bool(self.request_max_refresh_rate));
        insert(&mut map, "version_isolation", Value::Bool(self.version_isolation));
        insert(&mut map, "version_custom_info", Value::String(self.version_custom_info.clone()));
        insert(&mut map, "skip_game_integrity_check", Value::Bool(self.skip_game_integrity_check));
        insert(&mut map, "ram_allocation_mb", Value::Number(self.ram_allocation_mb as f64));
        insert(&mut map, "show_log_automatic", Value::Bool(self.show_log_automatic));
        insert(&mut map, "show_fps", Value::Bool(self.show_fps));
        insert(&mut map, "fps_display_mode", Value::String(self.fps_display_mode.as_str().into()));
        insert(&mut map, "language", Value::String(self.language.clone()));
        insert(&mut map, "gui_scale", Value::Number(self.gui_scale as f64));
        insert(&mut map, "max_fps", Value::Number(self.max_fps as f64));
        insert(&mut map, "render_distance", Value::Number(self.render_distance as f64));
        insert(&mut map, "simulation_distance", Value::Number(self.simulation_distance as f64));
        insert(&mut map, "master_volume", Value::Number(self.master_volume as f64));
        insert(&mut map, "touch_controls_enabled", Value::Bool(self.touch_controls_enabled));
        insert(&mut map, "touch_look_mode", Value::String(self.touch_look_mode.as_str().into()));
        insert(&mut map, "touch_look_sensitivity", Value::Number(self.touch_look_sensitivity as f64));
        insert(&mut map, "touch_move_sensitivity", Value::Number(self.touch_move_sensitivity as f64));
        insert(&mut map, "hotbar_width", Value::Number(self.hotbar_width as f64));
        insert(&mut map, "hotbar_height", Value::Number(self.hotbar_height as f64));
        insert(&mut map, "hotbar_double_click", Value::Bool(self.hotbar_double_click));
        insert(&mut map, "hotbar_long_click", Value::Bool(self.hotbar_long_click));
        insert(&mut map, "hotbar_long_click_delay", Value::Number(self.hotbar_long_click_delay as f64));
        insert(&mut map, "gesture_control", Value::Bool(self.gesture_control));
        insert(&mut map, "gesture_tap_action", Value::String(self.gesture_tap_action.as_str().into()));
        insert(&mut map, "gesture_long_press_action", Value::String(self.gesture_long_press_action.as_str().into()));
        insert(&mut map, "gesture_long_press_delay", Value::Number(self.gesture_long_press_delay as f64));
        insert(&mut map, "gyroscope_control", Value::Bool(self.gyroscope_control));
        insert(&mut map, "gyroscope_sensitivity", Value::Number(self.gyroscope_sensitivity as f64));
        insert(&mut map, "gyroscope_sample_rate", Value::Number(self.gyroscope_sample_rate as f64));
        insert(&mut map, "gyroscope_smoothing", Value::Bool(self.gyroscope_smoothing));
        insert(&mut map, "gyroscope_smoothing_window", Value::Number(self.gyroscope_smoothing_window as f64));
        insert(&mut map, "gyroscope_invert_x", Value::Bool(self.gyroscope_invert_x));
        insert(&mut map, "gyroscope_invert_y", Value::Bool(self.gyroscope_invert_y));
        insert(&mut map, "gamepad_control", Value::Bool(self.gamepad_control));
        insert(&mut map, "gamepad_input_mode", Value::String(self.gamepad_input_mode.clone()));
        insert(&mut map, "gamepad_dead_zone_scale", Value::Number(self.gamepad_dead_zone_scale as f64));
        insert(&mut map, "gamepad_mapping_config", Value::String(self.gamepad_mapping_config.clone()));
        Value::Object(map)
    }

    /// Overwrite the typed fields from a JSON object. Absent keys keep their value,
    /// so a profile written by an older or newer launcher imports cleanly.
    pub fn apply(&mut self, value: &Value) {
        if let Some(api) = value.get_str("graphics_api").and_then(GraphicsApi::parse) {
            self.graphics_api = api;
        }
        if let Some(ratio) = value.get_i64("resolution_ratio") {
            self.resolution_ratio = ratio.clamp(25, 300) as u32;
        }
        if let Some(rule) = value.get_str("resolution_rule").and_then(ResolutionRule::parse) {
            self.resolution_rule = rule;
        }
        if let Some(w) = value.get_i64("custom_resolution_width") {
            self.custom_resolution_width = w.max(0) as u32;
        }
        if let Some(h) = value.get_i64("custom_resolution_height") {
            self.custom_resolution_height = h.max(0) as u32;
        }
        if let Some(b) = value.get_bool("game_full_screen") {
            self.game_full_screen = b;
        }
        if let Some(b) = value.get_bool("sustained_performance") {
            self.sustained_performance = b;
        }
        if let Some(b) = value.get_bool("request_max_refresh_rate") {
            self.request_max_refresh_rate = b;
        }
        if let Some(b) = value.get_bool("version_isolation") {
            self.version_isolation = b;
        }
        if let Some(s) = value.get_str("version_custom_info") {
            self.version_custom_info = s.to_owned();
        }
        if let Some(b) = value.get_bool("skip_game_integrity_check") {
            self.skip_game_integrity_check = b;
        }
        if let Some(mb) = value.get_i64("ram_allocation_mb") {
            self.ram_allocation_mb = mb.max(256);
        }
        if let Some(b) = value.get_bool("show_log_automatic") {
            self.show_log_automatic = b;
        }
        if let Some(b) = value.get_bool("show_fps") {
            self.show_fps = b;
        }
        if let Some(mode) = value.get_str("fps_display_mode").and_then(FpsDisplayMode::parse) {
            self.fps_display_mode = mode;
        }
        if let Some(s) = value.get_str("language") {
            self.language = s.to_owned();
        }
        if let Some(g) = value.get_i64("gui_scale") {
            self.gui_scale = g.max(0) as u32;
        }
        if let Some(f) = value.get_i64("max_fps") {
            self.max_fps = f.clamp(10, 260) as u32;
        }
        if let Some(d) = value.get_i64("render_distance") {
            self.render_distance = d.clamp(2, 32) as i32;
        }
        if let Some(d) = value.get_i64("simulation_distance") {
            self.simulation_distance = d.clamp(5, 32) as i32;
        }
        if let Some(v) = value.get_f64("master_volume") {
            self.master_volume = v.clamp(0.0, 1.0) as f32;
        }
        if let Some(b) = value.get_bool("touch_controls_enabled") {
            self.touch_controls_enabled = b;
        }
        if let Some(mode) = value.get_str("touch_look_mode").and_then(TouchLookMode::parse) {
            self.touch_look_mode = mode;
        }
        if let Some(s) = value.get_i64("touch_look_sensitivity") {
            self.touch_look_sensitivity = s.clamp(25, 300) as u32;
        }
        if let Some(s) = value.get_i64("touch_move_sensitivity") {
            self.touch_move_sensitivity = s.clamp(25, 300) as u32;
        }
        if let Some(w) = value.get_i64("hotbar_width") {
            self.hotbar_width = w.clamp(0, 1000) as u32;
        }
        if let Some(h) = value.get_i64("hotbar_height") {
            self.hotbar_height = h.clamp(0, 1000) as u32;
        }
        if let Some(b) = value.get_bool("hotbar_double_click") {
            self.hotbar_double_click = b;
        }
        if let Some(b) = value.get_bool("hotbar_long_click") {
            self.hotbar_long_click = b;
        }
        if let Some(d) = value.get_i64("hotbar_long_click_delay") {
            self.hotbar_long_click_delay = d.clamp(100, 1000);
        }
        if let Some(b) = value.get_bool("gesture_control") {
            self.gesture_control = b;
        }
        if let Some(a) = value.get_str("gesture_tap_action").and_then(GestureAction::parse) {
            self.gesture_tap_action = a;
        }
        if let Some(a) = value.get_str("gesture_long_press_action").and_then(GestureAction::parse) {
            self.gesture_long_press_action = a;
        }
        if let Some(d) = value.get_i64("gesture_long_press_delay") {
            self.gesture_long_press_delay = d.clamp(100, 1000);
        }
        if let Some(b) = value.get_bool("gyroscope_control") {
            self.gyroscope_control = b;
        }
        if let Some(s) = value.get_i64("gyroscope_sensitivity") {
            self.gyroscope_sensitivity = s.clamp(25, 300) as u32;
        }
        if let Some(r) = value.get_i64("gyroscope_sample_rate") {
            self.gyroscope_sample_rate = r.clamp(5, 50) as u32;
        }
        if let Some(b) = value.get_bool("gyroscope_smoothing") {
            self.gyroscope_smoothing = b;
        }
        if let Some(w) = value.get_i64("gyroscope_smoothing_window") {
            self.gyroscope_smoothing_window = w.clamp(2, 10) as u32;
        }
        if let Some(b) = value.get_bool("gyroscope_invert_x") {
            self.gyroscope_invert_x = b;
        }
        if let Some(b) = value.get_bool("gyroscope_invert_y") {
            self.gyroscope_invert_y = b;
        }
        if let Some(b) = value.get_bool("gamepad_control") {
            self.gamepad_control = b;
        }
        if let Some(s) = value.get_str("gamepad_input_mode") {
            self.gamepad_input_mode = s.to_owned();
        }
        if let Some(s) = value.get_i64("gamepad_dead_zone_scale") {
            self.gamepad_dead_zone_scale = s.clamp(50, 200) as u32;
        }
        if let Some(s) = value.get_str("gamepad_mapping_config") {
            self.gamepad_mapping_config = s.to_owned();
        }
    }

    /// The window size the launcher asks for, given the device's native size and the
    /// resolution settings (ZalithLauncher2's resolution ratio/rule).
    pub fn resolution(&self, native: [u32; 2]) -> [u32; 2] {
        match self.resolution_rule {
            ResolutionRule::Custom if self.custom_resolution_width > 0 => {
                [self.custom_resolution_width, self.custom_resolution_height.max(1)]
            }
            _ => {
                let scale = self.resolution_ratio.max(1) as f64 / 100.0;
                [
                    (((native[0] as f64) * scale) as u32).max(1),
                    (((native[1] as f64) * scale) as u32).max(1),
                ]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_zalith_launcher2() {
        let s = CompatibilitySettings::default();
        assert_eq!(s.graphics_api, GraphicsApi::Auto);
        assert_eq!(s.resolution_ratio, 100);
        assert!(s.game_full_screen);
        assert!(!s.sustained_performance);
        assert!(s.request_max_refresh_rate);
        assert!(s.version_isolation);
        assert!(!s.skip_game_integrity_check);
        assert_eq!(s.ram_allocation_mb, 2048);
        assert!(s.show_fps);
        assert_eq!(s.fps_display_mode, FpsDisplayMode::Number);
        assert!(s.touch_controls_enabled);
        assert_eq!(s.touch_look_mode, TouchLookMode::Slide);
        assert_eq!(s.touch_look_sensitivity, 100);
        assert_eq!(s.hotbar_width, 500);
        assert_eq!(s.hotbar_height, 100);
        assert!(s.hotbar_double_click);
        assert!(s.hotbar_long_click);
        assert_eq!(s.hotbar_long_click_delay, 300);
        assert!(!s.gesture_control);
        assert_eq!(s.gesture_tap_action, GestureAction::MouseRight);
        assert_eq!(s.gesture_long_press_action, GestureAction::MouseLeft);
        assert_eq!(s.gesture_long_press_delay, 300);
        assert!(!s.gyroscope_control);
        assert_eq!(s.gyroscope_sensitivity, 100);
        assert_eq!(s.gyroscope_sample_rate, 16);
        assert!(s.gyroscope_smoothing);
        assert_eq!(s.gyroscope_smoothing_window, 4);
        assert!(s.gamepad_control);
        assert_eq!(s.gamepad_dead_zone_scale, 100);
        assert_eq!(s.gamepad_mapping_config, "default");
    }

    #[test]
    fn json_round_trip_keeps_every_field() {
        let original = CompatibilitySettings {
            graphics_api: GraphicsApi::Gles,
            resolution_ratio: 75,
            resolution_rule: ResolutionRule::Custom,
            custom_resolution_width: 1280,
            custom_resolution_height: 720,
            game_full_screen: false,
            sustained_performance: true,
            request_max_refresh_rate: false,
            version_isolation: false,
            version_custom_info: "custom build".into(),
            skip_game_integrity_check: true,
            ram_allocation_mb: 4096,
            show_log_automatic: true,
            show_fps: false,
            fps_display_mode: FpsDisplayMode::Graph,
            language: "zh_cn".into(),
            gui_scale: 2,
            max_fps: 60,
            render_distance: 16,
            simulation_distance: 8,
            master_volume: 0.5,
            touch_controls_enabled: false,
            touch_look_mode: TouchLookMode::Absolute,
            touch_look_sensitivity: 200,
            touch_move_sensitivity: 200,
            hotbar_width: 800,
            hotbar_height: 150,
            hotbar_double_click: false,
            hotbar_long_click: false,
            hotbar_long_click_delay: 500,
            gesture_control: true,
            gesture_tap_action: GestureAction::MouseLeft,
            gesture_long_press_action: GestureAction::MouseRight,
            gesture_long_press_delay: 400,
            gyroscope_control: true,
            gyroscope_sensitivity: 150,
            gyroscope_sample_rate: 30,
            gyroscope_smoothing: false,
            gyroscope_smoothing_window: 6,
            gyroscope_invert_x: true,
            gyroscope_invert_y: true,
            gamepad_control: false,
            gamepad_input_mode: "none".into(),
            gamepad_dead_zone_scale: 150,
            gamepad_mapping_config: "custom".into(),
        };
        let text = original.to_json();
        let loaded = CompatibilitySettings::from_json(&text);
        assert_eq!(text, loaded.to_json());
        assert_eq!(loaded.graphics_api, GraphicsApi::Gles);
        assert_eq!(loaded.resolution_ratio, 75);
        assert_eq!(loaded.resolution_rule, ResolutionRule::Custom);
        assert_eq!(loaded.custom_resolution_width, 1280);
        assert!(!loaded.game_full_screen);
        assert!(loaded.sustained_performance);
        assert_eq!(loaded.ram_allocation_mb, 4096);
        assert!(loaded.skip_game_integrity_check);
        assert_eq!(loaded.language, "zh_cn");
        assert_eq!(loaded.master_volume, 0.5);
        assert_eq!(loaded.touch_look_sensitivity, 200);
        assert_eq!(loaded.gyroscope_invert_x, true);
        assert_eq!(loaded.gamepad_mapping_config, "custom");
    }

    #[test]
    fn unknown_keys_are_kept_on_load() {
        let mut text = CompatibilitySettings::default().to_json();
        text = text.replace('}', ",\"future_key\":42}");
        let value = Value::parse(&text).expect("parses");
        let loaded = CompatibilitySettings::default();
        let mut merged = loaded;
        merged.apply(&value);
        assert_eq!(value.get_i64("future_key"), Some(42));
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        let text = r#"{"resolution_ratio":9999,"ram_allocation_mb":10,"max_fps":1,"render_distance":999,"touch_look_sensitivity":1,"hotbar_width":-5,"gyroscope_sample_rate":0,"gamepad_dead_zone_scale":1000,"master_volume":2.0}"#;
        let mut s = CompatibilitySettings::default();
        s.apply(&Value::parse(text).unwrap());
        assert_eq!(s.resolution_ratio, 300);
        assert_eq!(s.ram_allocation_mb, 256);
        assert_eq!(s.max_fps, 10);
        assert_eq!(s.render_distance, 32);
        assert_eq!(s.touch_look_sensitivity, 25);
        assert_eq!(s.hotbar_width, 0);
        assert_eq!(s.gyroscope_sample_rate, 5);
        assert_eq!(s.gamepad_dead_zone_scale, 200);
        assert_eq!(s.master_volume, 1.0);
    }

    #[test]
    fn resolution_scaling() {
        let mut s = CompatibilitySettings::default();
        assert_eq!(s.resolution([1920, 1080]), [1920, 1080]);
        s.resolution_ratio = 50;
        assert_eq!(s.resolution([1920, 1080]), [960, 540]);
        s.resolution_rule = ResolutionRule::Custom;
        s.custom_resolution_width = 1280;
        s.custom_resolution_height = 720;
        assert_eq!(s.resolution([1920, 1080]), [1280, 720]);
    }
}
