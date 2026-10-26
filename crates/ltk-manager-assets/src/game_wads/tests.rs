//! Unit tests for listing and reading archives, and for the mount cache.

use super::*;
use ltk_wad::{WadBuilder, WadChunkBuilder};
use std::io::Write as _;

/// The key the `game` table would file this path under, asked of the table
/// itself so a test cannot disagree with it about the algorithm.
fn path_hash(path: &str) -> u64 {
    crate::hashtables::Table::Game.key_config().hash(path)
}

fn final_dir(root: &Path) -> PathBuf {
    let dir = root.join("DATA").join("FINAL");
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn build_test_wad(path: &Path, chunk_paths: &[&str]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
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
fn list_reports_relative_names_sorted() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = final_dir(tmp.path());

    fs::write(dir.join("UI.wad.client"), [0u8; 5]).unwrap();
    fs::create_dir_all(dir.join("Champions")).unwrap();
    fs::write(dir.join("Champions").join("Aatrox.wad.client"), [0u8; 7]).unwrap();
    fs::write(dir.join("Champions").join("Ahri.WAD.CLIENT"), [0u8; 9]).unwrap();
    fs::write(dir.join("notes.txt"), b"decoy").unwrap();
    fs::write(dir.join("Legacy.wad"), b"decoy").unwrap();

    let wads = GameArchives::at(tmp.path()).list().unwrap();

    let names: Vec<&str> = wads.iter().map(|w| w.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Champions/Aatrox.wad.client",
            "Champions/Ahri.WAD.CLIENT",
            "UI.wad.client",
        ]
    );
    assert_eq!(wads[0].size_bytes, 7);
    assert_eq!(wads[1].size_bytes, 9);
    assert_eq!(wads[2].size_bytes, 5);
}

#[test]
fn a_client_list_reports_plugin_archives() {
    let tmp = tempfile::tempdir().unwrap();
    let plugins = tmp.path().join("Plugins");
    fs::create_dir_all(plugins.join("rcp-fe-lol-loot")).unwrap();
    fs::create_dir_all(plugins.join("rcp-be-lol-game-data")).unwrap();

    fs::write(plugins.join("rcp-fe-lol-loot").join("assets.wad"), [0u8; 3]).unwrap();
    fs::write(
        plugins
            .join("rcp-be-lol-game-data")
            .join("default-assets.WAD"),
        [0u8; 4],
    )
    .unwrap();
    fs::write(plugins.join("rcp-fe-lol-loot").join("index.js"), b"decoy").unwrap();

    let archives = GameArchives::lcu_at(&plugins);
    let names: Vec<String> = archives
        .list()
        .unwrap()
        .into_iter()
        .map(|w| w.name)
        .collect();

    assert_eq!(archives.source(), WadSource::Lcu);
    assert_eq!(
        names,
        [
            "rcp-be-lol-game-data/default-assets.WAD",
            "rcp-fe-lol-loot/assets.wad",
        ]
    );
}

#[test]
fn the_plugins_dir_sits_beside_the_game_dir() {
    let game = GameDir::from_path(Path::new("League of Legends").join("Game"));
    assert_eq!(
        game.lcu_plugins_dir(),
        Path::new("League of Legends").join("Plugins")
    );
}

#[test]
fn list_fails_without_a_final_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let err = GameArchives::at(tmp.path()).list().unwrap_err();
    assert!(matches!(err, AppError::ValidationFailed(_)));
}

#[test]
fn read_rejects_names_that_escape_final_dir() {
    let tmp = tempfile::tempdir().unwrap();
    final_dir(tmp.path());
    let archives = GameArchives::at(tmp.path());
    let resolver = LayeredHashDb::new();

    for name in [
        "../evil.wad.client",
        "..",
        "Champions/../../evil.wad.client",
        "/evil.wad.client",
    ] {
        let err = archives.read(name, &resolver).unwrap_err();
        assert!(
            matches!(err, AppError::InvalidPath(_)),
            "{name:?} should be rejected as an invalid path"
        );
    }
}

#[test]
fn read_of_a_missing_archive_is_an_io_error() {
    let tmp = tempfile::tempdir().unwrap();
    final_dir(tmp.path());
    let err = GameArchives::at(tmp.path())
        .read("Missing.wad.client", &LayeredHashDb::new())
        .unwrap_err();
    assert!(matches!(err, AppError::Io(_)));
}

#[test]
fn read_chunk_returns_the_chunk_data() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = final_dir(tmp.path());
    build_test_wad(
        &dir.join("Champions").join("Aatrox.wad.client"),
        &["assets/known.bin"],
    );
    let archives = GameArchives::at(tmp.path());

    let data = WadCache::default()
        .read_chunk(
            &archives,
            "Champions/Aatrox.wad.client",
            WadHash(path_hash("assets/known.bin")),
        )
        .unwrap();

    assert_eq!(data, [0xAA; 64]);
}

#[test]
fn read_chunk_of_a_hash_the_archive_lacks_is_an_invalid_path() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = final_dir(tmp.path());
    build_test_wad(
        &dir.join("Champions").join("Aatrox.wad.client"),
        &["assets/known.bin"],
    );

    let err = WadCache::default()
        .read_chunk(
            &GameArchives::at(tmp.path()),
            "Champions/Aatrox.wad.client",
            WadHash(1),
        )
        .unwrap_err();

    assert!(matches!(err, AppError::InvalidPath(_)));
}

#[test]
fn read_chunk_rejects_names_that_escape_final_dir() {
    let tmp = tempfile::tempdir().unwrap();
    final_dir(tmp.path());
    let archives = GameArchives::at(tmp.path());
    let cache = WadCache::default();

    let err = cache
        .read_chunk(&archives, "../evil.wad.client", WadHash(1))
        .unwrap_err();

    assert!(matches!(err, AppError::InvalidPath(_)));
    assert_eq!(cache.mounted(), 0, "a rejected name mounts nothing");
}

#[test]
fn every_chunk_of_one_archive_shares_a_single_mount() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = final_dir(tmp.path());
    build_test_wad(
        &dir.join("Champions").join("Aatrox.wad.client"),
        &["assets/first.bin", "assets/second.bin"],
    );
    let archives = GameArchives::at(tmp.path());
    let cache = WadCache::default();

    for path in ["assets/first.bin", "assets/second.bin"] {
        cache
            .read_chunk(
                &archives,
                "Champions/Aatrox.wad.client",
                WadHash(path_hash(path)),
            )
            .unwrap();
    }

    assert_eq!(cache.mounted(), 1);
}

#[test]
fn an_archive_past_the_capacity_pushes_the_oldest_one_out() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = final_dir(tmp.path());
    let names = ["One.wad.client", "Two.wad.client", "Three.wad.client"];
    for name in names {
        build_test_wad(&dir.join(name), &["assets/known.bin"]);
    }
    let archives = GameArchives::at(tmp.path());
    let cache = WadCache::new(NonZeroUsize::new(2).unwrap());

    for name in names {
        cache
            .read_chunk(&archives, name, WadHash(path_hash("assets/known.bin")))
            .unwrap();
    }

    assert_eq!(cache.mounted(), 2);
}

#[test]
fn clearing_unmounts_everything() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = final_dir(tmp.path());
    build_test_wad(&dir.join("One.wad.client"), &["assets/known.bin"]);
    let cache = WadCache::default();
    cache
        .read_chunk(
            &GameArchives::at(tmp.path()),
            "One.wad.client",
            WadHash(path_hash("assets/known.bin")),
        )
        .unwrap();

    cache.clear();

    assert_eq!(cache.mounted(), 0);
}

#[test]
fn read_resolves_known_chunks_and_leaves_the_rest_unresolved() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = final_dir(tmp.path());
    build_test_wad(
        &dir.join("Champions").join("Aatrox.wad.client"),
        &["assets/known.bin", "assets/unknown.bin"],
    );

    let known_hash = path_hash("assets/known.bin");
    let mut resolver = LayeredHashDb::new();
    resolver.insert(known_hash, "assets/known.bin");

    let entries = GameArchives::at(tmp.path())
        .read("Champions/Aatrox.wad.client", &resolver)
        .unwrap();

    assert_eq!(entries.len(), 2);
    for entry in &entries {
        assert_eq!(entry.size_bytes, 64);
        assert_eq!(entry.path_hash.len(), 16);
    }
    let known = entries
        .iter()
        .find(|e| e.path_hash == format!("{known_hash:016x}"))
        .unwrap();
    assert_eq!(known.path.as_deref(), Some("assets/known.bin"));
    let unknown = entries.iter().find(|e| e.path.is_none()).unwrap();
    assert_eq!(
        unknown.path_hash,
        format!("{:016x}", path_hash("assets/unknown.bin"))
    );
}

#[test]
fn chunk_index_lists_every_chunk_with_the_archives_that_contain_it() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = final_dir(tmp.path());
    build_test_wad(
        &dir.join("A.wad.client"),
        &["assets/shared.bin", "assets/a.bin"],
    );
    build_test_wad(&dir.join("B.wad.client"), &["assets/shared.bin"]);

    let index = GameArchives::at(tmp.path()).chunk_index(None).unwrap();

    let names: Vec<&str> = index.archives().iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["A.wad.client", "B.wad.client"]);
    assert_eq!(index.len(), 2);
    let holders: Vec<usize> = index
        .holders(WadHash(path_hash("assets/shared.bin")))
        .map(|id| id.index())
        .collect();
    assert_eq!(holders, [0, 1]);
}

#[test]
fn chunk_index_reads_the_cache_it_wrote_for_the_game() {
    let tmp = tempfile::tempdir().unwrap();
    build_test_wad(
        &final_dir(tmp.path()).join("A.wad.client"),
        &["assets/a.bin"],
    );
    let cache = tmp.path().join("cache").join("chunks.bin");
    let archives = GameArchives::at(tmp.path());

    let built = archives.chunk_index(Some(&cache)).unwrap();
    let loaded = archives.chunk_index(Some(&cache)).unwrap();

    assert!(cache.is_file());
    assert_eq!(built, loaded);
}

#[test]
fn chunk_index_writes_no_cache_for_the_league_client() {
    let tmp = tempfile::tempdir().unwrap();
    let plugins = tmp.path().join("Plugins");
    build_test_wad(
        &plugins.join("rcp-fe-lol-loot").join("assets.wad"),
        &["a.png"],
    );
    let cache = tmp.path().join("chunks.bin");

    let index = GameArchives::lcu_at(&plugins)
        .chunk_index(Some(&cache))
        .unwrap();

    assert_eq!(index.archives()[0].name, "rcp-fe-lol-loot/assets.wad");
    assert_eq!(index.len(), 1);
    assert!(!cache.exists());
}
