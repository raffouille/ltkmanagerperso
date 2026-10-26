//! The names of an index, which the build cuts from the tables into sheets.

use super::*;

#[test]
fn names_come_from_the_tables_the_build_was_given() {
    let chunks: &[Chunk<'_>] = &[("data/objects.bin", prop(&[("characters/aatrox", "Skin")]))];
    let wads: &[(&str, &[Chunk<'_>])] = &[("Objects.wad.client", chunks)];

    let (_tmp, unnamed) = build(wads, 1);
    assert!(unnamed.search("aatrox", || false).hits.is_empty());
    assert_eq!(unnamed.unnamed_objects().count(), 1);

    let (_tmp, named) = build_named(wads, 1, TestNames::over(&["characters/aatrox"], &["Skin"]));
    assert_eq!(ranked(&named, "aatrox"), ["characters/aatrox"]);
    assert_eq!(named.unnamed_objects().count(), 0);
    assert_eq!(named.stats().declarations, 1);
}
