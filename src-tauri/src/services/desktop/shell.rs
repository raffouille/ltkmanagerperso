use std::path::PathBuf;

use ltk_manager_assets::game_index::GameIndexState;
use ltk_manager_assets::strings::StringKeyIndexState;
use ltk_manager_problems::BinNames;
use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult, IpcResult};
use crate::mods::ModLibraryState;
use crate::services::objects::index::clear_indexes;
use crate::state::SettingsState;

/// Opens a file location in the system file explorer.
#[tauri::command]
#[specta::specta]
pub async fn reveal_in_explorer(path: String) -> IpcResult<()> {
    reveal_in_explorer_inner(&path).into()
}

pub(crate) fn reveal_in_explorer_inner(path: &str) -> AppResult<()> {
    let path = PathBuf::from(path);

    // Get the parent directory if it's a file
    let dir = if path.is_file() {
        path.parent().map(|p| p.to_path_buf()).unwrap_or(path)
    } else {
        path
    };

    #[cfg(target_os = "windows")]
    {
        let dir_str = dir.to_string_lossy().replace('/', "\\");
        std::process::Command::new("explorer")
            .arg(dir_str)
            .spawn()
            .map_err(|e| AppError::Other(format!("Failed to open explorer: {}", e)))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(dir)
            .spawn()
            .map_err(|e| AppError::Other(format!("Failed to open Finder: {}", e)))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| AppError::Other(format!("Failed to open file manager: {}", e)))?;
    }

    Ok(())
}

/// Minimizes the window to the system tray if the setting is enabled,
/// otherwise performs a regular minimize.
#[tauri::command]
#[specta::specta]
pub async fn minimize_to_tray(window: tauri::WebviewWindow) -> IpcResult<()> {
    minimize_to_tray_inner(window).into()
}

fn minimize_to_tray_inner(window: tauri::WebviewWindow) -> AppResult<()> {
    let to_tray = window.state::<SettingsState>().0.lock().minimize_to_tray;

    if to_tray {
        send_to_tray(&window)?;
    } else {
        window
            .minimize()
            .map_err(|e| AppError::Other(format!("Failed to minimize window: {}", e)))?;
    }

    Ok(())
}

/// Hides `window` and its webview, and drops the indexes that are rebuilt on next use.
pub(crate) fn send_to_tray(window: &tauri::WebviewWindow) -> AppResult<()> {
    window
        .hide()
        .map_err(|e| AppError::Other(format!("Failed to hide window: {}", e)))?;
    hide_webview(window).map_err(|e| AppError::Other(format!("Failed to hide webview: {}", e)))?;

    release_indexes(window.app_handle());
    Ok(())
}

/// Drops the indexes of the installed game, which the next reader rebuilds.
///
/// The frontend warms the object index again when the webview is shown.
fn release_indexes(app: &AppHandle) {
    clear_indexes(app);
    app.state::<GameIndexState>().clear_all();
    app.state::<StringKeyIndexState>().clear();
    BinNames::invalidate_game_index();
    app.state::<ModLibraryState>().0.release_game_content();
}

/// Hides the webview of `window` and lowers its memory target.
///
/// Hiding the window alone keeps the WebView2 controller visible, so Chromium keeps its
/// rendering memory.
fn hide_webview(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    window.as_ref().hide()?;
    set_low_memory_target(window, true)
}

/// Shows the webview of `window` and restores its normal memory target.
pub(crate) fn show_webview(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    set_low_memory_target(window, false)?;
    window.as_ref().show()
}

#[cfg(windows)]
fn set_low_memory_target(window: &tauri::WebviewWindow, low: bool) -> tauri::Result<()> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
    };
    use windows_core::Interface;

    let level = if low {
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
    } else {
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
    };

    window.with_webview(move |webview| {
        // SAFETY: `with_webview` runs on the thread that owns the controller.
        let set = unsafe {
            webview
                .controller()
                .CoreWebView2()
                .and_then(|core| core.cast::<ICoreWebView2_19>())
                .and_then(|core| core.SetMemoryUsageTargetLevel(level))
        };
        if let Err(e) = set {
            tracing::debug!("Could not set the WebView2 memory target: {e}");
        }
    })
}

#[cfg(not(windows))]
fn set_low_memory_target(_window: &tauri::WebviewWindow, _low: bool) -> tauri::Result<()> {
    Ok(())
}
