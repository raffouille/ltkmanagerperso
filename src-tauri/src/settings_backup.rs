//! Writes the settings to a backup file, and reads a backup back over the settings of this machine.
//!
//! See "Backup" in `docs/ux/SETTINGS.md`.

use crate::state::Settings;
use fs_err as fs;
use ltk_manager_base::utils::fs::atomic_write;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The `format` field that marks a file as a settings backup.
const FORMAT: &str = "ltk-manager-settings";

/// The newest backup version that this build reads and the version that it writes.
const VERSION: u32 = 1;

/// The document of a settings backup file.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Envelope {
    format: String,
    version: u32,
    /// The version of the manager that wrote the backup.
    app_version: String,
    settings: Value,
}

/// A reason that a settings backup cannot be written or read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error, specta::Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SettingsBackupError {
    /// The file is not a settings backup.
    #[error("the file is not an LTK Manager settings backup")]
    NotABackup,
    /// A newer version of the manager wrote the backup.
    #[error("the backup has version {version} and this build reads up to version {supported}")]
    NewerVersion { version: u32, supported: u32 },
    /// The file cannot be read or written.
    #[error("{detail}")]
    Io { detail: String },
}

impl From<std::io::Error> for SettingsBackupError {
    fn from(error: std::io::Error) -> Self {
        Self::Io {
            detail: error.to_string(),
        }
    }
}

/// Writes `settings` to a backup file at `path`.
///
/// The diagnostics secret is left out, so a restore on another machine keeps its own identity.
///
/// # Errors
///
/// Fails with [`SettingsBackupError::Io`] if the file cannot be written.
pub fn export(settings: &Settings, path: &Path) -> Result<(), SettingsBackupError> {
    let mut settings = settings.clone();
    settings.telemetry_secret = None;

    let envelope = Envelope {
        format: FORMAT.to_owned(),
        version: VERSION,
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        settings: serde_json::to_value(&settings).map_err(std::io::Error::other)?,
    };
    let contents = serde_json::to_vec_pretty(&envelope).map_err(std::io::Error::other)?;
    atomic_write(path, &contents)?;
    Ok(())
}

/// Reads the backup at `path` over `current`, without saving the result.
///
/// The backup sets every key that it carries. `current` keeps the data location, the
/// diagnostics identity and the first-run flags, and it keeps a path that the backup names if
/// that path does not exist on this machine. A key that this build does not know is ignored.
///
/// # Errors
///
/// Fails with [`SettingsBackupError::NotABackup`] if the file is not a backup, with
/// [`SettingsBackupError::NewerVersion`] if a newer version wrote it, and with
/// [`SettingsBackupError::Io`] if it cannot be read.
pub fn restore(current: &Settings, path: &Path) -> Result<Settings, SettingsBackupError> {
    let contents = fs::read(path)?;
    let envelope: Envelope =
        serde_json::from_slice(&contents).map_err(|_| SettingsBackupError::NotABackup)?;
    if envelope.format != FORMAT {
        return Err(SettingsBackupError::NotABackup);
    }
    if envelope.version > VERSION {
        return Err(SettingsBackupError::NewerVersion {
            version: envelope.version,
            supported: VERSION,
        });
    }

    merge(current, envelope.settings)
}

/// Overlays the keys of `backup` on `current`, then puts back what this machine keeps.
fn merge(current: &Settings, backup: Value) -> Result<Settings, SettingsBackupError> {
    let Value::Object(backup) = backup else {
        return Err(SettingsBackupError::NotABackup);
    };

    let mut merged = serde_json::to_value(current).map_err(std::io::Error::other)?;
    if let Value::Object(fields) = &mut merged {
        fields.extend(backup);
    }
    let mut restored: Settings =
        serde_json::from_value(merged).map_err(|_| SettingsBackupError::NotABackup)?;

    restored.config.mod_storage_path = current.config.mod_storage_path.clone();
    restored.telemetry_secret = current.telemetry_secret.clone();
    restored.first_run_complete = current.first_run_complete;
    restored.migration_dismissed = current.migration_dismissed;
    restored.has_seen_hdd_warning = current.has_seen_hdd_warning;
    restored.has_seen_diagnostics_notice = current.has_seen_diagnostics_notice;

    restored.config.league_path =
        existing_or(restored.config.league_path, &current.config.league_path);
    restored.config.workshop_path =
        existing_or(restored.config.workshop_path, &current.config.workshop_path);
    restored.backdrop_image = existing_or(restored.backdrop_image, &current.backdrop_image);

    Ok(restored)
}

/// Returns `restored` if it names a path that exists, and `current` otherwise.
fn existing_or(restored: Option<PathBuf>, current: &Option<PathBuf>) -> Option<PathBuf> {
    restored
        .filter(|path| path.exists())
        .or_else(|| current.clone())
}

#[cfg(test)]
mod tests;
