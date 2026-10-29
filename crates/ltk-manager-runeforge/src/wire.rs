//! The request and response bodies of the RuneForge `/api/v1` routes, in their camelCase JSON.
//!
//! Each type reads only the fields that the app uses. Serde skips the rest, so a field the server
//! adds does not break a read.

use serde::{Deserialize, Serialize};

/// Implements `Deserialize` for a string enum, reading a value outside `known` as `fallback`.
///
/// A derive with `#[serde(other)]` does the same, but specta refuses it on a string enum.
macro_rules! lenient_deserialize {
    ($type:ty, $fallback:expr, [$($known:expr),* $(,)?]) => {
        impl<'de> Deserialize<'de> for $type {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text = std::borrow::Cow::<str>::deserialize(deserializer)?;
                Ok([$($known),*]
                    .into_iter()
                    .find(|known| serde_json::to_value(known).ok().as_ref().and_then(serde_json::Value::as_str) == Some(text.as_ref()))
                    .unwrap_or($fallback))
            }
        }
    };
}

/// What a token may do, as the token page lets the creator tick it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub enum Scope {
    /// Reads the posts of the account, the publishing context and a release.
    #[serde(rename = "mods:read")]
    ModsRead,
    /// Creates a mod post from uploads.
    #[serde(rename = "mods:write")]
    ModsWrite,
    /// Creates uploads and releases.
    #[serde(rename = "releases:write")]
    ReleasesWrite,
    /// Publishes a post.
    #[serde(rename = "publish")]
    Publish,
    /// A scope that this build does not know.
    #[serde(rename = "unknown")]
    Unknown,
}

lenient_deserialize!(
    Scope,
    Scope::Unknown,
    [
        Scope::ModsRead,
        Scope::ModsWrite,
        Scope::ReleasesWrite,
        Scope::Publish
    ]
);

impl Scope {
    /// The scopes that publishing a new post from the app uses.
    pub const PUBLISHING: [Scope; 4] = [
        Scope::ModsRead,
        Scope::ModsWrite,
        Scope::ReleasesWrite,
        Scope::Publish,
    ];
}

/// `GET /api/v1/me`: the account that a token acts for and what it may do.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    /// The id of the user on the site.
    pub user_id: String,
    /// The user name. It is empty for an account that has not picked one.
    pub username: String,
    /// The name the site shows.
    pub display_name: Option<String>,
    /// The scopes of the token.
    pub scopes: Vec<Scope>,
}

/// One post of `GET /api/v1/me/mods`, with its newest release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct OwnedModPost {
    /// The id of the post.
    pub id: String,
    /// The name of the post.
    pub name: String,
    /// `public`, `private` or `unlisted`.
    pub visibility: String,
    /// Whether the post is published.
    pub is_published: bool,
    /// When the post last changed, as an ISO 8601 timestamp.
    pub updated_at: String,
    /// The newest release, or `None` for a post without one.
    pub latest_release: Option<LatestRelease>,
}

/// The newest release of an owned post.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LatestRelease {
    /// The id of the release.
    pub id: String,
    /// The tag of the release.
    pub tag: String,
    /// When the release was created, as an ISO 8601 timestamp.
    pub created_at: String,
}

/// The `{ mods }` wrapper of `GET /api/v1/me/mods`.
#[derive(Debug, Deserialize)]
pub(crate) struct OwnedMods {
    pub(crate) mods: Vec<OwnedModPost>,
}

/// `GET /api/v1/publishing-context`: the values a release and a new post may take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PublishingContext {
    /// The posts that the account may upload to.
    pub owned_posts: Vec<OwnedPostOption>,
    /// The game versions a release may target, newest first.
    pub game_versions: Vec<GameVersion>,
    /// Whether a new post may be published without a review.
    pub can_publish_without_review: bool,
    /// The categories a new post may take.
    pub categories: Vec<String>,
}

/// An owned post as the target picker offers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct OwnedPostOption {
    /// The id of the post.
    pub id: String,
    /// The name of the post.
    pub name: String,
    /// The tag of its latest semver release.
    pub latest_tag: Option<String>,
}

/// A game version that a release may target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct GameVersion {
    /// The id that a release request names.
    pub id: String,
    /// The patch, such as `25.21`.
    pub version: String,
}

/// The body of `POST /api/v1/uploads`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NewUpload<'a> {
    pub(crate) filename: &'a str,
    pub(crate) size: u64,
}

/// `POST /api/v1/uploads`: the upload, and the URL and headers that its bytes go to.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedUpload {
    /// The created upload.
    pub upload: Upload,
    /// The storage URL that the bytes are PUT to. It expires an hour after it is issued.
    pub presigned_url: String,
    /// The headers that the URL was signed with.
    pub put_headers: PutHeaders,
}

/// The headers that a presigned PUT was signed with. The PUT sends exactly these.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PutHeaders {
    /// The `Content-Type` of the package.
    #[serde(rename = "Content-Type")]
    pub content_type: String,
    /// The `Content-Disposition` that names the file.
    #[serde(rename = "Content-Disposition")]
    pub content_disposition: String,
}

/// An upload as its owner sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Upload {
    /// The id of the upload.
    pub id: String,
    /// The file name that the upload was created with.
    pub filename: String,
    /// The state of the upload, such as `pending` or `inspected`.
    pub status: String,
    /// The package format that the server read, `modpkg` or `fantome`.
    pub format: Option<String>,
}

/// One thing that an inspection or a scan found in a package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct Finding {
    /// How serious the finding is.
    pub severity: FindingSeverity,
    /// The stable code of the finding.
    pub code: String,
    /// The text that the server wrote for the finding.
    pub message: String,
    /// The path inside the package that the finding is about.
    pub path: Option<String>,
}

/// How serious a [`Finding`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum FindingSeverity {
    /// The server refuses the package.
    Error,
    /// The package is accepted with a problem.
    Warning,
    /// A note without a problem.
    Info,
}

/// `POST /api/v1/uploads/:id/complete`: what the server read and where the package may go.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct UploadReview {
    /// The inspected upload.
    pub upload: Upload,
    /// The findings of the inspection.
    pub findings: Vec<Finding>,
    /// The post that the package most likely updates.
    pub suggestions: Suggestions,
    /// The post and the release that the server proposes for the suggested target.
    pub prefill: Prefill,
}

/// Which owned post a package most likely updates, and the tag its release would get.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Suggestions {
    /// The suggested post, or `None` if no owned post matches.
    pub mod_id: Option<String>,
    /// How the suggested post was found.
    pub matched_by: Option<SuggestionMatch>,
    /// The patch bump of the latest semver release of the suggested post.
    pub next_tag: Option<String>,
}

/// How a suggested post was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum SuggestionMatch {
    /// The distributor stamp of the package names the post.
    Distributor,
    /// The post has the display name of the package.
    Name,
}

/// The post and the release that the server proposes, with where each value came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Prefill {
    /// The proposed post fields.
    pub post: PrefillPost,
    /// The proposed release fields.
    pub release: PrefillRelease,
    /// Where each release field came from.
    pub sources: PrefillSources,
    /// The findings of the package plus those of the site.
    pub findings: Vec<Finding>,
    /// The next free tag if the tag of the package is taken.
    pub suggested_tag: Option<String>,
    /// The post fields that the package would change on the target post.
    pub changed_post_fields: Vec<ChangedPostField>,
}

/// The post fields that a prefill proposes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PrefillPost {
    /// The name of the post.
    pub name: String,
    /// The one-line description.
    pub short_description: Option<String>,
    /// The category of the post.
    pub category: String,
    /// The staged thumbnail image, or the current one of the target post.
    pub thumbnail_image_id: Option<String>,
}

/// The release fields that a prefill proposes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PrefillRelease {
    /// The tag of the release.
    pub tag: String,
    /// The release notes.
    pub notes: String,
    /// The game version that the release targets.
    pub game_version_id: Option<String>,
    /// The label of the package artifact.
    pub artifact_label: String,
}

/// Where each release field of a prefill came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PrefillSources {
    /// The sources of the release fields.
    pub release: ReleaseSources,
}

/// Where each release field of a prefill came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ReleaseSources {
    /// The source of the tag.
    pub tag: FieldSource,
    /// The source of the notes.
    pub notes: FieldSource,
    /// The source of the game version.
    pub game_version_id: FieldSource,
}

/// Where a prefilled value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum FieldSource {
    /// The package.
    Package,
    /// The target post as it is.
    CurrentPost,
    /// The default of the site.
    Default,
}

/// A post field that the package would change on the target post.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct ChangedPostField {
    /// The field that changes.
    pub field: PostField,
}

/// A field of a mod post that a prefill proposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum PostField {
    /// The name of the post. A release never refreshes it.
    Name,
    /// The one-line description.
    ShortDescription,
    /// The long description.
    Description,
    /// The category.
    Category,
    /// The license.
    License,
    /// The thumbnail.
    ThumbnailImageId,
    /// The champions.
    ChampionIds,
    /// The maps.
    MapIds,
    /// The themes. A release never refreshes them.
    Themes,
    /// The features. A release never refreshes them.
    Features,
    /// A field that this build does not know.
    Unknown,
}

lenient_deserialize!(
    PostField,
    PostField::Unknown,
    [
        PostField::Name,
        PostField::ShortDescription,
        PostField::Description,
        PostField::Category,
        PostField::License,
        PostField::ThumbnailImageId,
        PostField::ChampionIds,
        PostField::MapIds,
        PostField::Themes,
        PostField::Features,
    ]
);

impl PostField {
    /// Returns `true` if a release may refresh the field from its package.
    #[must_use]
    pub fn is_refreshable(self) -> bool {
        matches!(
            self,
            Self::ShortDescription
                | Self::Description
                | Self::Category
                | Self::License
                | Self::ThumbnailImageId
                | Self::ChampionIds
                | Self::MapIds
        )
    }
}

/// The body of `POST /api/v1/mods`: the uploads and the overrides of the prefill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct NewMod {
    /// The inspected uploads. The first becomes the primary artifact.
    pub upload_ids: Vec<String>,
    /// The post fields that differ from the prefill.
    #[serde(default, skip_serializing_if = "PostOverrides::is_empty")]
    pub post: PostOverrides,
    /// The release fields that differ from the prefill.
    #[serde(default, skip_serializing_if = "ReleaseOverrides::is_empty")]
    pub release: ReleaseOverrides,
}

/// The post fields of a new post that differ from the prefill.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PostOverrides {
    /// The name of the post.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", specta(optional))]
    pub name: Option<String>,
    /// The category of the post.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", specta(optional))]
    pub category: Option<String>,
}

impl PostOverrides {
    fn is_empty(&self) -> bool {
        self.name.is_none() && self.category.is_none()
    }
}

/// The release fields of a new post that differ from the prefill.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ReleaseOverrides {
    /// The tag of the release.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", specta(optional))]
    pub tag: Option<String>,
    /// The release notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", specta(optional))]
    pub notes: Option<String>,
    /// The game version that the release targets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", specta(optional))]
    pub game_version_id: Option<String>,
}

impl ReleaseOverrides {
    fn is_empty(&self) -> bool {
        self.tag.is_none() && self.notes.is_none() && self.game_version_id.is_none()
    }
}

/// `POST /api/v1/mods`: the new private post and its first release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CreatedMod {
    /// The id of the post.
    pub mod_id: String,
    /// The id of its first release.
    pub release_id: String,
}

/// The body of `POST /api/v1/mods/:modId/releases`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct NewRelease {
    /// The inspected uploads. The first becomes the primary artifact.
    pub upload_ids: Vec<String>,
    /// The tag. The server uses the tag of the package if it is `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", specta(optional))]
    pub tag: Option<String>,
    /// The release notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", specta(optional))]
    pub description: Option<String>,
    /// The game version that the release targets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", specta(optional))]
    pub game_version: Option<String>,
    /// The post fields to refresh from the package.
    pub update_post: Vec<PostField>,
    /// Whether the status of the post becomes working.
    pub mark_as_working: bool,
}

/// `POST /api/v1/mods/:modId/releases`: the release, and the post fields that it changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CreatedRelease {
    /// The id of the post.
    pub mod_id: String,
    /// The id of the release.
    pub release_id: String,
    /// The tag of the release.
    pub tag: String,
    /// The post fields that the release refreshed.
    pub applied_post_changes: Vec<ChangedPostField>,
    /// Why the post refresh stopped part way. The release exists either way.
    pub post_update_error: Option<PostUpdateError>,
}

/// Why a release did not refresh every post field that it was asked to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct PostUpdateError {
    /// The text that the server wrote.
    pub message: String,
}

/// The body of `POST /api/v1/mods/:modId/publish`: the AI content answer, when the site asks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PublishAnswer {
    /// Whether the post content is AI generated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", specta(optional))]
    pub has_ai_generated_post_content: Option<bool>,
    /// The `aiContentDisclosureVersion` of the checklist that the answer replies to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", specta(optional))]
    pub expected_version: Option<String>,
}

/// Why `POST /api/v1/mods/:modId/publish` did not publish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct PublishChecklist {
    /// Whether the account publishes directly or through a review.
    pub path: PublishPath,
    /// What the post still lacks.
    pub missing: Vec<PublishRequirement>,
    /// The version that an AI content answer replies to.
    pub ai_content_disclosure_version: String,
    /// A moderator label is active, so only a yes answer is accepted.
    pub ai_moderator_enforced: bool,
    /// The site path where the creator requests a review, such as `/mods/<id>`.
    pub review_path: Option<String>,
}

/// How an account gets a post published.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum PublishPath {
    /// The account publishes without a review.
    Publish,
    /// The post goes live only through a review that the creator requests on the site.
    Review,
}

/// A requirement of publishing that a post does not meet yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum PublishRequirement {
    /// The post has no thumbnail.
    Thumbnail,
    /// The post has no release.
    Release,
    /// The post has no license.
    License,
    /// The AI content question has no answer.
    AiDisclosure,
    /// A requirement that this build does not know.
    Unknown,
}

lenient_deserialize!(
    PublishRequirement,
    PublishRequirement::Unknown,
    [
        PublishRequirement::Thumbnail,
        PublishRequirement::Release,
        PublishRequirement::License,
        PublishRequirement::AiDisclosure,
    ]
);

/// `GET /api/v1/mods/:modId/releases/:releaseId`: the release and its artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Release {
    /// The id of the release.
    pub id: String,
    /// The tag of the release.
    pub tag: String,
    /// The files of the release.
    pub artifacts: Vec<ReleaseArtifact>,
}

/// One file of a release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ReleaseArtifact {
    /// The id of the artifact.
    pub id: String,
    /// The file name.
    pub filename: String,
    /// The package format, `modpkg` or `fantome`.
    pub format: Option<String>,
    /// The hex sha256 of the package, once the deep scan has read it.
    pub content_hash: Option<String>,
    /// The verdict of the deep scan, once it has read the package.
    pub health: Option<ArtifactHealth>,
}

/// The verdict of the deep scan on one artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ArtifactHealth {
    /// `pending` until a scan finishes, then the worst severity among the findings.
    pub status: HealthStatus,
    /// The findings of the scan. A pending artifact may have none stored.
    #[serde(default)]
    pub findings: Vec<Finding>,
}

/// Where an artifact is with the deep scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum HealthStatus {
    /// The scan found no problem.
    Ok,
    /// The scan found warnings.
    Warnings,
    /// The scan found errors.
    Errors,
    /// The scan has not finished. A status that this build does not know reads as pending.
    Pending,
}

lenient_deserialize!(
    HealthStatus,
    HealthStatus::Pending,
    [
        HealthStatus::Ok,
        HealthStatus::Warnings,
        HealthStatus::Errors
    ]
);
