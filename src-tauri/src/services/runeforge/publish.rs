//! The publish of a workshop project to RuneForge: pack, upload, review, create and stamp.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use fs_err as fs;
use ltk_manager_base::events::{
    BackendEvent, EventSink, RuneforgePublishProgress, RuneforgePublishStage,
};
use ltk_manager_runeforge::{
    choose_target, site_url, Client, CreatedRelease, NewMod, NewRelease, OwnedModPost, Prefill,
    PublishAnswer, PublishChecklist, Release, RuneforgeError, Target, TargetChoice, UploadReview,
    Url, SITE_ID, SITE_NAME,
};
use ltk_mod_project::ModProjectDistributor;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use super::{state, RuneforgeState};
use crate::error::{AppError, AppResult, IpcResult};
use crate::events::TauriEventSink;
use crate::services::shared::off_thread;
use crate::workshop::{PackFormat, PackProjectArgs, WorkshopState};

/// Shortest time between two progress events of one publish.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// What the server read from a packed and uploaded project, and the post it targets first.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PublishReview {
    /// The project directory that was packed.
    pub project_path: String,
    /// The file name of the package.
    pub file_name: String,
    /// The size of the package in bytes.
    pub size: u64,
    /// The validation warnings of the project.
    pub warnings: Vec<String>,
    /// The inspection, the findings and the prefill for the target of `choice`.
    pub review: UploadReview,
    /// The post that the dialog selects first.
    pub choice: TargetChoice,
    /// The posts of the account, for the target picker.
    pub owned: Vec<OwnedModPost>,
}

/// What the publish dialog asks the server to create.
#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PublishRequest {
    /// A new post with its first release, published after the create if `publish` is set.
    NewMod {
        new_mod: NewMod,
        publish: Option<PublishAnswer>,
    },
    /// A release of the post `mod_id`.
    Release { mod_id: String, release: NewRelease },
}

/// What a submit created, and how the steps after the create went.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PublishOutcome {
    /// The id of the post.
    pub mod_id: String,
    /// The id of the release.
    pub release_id: String,
    /// The page of the post on the site.
    pub mod_url: String,
    /// The result of the publish call, or `None` if none was asked for.
    pub publish: Option<PublishResult>,
    /// The result of the stamp write.
    pub stamp: StampResult,
    /// Why the release did not refresh every post field that it was asked to.
    pub post_update_error: Option<String>,
}

/// The result of a publish call.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PublishResult {
    /// The post is public.
    Published,
    /// The post lacks what `checklist` lists. `review_url` is the page that requests a review.
    Refused {
        checklist: PublishChecklist,
        review_url: Option<String>,
    },
    /// The publish call failed with `error`. The post and the release exist.
    Failed { error: RuneforgeError },
}

/// The result of writing the distributor stamp into the project.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum StampResult {
    /// The project config names the post.
    Written,
    /// The write failed with `detail`. The release exists.
    Failed { detail: String },
}

/// Validates and packs the project at `project_path`, uploads the package and returns what the
/// server read from it.
///
/// Progress arrives as `runeforge-publish-progress`. [`runeforge_cancel`] stops the run, and a
/// cancel after the upload exists deletes it.
#[tauri::command]
#[specta::specta]
pub async fn runeforge_prepare(
    project_path: String,
    format: PackFormat,
    app_handle: AppHandle,
) -> IpcResult<PublishReview> {
    off_thread(move || {
        let cancel = claim(&app_handle, &project_path)?;
        let mut progress = Progress::new(&app_handle, &project_path);

        let result = prepare(&app_handle, &project_path, format, &cancel, &mut progress);
        release_claim(&app_handle, &project_path);

        match &result {
            Ok(review) => progress.emit(RuneforgePublishStage::Complete, review.size, review.size),
            Err(_) => progress.emit(RuneforgePublishStage::Error, 0, 0),
        }

        result
    })
    .await
}

/// Cancels the prepare in flight for `project_path`. Returns `false` if none runs.
#[tauri::command]
#[specta::specta]
pub fn runeforge_cancel(project_path: String, app_handle: AppHandle) -> IpcResult<bool> {
    let runeforge = state(&app_handle);
    let prepares = runeforge.prepares.lock();
    let Some(cancel) = prepares.get(&project_path) else {
        return IpcResult::ok(false);
    };

    cancel.store(true, Ordering::Relaxed);
    IpcResult::ok(true)
}

/// Returns the prefill of the upload `upload_id` for the post `mod_id`, or for a new post.
#[tauri::command]
#[specta::specta]
pub async fn runeforge_prefill(
    upload_id: String,
    mod_id: Option<String>,
    app_handle: AppHandle,
) -> IpcResult<Prefill> {
    off_thread(move || {
        state(&app_handle).call(|client| client.prefill(&upload_id, mod_id.as_deref()))
    })
    .await
}

/// Deletes the upload `upload_id`, which the dialog closed without submitting.
#[tauri::command]
#[specta::specta]
pub async fn runeforge_discard(upload_id: String, app_handle: AppHandle) -> IpcResult<()> {
    off_thread(move || state(&app_handle).call(|client| client.discard_upload(&upload_id))).await
}

/// Creates the post or the release of `request`, publishes a new post if asked, and stamps the
/// project at `project_path` with the post.
///
/// A retry of the same request creates nothing twice, because the create sends the
/// `Idempotency-Key` of the upload.
#[tauri::command]
#[specta::specta]
pub async fn runeforge_submit(
    project_path: String,
    request: PublishRequest,
    app_handle: AppHandle,
) -> IpcResult<PublishOutcome> {
    off_thread(move || {
        let runeforge = state(&app_handle);

        let (created, publish) = match request {
            PublishRequest::NewMod { new_mod, publish } => {
                let created = runeforge.call(|client| client.create_mod(&new_mod))?;
                (Created::from(created), publish)
            }
            PublishRequest::Release { mod_id, release } => {
                let created = runeforge.call(|client| client.create_release(&mod_id, &release))?;
                (Created::from(created), None)
            }
        };

        let client = runeforge.client()?;
        let stamp = write_stamp(&app_handle, &project_path, client.base(), &created.mod_id);
        let publish = publish.map(|answer| publish_result(&runeforge, &created.mod_id, &answer));

        tracing::info!(mod_id = %created.mod_id, release_id = %created.release_id, "Published a release to RuneForge");
        Ok(PublishOutcome {
            mod_url: site_url(client.base(), &format!("/mods/{}", created.mod_id)).to_string(),
            mod_id: created.mod_id,
            release_id: created.release_id,
            publish,
            stamp,
            post_update_error: created.post_update_error,
        })
    })
    .await
}

/// Publishes the post `mod_id` with `answer` to the AI content question.
#[tauri::command]
#[specta::specta]
pub async fn runeforge_publish(
    mod_id: String,
    answer: PublishAnswer,
    app_handle: AppHandle,
) -> IpcResult<PublishResult> {
    off_thread(move || Ok(publish_result(&state(&app_handle), &mod_id, &answer))).await
}

/// Returns the release `release_id` of the post `mod_id`, with the scan result of each file.
#[tauri::command]
#[specta::specta]
pub async fn runeforge_release(
    mod_id: String,
    release_id: String,
    app_handle: AppHandle,
) -> IpcResult<Release> {
    off_thread(move || state(&app_handle).call(|client| client.release(&mod_id, &release_id))).await
}

fn prepare(
    app: &AppHandle,
    project_path: &str,
    format: PackFormat,
    cancel: &Arc<AtomicBool>,
    progress: &mut Progress,
) -> AppResult<PublishReview> {
    let runeforge = state(app);
    let workshop = &app.state::<WorkshopState>().0;

    progress.emit(RuneforgePublishStage::Validating, 0, 0);
    let validation = workshop.validate_project(project_path)?;
    if !validation.valid {
        return Err(AppError::ValidationFailed(validation.errors.join("\n")));
    }

    let stamp = workshop
        .project(project_path)?
        .config()?
        .distributor
        .filter(|distributor| distributor.site_id == SITE_ID)
        .map(|distributor| distributor.mod_id);

    check_cancel(cancel)?;
    progress.emit(RuneforgePublishStage::Packing, 0, 0);
    let packed = workshop.pack_project(PackProjectArgs {
        project_path: project_path.to_owned(),
        output_dir: None,
        format,
    })?;
    let size = fs::metadata(&packed.output_path)?.len();

    check_cancel(cancel)?;
    let created = runeforge.call(|client| client.create_upload(&packed.file_name, size))?;
    let upload_id = created.upload.id.clone();

    /* Every failure after the create leaves an upload that no review reaches, so each one
    deletes it. */
    let uploaded = (|| -> AppResult<(UploadReview, Vec<OwnedModPost>, TargetChoice)> {
        progress.emit(RuneforgePublishStage::Uploading, 0, size);
        let mut upload_progress = progress.clone();
        runeforge.call(|client| {
            client.put_package(
                &created,
                packed.output_path.as_ref(),
                move |sent| upload_progress.throttled(RuneforgePublishStage::Uploading, sent, size),
                Arc::clone(cancel),
            )
        })?;

        check_cancel(cancel)?;
        progress.emit(RuneforgePublishStage::Inspecting, size, size);
        let mut review = runeforge.call(|client| client.complete_upload(&upload_id))?;
        let owned = runeforge.call(Client::owned_mods)?;
        let choice = choose_target(stamp.as_deref(), &owned, &review.suggestions);

        let target = match &choice.target {
            Target::Stamped { mod_id } | Target::Suggested { mod_id, .. } => Some(mod_id.as_str()),
            Target::New => None,
        };
        if target != review.suggestions.mod_id.as_deref() {
            review.prefill = runeforge.call(|client| client.prefill(&upload_id, target))?;
        }

        check_cancel(cancel)?;
        Ok((review, owned, choice))
    })();

    let (review, owned, choice) =
        uploaded.inspect_err(|_| discard_quietly(&runeforge, &upload_id))?;

    Ok(PublishReview {
        project_path: project_path.to_owned(),
        file_name: packed.file_name,
        size,
        warnings: validation.warnings,
        review,
        choice,
        owned,
    })
}

/// The post and the release that a create made.
struct Created {
    mod_id: String,
    release_id: String,
    post_update_error: Option<String>,
}

impl From<ltk_manager_runeforge::CreatedMod> for Created {
    fn from(created: ltk_manager_runeforge::CreatedMod) -> Self {
        Self {
            mod_id: created.mod_id,
            release_id: created.release_id,
            post_update_error: None,
        }
    }
}

impl From<CreatedRelease> for Created {
    fn from(created: CreatedRelease) -> Self {
        Self {
            mod_id: created.mod_id,
            release_id: created.release_id,
            post_update_error: created.post_update_error.map(|error| error.message),
        }
    }
}

/// Publishes the post `mod_id`, and folds a refusal or a failure into the result.
fn publish_result(
    runeforge: &RuneforgeState,
    mod_id: &str,
    answer: &PublishAnswer,
) -> PublishResult {
    let published = runeforge.call(|client| client.publish(mod_id, answer));

    match published {
        Ok(()) => PublishResult::Published,
        Err(error) => match error.domain::<RuneforgeError>().cloned() {
            Some(RuneforgeError::PublishRefused { checklist }) => {
                let review_url = checklist.review_path.as_deref().and_then(|path| {
                    runeforge
                        .client()
                        .ok()
                        .map(|client| site_url(client.base(), path).to_string())
                });
                PublishResult::Refused {
                    checklist,
                    review_url,
                }
            }
            Some(error) => PublishResult::Failed { error },
            None => PublishResult::Failed {
                error: RuneforgeError::Offline {
                    detail: error.to_string(),
                },
            },
        },
    }
}

/// Writes the RuneForge stamp of `mod_id` into the project at `project_path`.
fn write_stamp(app: &AppHandle, project_path: &str, base: &Url, mod_id: &str) -> StampResult {
    let distributor = ModProjectDistributor::new(
        SITE_ID.to_owned(),
        SITE_NAME.to_owned(),
        base.as_str().trim_end_matches('/').to_owned(),
        mod_id.to_owned(),
    );

    let written = app
        .state::<WorkshopState>()
        .0
        .project(project_path)
        .and_then(|project| project.write_distributor(distributor));

    match written {
        Ok(()) => StampResult::Written,
        Err(error) => {
            tracing::warn!(
                error = %error,
                project_path,
                mod_id,
                "Could not write the RuneForge stamp into the project. The next publish asks for the target"
            );
            StampResult::Failed {
                detail: error.to_string(),
            }
        }
    }
}

/// Deletes an upload that a cancelled or failed prepare leaves behind, and logs a failure.
fn discard_quietly(runeforge: &RuneforgeState, upload_id: &str) {
    if let Err(error) = runeforge.call(|client| client.discard_upload(upload_id)) {
        tracing::warn!(
            error = %error,
            upload_id,
            "Could not delete the RuneForge upload of a cancelled or failed publish. It expires within a day"
        );
    }
}

fn check_cancel(cancel: &AtomicBool) -> AppResult<()> {
    if cancel.load(Ordering::Relaxed) {
        return Err(RuneforgeError::Cancelled.into());
    }

    Ok(())
}

/// Claims the prepare slot of `project_path`, and returns its cancel flag.
fn claim(app: &AppHandle, project_path: &str) -> AppResult<Arc<AtomicBool>> {
    let runeforge = state(app);
    let mut prepares = runeforge.prepares.lock();
    if prepares.contains_key(project_path) {
        return Err(AppError::InternalState(format!(
            "a publish of {project_path} is already running. Wait for it to end or cancel it"
        )));
    }

    let cancel = Arc::new(AtomicBool::new(false));
    prepares.insert(project_path.to_owned(), Arc::clone(&cancel));
    Ok(cancel)
}

fn release_claim(app: &AppHandle, project_path: &str) {
    state(app).prepares.lock().remove(project_path);
}

/// The progress events of one publish, throttled to one every [`PROGRESS_INTERVAL`].
#[derive(Clone)]
struct Progress {
    events: Arc<TauriEventSink>,
    project_path: String,
    last: Option<Instant>,
}

impl Progress {
    fn new(app: &AppHandle, project_path: &str) -> Self {
        Self {
            events: Arc::new(TauriEventSink::new(app.clone())),
            project_path: project_path.to_owned(),
            last: None,
        }
    }

    fn emit(&mut self, stage: RuneforgePublishStage, bytes_sent: u64, total_bytes: u64) {
        self.last = Some(Instant::now());
        self.events.emit(BackendEvent::RuneforgePublishProgress(
            RuneforgePublishProgress {
                project_path: self.project_path.clone(),
                stage,
                bytes_sent,
                total_bytes,
            },
        ));
    }

    fn throttled(&mut self, stage: RuneforgePublishStage, bytes_sent: u64, total_bytes: u64) {
        let due = self
            .last
            .is_none_or(|last| last.elapsed() >= PROGRESS_INTERVAL);

        if due || bytes_sent == total_bytes {
            self.emit(stage, bytes_sent, total_bytes);
        }
    }
}
