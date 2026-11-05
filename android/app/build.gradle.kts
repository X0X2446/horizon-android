plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.horizon.android"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.horizon.android"
        minSdk = 24
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0"

        ndk {
            // arm64-v8a (most devices), armeabi-v7a (older), x86_64 (emulators)
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    sourceSets["main"].java.srcDirs("src/main/java")
    sourceSets["main"].jniLibs.srcDirs("libs")
}

dependencies {
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.constraintlayout:constraintlayout:2.1.4")
    implementation("com.google.android.material:material:1.12.0")
}

// --- Rust build (cargo-ndk) ---
//
// The game is a Rust cdylib (libhorizon.so). cargo-ndk builds it for each ABI and
// places the .so files in app/src/main/jniLibs/<abi>/. The Gradle build picks them
// up from there.
//
// Prerequisites (documented in docs/android.md):
//   - Rust toolchain with Android targets:
//       rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
//   - Android NDK (r26 or later)
//   - cargo-ndk: cargo install cargo-ndk
//
// The build task runs before the APK is assembled.

val rustProjectDir = file("../../")
val cargoNdkOutput = file("src/main/jniLibs")

tasks.register("buildRust") {
    group = "build"
    description = "Build the Rust game for Android ABIs via cargo-ndk"
    doLast {
        val abis = listOf(
            "arm64-v8a" to "aarch64-linux-android",
            "armeabi-v7a" to "armv7-linux-androideabi",
            "x86_64" to "x86_64-linux-android",
        )
        for ((abi, rustTarget) in abis) {
            val outDir = file("src/main/jniLibs/$abi")
            outDir.mkdirs()
            exec {
                commandLine(
                    "cargo", "ndk", "-t", rustTarget,
                    "-o", outDir.absolutePath,
                    "build", "--release", "--manifest-path",
                    file("horizon-app/Cargo.toml").absolutePath,
                )
                workingDir = rustProjectDir
            }
        }
    }
}

tasks.preBuild {
    dependsOn("buildRust")
}
