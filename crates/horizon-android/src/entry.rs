//! The platform entry: setting up the touch controller and reading the launcher's
//! request.
//!
//! `android_main` is the symbol NativeActivity calls in its own thread. The port
//! crate provides `prepare` (everything before the game starts) and the JNI bridge;
//! the wrapper crate (created by the Gradle build) provides `android_main`, which
//! calls `prepare`, installs the game's input sources and runs the game.
//!
//! This module is Android-only: it uses `AndroidApp` and the NDK's lifecycle.

use android_activity::AndroidApp;
use std::sync::Mutex;

/// The single controller instance the game's `poll_touch` drives.
static TOUCH: Mutex<Option<crate::touch::TouchController>> = Mutex::new(None);

/// Feed one touch (from a winit pointer event) into the controller.
pub fn feed_touch(point: crate::touch::TouchPoint) {
    let mut guard = TOUCH.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(controller) = guard.as_mut() {
        controller.feed(point, now_ms());
    }
}

/// The controller's matured gestures (long presses) for this frame.
pub fn tick_touch() -> Vec<crate::config::TouchAction> {
    let mut guard = TOUCH.lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_mut() {
        Some(controller) => controller.tick(now_ms()),
        None => Vec::new(),
    }
}

/// The actions the controller decided since the last call.
pub fn take_touch_actions() -> Vec<crate::config::TouchAction> {
    let mut guard = TOUCH.lock().unwrap_or_else(|e| e.into_inner());
    match guard.as_mut() {
        Some(controller) => controller.finish(),
        None => Vec::new(),
    }
}

/// Retarget the controller at a new screen size.
pub fn set_screen_size(width: u32, height: u32) {
    crate::jni::set_screen_size(width, height);
    if let Some(controller) = TOUCH.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        controller.set_screen_size(width, height);
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Everything the wrapper crate needs to run the game: the launch request, the
/// compatibility settings, and the touch controller (already installed).
pub struct Prepared {
    /// The launcher's request (home, resources, world, …).
    pub config: crate::game::LaunchConfig,
    /// The compatibility settings loaded from the profile.
    pub settings: crate::config::CompatibilitySettings,
}

/// Set up the platform: read the launcher's request, install the touch controller,
/// set the environment. Returns `None` when the launcher did not ask for a game.
pub fn prepare(app: AndroidApp) -> Option<Prepared> {
    crate::log::info("Horizon", "android_main");
    let files_dir = app
        .internal_data_path()
        .unwrap_or_else(|| std::env::temp_dir().join("horizon-android"));
    let config = crate::game::load_launch_config(&files_dir)?;
    let settings = match &config.config {
        Some(path) => crate::config::CompatibilitySettings::load(path),
        None => crate::config::CompatibilitySettings::default(),
    };
    let (w, h) = config.window_size.unwrap_or((1280, 720));

    // The game's own environment: its home.
    std::env::set_var("HORIZON_HOME", &config.home);
    set_screen_size(w, h);

    // The touch controller the game's poll_touch drains.
    *TOUCH.lock().unwrap_or_else(|e| e.into_inner()) = Some(crate::touch::TouchController::new(settings.clone()));

    crate::log::info(
        "Horizon",
        &format!(
            "launching: home={} version={} world={:?} server={:?}",
            config.home.display(),
            config.version,
            config.world,
            config.server
        ),
    );

    Some(Prepared { config, settings })
}

/// Release the touches the game may have left held.
pub fn release_touches() {
    let _ = take_touch_actions();
}
