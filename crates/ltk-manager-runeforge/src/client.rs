//! The blocking HTTP client of the RuneForge `/api/v1` routes.

use std::io::{self, Read};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use fs_err as fs;
use reqwest::blocking::{Body, RequestBuilder, Response};
use reqwest::header::{
    ACCEPT, AUTHORIZATION, CONTENT_DISPOSITION, CONTENT_TYPE, HeaderMap, HeaderValue,
};
use reqwest::{Method, Url, redirect};
use serde::Serialize;
use serde::de::{DeserializeOwned, IgnoredAny};

use crate::RuneforgeError;
use crate::token::ApiToken;
use crate::wire::{
    Account, CreatedMod, CreatedRelease, CreatedUpload, NewMod, NewRelease, NewUpload,
    OwnedModPost, OwnedMods, Prefill, PublishAnswer, PublishingContext, Release, UploadReview,
};

/// The site that a release build publishes to.
pub const DEFAULT_BASE_URL: &str = "https://runeforge.dev";

/// The environment variable that points a debug build at another site, such as a local one.
pub const BASE_URL_VARIABLE: &str = "LTK_RUNEFORGE_URL";

/// The site path where a creator makes and revokes tokens.
pub const TOKEN_PAGE_PATH: &str = "/user-settings/api-tokens";

/// How long a connection to the API or to storage may take to open.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// How long one API call may take.
const API_TIMEOUT: Duration = Duration::from_secs(60);

/// How long the PUT of the package bytes may take: the lifetime of the presigned URL.
const PUT_TIMEOUT: Duration = Duration::from_secs(60 * 60);

/// How many redirects a request follows, each to the host that it was sent to.
const MAX_REDIRECTS: usize = 10;

/// The header that makes a create safe to retry.
const IDEMPOTENCY_KEY: &str = "Idempotency-Key";

/// Returns the base URL of the site that the app publishes to.
///
/// A debug build reads [`BASE_URL_VARIABLE`] and falls back to [`DEFAULT_BASE_URL`]. A release
/// build always returns [`DEFAULT_BASE_URL`].
#[must_use]
pub fn base_url() -> Url {
    let configured = if cfg!(debug_assertions) {
        std::env::var(BASE_URL_VARIABLE)
            .ok()
            .and_then(|text| Url::parse(&text).ok())
            .filter(|url| matches!(url.scheme(), "http" | "https"))
    } else {
        None
    };

    configured
        .unwrap_or_else(|| Url::parse(DEFAULT_BASE_URL).expect("DEFAULT_BASE_URL is a valid URL"))
}

/// A connection to the RuneForge API as one account, through its token.
///
/// The client sends the token to the API host only. The PUT of the package bytes goes to a
/// presigned storage URL without it, and a redirect is followed only to the host that the request
/// was sent to.
#[derive(Debug)]
pub struct Client {
    http: reqwest::blocking::Client,
    base: Url,
    token: ApiToken,
}

impl Client {
    /// Builds a client for the site at `base` that authenticates with `token`.
    ///
    /// `version` names the running app in the `User-Agent`.
    ///
    /// # Errors
    ///
    /// Fails with [`RuneforgeError::Offline`] if the TLS backend cannot be set up.
    pub fn new(base: Url, token: ApiToken, version: &str) -> Result<Self, RuneforgeError> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let http = reqwest::blocking::Client::builder()
            .user_agent(format!("ltk-manager/{version}"))
            .default_headers(headers)
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(None)
            .redirect(redirect::Policy::custom(same_host_redirect))
            .build()
            .map_err(|error| RuneforgeError::transport(&error))?;

        Ok(Self { http, base, token })
    }

    /// Returns the base URL of the site.
    #[must_use]
    pub fn base(&self) -> &Url {
        &self.base
    }

    /// Returns the token that the client authenticates with.
    #[must_use]
    pub fn token(&self) -> &ApiToken {
        &self.token
    }

    /// Reads the account that the token acts for.
    ///
    /// # Errors
    ///
    /// Fails with [`RuneforgeError::Unauthorized`] if the server refuses the token, and with a
    /// transport variant if the server is not reached.
    pub fn me(&self) -> Result<Account, RuneforgeError> {
        self.call(self.api(Method::GET, &["me"]))
    }

    /// Reads the posts of the account, most recently updated first.
    ///
    /// # Errors
    ///
    /// Fails with [`RuneforgeError::Forbidden`] if the token lacks `mods:read`.
    pub fn owned_mods(&self) -> Result<Vec<OwnedModPost>, RuneforgeError> {
        let owned: OwnedMods = self.call(self.api(Method::GET, &["me", "mods"]))?;
        Ok(owned.mods)
    }

    /// Reads the game versions and categories that a release and a new post may take.
    ///
    /// # Errors
    ///
    /// Fails with [`RuneforgeError::Forbidden`] if the token lacks `mods:read`.
    pub fn publishing_context(&self) -> Result<PublishingContext, RuneforgeError> {
        self.call(self.api(Method::GET, &["publishing-context"]))
    }

    /// Creates an upload for a package named `filename` of `size` bytes.
    ///
    /// # Errors
    ///
    /// Fails with [`RuneforgeError::TooLarge`] if `size` is over the 1 GiB limit, and with
    /// [`RuneforgeError::Forbidden`] if the token lacks `releases:write`.
    pub fn create_upload(
        &self,
        filename: &str,
        size: u64,
    ) -> Result<CreatedUpload, RuneforgeError> {
        let body = NewUpload { filename, size };
        self.call(self.api(Method::POST, &["uploads"]).json(&body))
    }

    /// Sends the bytes of the file at `path` to the presigned URL of `upload`.
    ///
    /// Streams the file and calls `progress` with the bytes sent so far. Sends the signed headers
    /// of `upload` and no `Authorization`. Stops when `cancel` is set.
    ///
    /// # Errors
    ///
    /// Fails with [`RuneforgeError::Cancelled`] if `cancel` is set during the send, with
    /// [`RuneforgeError::Storage`] if storage refuses the bytes, and with
    /// [`RuneforgeError::Offline`] if the file cannot be read or storage is not reached.
    pub fn put_package(
        &self,
        upload: &CreatedUpload,
        path: &Path,
        progress: impl FnMut(u64) + Send + 'static,
        cancel: Arc<AtomicBool>,
    ) -> Result<(), RuneforgeError> {
        let file = fs::File::open(path).map_err(|error| RuneforgeError::Offline {
            detail: error.to_string(),
        })?;
        let length = file
            .metadata()
            .map_err(|error| RuneforgeError::Offline {
                detail: error.to_string(),
            })?
            .len();

        let url = Url::parse(&upload.presigned_url).map_err(|error| RuneforgeError::Malformed {
            detail: error.to_string(),
        })?;
        let body = CountingReader {
            inner: file,
            sent: 0,
            progress,
            cancel: Arc::clone(&cancel),
        };

        let sent = self
            .http
            .put(url)
            .header(CONTENT_TYPE, &upload.put_headers.content_type)
            .header(CONTENT_DISPOSITION, &upload.put_headers.content_disposition)
            .body(Body::sized(body, length))
            .timeout(PUT_TIMEOUT)
            .send();

        if cancel.load(Ordering::Relaxed) {
            return Err(RuneforgeError::Cancelled);
        }

        let response = sent.map_err(|error| RuneforgeError::transport(&error))?;
        let status = response.status();
        if !status.is_success() {
            return Err(RuneforgeError::Storage {
                status: status.as_u16(),
            });
        }

        Ok(())
    }

    /// Marks the upload `upload_id` complete, and reads what the server inspected in it.
    ///
    /// # Errors
    ///
    /// Fails with [`RuneforgeError::PackageRejected`] if the server refuses the package.
    pub fn complete_upload(&self, upload_id: &str) -> Result<UploadReview, RuneforgeError> {
        self.call(self.api(Method::POST, &["uploads", upload_id, "complete"]))
    }

    /// Reads the prefill of the upload `upload_id` for the post `mod_id`, or for a new post if
    /// `mod_id` is `None`.
    ///
    /// # Errors
    ///
    /// Fails with [`RuneforgeError::NotFound`] if the account owns no such post or upload.
    pub fn prefill(
        &self,
        upload_id: &str,
        mod_id: Option<&str>,
    ) -> Result<Prefill, RuneforgeError> {
        let mut request = self.api(Method::GET, &["uploads", upload_id, "prefill"]);
        if let Some(mod_id) = mod_id {
            request = request.query(&[("modId", mod_id)]);
        }

        self.call(request)
    }

    /// Deletes the upload `upload_id`, which no release has attached.
    ///
    /// # Errors
    ///
    /// Fails with [`RuneforgeError::UploadState`] if a release has attached the upload.
    pub fn discard_upload(&self, upload_id: &str) -> Result<(), RuneforgeError> {
        self.send(self.api(Method::DELETE, &["uploads", upload_id]))
            .map(drop)
    }

    /// Creates a private, unpublished post with its first release from `new_mod`.
    ///
    /// Sends the `Idempotency-Key` of the first upload, so a retry creates nothing twice.
    ///
    /// # Errors
    ///
    /// Fails with [`RuneforgeError::Forbidden`] if the token lacks `mods:write`.
    pub fn create_mod(&self, new_mod: &NewMod) -> Result<CreatedMod, RuneforgeError> {
        self.call(self.create(&["mods"], &new_mod.upload_ids, new_mod))
    }

    /// Creates a release of the post `mod_id` from `release`.
    ///
    /// Sends the `Idempotency-Key` of the first upload, so a retry creates nothing twice.
    ///
    /// # Errors
    ///
    /// Fails with [`RuneforgeError::TagTaken`] if the post already has the tag.
    pub fn create_release(
        &self,
        mod_id: &str,
        release: &NewRelease,
    ) -> Result<CreatedRelease, RuneforgeError> {
        self.call(self.create(&["mods", mod_id, "releases"], &release.upload_ids, release))
    }

    /// Publishes the post `mod_id`, with `answer` to the AI content question.
    ///
    /// Succeeds for a post that is already public.
    ///
    /// # Errors
    ///
    /// Fails with [`RuneforgeError::PublishRefused`] if the post lacks something that publishing
    /// needs, and with [`RuneforgeError::Forbidden`] if the token lacks `publish`.
    pub fn publish(&self, mod_id: &str, answer: &PublishAnswer) -> Result<(), RuneforgeError> {
        self.call::<IgnoredAny>(
            self.api(Method::POST, &["mods", mod_id, "publish"])
                .json(answer),
        )
        .map(drop)
    }

    /// Reads the release `release_id` of the post `mod_id`, with the scan result of each file.
    ///
    /// # Errors
    ///
    /// Fails with [`RuneforgeError::NotFound`] if the account cannot see the release.
    pub fn release(&self, mod_id: &str, release_id: &str) -> Result<Release, RuneforgeError> {
        self.call(self.api(Method::GET, &["mods", mod_id, "releases", release_id]))
    }

    /// Returns the site URL of `path`, such as the `reviewPath` of a checklist.
    #[must_use]
    pub fn site_url(&self, path: &str) -> Url {
        site_url(&self.base, path)
    }

    /// Starts an authenticated request to `/api/v1/<segments>`.
    fn api(&self, method: Method, segments: &[&str]) -> RequestBuilder {
        let mut url = self.base.clone();
        url.path_segments_mut()
            .expect("an http base URL has path segments")
            .pop_if_empty()
            .extend(["api", "v1"])
            .extend(segments);

        self.http
            .request(method, url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token.expose()))
            .timeout(API_TIMEOUT)
    }

    /// Starts a create that is safe to retry, keyed on the first of `upload_ids`.
    fn create(
        &self,
        segments: &[&str],
        upload_ids: &[String],
        body: &impl Serialize,
    ) -> RequestBuilder {
        let mut request = self.api(Method::POST, segments).json(body);
        if let Some(upload_id) = upload_ids.first() {
            request = request.header(IDEMPOTENCY_KEY, idempotency_key(upload_id));
        }

        request
    }

    /// Sends `request` and reads its JSON answer.
    fn call<T: DeserializeOwned>(&self, request: RequestBuilder) -> Result<T, RuneforgeError> {
        self.send(request)?
            .json()
            .map_err(|error| RuneforgeError::Malformed {
                detail: error.to_string(),
            })
    }

    /// Sends `request`, and returns its answer if the status is a success.
    fn send(&self, request: RequestBuilder) -> Result<Response, RuneforgeError> {
        let response = request
            .send()
            .map_err(|error| RuneforgeError::transport(&error))?;

        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }

        let headers = response.headers().clone();
        let body = response
            .bytes()
            .map_err(|error| RuneforgeError::transport(&error))?;
        Err(RuneforgeError::from_answer(status, &headers, &body))
    }
}

/// Returns the URL of the site path `path` under `base`.
#[must_use]
pub fn site_url(base: &Url, path: &str) -> Url {
    let mut url = base.clone();
    url.set_path(path);
    url.set_query(None);
    url
}

/// Returns the `Idempotency-Key` of a create from the upload `upload_id`.
///
/// An upload attaches to one post or release only, so the key is the same on every retry of one
/// review and needs no state.
#[must_use]
pub fn idempotency_key(upload_id: &str) -> String {
    format!("ltk-manager-{upload_id}")
}

/// Follows a redirect only to the host that the request was sent to.
fn same_host_redirect(attempt: redirect::Attempt<'_>) -> redirect::Action {
    let same_host = attempt
        .previous()
        .last()
        .is_some_and(|previous| previous.host_str() == attempt.url().host_str());

    if same_host && attempt.previous().len() < MAX_REDIRECTS {
        attempt.follow()
    } else {
        attempt.stop()
    }
}

/// A file reader that reports the bytes read so far, and fails once the publish is cancelled.
struct CountingReader<F> {
    inner: fs::File,
    sent: u64,
    progress: F,
    cancel: Arc<AtomicBool>,
}

impl<F: FnMut(u64)> Read for CountingReader<F> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err(io::Error::other("the publish was cancelled"));
        }

        let read = self.inner.read(buf)?;
        self.sent += read as u64;
        (self.progress)(self.sent);

        Ok(read)
    }
}

#[cfg(test)]
mod tests;
