# Horizon on Android

This document covers building and running Horizon (Minecraft: Java, rewritten in
Rust) on Android. It also covers the compatibility settings ported from
ZalithLauncher2.

## Architecture

```
┌─────────────────────────────────────────────┐
│ Android App (Gradle)                        │
│                                          │
│  MainActivity (Kotlin)                     │
│    - Compatibility settings UI             │
│    - Writes horizon-launch.json            │
│    - Starts GameActivity                   │
│                                          │
│  GameActivity (Kotlin, NativeActivity)     │
│    - Writes horizon-launch.json            │
│    - Gyroscope sensor → JNI                │
│    - Surface size → JNI                    │
│    - Pause/resume → JNI                    │
│                                          │
│  libhorizon.so (Rust cdylib)               │
│    - android_main (android-activity glue)  │
│    - JNI bridge (nativeVersion, etc.)      │
│    - Touch controller (pure logic)         │
│    - CompatibilitySettings (pure logic)    │
│    - horizon-client (the game)             │
└─────────────────────────────────────────────┘
```

The Rust code is split into two crates:

- `crates/horizon-android`: the port crate. Pure logic (settings, touch controller,
  paths, JSON) builds and tests on the host with zero dependencies. The Android
  glue (JNI bridge, entry point) is behind `cfg(target_os = "android")`.

- `android/horizon-app`: the wrapper crate. Links the port to the game
  (`horizon-client`) and provides `android_main`. Built by Gradle via cargo-ndk.

## Prerequisites

### Rust toolchain

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk
```

### Android SDK and NDK

- Android Studio (or standalone SDK + NDK r26+)
- JDK 17
- Gradle 8.5+

Set `ANDROID_HOME` (or `ANDROID_SDK_ROOT`) and create `android/local.properties`:

```properties
sdk.dir=/path/to/Android/Sdk
ndk.dir=/path/to/Android/Sdk/ndk/<version>
```

### Game resources

The game needs prepared Minecraft resources (assets, libraries, versions). Build
them with `horizon-prepare` (see the main README):

```bash
cargo run -p horizon-prepare -- --install <minecraft-jar>
```

This creates `.local/horizon-client/<version>/` with everything the game needs.

## Building the APK

```bash
cd android
./gradlew assembleRelease
```

The build:

1. Runs `cargo ndk build --release` for each ABI (arm64-v8a, armeabi-v7a, x86_64)
2. Copies the .so files into `app/src/main/jniLibs/<abi>/`
3. Compiles the Kotlin launcher
4. Assembles the APK

The APK is at `android/app/build/outputs/apk/release/app-release.apk`.

## The compatibility settings

The launcher exposes the same settings family ZalithLauncher2 has, adapted for the
Rust client. They are stored as JSON in `filesDir/horizon-settings.json`:

| Setting | Key | Default | Description |
|---------|-----|---------|-------------|
| Graphics API | `graphics_api` | `auto` | `auto`, `vulkan`, or `gles` |
| Resolution ratio | `resolution_ratio` | 100 | Percent of native resolution (25–300) |
| Resolution rule | `resolution_rule` | `percentage` | `percentage` or `custom` |
| Custom width | `custom_resolution_width` | 0 | Pixels (when rule is `custom`) |
| Custom height | `custom_resolution_height` | 0 | Pixels (when rule is `custom`) |
| Fullscreen | `game_full_screen` | true | Immersive mode |
| Sustained performance | `sustained_performance` | false | `Activity.setSustainedPerformanceMode` |
| Max refresh rate | `request_max_refresh_rate` | true | Request the display's max refresh rate |
| Version isolation | `version_isolation` | true | Separate game home per version |
| Skip integrity check | `skip_game_integrity_check` | false | Launch without verifying resources |
| RAM allocation | `ram_allocation_mb` | 2048 | Memory budget hint (informational on native) |
| Show FPS | `show_fps` | true | FPS counter overlay |
| FPS display mode | `fps_display_mode` | `number` | `number` or `graph` |
| Touch controls | `touch_controls_enabled` | true | Enable the touch controller |
| Touch look mode | `touch_look_mode` | `slide` | `slide` (drag) or `absolute` (tap-aim) |
| Touch look sensitivity | `touch_look_sensitivity` | 100 | Percent (25–300) |
| Touch move sensitivity | `touch_move_sensitivity` | 100 | Percent (25–300) |
| Hotbar width | `hotbar_width` | 500 | Percent of screen width (0–1000) |
| Hotbar height | `hotbar_height` | 100 | Percent of screen height (0–1000) |
| Hotbar double click | `hotbar_double_click` | true | Double-tap a slot = pick block |
| Hotbar long click | `hotbar_long_click` | true | Long-press a slot |
| Hotbar long click delay | `hotbar_long_click_delay` | 300 | Milliseconds (100–1000) |
| Gesture control | `gesture_control` | true | Long-press = hold a mouse button |
| Gesture tap action | `gesture_tap_action` | `mouse_left` | What a right-half long-press does |
| Gesture long press action | `gesture_long_press_action` | `mouse_right` | What a left-half long-press does |
| Gesture long press delay | `gesture_long_press_delay` | 300 | Milliseconds (100–1000) |
| Gyroscope control | `gyroscope_control` | false | Enable gyroscope look |
| Gyroscope sensitivity | `gyroscope_sensitivity` | 100 | Percent (25–300) |
| Gyroscope sample rate | `gyroscope_sample_rate` | 16 | Hz (5–50) |
| Gyroscope smoothing | `gyroscope_smoothing` | true | Smooth the samples |
| Gyroscope smoothing window | `gyroscope_smoothing_window` | 4 | Samples (2–10) |
| Gyroscope invert X | `gyroscope_invert_x` | false | Invert pitch |
| Gyroscope invert Y | `gyroscope_invert_y` | false | Invert yaw |
| Gamepad control | `gamepad_control` | true | Enable gamepad input |
| Gamepad input mode | `gamepad_input_mode` | `mapped` | `mapped` or `none` |
| Gamepad dead zone | `gamepad_dead_zone_scale` | 100 | Percent (50–200) |
| Gamepad mapping | `gamepad_mapping_config` | `default` | `default` or `custom` |
| Language | `language` | `en` | UI language |
| GUI scale | `gui_scale` | 0 | Vanilla GUI scale (0 = auto) |
| Max FPS | `max_fps` | 260 | Frame rate cap (10–260) |
| Render distance | `render_distance` | 12 | Chunks (2–32) |
| Simulation distance | `simulation_distance` | 12 | Chunks (5–32) |
| Master volume | `master_volume` | 1.0 | 0.0–1.0 |
| Show log | `show_log_automatic` | false | Auto-show the log on errors |

## The touch control scheme

The touch controller (pure Rust, unit-tested on host) implements the mobile control
scheme of the ZalithLauncher family:

- **Left half**: movement stick (drag = WASD by direction)
- **Right half**: look stick (drag = camera delta, scaled by sensitivity)
- **Tap left**: break (mouse left down + up)
- **Tap right**: place (mouse right down + up)
- **Double tap left**: jump
- **Double tap right**: sprint
- **Long press left**: hold `gesture_long_press_action` (default: mouse right)
- **Long press right**: hold `gesture_tap_action` (default: mouse left)
- **Hotbar strip** (bottom): tap = select slot, double tap = pick block
- **Pinch** (two fingers on right half): zoom (scroll wheel)

The controller's output is a batch of `TouchAction`s the game consumes once per
frame. The game turns them into its own input: synthetic window events for keys and
buttons, accumulated mouse motion for look.

## Host-build verification

The port crate's pure logic (settings, touch controller, paths, JSON) builds and
tests on the host with zero dependencies:

```bash
cd crates/horizon-android
cargo test --lib --offline
```

This runs 25 unit tests covering the settings schema, the touch controller's state
machine, path handling, and the JSON reader/writer.

## Running on a device

1. Install the APK: `adb install android/app/build/outputs/apk/release/app-release.apk`
2. The launcher starts. Set the graphics API, resolution, and touch settings.
3. Tap Play. The game starts.
4. Use the back button to quit (the game saves and exits cleanly).

## Limitations

- The sandbox cannot reach crates.io, so the full workspace cannot be compiled here.
  The APK must be built on a machine with full internet access.
- `ram_allocation_mb` is informational on a native build: the OS manages memory,
  not a JVM heap. The value is recorded and shown in the launcher.
- Sustained performance and max refresh rate are applied by the Kotlin side
  (`GameActivity`), not the Rust code.
- The game needs prepared Minecraft resources (`.local/horizon-client/<version>/`).
  Without them, the game shows an error (or skips the check if
  `skip_game_integrity_check` is on).
