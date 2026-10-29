//! The connected account as the settings file keeps it.

use serde::{Deserialize, Serialize};

use crate::token::ApiToken;
use crate::wire::{Account, Scope};

/// The account of the stored token. It holds nothing secret, so `settings.json` keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ConnectedAccount {
    /// The id of the user on the site.
    pub user_id: String,
    /// The user name. It is empty for an account that has not picked one.
    pub username: String,
    /// The name the site shows.
    pub display_name: Option<String>,
    /// The 8 characters after `rf_`, which the token page lists.
    pub token_prefix: String,
    /// The scopes of the token.
    pub scopes: Vec<Scope>,
}

impl ConnectedAccount {
    /// Returns the stored form of `account`, read with `token`.
    #[must_use]
    pub fn new(account: Account, token: &ApiToken) -> Self {
        Self {
            user_id: account.user_id,
            username: account.username,
            display_name: account.display_name,
            token_prefix: token.display_prefix().to_owned(),
            scopes: account.scopes,
        }
    }

    /// Returns the scopes of [`Scope::PUBLISHING`] that the token lacks.
    #[must_use]
    pub fn missing_scopes(&self) -> Vec<Scope> {
        Scope::PUBLISHING
            .into_iter()
            .filter(|scope| !self.scopes.contains(scope))
            .collect()
    }
}
