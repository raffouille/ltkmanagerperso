//! The build over a synthetic install: what it reads, sniffs, skips and gives up on.

use super::*;

#[test]
fn build_reads_a_prop_and_a_patch_and_skips_what_will_not_read() {
    let chunks: &[Chunk<'_>] = &[
        (
            "data/skin0.bin",
            prop(&[
                (
                    "characters/aatrox/skins/skin0",
                    "SkinCharacterDataProperties",
                ),
                (
                    "characters/aatrox/skins/skin0/resources",
                    "ResourceResolver",
                ),
            ]),
        ),
        (
            "data/patch.bin",
            patch(
                &[(
                    "characters/aatrox/skins/skin0/added",
                    "VfxSystemDefinitionData",
                )],
                "characters/aatrox/skins/skin0",
            ),
        ),
        ("data/texture.dds", vec![0xAA; 64]),
        (
            "data/broken.bin",
            b"PROP garbage that is not a bin".to_vec(),
        ),
    ];
    let (_tmp, index) = build(&[("Aatrox.wad.client", chunks)], 2);

    assert_eq!(
        declared(&index),
        [
            row(
                "characters/aatrox/skins/skin0/added",
                "VfxSystemDefinitionData",
                "data/patch.bin"
            ),
            row(
                "characters/aatrox/skins/skin0",
                "SkinCharacterDataProperties",
                "data/skin0.bin"
            ),
            row(
                "characters/aatrox/skins/skin0/resources",
                "ResourceResolver",
                "data/skin0.bin"
            ),
        ],
        "files in path order, a patch's added objects as rows, its patch record not"
    );

    let stats = index.stats();
    assert_eq!(stats.archives, 1);
    assert_eq!(stats.bins, 3, "every named .bin chunk, read or not");
    assert_eq!(stats.sniffed, 0, "every chunk was named");
    assert_eq!(stats.declarations, 3);
    assert_eq!(stats.skipped_chunks, 1, "the chunk that would not read");
    assert_eq!(stats.workers, 1, "one archive is one job");
}

#[test]
fn a_bare_named_chunk_is_read_under_its_name_when_its_magic_is_a_bins() {
    let chunks: &[Chunk<'_>] = &[
        (
            "clientstates/gameplay/ux/chat/uibase",
            prop(&[(
                "ClientStates/Gameplay/UX/Chat/UIBase/ChatFrame",
                "UiSceneData",
            )]),
        ),
        ("ux/floatingtext", vec![0xAA; 64]),
        (
            "data/notes.txt",
            prop(&[("characters/hidden", "Character")]),
        ),
    ];
    let (_tmp, index) = build(&[("UI.wad.client", chunks)], 1);

    assert_eq!(
        declared(&index),
        [row(
            "ClientStates/Gameplay/UX/Chat/UIBase/ChatFrame",
            "UiSceneData",
            "clientstates/gameplay/ux/chat/uibase"
        )],
        "the bare bin under its name, and neither the bare non-bin nor a bin behind another extension"
    );

    let stats = index.stats();
    assert_eq!(stats.bins, 1, "the bare bin alone");
    assert_eq!(stats.sniffed, 2, "both bare chunks were sniffed");
    assert_eq!(stats.sniffed_bins, 1, "one of them was a bin");
    assert_eq!(stats.skipped_chunks, 0, "a bare non-bin is not a skip");
}

#[test]
fn an_unnamed_chunk_is_sniffed_and_read_only_when_its_magic_is_a_bins() {
    let chunks: &[Chunk<'_>] = &[
        (
            "data/named.bin",
            prop(&[("characters/aatrox", "Character")]),
        ),
        (
            "data/secret.bin",
            prop(&[("characters/secret", "Character")]),
        ),
        ("assets/secret.dds", vec![0xAA; 4096]),
    ];
    let tmp = game_with(&[("Aatrox.wad.client", chunks)]);
    let archives = GameArchives::at(tmp.path());
    let chunks = archives.chunk_index(None).unwrap();
    let resolver = resolver_for(&["data/named.bin"]);
    let names = TestNames::over(&[], &[]).with_files(&["data/named.bin"]);
    let index =
        ObjectIndex::build(&chunks, &ChunkPaths(&resolver), &names, 1, &|| false, None).unwrap();
    let secret = WadHash::hash_str("data/secret.bin");

    assert_eq!(
        declared(&index),
        [
            row("characters/aatrox", "Character", "data/named.bin"),
            row("characters/secret", "Character", &hex_name(secret)),
        ],
        "the named chunk first, then the sniffed bin under its hex"
    );

    let stats = index.stats();
    assert_eq!(stats.sniffed, 2, "both unnamed chunks were sniffed");
    assert_eq!(stats.sniffed_bins, 1, "one of them was a bin");
    assert_eq!(stats.bins, 2, "the named bin and the sniffed one");
    assert_eq!(stats.skipped_chunks, 0, "a non-bin sniffed is not a skip");

    let hit = &index
        .search(&hex(BinHash::hash_str("characters/secret")), || false)
        .hits[0];
    assert_eq!(hit.file, hex_name(secret));
    assert_eq!(hit.file_hash, hex_name(secret));
    assert_eq!(hit.wad, "Aatrox.wad.client");
}

#[test]
fn a_sniffed_file_takes_the_name_a_later_table_gives_it() {
    let chunks: &[Chunk<'_>] = &[(
        "data/secret.bin",
        prop(&[("characters/secret", "Character")]),
    )];
    let tmp = game_with(&[("Aatrox.wad.client", chunks)]);
    let archives = GameArchives::at(tmp.path());
    let chunks = archives.chunk_index(None).unwrap();
    let resolver = resolver_for(&[]);
    let names =
        TestNames::over(&["characters/secret"], &["Character"]).with_files(&["data/secret.bin"]);

    let index =
        ObjectIndex::build(&chunks, &ChunkPaths(&resolver), &names, 1, &|| false, None).unwrap();

    assert_eq!(ranked(&index, "secret"), ["characters/secret"]);
    let hit = &index.search("secret", || false).hits[0];
    assert_eq!(hit.file, "data/secret.bin");
    assert_eq!(
        declared(&index),
        [row("characters/secret", "Character", "data/secret.bin")]
    );
}

#[test]
fn a_build_that_was_called_off_stops_rather_than_finishing() {
    let chunks: &[Chunk<'_>] = &[("data/a.bin", prop(&[("characters/aatrox", "Character")]))];
    let tmp = game_with(&[("A.wad.client", chunks)]);
    let archives = GameArchives::at(tmp.path());
    let chunks = archives.chunk_index(None).unwrap();
    let resolver = resolver_for(&["data/a.bin"]);

    let built = ObjectIndex::build(&chunks, &ChunkPaths(&resolver), &NoNames, 1, &|| true, None);
    assert!(built.is_err());
}
