//! Connecting a RuneForge account by its token, and what the Settings section reads of it.

use std::sync::Arc;

use ltk_manager_runeforge::{
    base_url, site_url, ApiToken, Client, ConnectedAccount, OwnedModPost, PublishingContext,
    TokenStore, TOKEN_PAGE_PATH,
};
use serde::Serialize;
use tauri::{AppHandle, Manager};

use super::state;
use crate::error::{AppError, AppResult, IpcResult};
use crate::running_version;
use crate::services::shared::off_thread;
use crate::state::{persist_settings, SettingsState};

/// The RuneForge connection as the Settings section shows it.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RuneforgeStatus {
    /// The connected account, or `None` while disconnected.
    pub account: Option<ConnectedAccount>,
    /// The server refused the stored token during this session.
    pub rejected: bool,
    /// The base URL of the site.
    pub site_url: String,
    /// The site page where the user makes and revokes tokens.
    pub token_page_url: String,
}

/// Returns the stored account and the state of the connection. Sends no request.
#[tauri::command]
#[specta::specta]
pub fn runeforge_status(app_handle: AppHandle) -> IpcResult<RuneforgeStatus> {
    IpcResult::ok(status(&app_handle))
}

/// Checks `token` with the server, stores it in the credential store and returns the account.
///
/// Stores nothing if the server refuses the token.
#[tauri::command]
#[specta::specta]
pub async fn runeforge_connect(token: String, app_handle: AppHandle) -> IpcResult<RuneforgeStatus> {
    off_thread(move || {
        let token =
            ApiToken::new(&token).map_err(|error| AppError::ValidationFailed(error.to_string()))?;
        let client = Client::new(base_url(), token, &running_version().to_string())?;
        let account = ConnectedAccount::new(client.me()?, client.token());

        TokenStore::native()?.save(client.token())?;
        state(&app_handle).replace_client(Some(Arc::new(client)));
        write_account(&app_handle, Some(account))?;

        tracing::info!("Connected a RuneForge account");
        Ok(status(&app_handle))
    })
    .await
}

/// Deletes the stored token and the account settings.
#[tauri::command]
#[specta::specta]
pub async fn runeforge_disconnect(app_handle: AppHandle) -> IpcResult<RuneforgeStatus> {
    off_thread(move || {
        TokenStore::native()?.delete()?;
        state(&app_handle).replace_client(None);
        write_account(&app_handle, None)?;

        tracing::info!("Disconnected the RuneForge account");
        Ok(status(&app_handle))
    })
    .await
}

/// Returns the posts of the account, most recently updated first.
#[tauri::command]
#[specta::specta]
pub async fn runeforge_owned_mods(app_handle: AppHandle) -> IpcResult<Vec<OwnedModPost>> {
    off_thread(move || state(&app_handle).call(Client::owned_mods)).await
}

/// Returns the game versions and the categories that a release and a new post may take.
#[tauri::command]
#[specta::specta]
pub async fn runeforge_context(app_handle: AppHandle) -> IpcResult<PublishingContext> {
    off_thread(move || state(&app_handle).call(Client::publishing_context)).await
}

fn status(app: &AppHandle) -> RuneforgeStatus {
    let base = base_url();

    RuneforgeStatus {
        account: app.state::<SettingsState>().0.lock().runeforge.clone(),
        rejected: state(app).is_rejected(),
        token_page_url: site_url(&base, TOKEN_PAGE_PATH).to_string(),
        site_url: base.to_string(),
    }
}

/// Writes `account` to the settings and to `settings.json`.
fn write_account(app: &AppHandle, account: Option<ConnectedAccount>) -> AppResult<()> {
    let state = app.state::<SettingsState>();
    let settings = {
        let mut settings = state.0.lock();
        settings.runeforge = account;
        settings.clone()
    };

    persist_settings(app, &settings)?;
    Ok(())
}
