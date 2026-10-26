use fs_err as fs;

use super::*;

#[test]
fn warming_builds_the_index_before_anything_asks() {
    let game = tempfile::tempdir().expect("a temp dir");
    fs::create_dir_all(game.path().join("DATA").join("FINAL")).expect("DATA/FINAL");
    let content = InstalledContent::at(game.path());
    assert!(content.chunks.get().is_none());

    content.warm();

    let index = content.chunks.get().expect("an index after warming");
    assert!(index.as_ref().is_some_and(ChunkIndex::is_empty));
    assert!(!content.contains(WadHash(1)));
}

#[test]
fn warming_an_install_with_no_archives_still_answers() {
    let game = tempfile::tempdir().expect("a temp dir");
    let content = InstalledContent::at(game.path());

    content.warm();

    assert!(content.chunks.get().is_some());
    assert_eq!(content.read(WadHash(1)), Ok(None));
}

#[test]
fn chunk_in_two_archives_reads_from_first_archive() {
    use ltk_hash::Hash as _;
    use ltk_manager_assets::test_util::place_game_wad;

    let root = tempfile::tempdir().expect("a temp dir");
    place_game_wad(
        root.path(),
        "A.wad.client",
        &[("shared.bin", b"aa"), ("only_a.bin", b"a")],
    );
    place_game_wad(root.path(), "B.wad.client", &[("shared.bin", b"bbbb")]);
    let content = InstalledContent::at(&root.path().join("league").join("Game"));

    let shared = WadHash::hash_str("shared.bin");
    assert_eq!(content.size(shared), Some(2));
    assert_eq!(content.read(shared), Ok(Some(b"aa".to_vec())));
    assert_eq!(content.size(WadHash::hash_str("only_a.bin")), Some(1));
    assert!(!content.contains(WadHash::hash_str("missing.bin")));
}

#[test]
fn index_objects_returns_declaring_chunk() {
    use ltk_hash::Hash as _;
    use ltk_manager_assets::test_util::{bin_bytes, place_game_wad};
    use ltk_meta::{Bin, BinObject};

    const SKIN_BIN: &str = "data/characters/aatrox/skins/skin0.bin";
    let skin = BinHash::hash_str("Characters/Aatrox/Skins/Skin0");
    let bin = Bin::new(
        [BinObject::new(
            skin,
            BinHash::hash_str("SkinCharacterDataProperties"),
        )],
        std::iter::empty::<&str>(),
    );
    let root = tempfile::tempdir().expect("a temp dir");
    place_game_wad(
        root.path(),
        "Aatrox.wad.client",
        &[(SKIN_BIN, &bin_bytes(&bin))],
    );
    let archives = GameArchives::at(&root.path().join("league").join("Game"));
    let mut tables = LayeredHashDb::new();
    tables.insert(WadHash::hash_str(SKIN_BIN).0, SKIN_BIN);

    let chunks = archives.chunk_index(None).expect("a chunk index");
    let objects = index_objects(&chunks, &tables).expect("an index");

    assert_eq!(
        objects
            .declarations(skin)
            .map(|(_, row)| row.chunk)
            .collect::<Vec<_>>(),
        [WadHash::hash_str(SKIN_BIN)]
    );
    assert_eq!(objects.declarations(BinHash(1)).count(), 0);
}
