//! The path-ordered listing of one prefix, and the full search over the index.

use super::*;

use ltk_manager_base::matcher::{FindQuery, PatternSyntax};

/// The objects one install of the tests declares, as `(path, class)`.
const OBJECTS: &[(&str, &str)] = &[
    (
        "characters/aatrox/skins/skin0",
        "SkinCharacterDataProperties",
    ),
    (
        "characters/aatrox/skins/skin0/resources",
        "ResourceResolver",
    ),
    (
        "characters/aatrox/skins/skin10",
        "SkinCharacterDataProperties",
    ),
    (
        "characters/aatrox/skins/skin2",
        "SkinCharacterDataProperties",
    ),
    ("characters/ahri/skins/skin0", "SkinCharacterDataProperties"),
    ("maps/shipping/map11/data", "MapData"),
];

/// The prefix rows of a listing as `(path, name, count)`.
fn prefixes(listing: &ObjectDirListing) -> Vec<(&str, &str, u32)> {
    listing
        .prefixes
        .iter()
        .map(|prefix| (prefix.path.as_str(), prefix.name.as_str(), prefix.count))
        .collect()
}

/// The object rows of a listing as `(name, class, count)`.
fn objects(listing: &ObjectDirListing) -> Vec<(&str, &str, u32)> {
    listing
        .objects
        .iter()
        .map(|object| {
            (
                object.name.as_str(),
                object.declarations[0].class.as_str(),
                object.count,
            )
        })
        .collect()
}

fn literal(pattern: &str) -> FindQuery {
    FindQuery::parse(pattern, PatternSyntax::Literal)
        .unwrap()
        .unwrap()
}

fn regex(pattern: &str) -> FindQuery {
    FindQuery::parse(pattern, PatternSyntax::Regex)
        .unwrap()
        .unwrap()
}

/// The object path of each hit, in the order the find listed them.
fn found(result: &ObjectFindResult) -> Vec<&str> {
    result.hits.iter().map(|hit| hit.path.as_str()).collect()
}

#[test]
fn the_root_folds_a_run_of_single_child_prefixes_and_counts_the_objects_under_each() {
    let (_tmp, index) = named_index(OBJECTS);
    let root = index.object_dir("").unwrap();

    assert_eq!(
        prefixes(&root),
        [
            ("characters", "characters", 5),
            ("maps/shipping/map11", "maps/shipping/map11", 1),
        ],
        "maps holds one prefix holding one prefix, which is one row"
    );
    assert!(root.objects.is_empty());
}

#[test]
fn a_prefix_lists_its_own_prefixes_folded_and_its_objects_in_natural_order() {
    let (_tmp, index) = named_index(OBJECTS);

    let characters = index.object_dir("characters").unwrap();
    assert_eq!(
        prefixes(&characters),
        [
            ("characters/aatrox/skins", "aatrox/skins", 4),
            ("characters/ahri/skins", "ahri/skins", 1),
        ]
    );
    assert!(characters.objects.is_empty());

    let skins = index.object_dir("characters/aatrox/skins").unwrap();
    assert!(skins.prefixes.is_empty());
    assert_eq!(
        objects(&skins),
        [
            ("skin0", "SkinCharacterDataProperties", 1),
            ("skin2", "SkinCharacterDataProperties", 0),
            ("skin10", "SkinCharacterDataProperties", 0),
        ],
        "skin0 is an object and a prefix, in one row with what is under it counted"
    );
    let skin0 = &skins.objects[0];
    assert_eq!(skin0.path, "characters/aatrox/skins/skin0");
    assert_eq!(
        skin0.object_hash,
        hex(BinHash::hash_str("characters/aatrox/skins/skin0"))
    );
    assert_eq!(skin0.declarations.len(), 1);
    assert_eq!(skin0.declarations[0].file, "data/objects.bin");
}

#[test]
fn a_node_that_is_an_object_lists_the_objects_under_it() {
    let (_tmp, index) = named_index(OBJECTS);

    let skin0 = index.object_dir("characters/aatrox/skins/skin0").unwrap();
    assert!(skin0.prefixes.is_empty());
    assert_eq!(objects(&skin0), [("resources", "ResourceResolver", 0)]);

    let leaf = index
        .object_dir("characters/aatrox/skins/skin0/resources")
        .unwrap();
    assert!(leaf.prefixes.is_empty() && leaf.objects.is_empty());
}

#[test]
fn a_prefix_no_path_runs_through_answers_none() {
    let (_tmp, index) = named_index(OBJECTS);
    assert!(index.object_dir("characters/zed").is_none());
    assert!(index.object_dir("character").is_none());
    assert!(index.object_dir("maps/shipping").is_some());
}

#[test]
fn the_unnamed_objects_group_last_under_the_question_mark_and_read_as_hex() {
    let objects: &[(&str, &str)] = &[
        ("characters/aatrox", "Character"),
        ("characters/secret", "Character"),
        ("characters/hidden", "Character"),
    ];
    let chunks: &[Chunk<'_>] = &[("data/objects.bin", prop(objects))];
    let (_tmp, index) = build_named(
        &[("Objects.wad.client", chunks)],
        1,
        TestNames::over(&["characters/aatrox"], &["Character"]),
    );

    let root = index.object_dir("").unwrap();
    assert_eq!(
        prefixes(&root),
        [("characters", "characters", 1), (UNNAMED_PREFIX, "?", 2)]
    );

    let unnamed = index.object_dir(UNNAMED_PREFIX).unwrap();
    assert!(unnamed.prefixes.is_empty());
    let mut hidden = [
        hex(BinHash::hash_str("characters/secret")),
        hex(BinHash::hash_str("characters/hidden")),
    ];
    hidden.sort();
    let listed: Vec<(&str, &str)> = unnamed
        .objects
        .iter()
        .map(|object| (object.path.as_str(), object.name.as_str()))
        .collect();
    assert_eq!(
        listed,
        [
            (hidden[0].as_str(), hidden[0].as_str()),
            (hidden[1].as_str(), hidden[1].as_str())
        ],
        "by hash, the path and the name both the hex"
    );
}

#[test]
fn an_index_no_table_names_lists_only_the_unnamed_group() {
    let (_tmp, index) = build(
        &[("Objects.wad.client", &[("data/objects.bin", prop(OBJECTS))])],
        1,
    );
    let root = index.object_dir("").unwrap();
    assert_eq!(prefixes(&root), [(UNNAMED_PREFIX, "?", 6)]);
}

#[test]
fn a_node_several_files_declare_carries_every_declaration_in_archive_order() {
    let shared: &[(&str, &str)] = &[("characters/shared", "Shared")];
    let (_tmp, index) = build_named(
        &[
            ("A.wad.client", &[("data/a.bin", prop(shared))]),
            ("B.wad.client", &[("data/b.bin", prop(shared))]),
        ],
        2,
        TestNames::over(&["characters/shared"], &["Shared"]),
    );

    let listing = index.object_dir("characters").unwrap();
    let files: Vec<&str> = listing.objects[0]
        .declarations
        .iter()
        .map(|declaration| declaration.file.as_str())
        .collect();
    assert_eq!(files, ["data/a.bin", "data/b.bin"]);
}

#[test]
fn find_matches_the_pattern_over_the_path_in_path_order_and_marks_the_runs() {
    let (_tmp, index) = named_index(OBJECTS);

    let result = index.find(Some(&literal("SKIN0")), None, || false);
    assert_eq!(
        found(&result),
        [
            "characters/aatrox/skins/skin0",
            "characters/aatrox/skins/skin0/resources",
            "characters/ahri/skins/skin0",
        ],
        "case-insensitive, a node before what is under it"
    );
    assert_eq!(result.total, 3);
    assert_eq!(result.hits[0].ranges, [(24, 29)]);
    assert_eq!(
        result.hits[0].object_hash,
        hex(BinHash::hash_str("characters/aatrox/skins/skin0"))
    );
    assert_eq!(result.hits[0].declarations[0].file, "data/objects.bin");
    assert!(!result.superseded && !result.unnamed);

    let result = index.find(Some(&regex("skin[0-9]$")), None, || false);
    assert_eq!(
        found(&result),
        [
            "characters/aatrox/skins/skin0",
            "characters/aatrox/skins/skin2",
            "characters/ahri/skins/skin0",
        ]
    );
}

#[test]
fn find_narrows_to_a_class_term_and_lists_a_class_on_its_own() {
    let (_tmp, index) = named_index(OBJECTS);

    let result = index.find(Some(&literal("skin0")), Some("Resource"), || false);
    assert_eq!(found(&result), ["characters/aatrox/skins/skin0/resources"]);

    let result = index.find(None, Some("MapData"), || false);
    assert_eq!(found(&result), ["maps/shipping/map11/data"]);
    assert!(result.hits[0].ranges.is_empty());

    let by_hash = hex(BinHash::hash_str("MapData"));
    let result = index.find(None, Some(&by_hash), || false);
    assert_eq!(found(&result), ["maps/shipping/map11/data"]);

    let result = index.find(None, Some("NoSuchClass"), || false);
    assert!(result.hits.is_empty() && result.total == 0);
}

#[test]
fn find_with_nothing_asked_matches_nothing() {
    let (_tmp, index) = named_index(OBJECTS);
    let result = index.find(None, None, || false);
    assert!(result.hits.is_empty() && result.total == 0);
}

#[test]
fn find_caps_the_hits_and_counts_on_past_the_cap() {
    let (_tmp, index) = named_index(OBJECTS);
    let result = index.find_capped(Some(&literal("skin")), None, 2, || false);
    assert_eq!(result.hits.len(), 2);
    assert_eq!(result.total, 5);
}

#[test]
fn find_matches_an_unnamed_object_by_its_hex_after_the_named() {
    let objects: &[(&str, &str)] = &[
        ("characters/aatrox", "Character"),
        ("characters/secret", "Character"),
    ];
    let chunks: &[Chunk<'_>] = &[("data/objects.bin", prop(objects))];
    let (_tmp, index) = build_named(
        &[("Objects.wad.client", chunks)],
        1,
        TestNames::over(&["characters/aatrox"], &["Character"]),
    );
    let secret = hex(BinHash::hash_str("characters/secret"));

    let result = index.find(Some(&literal(&secret[2..6])), None, || false);
    assert_eq!(found(&result), [secret.as_str()]);

    let result = index.find(None, Some("Character"), || false);
    assert_eq!(found(&result), ["characters/aatrox", secret.as_str()]);
}

#[test]
fn find_that_was_overtaken_says_so() {
    let (_tmp, index) = named_index(OBJECTS);
    let result = index.find(Some(&literal("skin")), None, || true);
    assert!(result.superseded);
}
