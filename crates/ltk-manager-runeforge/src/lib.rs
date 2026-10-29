//! Publishes mod packages to RuneForge through its token API.
//!
//! [`Client`] calls the `/api/v1` routes as one account, [`ApiToken`] is the secret it
//! authenticates with, and [`TokenStore`] keeps the token in the OS credential store. The routes
//! and their contract are in `docs/PUBLIC_API.md` of `runeforge-io/runeforge`.

mod account;
mod choice;
mod client;
mod error;
mod token;
mod wire;

pub use account::ConnectedAccount;
pub use choice::{SITE_ID, SITE_NAME, Target, TargetChoice, choose_target};
pub use client::{
    BASE_URL_VARIABLE, Client, DEFAULT_BASE_URL, TOKEN_PAGE_PATH, base_url, idempotency_key,
    site_url,
};
pub use error::RuneforgeError;
pub use reqwest::Url;
pub use token::{ApiToken, TokenError, TokenStore};
pub use wire::*;
