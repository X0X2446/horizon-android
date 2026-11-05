//! Horizon's Android app: the wrapper crate.
//!
//! This crate is built by Gradle (cargo-ndk) for the Android target. It links the
//! port (`horizon-android`) to the game (`horizon-client`) and provides the
//! `android_main` entry point NativeActivity calls. The port crate is
//! dependency-free on the host (unit-tested there); this crate is Android-only.
//!
//! # What `android_main` does
//!
//! 1. Calls [`horizon_android::entry::prepare`] to read the launcher's request and
//!    install the touch controller.
//! 2. Sets the quit callback (JNI `nativeQuit` → `horizon_client::game::request_quit`).
//! 3. Installs the touch and gyroscope sources the game polls.
//! 4. Runs the game (`horizon_client::entry::run_on_android`).
//! 5. Releases the touches the game may have left held.

use android_activity::AndroidApp;

/// The touch source the game polls: the port's controller behind `entry`'s mutex.
struct AndroidTouchSource;

impl horizon_client_input::touch::TouchSource for AndroidTouchSource {
    fn on_pointer(
        &self,
        x: f32,
        y: f32,
        pointer_id: i32,
        phase: horizon_client_input::touch::PointerPhase,
    ) {
        horizon_android::entry::feed_touch(horizon_android::touch::TouchPoint {
            id: pointer_id,
            x,
            y,
            phase: match phase {
                horizon_client_input::touch::PointerPhase::Down => horizon_android::touch::TouchPhase::Down,
                horizon_client_input::touch::PointerPhase::Move => horizon_android::touch::TouchPhase::Move,
                horizon_client_input::touch::PointerPhase::Up => horizon_android::touch::TouchPhase::Up,
            },
            pressure: 0.0,
        });
    }

    fn tick(&self) -> Vec<horizon_client_input::touch::TouchAction> {
        horizon_android::entry::tick_touch()
            .into_iter()
            .map(convert)
            .collect()
    }

    fn take_actions(&self) -> Vec<horizon_client_input::touch::TouchAction> {
        horizon_android::entry::take_touch_actions()
            .into_iter()
            .map(convert)
            .collect()
    }
}

/// The gyroscope samples the JNI bridge collected.
struct AndroidGyroSource;

impl horizon_client_input::touch::GyroSource for AndroidGyroSource {
    fn sample(&self) -> Option<horizon_client_input::touch::GyroSample> {
        horizon_android::jni::gyroscope().map(|(x, y, z, t)| horizon_client_input::touch::GyroSample {
            x,
            y,
            z,
            t_ns: t,
        })
    }
}

/// The port's actions as the game's own.
fn convert(action: horizon_android::config::TouchAction) -> horizon_client_input::touch::TouchAction {
    use horizon_android::config::TouchAction as Port;
    use horizon_client_input::touch::TouchAction as Game;
    match action {
        Port::Look { dx, dy } => Game::Look { dx, dy },
        Port::Move { key, down } => Game::Move {
            key: match key {
                horizon_android::config::TouchMove::Forward => horizon_client_input::touch::TouchMove::Forward,
                horizon_android::config::TouchMove::Back => horizon_client_input::touch::TouchMove::Back,
                horizon_android::config::TouchMove::Left => horizon_client_input::touch::TouchMove::Left,
                horizon_android::config::TouchMove::Right => horizon_client_input::touch::TouchMove::Right,
            },
            down,
        },
        Port::Mouse { button, down } => Game::Mouse {
            button: match button {
                horizon_android::config::TouchMouse::Left => horizon_client_input::touch::TouchMouse::Left,
                horizon_android::config::TouchMouse::Right => horizon_client_input::touch::TouchMouse::Right,
            },
            down,
        },
        Port::Hotbar(slot) => Game::Hotbar(slot),
        Port::Scroll { delta } => Game::Scroll { delta },
        Port::Zoom { delta } => Game::Zoom { delta },
        Port::Jump => Game::Jump,
        Port::Sprint => Game::Sprint,
        Port::PickBlock => Game::PickBlock,
    }
}

/// The entry point NativeActivity calls in its own thread.
#[no_mangle]
pub fn android_main(app: AndroidApp) {
    log::info!("Horizon", "android_main");

    let Some(prepared) = horizon_android::entry::prepare(app) else {
        log::error!(
            "Horizon",
            "no launch request: the launcher must start the game"
        );
        return;
    };

    // The quit flag the JNI bridge sets reaches the game's idle loop.
    horizon_android::jni::set_quit_callback(horizon_client::game::request_quit);

    // The touch and gyroscope sources the game polls.
    horizon_client_input::touch::set_touch_source(std::sync::Arc::new(AndroidTouchSource));
    horizon_client_input::touch::set_gyro_source(std::sync::Arc::new(AndroidGyroSource));
    log::info!("Horizon", "touch and gyroscope sources installed");

    // Build the command line from the compatibility settings and run the game.
    let args = horizon_android::game::build_args(&prepared.config, &prepared.settings);
    horizon_android::game::set_running();
    log::info!(
        "Horizon",
        &format!(
            "launching: home={} version={} world={:?} server={:?}",
            prepared.config.home.display(),
            prepared.config.version,
            prepared.config.world,
            prepared.config.server
        )
    );
    let result = horizon_client::entry::run_on_android(args);
    horizon_android::game::set_idle();
    if let Err(e) = result {
        log::error!("Horizon", &format!("the game stopped: {e}"));
    }

    // Release the touches the game may have left held.
    horizon_android::entry::release_touches();
}
