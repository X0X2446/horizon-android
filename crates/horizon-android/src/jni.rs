//! The JNI surface the APK's Kotlin side calls.
//!
//! Java calls into native; native never calls back into Java (the game's own input
//! and window events flow the other way, through winit). The bridge is deliberately
//! small: the launcher reads the version, tells the touch controller the screen size,
//! feeds the gyroscope, and asks the game to quit.

use jni::objects::{JClass, JString};
use jni::JNIEnv;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, Ordering};
use std::sync::OnceLock;

/// The quit callback: the wrapper crate sets it to `horizon_client::game::request_quit`.
static QUIT_CALLBACK: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

/// Set the callback `nativeQuit` calls. The wrapper crate (Gradle build) sets it
/// before the game starts.
pub fn set_quit_callback(callback: impl Fn() + Send + Sync + 'static) {
    let _ = QUIT_CALLBACK.set(Box::new(callback));
}

/// The screen size the touch controller works in (set by the activity).
static SCREEN_W: AtomicU32 = AtomicU32::new(0);
static SCREEN_H: AtomicU32 = AtomicU32::new(0);
/// Set when the activity is paused: the game releases its touches.
static PAUSED: AtomicBool = AtomicBool::new(false);
/// The gyroscope's latest sample (rotation vector, device frame).
static GYRO_X: AtomicI64 = AtomicI64::new(0);
static GYRO_Y: AtomicI64 = AtomicI64::new(0);
static GYRO_Z: AtomicI64 = AtomicI64::new(0);
static GYRO_T: AtomicI64 = AtomicI64::new(0);
static GYRO_SEEN: AtomicBool = AtomicBool::new(false);

/// The screen size in pixels.
pub fn screen_size() -> (u32, u32) {
    (SCREEN_W.load(Ordering::Relaxed), SCREEN_H.load(Ordering::Relaxed))
}

/// Set the screen size (from the activity's `onSurfaceChanged` / config).
pub fn set_screen_size(width: u32, height: u32) {
    SCREEN_W.store(width, Ordering::Relaxed);
    SCREEN_H.store(height, Ordering::Relaxed);
}

/// Whether the activity is paused (the game should release its touches).
pub fn paused() -> bool {
    PAUSED.load(Ordering::Relaxed)
}

pub fn set_paused(paused: bool) {
    PAUSED.store(paused, Ordering::Relaxed);
}

/// The gyroscope's latest sample, if any.
pub fn gyroscope() -> Option<(f32, f32, f32, i64)> {
    if GYRO_SEEN.load(Ordering::Relaxed) {
        Some((
            GYRO_X.load(Ordering::Relaxed) as f32,
            GYRO_Y.load(Ordering::Relaxed) as f32,
            GYRO_Z.load(Ordering::Relaxed) as f32,
            GYRO_T.load(Ordering::Relaxed),
        ))
    } else {
        None
    }
}

fn feed_gyroscope(x: f32, y: f32, z: f32, t: i64) {
    GYRO_X.store((x * 1000.0) as i64, Ordering::Relaxed);
    GYRO_Y.store((y * 1000.0) as i64, Ordering::Relaxed);
    GYRO_Z.store((z * 1000.0) as i64, Ordering::Relaxed);
    GYRO_T.store(t, Ordering::Relaxed);
    GYRO_SEEN.store(true, Ordering::Relaxed);
}

/// `horizon-android`'s version, for the launcher's About page.
#[no_mangle]
pub extern "system" fn nativeVersion(env: JNIEnv, _class: JClass) -> JString {
    env.new_string(crate::PORT_VERSION)
        .unwrap_or_else(|_| unsafe { JString::from_raw(std::ptr::null_mut()) })
}

/// The game's state, as one of `idle`, `launching`, `running`, `stopping`.
/// The launcher shows it next to the play button; the game itself updates it.
#[no_mangle]
pub extern "system" fn nativeGameState(env: JNIEnv, _class: JClass) -> JString {
    let state = crate::game::state();
    env.new_string(state)
        .unwrap_or_else(|_| unsafe { JString::from_raw(std::ptr::null_mut()) })
}

/// The screen size in pixels (the activity calls it on surface changes).
#[no_mangle]
pub extern "system" fn nativeSetScreenSize(
    _env: JNIEnv,
    _class: JClass,
    width: jni::sys::jint,
    height: jni::sys::jint,
) {
    if width > 0 && height > 0 {
        set_screen_size(width as u32, height as u32);
    }
}

/// The activity's lifecycle: the game releases its touches while paused.
#[no_mangle]
pub extern "system" fn nativeOnPaused(_env: JNIEnv, _class: JClass) {
    set_paused(true);
}

#[no_mangle]
pub extern "system" fn nativeOnResumed(_env: JNIEnv, _class: JClass) {
    set_paused(false);
}

/// A gyroscope sample (rotation vector in the device frame, and a timestamp).
#[no_mangle]
pub extern "system" fn nativeOnGyroscope(
    _env: JNIEnv,
    _class: JClass,
    x: jni::sys::jfloat,
    y: jni::sys::jfloat,
    z: jni::sys::jfloat,
    timestamp_ns: jni::sys::jlong,
) {
    feed_gyroscope(x as f32, y as f32, z as f32, timestamp_ns as i64);
}

/// Ask the game to save and quit (the launcher's "stop" button).
#[no_mangle]
pub extern "system" fn nativeQuit(_env: JNIEnv, _class: JClass) {
    if let Some(callback) = QUIT_CALLBACK.get() {
        callback();
    }
}
