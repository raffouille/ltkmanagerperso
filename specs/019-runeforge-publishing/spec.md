# Feature Specification: Publishing a project to RuneForge

**Feature Branch**: `019-runeforge-publishing`
**Created**: 2026-10-26
**Updated**: 2026-10-27, after the server shipped (`runeforge-io/runeforge` PR #457 and PR #459)
**Status**: In progress
**Input**: RuneForge issue [runeforge-io/runeforge#435](https://github.com/runeforge-io/runeforge/issues/435),
which adds API tokens and a bearer-authenticated `/api/v1` to the site and leaves the LTK Manager
client to a separate spec. This is that spec.

## Problem Statement

A creator builds a mod in the workshop and packs it. To publish it, they leave the app: they open
`<project>/build/` in Explorer, open runeforge.dev in a browser, find the mod post, start a release,
drag the file in and type the tag and notes again. The tag, the description, the champions and the
thumbnail are already in `mod.config.json`. The creator repeats this for every release of every mod.

LTK Manager has no account concept. It downloads from RuneForge through `ltk://install` and
reads nothing else from the site. RuneForge now inspects an uploaded package and prefills the post
and the release from it (the upload dialog, `runeforge-io/runeforge` PR #444). Issue #435 opens the
same pipeline to a desktop client through a personal API token. The client side does not exist.

## Solution

The creator pastes a RuneForge API token into Settings once. The app checks it, shows the account
name and stores the token in the Windows Credential Manager. A project then has "Publish to
RuneForge". The action packs the project as `.modpkg` and uploads it with progress. The server
inspects the package, and the app shows the server's review in a dialog: the target post, the tag,
the notes, the game version and the findings. The creator confirms, and the app creates the mod
post or the release. If the token allows it, the app also publishes a new post.

After the first publish, the app writes a `distributor` stamp with the RuneForge mod id into
`mod.config.json`. Every later pack carries the stamp. The next publish of that project targets the
same post without a picker.

The server defines the contract in `docs/PUBLIC_API.md` of `runeforge-io/runeforge` (section "API
contract" below). The app adds a RuneForge client
crate, a credential store entry, a `runeforge` Tauri service, a Settings section and a publish
dialog in the workshop.

## User Stories

1. As a creator, I want to paste a RuneForge token in Settings, so that the app can publish for me.
2. As a creator, I want the app to show the account the token belongs to, so that I know it is the right account.
3. As a creator, I want the app to warn me when the token lacks a scope it needs, so that I can create a token with the right scopes.
4. As a creator, I want the token stored by Windows and not in a settings file, so that a copied settings file or a support bundle does not contain it.
5. As a creator, I want to disconnect, so that the token is removed from this machine.
6. As a creator, I want a link to the RuneForge token page, so that I can create or revoke a token without searching for it.
7. As a creator, I want "Publish to RuneForge" on a project card and in the project editor, so that publishing starts from the project.
8. As a creator, I want the app to validate and pack the project before it uploads, so that I never upload a stale build.
9. As a creator, I want upload progress, so that a large package does not look stuck.
10. As a creator, I want to cancel during packing or uploading, so that a wrong click costs nothing.
11. As a creator, I want to see what the server read from the package and its findings before anything is created, so that I can fix a problem first.
12. As a creator, I want a refused package to show the findings that refused it, so that I know what to fix in the project.
13. As a creator, I want a stamped project to publish to its post without a picker, so that an update is one confirmation.
14. As a creator, I want to choose among my posts or a new post when the project is not stamped, so that the first publish goes to the right place.
15. As a creator, I want the tag prefilled from the project version, so that I do not type it again.
16. As a creator, I want a taken tag refused with the next free tag offered, so that I can accept the fix in one click.
17. As a creator, I want to edit the release notes and game version before I confirm, so that the release reads correctly.
18. As a creator, I want to choose which post fields an update refreshes from the package, so that a release does not overwrite a description I edited on the site.
19. As a creator, I want "Set mod status to Working" on a release, as the site offers it, so that a fix for a broken mod clears its status.
20. As a creator, I want to publish a new post from the app when my account may publish, so that the first release does not need the browser.
21. As a creator who needs a review, I want the app to show why the site refused to publish and open the review request, so that I know the next step.
22. As a creator, I want "Open on RuneForge" after a publish, so that I can check the page.
23. As a creator, I want to see the scan result of the uploaded package, so that I know if the site found a problem after the upload.
24. As a creator, I want a lost connection during the final step not to create a second release when I retry, so that a retry is safe.
25. As a creator, I want an expired or revoked token reported as such, with a way to reconnect, so that the error is actionable.
26. As a creator, I want the app to make no RuneForge request until I connect, so that the feature costs nothing if I do not use it.
27. As a maintainer, I want the token never written to a log, a crash dump or the telemetry stream, so that diagnostics cannot leak it.
28. As a maintainer, I want the token sent only to the RuneForge API host, so that the storage host of the presigned upload never receives it.
29. As a maintainer, I want the client tested against a local HTTP server, so that the tests need neither the network nor a RuneForge account.

## API contract

The source of truth is `docs/PUBLIC_API.md` in `runeforge-io/runeforge`, which PR #457 added with
the token routes. Every call sends `Authorization: Bearer rf_...` and `Accept: application/json`,
and no `Origin`. Bodies are JSON with camelCase keys, sent as `Content-Type: application/json`.

| Step          | Call                                                                                                               | Scope            | Answer the app reads                                                                                                                                                   |
| ------------- | ------------------------------------------------------------------------------------------------------------------ | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Check token   | `GET /api/v1/me`                                                                                                   | none             | `userId`, `username` (can be empty), `displayName`, `permissions`, `scopes`, `token: { id }`                                                                           |
| List posts    | `GET /api/v1/me/mods`                                                                                              | `mods:read`      | `{ mods }`, each `{ id, name, visibility, isPublished, updatedAt, latestRelease }`, most recently updated first. `latestRelease` is `{ id, tag, createdAt }` or `null` |
| Context       | `GET /api/v1/publishing-context`                                                                                   | `mods:read`      | owned posts with `latestTag`, `gameVersions` (newest first), `canPublishWithoutReview`, `categories`, `themes`, `features`                                             |
| Create upload | `POST /api/v1/uploads` `{ filename, size }`                                                                        | `releases:write` | `upload.id`, `presignedUrl`, `putHeaders`                                                                                                                              |
| Send bytes    | `PUT <presignedUrl>` with `putHeaders` and the file body                                                           | none             | `2xx`                                                                                                                                                                  |
| Complete      | `POST /api/v1/uploads/:id/complete`                                                                                | `releases:write` | `upload`, `inspection`, `findings`, `suggestions`, `prefill`                                                                                                           |
| Re-prefill    | `GET /api/v1/uploads/:id/prefill?modId=`                                                                           | `releases:write` | `prefill` for another target                                                                                                                                           |
| Discard       | `DELETE /api/v1/uploads/:id`                                                                                       | `releases:write` | `204`                                                                                                                                                                  |
| New post      | `POST /api/v1/mods` `{ uploadIds, post?, release? }`                                                               | `mods:write`     | `modId`, `releaseId`                                                                                                                                                   |
| New release   | `POST /api/v1/mods/:modId/releases` `{ uploadIds, tag?, description?, gameVersion?, updatePost?, markAsWorking? }` | `releases:write` | `modId`, `releaseId`, `tag`, `artifacts`, `appliedPostChanges`, `postUpdateError`                                                                                      |
| Publish       | `POST /api/v1/mods/:modId/publish` `{ hasAiGeneratedPostContent?, expectedVersion? }`                              | `publish`        | `{ modId, isPublished: true }`, also for a post that is already public                                                                                                 |
| Read release  | `GET /api/v1/mods/:modId/releases/:releaseId`                                                                      | `mods:read`      | artifacts with `format`, hash and `health`                                                                                                                             |

Failures answer `{ "error": { "code", "message", "findings"?, "suggestedTag"?, "checklist"?,
"scope"? } }`. The app maps `code` and does not parse `message`:

| `code`              | Status | The app does                                                                                                                                                     |
| ------------------- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `unauthorized`      | 401    | Marks the connection invalid and offers to reconnect. The message says if the token is malformed, unknown, revoked or expired, and the app does not branch on it |
| `forbidden`         | 403    | Names the missing scope from `error.scope`. Without `scope`, shows that the account cannot do this, for example a write of a banned account                      |
| `not_found`         | 404    | Shows that the post or the upload does not exist for this account                                                                                                |
| `package_rejected`  | 422    | Shows `findings` in the review and disables confirm                                                                                                              |
| `tag_taken`         | 409    | Offers `suggestedTag` in the tag field                                                                                                                           |
| `upload_state`      | 409    | Starts the publish again from the upload                                                                                                                         |
| `publish_refused`   | 409    | Shows `checklist` in Done. The post and the release exist                                                                                                        |
| `conflict`          | 409    | Shows that an earlier request with the same `Idempotency-Key` differs or still runs, or that the post is under moderation                                        |
| `payload_too_large` | 413    | Shows the 1 GB limit                                                                                                                                             |
| `too_many_requests` | 429    | Shows the wait from `Retry-After`, in seconds. The limit is 120 requests a minute per token                                                                      |
| anything else       | any    | Shows a generic failure with the status                                                                                                                          |

Facts the client depends on:

- The presigned PUT is signed for the exact `Content-Length` and for both `putHeaders`. The app
  sends exactly those headers and no `Authorization`. A missing or changed header answers 403 from
  storage.
- The presigned URL expires one hour after it is issued. An upload object expires a day after it
  is created, or a week after it is completed, if it is never attached.
- A package must be named `.modpkg` or `.fantome`, and its bytes must match the extension. The
  server refuses anything else.
- The server reads the stamp from both formats (PR #459): `distributor.site_id` and
  `distributor.mod_id` in `.modpkg` metadata, `Distributor.SiteId` and `Distributor.ModId` in a
  `.fantome` `META/info.json`.
- `suggestions.modId` is set when the package carries `distributor.siteId == "runeforge"` and the
  token's user owns that post (`matchedBy: "distributor"`), or when an owned post has the
  package's display name (`matchedBy: "name"`).
- The two create calls honour `Idempotency-Key` for 24 hours per token. A replayed answer carries
  `Idempotent-Replayed: true`. The same key with another body answers 409 `conflict`. A failed
  request releases its key.
- A refused publish answers 409 `publish_refused` with `error.checklist`: `path` (`publish` or
  `review`), `missing` (any of `thumbnail`, `release`, `license`, `ai_disclosure`),
  `aiContentDisclosureVersion`, `aiModeratorEnforced` and `reviewPath`. `ai_disclosure` is
  missing while the publication gate of the site is on and the call sent no answer. The retry
  sends `hasAiGeneratedPostContent` with `expectedVersion` set to `aiContentDisclosureVersion`. If
  `aiModeratorEnforced` is set, only `true` is accepted. `reviewPath` is a site path such as
  `/mods/<id>`, joined to the base URL. The review is requested from the checklist on that page.
- `contentHash` and `health` of a release artifact are `null` until the deep scan has read the
  package. `health` is `{ status: pending | ok | warnings | errors, findings, scannedAt?,
scannerVersion? }`.
- Stable links: the token page is `/user-settings/api-tokens`. A new token there has the scopes
  `mods:read`, `mods:write` and `releases:write` by default, and the user must tick `publish`.

## Implementation Decisions

### Packing for RuneForge

- Publishing packs through `Workshop::pack_project` into the usual
  `<project>/build/<name>_<version>.<ext>`. `.modpkg` is the default, and ADR-0001 keeps a
  `.modpkg` packed on install. `.fantome` is offered too, because both packers copy the stamp and
  the server reads it from both formats.
- `ProjectDir::validate` runs first. A validation error stops the publish before any request.
  Warnings appear in the review next to the server findings.

### The distributor stamp

- `ltk_mod_project` 0.18.0 has `ModProject.distributor: Option<ModProjectDistributor>` with
  `site_id`, `site_name`, `site_url` and `mod_id`, and `ltk_fantome` 0.17.0 has
  `FantomeInfo.distributor` (league-mod PR #312, released in PR #313). Both packers copy the
  stamp in. The release made `ModProject` and `FantomeInfo` `#[non_exhaustive]`, so the app
  builds them from `Default` and assigns the fields. The bump is breaking, and moves
  `ltk_modpkg` to 0.12.15 and `ltk_overlay` to 0.19.0 with them.
- After a successful create, the app writes `distributor { site_id: "runeforge", site_name:
"RuneForge", site_url, mod_id }` through `ProjectDir::write_config`. `site_url` is the base URL
  that the client used, `https://runeforge.dev` in a release build. The write keeps every other
  field of the config.
- A project that is stamped with a post the user does not own still uploads. The server ignores
  the stamp and suggests by name, and the review shows that the stamped post is not the
  account's.

### The RuneForge client crate

- New crate `crates/ltk-manager-runeforge` holds the wire types, the HTTP client and the error
  type. It depends on `ltk-manager-base` and on nothing in `src-tauri`, so its tests run with
  `cargo test -p ltk-manager-runeforge`.
- The client is a `reqwest::blocking::Client` built once per connection. This matches the other
  call sites, which run inside `off_thread`. The User-Agent is `ltk-manager/{version}`. API calls
  use a 15 s connect timeout and a 60 s total timeout. The PUT has no total timeout under one
  hour, the presign lifetime.
- `ApiToken` is a validated newtype. Construction requires the `rf_` prefix and the URL-safe
  alphabet. `Debug` and `Display` print `rf_` plus the 8 character display prefix and nothing
  more. The type does not implement `Serialize`, so the token cannot reach `settings.json`, an
  IPC payload or a log by accident.
- The base URL is `https://runeforge.dev`. A debug build reads `LTK_RUNEFORGE_URL` so a developer
  can point the app at a local site on `http://localhost:3000`. A release build ignores the
  variable. The client attaches the bearer header only to requests whose host is the base URL
  host.
- Upload bytes stream from the packed file through a reader that counts bytes for progress. The
  body is never held in memory.
- `RuneforgeError` is a `thiserror` enum that serializes with a `kind` tag: `Unauthorized`,
  `Forbidden { scope }`, `NotFound`, `PackageRejected { findings }`, `TagTaken { suggestedTag }`,
  `UploadState`, `PublishRefused { checklist }`, `Conflict`, `TooLarge`,
  `RateLimited { retryAfter }`, `Storage { status }`, `Http { status, code }`, `Malformed`,
  `Offline`, `Cancelled`, `NotConnected` and `Credential`. It reaches the frontend whole as
  `AppErrorResponse::Runeforge { error }`, in the shape of `Workshop { error }`, because the
  findings, the suggested tag and the checklist are data that the dialog renders. The frontend
  translates it with keys such as `error.RUNEFORGE.TAG_TAKEN.title`.
- The `Idempotency-Key` of a create is `ltk-manager-<upload id>`. An upload attaches to one post
  or release only, so the key is the same on every retry of one review and needs no state.

### The token

- The token is stored in the Windows Credential Manager through `keyring-core` and
  `windows-native-keyring-store`, under the service `ltk-manager/runeforge` and the account
  `default`. Another platform has no store, and connect fails there with `Credential`. This is the first secret the app
  keeps outside a file, and it needs an ADR: "A RuneForge token lives in the OS credential store".
- `settings.json` keeps only what is safe to read, under one group: `runeforge.userId`,
  `runeforge.username`, `runeforge.displayName`, `runeforge.tokenPrefix` and `runeforge.scopes`.
  The token prefix is the 8 characters after `rf_`. The Settings section reads these without
  touching the credential store. `save_settings` keeps the stored group, so only the runeforge
  commands write it.
- Connect calls `GET /api/v1/me` with the pasted token. It stores the token only if the call
  succeeds. Disconnect deletes the credential and the four settings.
- A 401 from any call clears nothing. It marks the connection invalid for the session and shows
  "Reconnect" in the Settings section and in the publish dialog. A token that the user revoked on
  the site therefore stays on the machine until the user disconnects or replaces it.
- The token is never passed to the frontend after connect. Every call that needs it runs in the
  backend.

### The `runeforge` Tauri service

A new row in `src-tauri/src/services/table.rs` registers the service `runeforge`. Every command
returns `IpcResult<T>`, and every network call runs through `off_thread`:

| Command                                   | Does                                                                                                                |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `runeforge_connect(token)`                | Checks the token with `me`, stores it, returns the `Account`                                                        |
| `runeforge_disconnect()`                  | Deletes the credential and the account settings                                                                     |
| `runeforge_status()`                      | Returns the stored account, whether the server refused the token this session, and the site links. Sends no request |
| `runeforge_owned_mods()`                  | Returns `me/mods` for the target picker                                                                             |
| `runeforge_context()`                     | Returns `publishing-context` for the game versions and the categories                                               |
| `runeforge_prepare(project_path, format)` | Validates, packs, creates the upload, PUTs it, completes it, returns the `PublishReview`                            |
| `runeforge_cancel(project_path)`          | Cancels the prepare in flight for the project                                                                       |
| `runeforge_prefill(upload_id, mod_id)`    | Returns the prefill for another target                                                                              |
| `runeforge_submit(request)`               | Creates the post or the release, publishes when asked, writes the stamp, returns the `PublishOutcome`               |
| `runeforge_publish(mod_id, answer)`       | Publishes a post again, with the AI content answer when the checklist asks for one                                  |
| `runeforge_discard(upload_id)`            | Deletes an upload that the dialog closed without submitting                                                         |
| `runeforge_release(mod_id, release_id)`   | Returns the release with its artifacts and their `health`                                                           |

- `runeforge_prepare` holds an `InFlight` slot per project. Cancel aborts the pack or the PUT. If
  the upload exists, cancel also discards it.
- Progress is a new `BackendEvent::RuneforgePublishProgress { project_id, stage, bytes_sent,
total_bytes }`, with `stage` from `Validating`, `Packing`, `Uploading`, `Inspecting`,
  `Creating`, `Publishing`, `Complete` and `Error`. It is throttled to one event every 100 ms,
  like `ProtocolInstallProgress`.
- `PublishRequest` is an enum: `NewMod { upload_id, post, release, publish }` or `Release {
upload_id, mod_id, tag, notes, game_version, update_post, mark_as_working }`. Its fields mirror
  the server's request bodies.
- `runeforge_submit` sends the `Idempotency-Key` of the upload, so a retry after a timeout cannot
  create a second post or release.
- Publish is a separate call after the create. A refused publish is part of a successful
  outcome: the post and the release exist, and the outcome carries the checklist.
- The stamp is written after the create succeeds and before publish runs. A failed stamp write is
  reported in the outcome and does not undo the release.

### The frontend

- A new module `src/modules/runeforge/` holds the Settings section, the publish dialog and their
  query hooks. Its strings are in `messages/en/runeforge.json`, which is added to `pathPattern` in
  `project.inlang/settings.json` before `common.json`.
- `RuneforgeSection.tsx` in Settings shows the connected account, its scopes and a missing-scope
  warning, plus Connect, Reconnect and Disconnect. The token field is a password input. It links
  to the RuneForge token page that #435 adds.
- "Publish to RuneForge" is added to `ProjectActions`, `ProjectCardMenuItems` and the project
  editor. Without a connection it opens the Settings section instead.
- The publish dialog uses `Dialog.Shell` and a `createDialogStore` mounted by the module's
  `<ModuleDialogs />` (ADR-0033). It has four states:
  - Preparing: the stage and the byte progress, and Cancel
  - Review: the target (the stamped post, the suggestion, an owned post or a new post), the tag
    with its source, the notes, the game version, the post fields to refresh (release only),
    "Set mod status to Working" (release only, on by default), "Publish now" (new post only, shown
    when the token has `publish`), the validation warnings and the server findings
  - Submitting: the create step, then the publish step
  - Done: "Open on RuneForge", the publish result or the checklist with a link to `reviewPath`,
    the AI content question when `ai_disclosure` is missing, the stamp result and the scan
    status of the artifact with "Check again"
- Closing the dialog in Review discards the upload.
- `docs/ux/WORKSHOP.md` gets a "Publishing to RuneForge" section, and `docs/PRIVACY_PAGE.md`
  states which requests the app sends to RuneForge and when.

### What the server added

PR #457 and PR #459 closed every item that this spec asked of the server:

1. `GET /api/v1/publishing-context` answers the game versions and the categories for a token.
2. `DELETE /api/v1/uploads/:id` accepts a token with `releases:write`.
3. The token page is `/user-settings/api-tokens`. A review is requested on the post page that
   `checklist.reviewPath` names, so there is no separate review page.
4. The `.fantome` inspector reads `Distributor` from `info.json`.

## Testing Decisions

A good test checks what the client sends and how it reads each answer, through the crate's public
API or a command. It does not test `reqwest`, the credential store or React internals.

- **Client crate unit tests** in `crates/ltk-manager-runeforge/src/client/tests.rs` run against a
  `TcpListener` server in the style of `patcher/delivery/tests.rs`:
  - each call sends the bearer header to the API host
  - the PUT sends exactly `putHeaders`, the file length and no `Authorization`
  - each error `code` maps to its `RuneforgeError` variant, with findings, the suggested tag, the
    scope, the checklist and `Retry-After` kept
  - a retried submit sends the same `Idempotency-Key`
  - progress events report increasing `bytes_sent` that ends at `total_bytes`
  - cancel during the PUT stops the request and discards the upload
- **Pure unit tests** over `ApiToken` (valid, missing prefix, bad character, `Debug` and
  `Display` redaction) and over the target choice (stamped and owned, stamped and not owned,
  suggested by name, none).
- **Workshop tests** over the stamp write: it adds `distributor` to a `mod.config.json`, keeps
  every other field, and a second pack carries the stamp into the `.modpkg` metadata.
- **Frontend Vitest** over the publish dialog with `src/test/mocks/tauri.ts`: each state renders,
  a `tag_taken` failure fills the suggested tag, a refused publish shows the checklist, and
  closing in Review calls `runeforge_discard`.
- A test against the real site was considered and not chosen. It needs an account and the
  network, and the contract is fixed by the server's own integration tests.

## Delivery

Each line is a candidate issue in this repository unless it names another one:

1. `league-mod`: `distributor` in `ModProject`, carried into the modpkg metadata, released and
   bumped here. Done upstream in league-mod PR #312 and PR #313.
2. `runeforge-io/runeforge`: #435, plus the items in "What the server added". Done in PR #457 and
   PR #459.
3. `ltk-manager-runeforge` crate: wire types, client, `ApiToken`, `RuneforgeError`, tests against
   a local server. This can start before #435 merges, from the contract above.
4. Token storage and the connect commands, the ADR, and the Settings section.
5. `runeforge_prepare` with progress and cancel, and `runeforge_discard`.
6. `runeforge_submit`, the stamp write, the publish call and `runeforge_release`.
7. The publish dialog, the entry points, the messages and the UX and privacy docs.

Items 3 to 7 depend on item 1 for the stamp and on item 2 to run against a real site.

## Out of Scope

- The deep-link connect flow (`ltk://connect?token=...&state=...`) that
  `docs/PACKAGE_PUBLISHING.md` section 5.7 lists as the second connection method. The paste flow
  ships first, and the credential store is the same for both.
- Several packages in one release. The server accepts up to 20 uploads per release, and the
  first version sends one.
- Editing a post's gallery, links, members or license text from the app.
- A command line client. It uses the same contract and would reuse the client crate.
- Background polling of the scan. The dialog reads the release once when it opens Done, and again
  when the user asks.

## Further Notes

- `ltk://install` already trusts `runeforge.dev` by default (`src-tauri/src/state.rs`). Publishing
  does not change install behaviour.
- `download_mod_file` calls bare `reqwest::blocking::get` with no User-Agent and no timeout. The
  RuneForge client does not reuse it.
- Two other secrets are stored in plain files today: `Settings.telemetry_secret` in
  `settings.json` and the patcher delivery bearer token in `delivery.json`. The ADR for the
  RuneForge token can decide if they move to the credential store as well.
- Server design context: `docs/PACKAGE_PUBLISHING.md` in `runeforge-io/runeforge`, sections 4.4,
  5.2, 5.6 and 5.7.
