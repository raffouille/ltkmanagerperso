//! The grouped view: every object of a class, by the file that declares it.

use super::*;

const SKIN: &str = "SkinCharacterDataProperties";

/// An install of two archives, the aatrox one declaring its skins across two files.
fn install() -> (TempDir, ObjectIndex) {
    let aatrox: &[Chunk<'_>] = &[
        (
            "data/skins.bin",
            prop(&[
                ("characters/aatrox/skins/skin10", SKIN),
                ("characters/aatrox/skins/skin2", SKIN),
            ]),
        ),
        (
            "data/resources.bin",
            prop(&[(
                "characters/aatrox/skins/skin2/resources",
                "ResourceResolver",
            )]),
        ),
    ];
    let ahri: &[Chunk<'_>] = &[(
        "data/ahri.bin",
        prop(&[("characters/ahri/skins/skin0", SKIN)]),
    )];
    let wads: &[(&str, &[Chunk<'_>])] = &[("Aatrox.wad.client", aatrox), ("Ahri.wad.client", ahri)];

    let paths = [
        "characters/aatrox/skins/skin10",
        "characters/aatrox/skins/skin2",
        "characters/aatrox/skins/skin2/resources",
        "characters/ahri/skins/skin0",
    ];
    build_named(
        wads,
        1,
        TestNames::over(&paths, &[SKIN, "ResourceResolver"]),
    )
}

/// Each group as `(declaring file, the object paths under it)`.
fn grouped(result: &ReferenceResult) -> Vec<(&str, Vec<&str>)> {
    result
        .groups
        .iter()
        .map(|group| {
            (
                group.file.as_str(),
                group
                    .objects
                    .iter()
                    .map(|object| object.path.as_str())
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn class_references_group_by_declaring_file_in_archive_order() {
    let (_tmp, index) = install();
    let result = index.class_references(BinHash::hash_str(SKIN), || false);

    assert_eq!(
        grouped(&result),
        [
            (
                "data/skins.bin",
                vec![
                    "characters/aatrox/skins/skin2",
                    "characters/aatrox/skins/skin10",
                ]
            ),
            ("data/ahri.bin", vec!["characters/ahri/skins/skin0"]),
        ],
        "the files in archive order, the objects of one file in natural path order"
    );
    assert_eq!(result.total, 3);
    assert!(!result.superseded);
}

#[test]
fn a_group_carries_the_class_and_the_asset_of_every_object_in_it() {
    let (_tmp, index) = install();
    let result = index.class_references(BinHash::hash_str("ResourceResolver"), || false);

    let group = &result.groups[0];
    assert_eq!(
        group.asset,
        AssetRef::GameChunk {
            wad: "Aatrox.wad.client".to_owned(),
            path_hash: hex_name(WadHash::hash_str("data/resources.bin")),
        }
    );
    let object = &group.objects[0];
    assert_eq!(object.path, "characters/aatrox/skins/skin2/resources");
    assert_eq!(
        object.object_hash,
        hex(BinHash::hash_str(object.path.as_str()))
    );
    assert_eq!(object.class, "ResourceResolver");
    assert_eq!(
        object.class_hash,
        hex(BinHash::hash_str("ResourceResolver"))
    );
}

#[test]
fn a_class_counts_the_objects_that_declare_it() {
    let (_tmp, index) = install();

    assert_eq!(index.class_object_count(BinHash::hash_str(SKIN)), 3);
    assert_eq!(index.class_object_count(BinHash::hash_str("MapData")), 0);
}

#[test]
fn a_class_nothing_declares_has_no_group() {
    let (_tmp, index) = install();
    let result = index.class_references(BinHash::hash_str("MapData"), || false);

    assert!(result.groups.is_empty());
    assert_eq!(result.total, 0);
}

#[test]
fn class_references_cap_the_groups_and_count_on_past_the_cap() {
    let (_tmp, index) = install();
    let result = index.class_references_capped(BinHash::hash_str(SKIN), 1, || false);

    assert_eq!(
        grouped(&result),
        [("data/skins.bin", vec!["characters/aatrox/skins/skin10"])],
        "the cap holds the first row met, so the second file is never opened"
    );
    assert_eq!(result.total, 3);
}

#[test]
fn class_references_that_were_overtaken_say_so() {
    let (_tmp, index) = install();
    let result = index.class_references(BinHash::hash_str(SKIN), || true);

    assert!(result.superseded);
    assert!(result.groups.is_empty());
}

#[test]
fn an_object_no_table_names_sorts_after_the_named_of_its_file() {
    let chunks: &[Chunk<'_>] = &[(
        "data/objects.bin",
        prop(&[("characters/zed", SKIN), ("characters/secret", SKIN)]),
    )];
    let (_tmp, index) = build_named(
        &[("Objects.wad.client", chunks)],
        1,
        TestNames::over(&["characters/zed"], &[SKIN]),
    );
    let secret = hex(BinHash::hash_str("characters/secret"));

    let result = index.class_references(BinHash::hash_str(SKIN), || false);
    assert_eq!(
        grouped(&result),
        [("data/objects.bin", vec!["characters/zed", secret.as_str()])]
    );
}
