//! Unit tests for the layer list, and for adding files to a layer and removing them.

use super::*;
use ltk_manager_base::error::AppError;

fn make_project_with_layers(dir: &std::path::Path, layers: Vec<ModProjectLayer>) {
    let mut mod_project = ltk_mod_project::ModProject::default();
    mod_project.name = "test-mod".to_string();
    mod_project.display_name = "Test Mod".to_string();
    mod_project.version = "1.0.0".to_string();
    mod_project.layers = layers;
    fs::write(
        dir.join("mod.config.json"),
        serde_json::to_string_pretty(&mod_project).unwrap(),
    )
    .unwrap();
}

fn load_layers(dir: &std::path::Path) -> Vec<ModProjectLayer> {
    ltk_mod_project::ModProject::load(Utf8Path::from_path(dir).unwrap())
        .unwrap()
        .layers
}

#[test]
fn create_layer_adds_to_config() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        ltk_mod_project::ModProjectLayer::default_table(),
    );

    let project = ProjectDir::open(dir.path())
        .unwrap()
        .create_layer("chroma", None, None)
        .unwrap();

    assert_eq!(project.layers.len(), 2);
    assert_eq!(project.layers[1].name, "chroma");
    assert_eq!(project.layers[1].priority, 1);

    let layers = load_layers(dir.path());
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[1].name, "chroma");

    let chroma_content_dir = dir.path().join("content").join("chroma");
    assert!(chroma_content_dir.exists());
}

#[test]
fn create_layer_accepts_underscores() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        ltk_mod_project::ModProjectLayer::default_table(),
    );

    let project = ProjectDir::open(dir.path())
        .unwrap()
        .create_layer("item_shop", None, None)
        .unwrap();

    assert_eq!(project.layers[1].name, "item_shop");
}

#[test]
fn create_layer_invalid_name_rejected() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        ltk_mod_project::ModProjectLayer::default_table(),
    );

    assert!(matches!(
        ProjectDir::open(dir.path())
            .unwrap()
            .create_layer("Bad Name", None, None),
        Err(AppError::ValidationFailed(_))
    ));
    assert!(matches!(
        ProjectDir::open(dir.path())
            .unwrap()
            .create_layer("UPPER", None, None),
        Err(AppError::ValidationFailed(_))
    ));
}

#[test]
fn create_layer_duplicate_name_detected() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        ltk_mod_project::ModProjectLayer::default_table(),
    );

    assert!(matches!(
        ProjectDir::open(dir.path()).unwrap().create_layer("base", None, None),
        Err(AppError::ValidationFailed(msg)) if msg.contains("already exists")
    ));
}

#[test]
fn delete_base_layer_rejected() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        ltk_mod_project::ModProjectLayer::default_table(),
    );

    assert!(matches!(
        ProjectDir::open(dir.path()).unwrap().delete_layer("base"),
        Err(AppError::ValidationFailed(msg)) if msg.contains("base")
    ));
}

#[test]
fn delete_nonexistent_layer_detected() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        ltk_mod_project::ModProjectLayer::default_table(),
    );

    assert!(matches!(
        ProjectDir::open(dir.path()).unwrap().delete_layer("nonexistent"),
        Err(AppError::ValidationFailed(msg)) if msg.contains("not found")
    ));
}

#[test]
fn delete_layer_removes_from_config() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        vec![
            ModProjectLayer::base(),
            ModProjectLayer {
                name: "chroma".to_string(),
                display_name: None,
                priority: 1,
                description: None,
                string_overrides: IndexMap::new(),
            },
        ],
    );
    fs::create_dir_all(dir.path().join("content").join("chroma")).unwrap();

    let project = ProjectDir::open(dir.path())
        .unwrap()
        .delete_layer("chroma")
        .unwrap();

    assert_eq!(project.layers.len(), 1);
    assert_eq!(project.layers[0].name, "base");

    let layers = load_layers(dir.path());
    assert_eq!(layers.len(), 1);
    assert_eq!(layers[0].name, "base");
}

#[test]
fn reorder_layers_base_included_rejected() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        vec![
            ModProjectLayer::base(),
            ModProjectLayer {
                name: "chroma".to_string(),
                display_name: None,
                priority: 1,
                description: None,
                string_overrides: IndexMap::new(),
            },
        ],
    );

    let result = ProjectDir::open(dir.path())
        .unwrap()
        .reorder_layers(vec!["base".to_string(), "chroma".to_string()]);
    match result {
        Err(AppError::ValidationFailed(msg)) => {
            assert!(
                msg.to_lowercase().contains("base"),
                "expected 'base' in message, got: {msg}"
            );
        }
        other => panic!("expected ValidationFailed, got: {:?}", other),
    }
}

#[test]
fn reorder_layers_wrong_set_rejected() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        vec![
            ModProjectLayer::base(),
            ModProjectLayer {
                name: "chroma".to_string(),
                display_name: None,
                priority: 1,
                description: None,
                string_overrides: IndexMap::new(),
            },
            ModProjectLayer {
                name: "vfx".to_string(),
                display_name: None,
                priority: 2,
                description: None,
                string_overrides: IndexMap::new(),
            },
        ],
    );

    assert!(matches!(
        ProjectDir::open(dir.path())
            .unwrap()
            .reorder_layers(vec!["chroma".to_string(), "wrong".to_string()]),
        Err(AppError::ValidationFailed(_))
    ));
}

#[test]
fn reorder_layers_reassigns_priorities() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        vec![
            ModProjectLayer::base(),
            ModProjectLayer {
                name: "chroma".to_string(),
                display_name: None,
                priority: 1,
                description: None,
                string_overrides: IndexMap::new(),
            },
            ModProjectLayer {
                name: "vfx".to_string(),
                display_name: None,
                priority: 2,
                description: None,
                string_overrides: IndexMap::new(),
            },
        ],
    );

    let project = ProjectDir::open(dir.path())
        .unwrap()
        .reorder_layers(vec!["vfx".to_string(), "chroma".to_string()])
        .unwrap();

    assert_eq!(project.layers[0].name, "base");
    assert_eq!(project.layers[0].priority, 0);
    assert_eq!(project.layers[1].name, "vfx");
    assert_eq!(project.layers[1].priority, 1);
    assert_eq!(project.layers[2].name, "chroma");
    assert_eq!(project.layers[2].priority, 2);

    let layers = load_layers(dir.path());
    assert_eq!(layers[1].name, "vfx");
    assert_eq!(layers[1].priority, 1);
}

/// The key the `game` table would file this path under, asked of the table
/// itself so a test cannot disagree with it about the algorithm.
fn path_hash(path: &str) -> u64 {
    ltk_manager_assets::hashtables::Table::Game
        .key_config()
        .hash(path)
}

/// A resolver holding no tables, for the cases where naming is not what
/// is under test.
fn no_names() -> WadPathResolver {
    WadPathResolver::new(ltk_manager_assets::hashtables::LayeredHashDb::new())
}

fn build_test_wad(path: &std::path::Path, chunk_paths: &[&str]) {
    use ltk_wad::{WadBuilder, WadChunkBuilder};
    use std::io::Write;

    let mut builder = WadBuilder::default();
    for chunk_path in chunk_paths {
        builder = builder.with_chunk(WadChunkBuilder::default().with_path(*chunk_path));
    }

    let mut file = fs::File::create(path).unwrap();
    builder
        .build_to_writer(&mut file, |_path_hash, cursor| {
            cursor.write_all(&[0xAA; 64])?;
            Ok(())
        })
        .unwrap();
}

#[test]
fn extract_wad_into_dir_names_chunks_the_tables_know() {
    let path = "assets/characters/aatrox/aatrox.bin";
    let src_dir = tempfile::tempdir().unwrap();
    let src = src_dir.path().join("Aatrox.wad.client");
    build_test_wad(&src, &[path]);

    let mut db = ltk_manager_assets::hashtables::LayeredHashDb::new();
    db.insert(path_hash(path), path);

    let dst = tempfile::tempdir().unwrap();
    extract_wad_into_dir(&src, dst.path(), &WadPathResolver::new(db)).unwrap();

    assert!(
        dst.path().join(path).is_file(),
        "expected the chunk at its named path, found {:?}",
        fs::read_dir(dst.path()).unwrap().flatten().count()
    );
}

#[test]
fn extract_wad_into_dir_falls_back_to_hex_names() {
    let path = "assets/characters/aatrox/aatrox.bin";
    let src_dir = tempfile::tempdir().unwrap();
    let src = src_dir.path().join("Aatrox.wad.client");
    build_test_wad(&src, &[path]);

    let dst = tempfile::tempdir().unwrap();
    let empty = WadPathResolver::new(ltk_manager_assets::hashtables::LayeredHashDb::new());
    extract_wad_into_dir(&src, dst.path(), &empty).unwrap();

    let hex = format!("{:016x}", path_hash(path));
    let named: Vec<String> = fs::read_dir(dst.path())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        named.iter().any(|name| name.starts_with(&hex)),
        "expected a hex-named chunk, got {named:?}"
    );
}

#[test]
fn add_files_to_layer_extracts_wad_file() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        ltk_mod_project::ModProjectLayer::default_table(),
    );

    let src_dir = tempfile::tempdir().unwrap();
    let src_file = src_dir.path().join("Aatrox.wad.client");
    build_test_wad(&src_file, &["assets/test1.bin", "assets/test2.bin"]);

    let report = ProjectDir::open(dir.path())
        .unwrap()
        .add_files_to_layer("base", vec![src_file], &no_names())
        .unwrap();

    assert_eq!(report.added, vec!["Aatrox.wad.client".to_string()]);
    let dest = dir
        .path()
        .join("content")
        .join("base")
        .join("Aatrox.wad.client");
    assert!(
        dest.is_dir(),
        "expected extracted directory at {}",
        dest.display()
    );

    let extracted: Vec<_> = fs::read_dir(&dest)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        !extracted.is_empty(),
        "expected at least one extracted entry under {}",
        dest.display()
    );
}

#[test]
fn add_files_to_layer_copies_directory() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        ltk_mod_project::ModProjectLayer::default_table(),
    );

    let src_dir = tempfile::tempdir().unwrap();
    let wad_dir = src_dir.path().join("Champion.wad.client");
    fs::create_dir_all(wad_dir.join("nested")).unwrap();
    fs::write(wad_dir.join("meta.json"), "{}").unwrap();
    fs::write(wad_dir.join("nested").join("a.bin"), b"x").unwrap();

    let report = ProjectDir::open(dir.path())
        .unwrap()
        .add_files_to_layer("base", vec![wad_dir], &no_names())
        .unwrap();

    assert_eq!(report.added, vec!["Champion.wad.client".to_string()]);
    let dest = dir
        .path()
        .join("content")
        .join("base")
        .join("Champion.wad.client");
    assert!(dest.is_dir());
    assert!(dest.join("meta.json").is_file());
    assert!(dest.join("nested").join("a.bin").is_file());
}

#[test]
fn add_files_to_layer_rejects_non_wad() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        ltk_mod_project::ModProjectLayer::default_table(),
    );

    let src_dir = tempfile::tempdir().unwrap();
    let bad = src_dir.path().join("readme.txt");
    fs::write(&bad, b"hi").unwrap();

    let result =
        ProjectDir::open(dir.path())
            .unwrap()
            .add_files_to_layer("base", vec![bad], &no_names());
    assert!(matches!(result, Err(AppError::ValidationFailed(_))));
}

/// Lay out `<layer>/<relative>` for each entry, with the file's own name as
/// its content so a surviving file can be told from a resurrected one.
fn seed_layer(dir: &std::path::Path, layer: &str, entries: &[&str]) -> PathBuf {
    let layer_dir = dir.join("content").join(layer);
    for relative in entries {
        let path = layer_dir.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, relative.as_bytes()).unwrap();
    }
    layer_dir
}

#[test]
fn delete_layer_content_removes_one_file() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        ltk_mod_project::ModProjectLayer::default_table(),
    );
    let layer_dir = seed_layer(
        dir.path(),
        "base",
        &[
            "Aatrox.wad.client/data/a.bin",
            "Aatrox.wad.client/data/b.bin",
        ],
    );

    ProjectDir::open(dir.path())
        .unwrap()
        .delete_layer_content("base", "Aatrox.wad.client/data/a.bin")
        .unwrap();

    assert!(!layer_dir.join("Aatrox.wad.client/data/a.bin").exists());
    assert!(layer_dir.join("Aatrox.wad.client/data/b.bin").is_file());
}

#[test]
fn delete_layer_content_removes_a_directory_whole() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        ltk_mod_project::ModProjectLayer::default_table(),
    );
    let layer_dir = seed_layer(
        dir.path(),
        "base",
        &["Aatrox.wad.client/data/a.bin", "Sona.wad.client/data/b.bin"],
    );

    ProjectDir::open(dir.path())
        .unwrap()
        .delete_layer_content("base", "Aatrox.wad.client")
        .unwrap();

    assert!(!layer_dir.join("Aatrox.wad.client").exists());
    assert!(layer_dir.join("Sona.wad.client/data/b.bin").is_file());
}

#[test]
fn delete_layer_content_prunes_the_directories_it_empties() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        ltk_mod_project::ModProjectLayer::default_table(),
    );
    let layer_dir = seed_layer(dir.path(), "base", &["Aatrox.wad.client/data/deep/a.bin"]);

    ProjectDir::open(dir.path())
        .unwrap()
        .delete_layer_content("base", "Aatrox.wad.client/data/deep/a.bin")
        .unwrap();

    assert!(!layer_dir.join("Aatrox.wad.client").exists());
    assert!(layer_dir.is_dir(), "the layer itself must survive");
}

#[test]
fn delete_layer_content_keeps_a_parent_that_still_holds_something() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        ltk_mod_project::ModProjectLayer::default_table(),
    );
    let layer_dir = seed_layer(
        dir.path(),
        "base",
        &[
            "Aatrox.wad.client/data/a.bin",
            "Aatrox.wad.client/meta.json",
        ],
    );

    ProjectDir::open(dir.path())
        .unwrap()
        .delete_layer_content("base", "Aatrox.wad.client/data/a.bin")
        .unwrap();

    assert!(!layer_dir.join("Aatrox.wad.client/data").exists());
    assert!(layer_dir.join("Aatrox.wad.client/meta.json").is_file());
}

#[test]
fn delete_layer_content_rejects_an_escaping_path() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        ltk_mod_project::ModProjectLayer::default_table(),
    );
    seed_layer(dir.path(), "base", &["Aatrox.wad.client/a.bin"]);
    let outsider = dir.path().join("mod.config.json");

    for path in [
        "../../mod.config.json",
        "Aatrox.wad.client/../../../mod.config.json",
        "",
        ".",
    ] {
        let result = ProjectDir::open(dir.path())
            .unwrap()
            .delete_layer_content("base", path);
        assert!(
            matches!(result, Err(AppError::ValidationFailed(_))),
            "expected {path:?} to be rejected, got {result:?}"
        );
    }

    assert!(
        outsider.is_file(),
        "nothing outside the layer may be touched"
    );
}

#[test]
fn delete_layer_content_reports_an_entry_that_is_already_gone() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        ltk_mod_project::ModProjectLayer::default_table(),
    );
    seed_layer(dir.path(), "base", &["Aatrox.wad.client/a.bin"]);

    assert!(matches!(
    ProjectDir::open(dir.path())
        .unwrap()
        .delete_layer_content("base", "Aatrox.wad.client/gone.bin"),
    Err(AppError::ValidationFailed(msg)) if msg.contains("no longer")
    ));
}

#[test]
fn add_files_to_layer_aborts_on_conflict() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        ltk_mod_project::ModProjectLayer::default_table(),
    );

    let layer_dir = dir.path().join("content").join("base");
    fs::create_dir_all(&layer_dir).unwrap();
    fs::write(layer_dir.join("Aatrox.wad.client"), b"existing").unwrap();

    let src_dir = tempfile::tempdir().unwrap();
    let new_a = src_dir.path().join("Aatrox.wad.client");
    let new_b = src_dir.path().join("Sona.wad.client");
    fs::write(&new_a, b"new").unwrap();
    fs::write(&new_b, b"new").unwrap();

    let result = ProjectDir::open(dir.path()).unwrap().add_files_to_layer(
        "base",
        vec![new_a, new_b],
        &no_names(),
    );
    match result.as_ref().map_err(AppError::domain::<WorkshopError>) {
        Err(Some(WorkshopError::LayerFileConflict { conflicts })) => {
            assert_eq!(*conflicts, vec!["Aatrox.wad.client".to_string()]);
        }
        other => panic!("expected LayerFileConflict, got: {:?}", other),
    }

    // Sona.wad.client must not have been copied.
    assert!(!layer_dir.join("Sona.wad.client").exists());
    // Existing file untouched.
    assert_eq!(
        fs::read(layer_dir.join("Aatrox.wad.client")).unwrap(),
        b"existing"
    );
}

#[test]
fn update_layer_description_persists() {
    let dir = tempfile::tempdir().unwrap();
    make_project_with_layers(
        dir.path(),
        ltk_mod_project::ModProjectLayer::default_table(),
    );

    let project = ProjectDir::open(dir.path())
        .unwrap()
        .update_layer_description("base", Some("Updated description".to_string()))
        .unwrap();

    assert_eq!(
        project.layers[0].description.as_deref(),
        Some("Updated description")
    );

    let layers = load_layers(dir.path());
    assert_eq!(
        layers[0].description.as_deref(),
        Some("Updated description")
    );
}
