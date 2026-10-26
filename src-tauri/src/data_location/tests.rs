use super::*;

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

/// A data directory with a library, a cache of the shell and one file that is not data.
fn data_folder() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("library.json"), b"{}");
    write(&dir.path().join("mods/a/mod.config.json"), b"abcd");
    write(&dir.path().join(MOD_PREVIEWS).join("a/skin.bin"), b"xy");
    write(&dir.path().join("settings.json"), b"{}");
    dir
}

#[test]
fn plan_lists_data_entries_and_their_bytes() {
    let from = data_folder();
    let to = tempfile::tempdir().unwrap();

    let plan = plan(from.path(), to.path()).unwrap();

    assert_eq!(plan.kind, DataMoveKind::Move);
    let names: Vec<_> = plan
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert_eq!(names, ["library.json", MOD_PREVIEWS, "mods"]);
    assert_eq!(plan.bytes(), 8);
}

#[test]
fn plan_adopts_folder_with_library_if_current_folder_has_none() {
    let from = tempfile::tempdir().unwrap();
    write(&from.path().join(SHADERS).join("a.glsl"), b"x");
    let to = data_folder();

    let plan = plan(from.path(), to.path()).unwrap();

    assert_eq!(plan.kind, DataMoveKind::Adopt);
    assert!(plan.entries.is_empty());
}

#[test]
fn plan_refuses_folder_that_holds_data_entries() {
    let from = data_folder();
    let to = tempfile::tempdir().unwrap();
    write(&to.path().join("mods/b.modpkg"), b"x");

    let error = plan(from.path(), to.path()).unwrap_err();

    assert_eq!(
        error,
        DataMoveError::Occupied {
            entries: vec!["mods".to_owned()]
        }
    );
}

#[test]
fn plan_refuses_same_folder() {
    let from = data_folder();

    assert_eq!(
        plan(from.path(), from.path()).unwrap_err(),
        DataMoveError::SameFolder
    );
}

#[test]
fn plan_refuses_folder_inside_data_that_moves() {
    let from = data_folder();

    assert_eq!(
        plan(from.path(), &from.path().join("mods/inner")).unwrap_err(),
        DataMoveError::InsideData
    );
}

#[test]
fn plan_accepts_folder_inside_current_folder_outside_data() {
    let from = data_folder();

    let plan = plan(from.path(), &from.path().join("LTK")).unwrap();

    assert_eq!(plan.kind, DataMoveKind::Move);
}

#[test]
fn run_moves_data_entries_and_keeps_other_files() {
    let from = data_folder();
    let to = tempfile::tempdir().unwrap();
    let target = to.path().join("LTK");
    let plan = plan(from.path(), &target).unwrap();

    run(&plan, |_, _| {}).unwrap();

    assert_eq!(
        fs::read(target.join("mods/a/mod.config.json")).unwrap(),
        b"abcd"
    );
    assert!(target.join(MOD_PREVIEWS).join("a/skin.bin").is_file());
    assert!(target.join("library.json").is_file());
    assert!(!from.path().join("mods").exists());
    assert!(!from.path().join("library.json").exists());
    assert!(from.path().join("settings.json").is_file());
}

#[test]
fn copy_entry_reports_bytes_of_each_file() {
    let from = data_folder();
    let to = tempfile::tempdir().unwrap();
    let mut copied = Vec::new();

    copy_entry(
        &from.path().join("mods"),
        &to.path().join("mods"),
        |bytes| copied.push(bytes),
    )
    .unwrap();

    assert_eq!(copied, [4]);
    assert!(to.path().join("mods/a/mod.config.json").is_file());
}

#[test]
fn run_returns_moved_entries_if_an_entry_fails() {
    let from = data_folder();
    let to = tempfile::tempdir().unwrap();
    let plan = plan(from.path(), to.path()).unwrap();
    fs::remove_dir_all(from.path().join("mods")).unwrap();

    assert!(matches!(
        run(&plan, |_, _| {}),
        Err(DataMoveError::Io { .. })
    ));

    assert!(from.path().join("library.json").is_file());
    assert!(from.path().join(MOD_PREVIEWS).join("a/skin.bin").is_file());
    assert!(!to.path().join("library.json").exists());
}
