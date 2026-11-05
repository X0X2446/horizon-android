//! Horizon's Android port.
//!
//! The crate is the platform side of the port: it owns the compatibility settings
//! (the same family ZalithLauncher2 exposes to players: renderer backend, resolution
//! scaling, memory allocation, touch controls, gyroscope, version isolation...), the
//! touch controller that turns screen touches into the game's input, the Android file
//! layout and the `android_main` / JNI entry points the APK loads.
//!
//! Everything that is not Android-specific (settings, touch logic, paths, JSON) is
//! pure Rust with no dependencies, so it builds and is unit-tested on the host. The
//! Android-only parts (`entry`, `jni`, the `game` feature) sit behind `cfg` gates.
//!
//! # Layout
//!
//! - [`config`]: [`CompatibilitySettings`], the persisted compatibility profile.
//! - [`touch`]: [`TouchController`], raw touches to game actions.
//! - [`paths`]: the game home on Android.
//! - [`json`]: the small JSON reader/writer the settings file uses.
//! - [`entry`] (Android only): `android_main` and the JNI bridge.
//! - [`game`] (feature `game`): the launch into `horizon-client`.

#![forbid(unsafe_op_in_unsafe_fn)]

pub mod config;
pub mod json;
pub mod paths;
pub mod touch;

#[cfg(target_os = "android")]
pub mod entry;

pub mod game;

#[cfg(target_os = "android")]
pub mod jni;

#[cfg(target_os = "android")]
pub mod log;

/// The version this build of the port reports to the launcher.
pub const PORT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// `libhorizon.so`'s JNI library name, as `System.loadLibrary` in the APK expects it.
pub const LIB_NAME: &str = "horizon";
