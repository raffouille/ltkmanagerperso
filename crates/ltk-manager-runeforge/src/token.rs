//! The personal API token that a RuneForge account issues, and the credential store entry that keeps it.

use std::fmt;
use std::sync::Arc;

use keyring_core::{CredentialStore, Entry};

use crate::RuneforgeError;

/// The text every RuneForge token starts with.
const PREFIX: &str = "rf_";

/// How many characters after [`PREFIX`] identify a token on the site's token page.
const DISPLAY_PREFIX_LEN: usize = 8;

/// The credential store service that holds the token.
const SERVICE: &str = "ltk-manager/runeforge";

/// The credential store account that holds the token. The app keeps one connection.
const ACCOUNT: &str = "default";

/// A RuneForge API token, `rf_` followed by URL-safe base64.
///
/// `Debug` and `Display` print the prefix that the token page lists and nothing more. The type
/// does not implement `Serialize`, so the secret reaches no settings file, IPC payload or log.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiToken(String);

impl ApiToken {
    /// Parses `text` as a token, trimmed of surrounding whitespace.
    ///
    /// # Errors
    ///
    /// Fails with [`TokenError`] if the text does not start with `rf_`, has nothing after it, or
    /// holds a character outside the URL-safe base64 alphabet.
    pub fn new(text: &str) -> Result<Self, TokenError> {
        let text = text.trim();
        let Some(body) = text.strip_prefix(PREFIX) else {
            return Err(TokenError::MissingPrefix);
        };

        if body.len() < DISPLAY_PREFIX_LEN {
            return Err(TokenError::TooShort);
        }

        if !body
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(TokenError::InvalidCharacter);
        }

        Ok(Self(text.to_owned()))
    }

    /// Returns the 8 characters after `rf_`, which the token page lists to tell tokens apart.
    #[must_use]
    pub fn display_prefix(&self) -> &str {
        &self.0[PREFIX.len()..PREFIX.len() + DISPLAY_PREFIX_LEN]
    }

    /// Returns the whole secret, for the `Authorization` header and the credential store only.
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ApiToken({self})")
    }
}

impl fmt::Display for ApiToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{PREFIX}{}...", self.display_prefix())
    }
}

/// Why a text is not a RuneForge token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum TokenError {
    /// The text does not start with `rf_`.
    #[error("a RuneForge token starts with rf_")]
    MissingPrefix,
    /// Fewer than 8 characters follow `rf_`.
    #[error("the token is too short")]
    TooShort,
    /// A character is outside the URL-safe base64 alphabet.
    #[error("the token holds a character that a RuneForge token cannot hold")]
    InvalidCharacter,
}

/// The credential store entry that keeps the token of the connected account.
pub struct TokenStore {
    store: Arc<CredentialStore>,
}

impl TokenStore {
    /// Opens the token entry in the native credential store of the platform.
    ///
    /// # Errors
    ///
    /// Fails with [`RuneforgeError::Credential`] if the store cannot be opened, or on a platform
    /// that has no native store wired in. Only Windows has one.
    pub fn native() -> Result<Self, RuneforgeError> {
        #[cfg(windows)]
        {
            let store = windows_native_keyring_store::Store::new().map_err(credential_error)?;
            Ok(Self::new(store))
        }

        #[cfg(not(windows))]
        {
            Err(RuneforgeError::Credential {
                detail: "this platform has no credential store for the RuneForge token".to_owned(),
            })
        }
    }

    /// Opens the token entry in `store`.
    #[must_use]
    pub fn new(store: Arc<CredentialStore>) -> Self {
        Self { store }
    }

    /// Reads the stored token. Returns `None` if no token is stored.
    ///
    /// # Errors
    ///
    /// Fails with [`RuneforgeError::Credential`] if the store cannot be read, or if the stored
    /// text is not a token.
    pub fn load(&self) -> Result<Option<ApiToken>, RuneforgeError> {
        match self.entry()?.get_password() {
            Ok(text) => {
                ApiToken::new(&text)
                    .map(Some)
                    .map_err(|error| RuneforgeError::Credential {
                        detail: error.to_string(),
                    })
            }
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(error) => Err(credential_error(error)),
        }
    }

    /// Writes `token` over any stored token.
    ///
    /// # Errors
    ///
    /// Fails with [`RuneforgeError::Credential`] if the store refuses the write.
    pub fn save(&self, token: &ApiToken) -> Result<(), RuneforgeError> {
        self.entry()?
            .set_password(token.expose())
            .map_err(credential_error)
    }

    /// Deletes the stored token. Deleting a missing token succeeds.
    ///
    /// # Errors
    ///
    /// Fails with [`RuneforgeError::Credential`] if the store refuses the delete.
    pub fn delete(&self) -> Result<(), RuneforgeError> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(error) => Err(credential_error(error)),
        }
    }

    fn entry(&self) -> Result<Entry, RuneforgeError> {
        self.store
            .build(SERVICE, ACCOUNT, None)
            .map_err(credential_error)
    }
}

impl fmt::Debug for TokenStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenStore").finish_non_exhaustive()
    }
}

fn credential_error(error: keyring_core::Error) -> RuneforgeError {
    RuneforgeError::Credential {
        detail: error.to_string(),
    }
}

#[cfg(test)]
mod tests;
