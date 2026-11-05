//! The launch request: reading what the launcher asked for and building the
//! game's command line.
//!
//! The Kotlin launcher writes `horizon-launch.json` into the app's files directory
//! and starts the NativeActivity. This module reads that request and turns it into
//! the arguments `horizon-client` understands — the same way the desktop launcher
//! builds its command line. It has no dependencies on the game, so it builds and
//! is unit-tested on the host.
//!
//! The wrapper crate (created by the Gradle build) calls [`build_args`] and hands
//! the result to `horizon_client::entry::run_on_android`.

use crate::config::CompatibilitySettings;
use crate::json::Value;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};

/// The launcher's request: what to launch and where.
#[derive(Clone, Debug)]
pub struct LaunchConfig {
    /// The game home (`--home`, `--game-dir`): saves, options.txt, logs.
    pub home: PathBuf,
    /// The prepared resources (`--resources`).
    pub resources: PathBuf,
    /// The game version, e.g. `26.3`.
    pub version: String,
    /// The compatibility settings file, or none for the defaults.
    pub config: Option<PathBuf>,
    /// A world to open (`--world`).
    pub world: Option<String>,
    /// A server to join (`--server`).
    pub server: Option<String>,
    /// The player name (`--name`).
    pub player: Option<String>,
    /// The signed-in account's session file (`--session-file`).
    pub session: Option<PathBuf>,
    /// The window size the device's surface offers (the launcher measures it).
    pub window_size: Option<(u32, u32)>,
}

/// The launcher's request file, inside the app's files directory.
pub const LAUNCH_FILE: &str = "horizon-launch.json";

/// Where the game's state is written (the Kotlin side reads it).
static STATE: AtomicU8 = AtomicU8::new(0);

const STATE_IDLE: u8 = 0;
const STATE_LAUNCHING: u8 = 1;
const STATE_RUNNING: u8 = 2;
const STATE_STOPPING: u8 = 3;

/// The game's state for the launcher: `idle`, `launching`, `running`, `stopping`.
pub fn state() -> &'static str {
    match STATE.load(Ordering::Relaxed) {
        STATE_LAUNCHING => "launching",
        STATE_RUNNING => "running",
        STATE_STOPPING => "stopping",
        _ => "idle",
    }
}

fn set_state(state: u8) {
    STATE.store(state, Ordering::Relaxed);
}

/// Mark the game as running (the wrapper crate calls this around its run).
pub fn set_running() {
    set_state(STATE_RUNNING);
}

/// Mark the game as stopping.
pub fn set_stopping() {
    set_state(STATE_STOPPING);
}

/// Mark the game as idle (the wrapper crate calls this after the run returns).
pub fn set_idle() {
    set_state(STATE_IDLE);
}

/// Read the launcher's request from the app's files directory.
pub fn load_launch_config(files_dir: &std::path::Path) -> Option<LaunchConfig> {
    let text = std::fs::read_to_string(files_dir.join(LAUNCH_FILE)).ok()?;
    let value = Value::parse(&text).ok()?;
    let home = value.get_str("home")?.to_owned();
    let resources = value
        .get_str("resources")
        .unwrap_or(&home)
        .to_owned();
    Some(LaunchConfig {
        home: PathBuf::from(home),
        resources: PathBuf::from(resources),
        version: value.get_str("version").unwrap_or("").to_owned(),
        config: value.get_str("config").map(PathBuf::from),
        world: value.get_str("world").map(str::to_owned),
        server: value.get_str("server").map(str::to_owned),
        player: value.get_str("player").map(str::to_owned),
        session: value.get_str("session").map(PathBuf::from),
        window_size: match (value.get_i64("width"), value.get_i64("height")) {
            (Some(w), Some(h)) if w > 0 && h > 0 => Some((w as u32, h as u32)),
            _ => None,
        },
    })
}

/// The command line the game starts with: the launcher's request plus everything the
/// compatibility settings decide.
pub fn build_args(config: &LaunchConfig, settings: &CompatibilitySettings) -> Vec<String> {
    let mut args = vec![
        "--home".into(),
        config.home.display().to_string(),
        "--game-dir".into(),
        config.home.display().to_string(),
        "--resources".into(),
        config.resources.display().to_string(),
        // The window starts at the scaled resolution; the surface is created at it.
        "--width".into(),
        config
            .window_size
            .map(|(w, _)| w.to_string())
            .unwrap_or_else(|| "1280".into()),
        "--height".into(),
        config
            .window_size
            .map(|(_, h)| h.to_string())
            .unwrap_or_else(|| "720".into()),
        // The graphics API the compatibility profile chose.
        "--graphics-api".into(),
        settings.graphics_api.as_str().into(),
    ];
    if settings.skip_game_integrity_check {
        args.push("--skip-integrity-check".into());
    }
    if let Some(world) = &config.world {
        args.push("--world".into());
        args.push(world.clone());
    }
    if let Some(server) = &config.server {
        args.push("--server".into());
        args.push(server.clone());
    }
    if let Some(player) = &config.player {
        args.push("--name".into());
        args.push(player.clone());
    }
    if let Some(session) = &config.session {
        args.push("--session-file".into());
        args.push(session.display().to_string());
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CompatibilitySettings, GraphicsApi};
    use std::path::Path;

    fn test_config() -> LaunchConfig {
        LaunchConfig {
            home: PathBuf::from("/data/user/0/com.horizon.android/files/horizon/versions/26.3"),
            resources: PathBuf::from("/data/user/0/com.horizon.android/files/horizon/versions/26.3"),
            version: "26.3".into(),
            config: None,
            world: Some("New World".into()),
            server: None,
            player: Some("Player".into()),
            session: None,
            window_size: Some((1280, 720)),
        }
    }

    #[test]
    fn build_args_includes_home_resources_and_window() {
        let config = test_config();
        let settings = CompatibilitySettings::default();
        let args = build_args(&config, &settings);

        let home = config.home.display().to_string();
        let resources = config.resources.display().to_string();
        assert!(args.contains(&"--home".to_string()));
        assert!(args.contains(&home));
        assert!(args.contains(&"--game-dir".to_string()));
        assert!(args.contains(&resources));
        assert!(args.contains(&"--width".to_string()));
        assert!(args.contains(&"1280".to_string()));
        assert!(args.contains(&"--height".to_string()));
        assert!(args.contains(&"720".to_string()));
        assert!(args.contains(&"--graphics-api".to_string()));
        assert!(args.contains(&"auto".to_string()));
    }

    #[test]
    fn build_args_includes_world_and_player() {
        let config = test_config();
        let settings = CompatibilitySettings::default();
        let args = build_args(&config, &settings);

        assert!(args.contains(&"--world".to_string()));
        assert!(args.contains(&"New World".to_string()));
        assert!(args.contains(&"--name".to_string()));
        assert!(args.contains(&"Player".to_string()));
    }

    #[test]
    fn build_args_skips_integrity_check_when_set() {
        let config = test_config();
        let mut settings = CompatibilitySettings::default();
        settings.skip_game_integrity_check = true;
        let args = build_args(&config, &settings);
        assert!(args.contains(&"--skip-integrity-check".to_string()));
    }

    #[test]
    fn build_args_omits_server_when_absent() {
        let mut config = test_config();
        config.server = None;
        let settings = CompatibilitySettings::default();
        let args = build_args(&config, &settings);
        assert!(!args.contains(&"--server".to_string()));
    }

    #[test]
    fn state_starts_idle() {
        assert_eq!(state(), "idle");
        set_running();
        assert_eq!(state(), "running");
        set_stopping();
        assert_eq!(state(), "stopping");
        set_idle();
        assert_eq!(state(), "idle");
    }
}
