use super::*;
use crate::state::Theme;

fn current() -> Settings {
    let mut settings = Settings {
        first_run_complete: true,
        telemetry_secret: Some("this-machine".to_owned()),
        ..Settings::default()
    };
    settings.config.mod_storage_path = Some(PathBuf::from("D:/LTK"));
    settings.config.league_path = Some(PathBuf::from("C:/Riot Games/League of Legends"));
    settings
}

fn write_backup(dir: &Path, envelope: &Value) -> PathBuf {
    let path = dir.join("backup.json");
    fs::write(&path, serde_json::to_vec(envelope).unwrap()).unwrap();
    path
}

fn envelope(version: u32, settings: Value) -> Value {
    serde_json::json!({
        "format": FORMAT,
        "version": version,
        "appVersion": "1.0.0",
        "settings": settings,
    })
}

#[test]
fn export_leaves_out_the_diagnostics_secret() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("backup.json");

    export(&current(), &path).unwrap();

    let written: Envelope = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(written.format, FORMAT);
    assert_eq!(written.settings["telemetrySecret"], Value::Null);
}

#[test]
fn restore_round_trips_an_export() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("backup.json");
    let mut exported = current();
    exported.theme = Theme::Light;
    exported.minimize_to_tray = false;
    export(&exported, &path).unwrap();

    let restored = restore(&current(), &path).unwrap();

    assert_eq!(restored.theme, Theme::Light);
    assert!(!restored.minimize_to_tray);
}

#[test]
fn restore_keeps_what_this_machine_keeps() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_backup(
        dir.path(),
        &envelope(
            1,
            serde_json::json!({
                "modStoragePath": "E:/Other",
                "telemetrySecret": "other-machine",
                "firstRunComplete": false,
                "leaguePath": "Z:/Missing/League of Legends",
                "autoRun": true,
            }),
        ),
    );

    let restored = restore(&current(), &path).unwrap();

    assert_eq!(
        restored.config.mod_storage_path,
        current().config.mod_storage_path
    );
    assert_eq!(restored.telemetry_secret.as_deref(), Some("this-machine"));
    assert!(restored.first_run_complete);
    assert_eq!(restored.config.league_path, current().config.league_path);
    assert!(restored.auto_run);
}

#[test]
fn restore_takes_a_path_that_exists_on_this_machine() {
    let dir = tempfile::tempdir().unwrap();
    let workshop = dir.path().join("workshop");
    fs::create_dir_all(&workshop).unwrap();
    let path = write_backup(
        dir.path(),
        &envelope(1, serde_json::json!({ "workshopPath": workshop })),
    );

    let restored = restore(&current(), &path).unwrap();

    assert_eq!(restored.config.workshop_path, Some(workshop));
}

#[test]
fn restore_ignores_unknown_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_backup(
        dir.path(),
        &envelope(
            1,
            serde_json::json!({ "settingFromTheFuture": 3, "showModTags": false }),
        ),
    );

    let restored = restore(&current(), &path).unwrap();

    assert!(!restored.show_mod_tags);
}

#[test]
fn restore_refuses_file_that_is_not_a_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, serde_json::to_vec(&current()).unwrap()).unwrap();

    assert_eq!(
        restore(&current(), &path).unwrap_err(),
        SettingsBackupError::NotABackup
    );
}

#[test]
fn restore_refuses_backup_from_newer_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_backup(dir.path(), &envelope(VERSION + 1, serde_json::json!({})));

    assert_eq!(
        restore(&current(), &path).unwrap_err(),
        SettingsBackupError::NewerVersion {
            version: VERSION + 1,
            supported: VERSION
        }
    );
}
