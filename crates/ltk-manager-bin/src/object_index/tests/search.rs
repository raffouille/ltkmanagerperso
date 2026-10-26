//! The ranked search over the index, and the `class:` term it splits off.

use super::*;

fn term(value: &str, last: bool) -> Option<ClassTerm<'_>> {
    Some(ClassTerm { value, last })
}

#[test]
fn splits_the_class_term_off_the_rest_of_the_query() {
    assert_eq!(
        ClassTerm::split("class:skinchar smolder"),
        (term("skinchar", false), Cow::from("smolder"))
    );
    assert_eq!(
        ClassTerm::split("smolder Class:SkinChar"),
        (term("SkinChar", true), Cow::from("smolder")),
        "the term is read in any case, and last means last"
    );
    assert_eq!(
        ClassTerm::split("smolder skin0"),
        (None, Cow::from("smolder skin0"))
    );
    assert_eq!(
        ClassTerm::split("class:"),
        (term("", true), Cow::from("")),
        "an empty value is a term, and matches every class"
    );
}

/// An index over three classes, every object and class named.
fn classed() -> (TempDir, ObjectIndex) {
    named_index(&[
        (
            "characters/smolder/skins/skin0",
            "SkinCharacterDataProperties",
        ),
        (
            "characters/smolder/skins/skin1",
            "SkinCharacterDataProperties",
        ),
        ("characters/smolder/skins/skin0/resources", "SkinResources"),
        (
            "characters/smolder/skins/skin0/vfx",
            "VfxSystemDefinitionData",
        ),
    ])
}

#[test]
fn a_class_prefix_narrows_the_rows_and_the_rest_matches_the_path() {
    let (_tmp, index) = classed();

    assert_eq!(
        ranked(&index, "class:skinchar skin0"),
        ["characters/smolder/skins/skin0"],
        "the resource's class starts with skin, not skinchar"
    );
    assert_eq!(
        ranked(&index, "class:SKINRES skin0"),
        ["characters/smolder/skins/skin0/resources"],
        "case-insensitively"
    );
    assert!(ranked(&index, "class:nothing skin0").is_empty());
}

#[test]
fn a_class_hash_narrows_to_that_class_alone() {
    let (_tmp, index) = classed();
    let vfx = BinHash::hash_str("VfxSystemDefinitionData");

    assert_eq!(
        ranked(&index, &format!("class:{} smolder", hex(vfx))),
        ["characters/smolder/skins/skin0/vfx"]
    );
    assert_eq!(
        ranked(&index, &format!("class:{:08X} smolder", vfx.0)),
        ["characters/smolder/skins/skin0/vfx"],
        "with or without the 0x, either case"
    );
}

#[test]
fn a_class_no_table_names_is_reached_by_its_hex_and_reads_as_hex() {
    let chunks: &[Chunk<'_>] = &[(
        "data/objects.bin",
        prop(&[("characters/smolder", "Unnamed")]),
    )];
    let (_tmp, index) = build_named(
        &[("Objects.wad.client", chunks)],
        1,
        TestNames::over(&["characters/smolder"], &[]),
    );
    let class = BinHash::hash_str("Unnamed");

    let result = index.search(&format!("class:{} smolder", hex(class)), || false);
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].class, hex(class));
    assert!(
        ranked(&index, "class:unn smolder").is_empty(),
        "a prefix reads names, and the class has none"
    );
}

#[test]
fn an_ambiguous_last_class_term_lists_the_classes_with_their_counts() {
    let (_tmp, index) = classed();

    let result = index.search("smolder class:skin", || false);
    assert!(result.hits.is_empty());
    let classes: Vec<(&str, u32)> = result
        .classes
        .iter()
        .map(|class| (class.class.as_str(), class.rows))
        .collect();
    assert_eq!(
        classes,
        [("SkinCharacterDataProperties", 2), ("SkinResources", 1)],
        "by name, each with the rows it would narrow to"
    );
    assert_eq!(
        result.classes[0].class_hash,
        hex(BinHash::hash_str("SkinCharacterDataProperties"))
    );
    assert_eq!(result.total, 2);
}

#[test]
fn an_empty_class_term_lists_every_class() {
    let (_tmp, index) = classed();

    let result = index.search("class:", || false);
    assert_eq!(result.classes.len(), 3);
    assert!(result.hits.is_empty());
}

#[test]
fn an_ambiguous_class_term_that_is_not_last_narrows_to_every_class_it_matches() {
    let (_tmp, index) = classed();

    assert_eq!(
        ranked(&index, "class:skin skin0"),
        [
            "characters/smolder/skins/skin0",
            "characters/smolder/skins/skin0/resources",
        ],
        "both classes starting with skin, and no completions"
    );
    assert!(
        index
            .search("class:skin skin0", || false)
            .classes
            .is_empty()
    );
}

#[test]
fn one_class_left_lists_its_rows_even_with_no_path_term() {
    let (_tmp, index) = classed();

    let result = index.search("class:skinchar", || false);
    assert!(result.classes.is_empty());
    let paths: Vec<&str> = result.hits.iter().map(|hit| hit.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "characters/smolder/skins/skin0",
            "characters/smolder/skins/skin1",
        ],
        "every row of the class, by path"
    );
    assert_eq!(result.total, 2);
    assert!(result.hits[0].ranges.is_empty());
}

#[test]
fn a_hash_query_under_a_class_term_still_looks_the_object_up() {
    let (_tmp, index) = classed();
    let skin0 = BinHash::hash_str("characters/smolder/skins/skin0");

    assert_eq!(
        ranked(&index, &format!("class:skinchar {}", hex(skin0))),
        ["characters/smolder/skins/skin0"]
    );
    assert!(ranked(&index, &format!("class:vfx {}", hex(skin0))).is_empty());
}

#[test]
fn search_ranks_the_last_segment_as_the_name() {
    let (_tmp, index) = named_index(&[
        (
            "characters/aatrox/skins/skin0/resources",
            "ResourceResolver",
        ),
        (
            "characters/aatrox/skins/skin0",
            "SkinCharacterDataProperties",
        ),
        ("characters/aatrox/old_skin0", "SkinCharacterDataProperties"),
    ]);

    assert_eq!(
        ranked(&index, "skin0"),
        [
            "characters/aatrox/skins/skin0",
            "characters/aatrox/old_skin0",
            "characters/aatrox/skins/skin0/resources",
        ],
        "a segment the query opens, then one holding it, then a match reaching the path"
    );
}

#[test]
fn search_marks_the_match_on_the_whole_object_path() {
    let (_tmp, index) = named_index(&[("characters/aatrox/skins/skin0", "Skin")]);

    let hit = &index.search("skin0", || false).hits[0];
    let marked: Vec<&str> = hit
        .ranges
        .iter()
        .map(|&(start, end)| &hit.path[start as usize..end as usize])
        .collect();
    assert_eq!(marked, ["skin0"]);
    assert_eq!(hit.band, 0);
}

#[test]
fn search_names_the_class_and_the_declaring_file() {
    let (_tmp, index) = named_index(&[(
        "characters/aatrox/skins/skin0",
        "SkinCharacterDataProperties",
    )]);

    let hit = &index.search("aatrox", || false).hits[0];
    assert_eq!(hit.class, "SkinCharacterDataProperties");
    assert_eq!(hit.file, "data/objects.bin");
    assert_eq!(
        hit.file_hash,
        format!("{:016x}", path_hash("data/objects.bin"))
    );
    assert_eq!(hit.wad, "Objects.wad.client");
    assert_eq!(
        hit.object_hash,
        hex(BinHash::hash_str("characters/aatrox/skins/skin0"))
    );
}

#[test]
fn search_obeys_the_shared_ranking_fixture() {
    #[derive(serde::Deserialize)]
    struct Fixture {
        cases: Vec<Case>,
    }
    #[derive(serde::Deserialize)]
    struct Case {
        name: String,
        query: String,
        expect: Vec<String>,
        #[serde(default)]
        reject: Vec<String>,
    }

    let raw = include_str!(
        "../../../../../src/modules/workshop/palette/utils/__tests__/ranking.fixture.json"
    );
    let fixture: Fixture = serde_json::from_str(raw).unwrap();

    for case in fixture.cases {
        let held: Vec<(&str, &str)> = case
            .expect
            .iter()
            .chain(case.reject.iter())
            .map(|path| (path.as_str(), "Object"))
            .collect();
        let (_tmp, index) = named_index(&held);

        assert_eq!(ranked(&index, &case.query), case.expect, "{}", case.name);
    }
}

#[test]
fn eight_hex_digits_find_the_object_by_hash() {
    let (_tmp, index) = named_index(&[
        ("characters/aatrox/skins/skin0", "Skin"),
        ("characters/ahri/skins/skin0", "Skin"),
    ]);
    let wanted = BinHash::hash_str("characters/ahri/skins/skin0");

    assert_eq!(
        ranked(&index, &format!("{:08x}", wanted.0)),
        ["characters/ahri/skins/skin0"]
    );
    assert_eq!(
        ranked(&index, &hex(wanted)),
        ["characters/ahri/skins/skin0"],
        "with the 0x too"
    );
    assert_eq!(
        ranked(&index, &format!("{:08X}", wanted.0)),
        ["characters/ahri/skins/skin0"],
        "either case"
    );
}

#[test]
fn an_object_no_table_names_reads_as_its_hex_and_answers_to_its_hash_alone() {
    let chunks: &[Chunk<'_>] = &[(
        "data/objects.bin",
        prop(&[("characters/secret", "Skin"), ("characters/known", "Skin")]),
    )];
    let (_tmp, index) = build_named(
        &[("Objects.wad.client", chunks)],
        1,
        TestNames::over(&["characters/known"], &[]),
    );
    let secret = BinHash::hash_str("characters/secret");

    let result = index.search("characters", || false);
    assert_eq!(
        result.hits.len(),
        1,
        "a text query never reaches an unnamed object"
    );
    assert!(
        !result.unnamed,
        "one name is enough to say the table is there"
    );

    let hit = &index.search(&hex(secret), || false).hits[0];
    assert_eq!(hit.path, hex(secret));
    assert_eq!(
        hit.class,
        hex(BinHash::hash_str("Skin")),
        "a class no table names reads as its hex"
    );
}

#[test]
fn an_index_no_table_names_says_so() {
    let chunks: &[Chunk<'_>] = &[("data/objects.bin", prop(&[("characters/aatrox", "Skin")]))];
    let (_tmp, index) = build(&[("Objects.wad.client", chunks)], 1);

    let result = index.search("aatrox", || false);
    assert!(result.hits.is_empty());
    assert!(result.unnamed);
}

#[test]
fn an_empty_query_matches_nothing() {
    let (_tmp, index) = named_index(&[("characters/aatrox", "Skin")]);
    assert!(ranked(&index, "  ").is_empty());
}

#[test]
fn a_hash_still_finds_its_object_in_an_index_no_table_names() {
    let chunks: &[Chunk<'_>] = &[("data/objects.bin", prop(&[("characters/aatrox", "Skin")]))];
    let (_tmp, index) = build(&[("Objects.wad.client", chunks)], 1);
    let aatrox = BinHash::hash_str("characters/aatrox");

    let result = index.search(&hex(aatrox), || false);
    assert_eq!(result.hits.len(), 1, "the lookup needs no name");
    assert_eq!(result.hits[0].path, hex(aatrox));
    assert!(
        result.unnamed,
        "and the caller is still told the table is missing"
    );
}
