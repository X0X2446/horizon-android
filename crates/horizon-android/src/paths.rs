//! The game home on Android.
//!
//! The layout follows the desktop game's folder (`saves/`, `logs/`, `options.txt`,
//! `.local/`) inside the app's private files directory, with one subfolder per game
//! version when [`CompatibilitySettings::version_isolation`] is on — the same
//! isolation ZalithLauncher2's `versionIsolation` setting gives its profiles.

use crate::config::CompatibilitySettings;
use std::path::{Path, PathBuf};

/// Where the game's files live on Android.
#[derive(Clone, Debug)]
pub struct HorizonPaths {
    /// The app's files directory (`context.getFilesDir()`), the root of everything.
    files_dir: PathBuf,
}

impl HorizonPaths {
    /// Build from the app's files directory.
    pub fn new(files_dir: impl Into<PathBuf>) -> Self {
        Self {
            files_dir: files_dir.into(),
        }
    }

    /// The app's files directory.
    pub fn files_dir(&self) -> &Path {
        &self.files_dir
    }

    /// `horizon/` inside the files directory — the default game home.
    pub fn horizon_dir(&self) -> PathBuf {
        self.files_dir.join("horizon")
    }

    /// The game home for a version: `horizon/versions/<version>/` with version
    /// isolation on, `horizon/` without it.
    pub fn game_home(&self, settings: &CompatibilitySettings, version: &str) -> PathBuf {
        if settings.version_isolation && !version.is_empty() {
            self.horizon_dir()
                .join("versions")
                .join(sanitize(version))
        } else {
            self.horizon_dir()
        }
    }

    /// The prepared game resources the launcher built (`.local/horizon-client/26.3`).
    pub fn resources_dir(&self, settings: &CompatibilitySettings, version: &str) -> PathBuf {
        let home = self.game_home(settings, version);
        home.join(".local").join("horizon-client").join(version)
    }

    /// The game's saves.
    pub fn saves_dir(&self, settings: &CompatibilitySettings, version: &str) -> PathBuf {
        self.game_home(settings, version).join("saves")
    }

    /// The game's logs and crash reports.
    pub fn logs_dir(&self, settings: &CompatibilitySettings, version: &str) -> PathBuf {
        self.game_home(settings, version).join("logs")
    }

    pub fn crash_reports_dir(&self, settings: &CompatibilitySettings, version: &str) -> PathBuf {
        self.game_home(settings, version).join("crash-reports")
    }

    /// The compatibility settings file inside the game home.
    pub fn settings_file(&self, settings: &CompatibilitySettings, version: &str) -> PathBuf {
        self.game_home(settings, version)
            .join(CompatibilitySettings::FILE_NAME)
    }

    /// The launcher's own state (accounts, version lists, last pick).
    pub fn launcher_dir(&self) -> PathBuf {
        self.files_dir.join("horizon-launcher")
    }

    /// The account sessions the launcher keeps (`--session-file`).
    pub fn sessions_dir(&self) -> PathBuf {
        self.launcher_dir().join("sessions")
    }

    /// The screenshots the game writes.
    pub fn screenshots_dir(&self, settings: &CompatibilitySettings, version: &str) -> PathBuf {
        self.game_home(settings, version).join("screenshots")
    }

    /// `resources/` next to the executable, for a release that ships the prepared
    /// files inside the APK (the desktop launcher's layout).
    pub fn bundled_resources_dir(&self) -> PathBuf {
        self.files_dir.join("resources")
    }
}

/// A version string safe to use as one path component.
fn sanitize(version: &str) -> String {
    let cleaned: String = version
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "default".to_string()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_isolation_separates_homes() {
        let paths = HorizonPaths::new("/data/user/0/example.app/files");
        let settings = CompatibilitySettings {
            version_isolation: true,
            ..Default::default()
        };
        assert_eq!(
            paths.game_home(&settings, "26.3"),
            std::path::Path::new("/data/user/0/example.app/files/horizon/versions/26.3")
        );
        assert_eq!(
            paths.resources_dir(&settings, "26.3"),
            std::path::Path::new("/data/user/0/example.app/files/horizon/versions/26.3/.local/horizon-client/26.3")
        );
        let shared = CompatibilitySettings {
            version_isolation: false,
            ..Default::default()
        };
        assert_eq!(
            paths.game_home(&shared, "26.3"),
            std::path::Path::new("/data/user/0/example.app/files/horizon")
        );
    }

    #[test]
    fn unsafe_version_names_are_sanitized() {
        let paths = HorizonPaths::new("/files");
        let settings = CompatibilitySettings {
            version_isolation: true,
            ..Default::default()
        };
        assert_eq!(
            paths.game_home(&settings, "26.3.1/beta"),
            std::path::Path::new("/files/horizon/versions/26.3.1_beta")
        );
    }
}
