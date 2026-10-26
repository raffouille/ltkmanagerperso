//! Reports the disk usage of the manager's data and clears the parts that the app rebuilds.
//!
//! See "Disk usage" in `docs/ux/SETTINGS.md`.

use crate::data_location::{data_dir, size_of, MOD_PREVIEWS, SHADERS};
use crate::error::{AppError, AppResult};
use fs_err as fs;
use ltk_manager_base::config::Config;
use ltk_manager_library::mods::{ModLibrary, StorageLayout as _};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Manager, Runtime};

/// The marker under the app data directory that asks the next start to clear the webview cache.
const WEBVIEW_CLEAR_MARKER: &str = ".clear-webview-cache";

/// The cache folders of the WebView2 profile: the HTTP cache, the V8 code cache and the GPU
/// cache. `Local Storage`, which holds the persisted UI state, is not one of them.
const WEBVIEW_CACHE_DIRS: [&str; 3] = ["Cache", "Code Cache", "GPUCache"];

/// A part of the manager's data on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum DataArea {
    /// The installed mods, which no clear removes.
    Mods,
    /// The overlay of each profile and the projects of the built-in mods.
    OverlayBuilds,
    /// The unpacked copies of packed mods that the previews read.
    ModPreviews,
    /// The translated shaders.
    Shaders,
    /// Overlay artifacts at the root of the data directory, which no build reads.
    Leftovers,
    /// The cache folders of the webview. Only on Windows.
    WebviewCache,
}

/// The bytes that one [`DataArea`] takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AreaUsage {
    pub area: DataArea,
    pub bytes: u64,
}

/// The disk usage of the manager's data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DataUsage {
    /// The data directory.
    pub location: PathBuf,
    pub areas: Vec<AreaUsage>,
    /// `true` if the next start clears the webview cache.
    pub webview_clear_pending: bool,
}

/// Measures every [`DataArea`] under the data directory that `config` selects.
///
/// # Errors
///
/// Fails if the data directory cannot be resolved.
pub fn usage(
    app_handle: &AppHandle,
    config: &Config,
    library: &ModLibrary,
) -> AppResult<DataUsage> {
    let location = data_dir(app_handle, config)
        .ok_or_else(|| AppError::Other("The data directory is unavailable".to_owned()))?;

    let mut areas = vec![
        AreaUsage {
            area: DataArea::Mods,
            bytes: size_of(&location.mods_dir()) + size_of(&location.archives_dir()),
        },
        AreaUsage {
            area: DataArea::OverlayBuilds,
            bytes: size_of(&location.profiles_dir()) + size_of(&location.builtin_dir()),
        },
        AreaUsage {
            area: DataArea::ModPreviews,
            bytes: size_of(&location.join(MOD_PREVIEWS)),
        },
        AreaUsage {
            area: DataArea::Shaders,
            bytes: size_of(&location.join(SHADERS)),
        },
        AreaUsage {
            area: DataArea::Leftovers,
            bytes: library
                .overlay_leftovers(config)?
                .iter()
                .map(|path| size_of(path))
                .sum(),
        },
    ];

    if let Some(profile) = webview_profile(app_handle) {
        areas.push(AreaUsage {
            area: DataArea::WebviewCache,
            bytes: WEBVIEW_CACHE_DIRS
                .iter()
                .map(|dir| size_of(&profile.join(dir)))
                .sum(),
        });
    }

    Ok(DataUsage {
        location,
        areas,
        webview_clear_pending: webview_clear_marker(app_handle).is_some_and(|path| path.exists()),
    })
}

/// Clears `area`, which the app rebuilds when it needs it again.
///
/// [`DataArea::WebviewCache`] is cleared on the next start, because the running webview holds
/// its files open. The caller stops the overlay areas from being cleared while the patcher runs.
///
/// # Errors
///
/// Fails with [`AppError::ValidationFailed`] for [`DataArea::Mods`], and for
/// [`DataArea::WebviewCache`] off Windows. Fails with [`AppError::Io`] if a folder cannot be
/// removed or the marker cannot be written.
pub fn clear(
    app_handle: &AppHandle,
    config: &Config,
    library: &ModLibrary,
    area: DataArea,
) -> AppResult<()> {
    let location = || {
        data_dir(app_handle, config)
            .ok_or_else(|| AppError::Other("The data directory is unavailable".to_owned()))
    };

    match area {
        DataArea::Mods => Err(AppError::ValidationFailed(
            "the mod library is not a cache".to_owned(),
        )),
        DataArea::OverlayBuilds => library.clear_overlay_builds(config),
        DataArea::Leftovers => library.clear_overlay_leftovers(config),
        DataArea::ModPreviews => remove_dir(&location()?.join(MOD_PREVIEWS)),
        DataArea::Shaders => remove_dir(&location()?.join(SHADERS)),
        DataArea::WebviewCache => {
            if webview_profile(app_handle).is_none() {
                return Err(AppError::ValidationFailed(
                    "the webview cache is cleared only on Windows".to_owned(),
                ));
            }

            let marker = webview_clear_marker(app_handle).ok_or_else(|| {
                AppError::Other("The app data directory is unavailable".to_owned())
            })?;
            if let Some(parent) = marker.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&marker, b"")?;
            Ok(())
        }
    }
}

/// Returns the plugin that clears the webview cache at startup if [`clear`] asked for it.
///
/// The plugin initializes after the single-instance check and before the main window is
/// created, so no running webview holds the files. Register it after the single-instance plugin.
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("webview-cache")
        .setup(|app, _| {
            clear_requested_webview_cache(app.app_handle());
            Ok(())
        })
        .build()
}

/// Removes the webview cache folders and the marker, if the marker exists.
fn clear_requested_webview_cache<R: Runtime>(app_handle: &AppHandle<R>) {
    let Some(marker) = webview_clear_marker(app_handle) else {
        return;
    };
    if !marker.exists() {
        return;
    }

    if let Some(profile) = webview_profile(app_handle) {
        for dir in WEBVIEW_CACHE_DIRS {
            if let Err(error) = remove_dir(&profile.join(dir)) {
                tracing::warn!(%error, dir, "Could not clear a webview cache folder");
            }
        }
        tracing::info!("Cleared the webview cache");
    }

    if let Err(error) = fs::remove_file(&marker) {
        tracing::warn!(%error, "Could not remove the webview cache marker");
    }
}

/// Returns the WebView2 profile folder, or `None` off Windows.
fn webview_profile<R: Runtime>(app_handle: &AppHandle<R>) -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }

    let local = app_handle.path().app_local_data_dir().ok()?;
    Some(local.join("EBWebView").join("Default"))
}

fn webview_clear_marker<R: Runtime>(app_handle: &AppHandle<R>) -> Option<PathBuf> {
    let dir = app_handle.path().app_data_dir().ok()?;
    Some(dir.join(WEBVIEW_CLEAR_MARKER))
}

/// Removes the directory at `path`. A directory that does not exist is not an error.
fn remove_dir(path: &Path) -> AppResult<()> {
    match fs::remove_dir_all(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}
