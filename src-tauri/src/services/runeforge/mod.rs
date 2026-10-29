//! The runeforge service: the connected RuneForge account and the publish of a project to it.

mod account;
mod publish;

pub use account::*;
pub use publish::*;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ltk_manager_runeforge::{base_url, Client, RuneforgeError, TokenStore};
use parking_lot::Mutex;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Manager, Wry};

use crate::error::AppResult;
use crate::running_version;

/// The runeforge service's row of `services/table.rs`.
pub struct Table;

/// The plugin answering the runeforge commands.
pub fn plugin() -> TauriPlugin<Wry> {
    super::plugin::<Table>().build()
}

/// The client of the stored token, the session state of the connection and the prepares in flight.
#[derive(Default)]
pub struct RuneforgeState {
    client: Mutex<Option<Arc<Client>>>,
    rejected: AtomicBool,
    prepares: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl RuneforgeState {
    /// Returns the client of the stored token, built on first use.
    ///
    /// Reads the credential store only when no client is cached, so the app sends no RuneForge
    /// request and opens no credential until the user connects.
    fn client(&self) -> AppResult<Arc<Client>> {
        let mut cached = self.client.lock();
        if let Some(client) = cached.as_ref() {
            return Ok(Arc::clone(client));
        }

        let token = TokenStore::native()?
            .load()?
            .ok_or(RuneforgeError::NotConnected)?;
        let client = Arc::new(Client::new(
            base_url(),
            token,
            &running_version().to_string(),
        )?);

        *cached = Some(Arc::clone(&client));
        Ok(client)
    }

    /// Replaces the cached client, after a connect or a disconnect.
    fn replace_client(&self, client: Option<Arc<Client>>) {
        *self.client.lock() = client;
        self.rejected.store(false, Ordering::Relaxed);
    }

    /// Returns `true` if the server refused the token during this session.
    fn is_rejected(&self) -> bool {
        self.rejected.load(Ordering::Relaxed)
    }

    /// Runs `call` with the client, and marks the connection rejected if the server refuses the
    /// token.
    fn call<T>(&self, call: impl FnOnce(&Client) -> Result<T, RuneforgeError>) -> AppResult<T> {
        let client = self.client()?;
        let result = call(&client);

        if let Err(error) = &result {
            if error.is_unauthorized() {
                self.rejected.store(true, Ordering::Relaxed);
            }
        }

        Ok(result?)
    }
}

/// Returns the managed [`RuneforgeState`] of `app`.
fn state(app: &AppHandle) -> tauri::State<'_, RuneforgeState> {
    app.state::<RuneforgeState>()
}
