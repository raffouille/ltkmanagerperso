use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;

use assert_matches::assert_matches;

use super::*;
use crate::wire::{FindingSeverity, PostField, PublishPath, PublishRequirement, Scope};

const TOKEN: &str = "rf_abcdEFGH0123456789abcdefghijklmnopqrstuvwxyz0";

/// One request that the test server read.
#[derive(Debug, Clone)]
struct Request {
    method: String,
    path: String,
    query: Option<String>,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).unwrap()
    }
}

/// What the test server answers to one request.
struct Answer {
    status: u16,
    headers: Vec<(&'static str, String)>,
    body: String,
}

impl Answer {
    fn json(status: u16, body: serde_json::Value) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.to_string(),
        }
    }

    fn error(status: u16, error: serde_json::Value) -> Self {
        Self::json(status, serde_json::json!({ "error": error }))
    }
}

type Handler = dyn Fn(&Request) -> Answer + Send + Sync;

/// A local HTTP server that answers through `handler` and records every request.
struct Server {
    url: Url,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl Server {
    fn start(handler: impl Fn(&Request) -> Answer + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));

        let log = Arc::clone(&requests);
        let handler: Arc<Handler> = Arc::new(handler);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                answer(stream, handler.as_ref(), &log);
            }
        });

        Self { url, requests }
    }

    fn client(&self) -> Client {
        Client::new(self.url.clone(), ApiToken::new(TOKEN).unwrap(), "1.2.3").unwrap()
    }

    fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}

fn answer(stream: TcpStream, handler: &Handler, log: &Mutex<Vec<Request>>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    if reader.read_line(&mut line).unwrap_or(0) == 0 {
        return;
    }

    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap().to_owned();
    let target = parts.next().unwrap().to_owned();
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path.to_owned(), Some(query.to_owned())),
        None => (target, None),
    };

    let mut headers = BTreeMap::new();
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).unwrap();
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }

        let (name, value) = header.split_once(':').unwrap();
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
    }

    let length: usize = headers
        .get("content-length")
        .map_or(0, |length| length.parse().unwrap());
    let mut body = vec![0; length];
    if reader.read_exact(&mut body).is_err() {
        return;
    }

    let request = Request {
        method,
        path,
        query,
        headers,
        body,
    };
    let answer = handler(&request);
    log.lock().unwrap().push(request);

    let mut stream = stream;
    let mut head = format!(
        "HTTP/1.1 {} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        answer.status,
        answer.body.len()
    );
    for (name, value) in &answer.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");

    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(answer.body.as_bytes());
}

fn account_json() -> serde_json::Value {
    serde_json::json!({
        "userId": "user-1",
        "username": "",
        "displayName": "Creator",
        "permissions": ["publish_mods"],
        "scopes": ["mods:read", "releases:write", "something:new"],
        "token": { "id": "token-1" }
    })
}

fn prefill_json() -> serde_json::Value {
    serde_json::json!({
        "post": {
            "name": "My Mod",
            "shortDescription": null,
            "description": "Long",
            "category": "skin",
            "license": { "spdx": "MIT" },
            "thumbnailImageId": null,
            "championIds": [1],
            "mapIds": [],
            "themes": [],
            "features": []
        },
        "release": {
            "tag": "v1.0.0",
            "notes": "Notes",
            "gameVersionId": null,
            "artifactLabel": "My Mod"
        },
        "sources": {
            "post": { "name": "package" },
            "release": {
                "tag": "package",
                "notes": "default",
                "gameVersionId": "current-post",
                "artifactLabel": "package"
            }
        },
        "findings": [],
        "suggestedTag": null,
        "changedPostFields": [
            { "field": "description", "current": null, "proposed": "Long" }
        ],
        "leftovers": { "tags": [], "champions": [], "maps": [] }
    })
}

fn created_upload_json(url: &Url) -> serde_json::Value {
    serde_json::json!({
        "upload": { "id": "upload-1", "filename": "my-mod_1.0.0.modpkg", "status": "pending", "format": null },
        "presignedUrl": format!("{url}storage/object?X-Amz-Signature=abc"),
        "putHeaders": {
            "Content-Type": "application/octet-stream",
            "Content-Disposition": "attachment; filename=\"my-mod_1.0.0.modpkg\""
        }
    })
}

#[test]
fn me_sends_bearer_token_and_reads_account() {
    let server = Server::start(|_| Answer::json(200, account_json()));

    let account = server.client().me().unwrap();

    assert_eq!(account.user_id, "user-1");
    assert_eq!(account.username, "");
    assert_eq!(account.display_name.as_deref(), Some("Creator"));
    assert_eq!(
        account.scopes,
        vec![Scope::ModsRead, Scope::ReleasesWrite, Scope::Unknown]
    );

    let request = &server.requests()[0];
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/api/v1/me");
    assert_eq!(
        request.header("authorization"),
        Some(format!("Bearer {TOKEN}").as_str())
    );
    assert_eq!(request.header("accept"), Some("application/json"));
    assert_eq!(request.header("user-agent"), Some("ltk-manager/1.2.3"));
    assert_eq!(request.header("origin"), None);
}

#[test]
fn owned_mods_reads_wrapped_posts() {
    let server = Server::start(|_| {
        Answer::json(
            200,
            serde_json::json!({ "mods": [{
                "id": "mod-1",
                "name": "My Mod",
                "visibility": "private",
                "isPublished": false,
                "updatedAt": "2026-10-27T00:00:00Z",
                "latestRelease": { "id": "rel-1", "tag": "v1.0.0", "createdAt": "2026-10-26T00:00:00Z" }
            }] }),
        )
    });

    let mods = server.client().owned_mods().unwrap();

    assert_eq!(mods.len(), 1);
    assert_eq!(mods[0].latest_release.as_ref().unwrap().tag, "v1.0.0");
    assert_eq!(server.requests()[0].path, "/api/v1/me/mods");
}

#[test]
fn publishing_context_reads_game_versions_and_categories() {
    let server = Server::start(|_| {
        Answer::json(
            200,
            serde_json::json!({
                "ownedPosts": [{ "id": "mod-1", "name": "My Mod", "latestTag": "v1.0.0" }],
                "champions": [],
                "maps": [],
                "gameVersions": [{ "id": "gv-2", "version": "25.21" }, { "id": "gv-1", "version": "25.20" }],
                "canPublishWithoutReview": true,
                "categories": ["skin", "map"],
                "themes": [],
                "features": []
            }),
        )
    });

    let context = server.client().publishing_context().unwrap();

    assert_eq!(context.game_versions[0].version, "25.21");
    assert_eq!(context.categories, vec!["skin", "map"]);
    assert!(context.can_publish_without_review);
}

#[test]
fn create_upload_posts_filename_and_size() {
    let server = Server::start(|request| Answer::json(201, created_upload_json(&base_of(request))));

    let created = server
        .client()
        .create_upload("my-mod_1.0.0.modpkg", 1234)
        .unwrap();

    assert_eq!(created.upload.id, "upload-1");
    assert_eq!(created.put_headers.content_type, "application/octet-stream");

    let request = &server.requests()[0];
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/api/v1/uploads");
    assert_eq!(request.header("content-type"), Some("application/json"));
    assert_eq!(
        request.json(),
        serde_json::json!({ "filename": "my-mod_1.0.0.modpkg", "size": 1234 })
    );
}

/// The base URL that a request reached the server at.
fn base_of(request: &Request) -> Url {
    Url::parse(&format!("http://{}/", request.header("host").unwrap())).unwrap()
}

#[test]
fn put_package_sends_signed_headers_and_no_authorization() {
    let server = Server::start(|_| Answer::json(200, serde_json::json!({})));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("my-mod_1.0.0.modpkg");
    fs::write(&path, vec![7u8; 300_000]).unwrap();
    let created: CreatedUpload = serde_json::from_value(created_upload_json(&server.url)).unwrap();

    server
        .client()
        .put_package(&created, &path, |_| {}, Arc::new(AtomicBool::new(false)))
        .unwrap();

    let request = &server.requests()[0];
    assert_eq!(request.method, "PUT");
    assert_eq!(request.path, "/storage/object");
    assert_eq!(request.query.as_deref(), Some("X-Amz-Signature=abc"));
    assert_eq!(request.header("authorization"), None);
    assert_eq!(
        request.header("content-type"),
        Some("application/octet-stream")
    );
    assert_eq!(
        request.header("content-disposition"),
        Some("attachment; filename=\"my-mod_1.0.0.modpkg\"")
    );
    assert_eq!(request.header("content-length"), Some("300000"));
    assert_eq!(request.body.len(), 300_000);
}

#[test]
fn put_package_reports_increasing_progress_that_ends_at_file_length() {
    let server = Server::start(|_| Answer::json(200, serde_json::json!({})));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("package.modpkg");
    fs::write(&path, vec![1u8; 200_000]).unwrap();
    let created: CreatedUpload = serde_json::from_value(created_upload_json(&server.url)).unwrap();
    let reports = Arc::new(Mutex::new(Vec::new()));

    let log = Arc::clone(&reports);
    server
        .client()
        .put_package(
            &created,
            &path,
            move |sent| log.lock().unwrap().push(sent),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();

    let reports = reports.lock().unwrap();
    assert!(reports.windows(2).all(|pair| pair[0] <= pair[1]));
    assert_eq!(reports.last(), Some(&200_000));
}

#[test]
fn put_package_fails_with_cancelled_once_cancel_is_set() {
    let server = Server::start(|_| Answer::json(200, serde_json::json!({})));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("package.modpkg");
    fs::write(&path, vec![1u8; 2_000_000]).unwrap();
    let created: CreatedUpload = serde_json::from_value(created_upload_json(&server.url)).unwrap();
    let cancel = Arc::new(AtomicBool::new(false));

    let flag = Arc::clone(&cancel);
    let result = server.client().put_package(
        &created,
        &path,
        move |sent| {
            if sent > 100_000 {
                flag.store(true, Ordering::Relaxed);
            }
        },
        Arc::clone(&cancel),
    );

    assert_matches!(result, Err(RuneforgeError::Cancelled));
}

#[test]
fn put_package_fails_with_storage_status() {
    let server = Server::start(|_| Answer::json(403, serde_json::json!({})));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("package.modpkg");
    fs::write(&path, b"bytes").unwrap();
    let created: CreatedUpload = serde_json::from_value(created_upload_json(&server.url)).unwrap();

    let result =
        server
            .client()
            .put_package(&created, &path, |_| {}, Arc::new(AtomicBool::new(false)));

    assert_matches!(result, Err(RuneforgeError::Storage { status: 403 }));
}

#[test]
fn complete_upload_reads_review() {
    let server = Server::start(|_| {
        Answer::json(
            200,
            serde_json::json!({
                "upload": { "id": "upload-1", "filename": "a.modpkg", "status": "inspected", "format": "modpkg" },
                "inspection": { "anything": true },
                "findings": [{ "severity": "warning", "code": "no_readme", "message": "No readme" }],
                "suggestions": { "modId": "mod-1", "matchedBy": "distributor", "nextTag": "v1.0.1" },
                "prefill": prefill_json()
            }),
        )
    });

    let review = server.client().complete_upload("upload-1").unwrap();

    assert_eq!(review.findings[0].severity, FindingSeverity::Warning);
    assert_eq!(review.suggestions.mod_id.as_deref(), Some("mod-1"));
    assert_eq!(review.prefill.release.tag, "v1.0.0");
    assert_eq!(
        review.prefill.changed_post_fields[0].field,
        PostField::Description
    );
    assert_eq!(
        server.requests()[0].path,
        "/api/v1/uploads/upload-1/complete"
    );
}

#[test]
fn prefill_sends_mod_id_query() {
    let server = Server::start(|_| Answer::json(200, prefill_json()));

    server.client().prefill("upload-1", Some("mod-1")).unwrap();
    server.client().prefill("upload-1", None).unwrap();

    let requests = server.requests();
    assert_eq!(requests[0].path, "/api/v1/uploads/upload-1/prefill");
    assert_eq!(requests[0].query.as_deref(), Some("modId=mod-1"));
    assert_eq!(requests[1].query, None);
}

#[test]
fn discard_upload_sends_delete() {
    let server = Server::start(|_| Answer {
        status: 204,
        headers: Vec::new(),
        body: String::new(),
    });

    server.client().discard_upload("upload-1").unwrap();

    let request = &server.requests()[0];
    assert_eq!(request.method, "DELETE");
    assert_eq!(request.path, "/api/v1/uploads/upload-1");
}

#[test]
fn create_release_sends_same_idempotency_key_on_retry() {
    let server = Server::start(|_| {
        Answer::json(
            201,
            serde_json::json!({
                "modId": "mod-1",
                "releaseId": "rel-2",
                "tag": "v1.0.1",
                "artifacts": [],
                "appliedPostChanges": [],
                "postUpdateError": null
            }),
        )
    });
    let release = NewRelease {
        upload_ids: vec!["upload-1".to_owned()],
        tag: Some("v1.0.1".to_owned()),
        description: None,
        game_version: Some("gv-2".to_owned()),
        update_post: vec![PostField::Description],
        mark_as_working: true,
    };

    let client = server.client();
    client.create_release("mod-1", &release).unwrap();
    client.create_release("mod-1", &release).unwrap();

    let requests = server.requests();
    assert_eq!(requests[0].path, "/api/v1/mods/mod-1/releases");
    assert_eq!(
        requests[0].header("idempotency-key"),
        Some("ltk-manager-upload-1")
    );
    assert_eq!(
        requests[0].header("idempotency-key"),
        requests[1].header("idempotency-key")
    );
    assert_eq!(
        requests[0].json(),
        serde_json::json!({
            "uploadIds": ["upload-1"],
            "tag": "v1.0.1",
            "gameVersion": "gv-2",
            "updatePost": ["description"],
            "markAsWorking": true
        })
    );
}

#[test]
fn create_mod_sends_idempotency_key_and_omits_empty_overrides() {
    let server = Server::start(|_| {
        Answer::json(
            201,
            serde_json::json!({ "modId": "mod-1", "releaseId": "rel-1" }),
        )
    });
    let new_mod = NewMod {
        upload_ids: vec!["upload-1".to_owned()],
        post: crate::wire::PostOverrides::default(),
        release: crate::wire::ReleaseOverrides {
            tag: Some("v2.0.0".to_owned()),
            ..Default::default()
        },
    };

    let created = server.client().create_mod(&new_mod).unwrap();

    assert_eq!(created.mod_id, "mod-1");
    let request = &server.requests()[0];
    assert_eq!(request.path, "/api/v1/mods");
    assert_eq!(
        request.header("idempotency-key"),
        Some("ltk-manager-upload-1")
    );
    assert_eq!(
        request.json(),
        serde_json::json!({ "uploadIds": ["upload-1"], "release": { "tag": "v2.0.0" } })
    );
}

#[test]
fn publish_sends_ai_answer() {
    let server = Server::start(|_| {
        Answer::json(
            200,
            serde_json::json!({ "modId": "mod-1", "isPublished": true }),
        )
    });

    server
        .client()
        .publish(
            "mod-1",
            &PublishAnswer {
                has_ai_generated_post_content: Some(false),
                expected_version: Some("3".to_owned()),
            },
        )
        .unwrap();

    let request = &server.requests()[0];
    assert_eq!(request.path, "/api/v1/mods/mod-1/publish");
    assert_eq!(
        request.json(),
        serde_json::json!({ "hasAiGeneratedPostContent": false, "expectedVersion": "3" })
    );
}

#[test]
fn release_reads_artifact_health() {
    let server = Server::start(|_| {
        Answer::json(
            200,
            serde_json::json!({
                "id": "rel-1",
                "modId": "mod-1",
                "tag": "v1.0.0",
                "artifacts": [
                    { "id": "a-1", "filename": "a.modpkg", "format": "modpkg", "contentHash": null, "health": null },
                    { "id": "a-2", "filename": "b.modpkg", "format": "modpkg", "contentHash": "ff",
                      "health": { "status": "warnings", "findings": [], "scannedAt": "2026-10-27T00:00:00Z" } },
                    { "id": "a-3", "filename": "c.modpkg", "format": null, "contentHash": null,
                      "health": { "status": "something-new" } }
                ]
            }),
        )
    });

    let release = server.client().release("mod-1", "rel-1").unwrap();

    assert_eq!(release.artifacts[0].health, None);
    assert_eq!(
        release.artifacts[1].health.as_ref().unwrap().status,
        crate::wire::HealthStatus::Warnings
    );
    assert_eq!(
        release.artifacts[2].health.as_ref().unwrap().status,
        crate::wire::HealthStatus::Pending
    );
    assert_eq!(
        server.requests()[0].path,
        "/api/v1/mods/mod-1/releases/rel-1"
    );
}

/// Answers every request with `answer`, and returns the error that `me` fails with.
fn me_error(answer: impl Fn() -> Answer + Send + Sync + 'static) -> RuneforgeError {
    let server = Server::start(move |_| answer());
    server.client().me().unwrap_err()
}

#[test]
fn unauthorized_code_maps_to_unauthorized() {
    let error = me_error(|| {
        Answer::error(
            401,
            serde_json::json!({ "code": "unauthorized", "message": "revoked" }),
        )
    });

    assert_eq!(error, RuneforgeError::Unauthorized);
}

#[test]
fn forbidden_code_keeps_missing_scope() {
    let error = me_error(|| {
        Answer::error(
            403,
            serde_json::json!({ "code": "forbidden", "message": "no", "scope": "publish" }),
        )
    });

    assert_eq!(
        error,
        RuneforgeError::Forbidden {
            scope: Some(Scope::Publish)
        }
    );
}

#[test]
fn forbidden_code_without_scope_maps_to_account_refusal() {
    let error = me_error(|| {
        Answer::error(
            403,
            serde_json::json!({ "code": "forbidden", "message": "banned" }),
        )
    });

    assert_eq!(error, RuneforgeError::Forbidden { scope: None });
}

#[test]
fn package_rejected_code_keeps_findings() {
    let error = me_error(|| {
        Answer::error(
            422,
            serde_json::json!({
                "code": "package_rejected",
                "message": "no",
                "findings": [{ "severity": "error", "code": "bad_wad", "message": "Bad", "path": "a.wad" }]
            }),
        )
    });

    assert_matches!(error, RuneforgeError::PackageRejected { findings } => {
        assert_eq!(findings[0].code, "bad_wad");
        assert_eq!(findings[0].path.as_deref(), Some("a.wad"));
    });
}

#[test]
fn tag_taken_code_keeps_suggested_tag() {
    let error = me_error(|| {
        Answer::error(
            409,
            serde_json::json!({ "code": "tag_taken", "message": "no", "suggestedTag": "v1.0.2" }),
        )
    });

    assert_eq!(
        error,
        RuneforgeError::TagTaken {
            suggested_tag: "v1.0.2".to_owned()
        }
    );
}

#[test]
fn publish_refused_code_keeps_checklist() {
    let error = me_error(|| {
        Answer::error(
            409,
            serde_json::json!({
                "code": "publish_refused",
                "message": "no",
                "checklist": {
                    "path": "review",
                    "missing": ["thumbnail", "ai_disclosure"],
                    "aiContentDisclosureVersion": "3",
                    "aiModeratorEnforced": true,
                    "reviewPath": "/mods/mod-1"
                }
            }),
        )
    });

    assert_matches!(error, RuneforgeError::PublishRefused { checklist } => {
        assert_eq!(checklist.path, PublishPath::Review);
        assert_eq!(
            checklist.missing,
            vec![PublishRequirement::Thumbnail, PublishRequirement::AiDisclosure]
        );
        assert_eq!(checklist.ai_content_disclosure_version, "3");
        assert!(checklist.ai_moderator_enforced);
        assert_eq!(checklist.review_path.as_deref(), Some("/mods/mod-1"));
    });
}

#[test]
fn too_many_requests_code_keeps_retry_after() {
    let error = me_error(|| Answer {
        status: 429,
        headers: vec![("Retry-After", "42".to_owned())],
        body: serde_json::json!({ "error": { "code": "too_many_requests", "message": "slow" } })
            .to_string(),
    });

    assert_eq!(
        error,
        RuneforgeError::RateLimited {
            retry_after: Some(42)
        }
    );
}

#[test]
fn state_codes_map_to_their_variants() {
    let upload_state = me_error(|| {
        Answer::error(
            409,
            serde_json::json!({ "code": "upload_state", "message": "x" }),
        )
    });
    let conflict = me_error(|| {
        Answer::error(
            409,
            serde_json::json!({ "code": "conflict", "message": "x" }),
        )
    });
    let too_large = me_error(|| {
        Answer::error(
            413,
            serde_json::json!({ "code": "payload_too_large", "message": "x" }),
        )
    });
    let not_found = me_error(|| {
        Answer::error(
            404,
            serde_json::json!({ "code": "not_found", "message": "x" }),
        )
    });

    assert_eq!(upload_state, RuneforgeError::UploadState);
    assert_eq!(conflict, RuneforgeError::Conflict);
    assert_eq!(too_large, RuneforgeError::TooLarge);
    assert_eq!(not_found, RuneforgeError::NotFound);
}

#[test]
fn unknown_code_maps_to_http_with_status_and_code() {
    let error = me_error(|| {
        Answer::error(
            500,
            serde_json::json!({ "code": "internal_error", "message": "x" }),
        )
    });

    assert_eq!(
        error,
        RuneforgeError::Http {
            status: 500,
            code: "internal_error".to_owned()
        }
    );
}

#[test]
fn body_without_error_shape_maps_by_status() {
    let error = me_error(|| Answer {
        status: 502,
        headers: Vec::new(),
        body: "<html>bad gateway</html>".to_owned(),
    });

    assert_eq!(
        error,
        RuneforgeError::Http {
            status: 502,
            code: "unknown".to_owned()
        }
    );
}

#[test]
fn success_with_unreadable_body_maps_to_malformed() {
    let error = me_error(|| Answer::json(200, serde_json::json!({ "unexpected": true })));

    assert_matches!(error, RuneforgeError::Malformed { .. });
}

#[test]
fn unreachable_server_maps_to_offline() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    drop(listener);

    let client = Client::new(url, ApiToken::new(TOKEN).unwrap(), "1.2.3").unwrap();

    assert_matches!(client.me(), Err(RuneforgeError::Offline { .. }));
}

#[test]
fn site_url_joins_review_path_to_base() {
    let base = Url::parse("https://runeforge.dev").unwrap();

    assert_eq!(
        site_url(&base, "/mods/mod-1").as_str(),
        "https://runeforge.dev/mods/mod-1"
    );
}
