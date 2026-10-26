//! What one object's declarations answer, and the order a link resolves them in.

use super::*;

#[test]
fn declares_answers_by_object_hash() {
    let (_tmp, index) = named_index(&[("characters/aatrox", "Character")]);

    assert!(index.declares(BinHash::hash_str("characters/aatrox")));
    assert!(!index.declares(BinHash::hash_str("characters/ahri")));
}

#[test]
fn an_object_two_files_declare_has_a_row_per_file_in_archive_order() {
    let first: &[Chunk<'_>] = &[("data/a.bin", prop(&[("characters/shared", "Shared")]))];
    let second: &[Chunk<'_>] = &[("data/b.bin", prop(&[("characters/shared", "Shared")]))];
    let (_tmp, index) = build(&[("B.wad.client", second), ("A.wad.client", first)], 2);

    assert_eq!(
        declared(&index),
        [
            row("characters/shared", "Shared", "data/a.bin"),
            row("characters/shared", "Shared", "data/b.bin"),
        ],
        "archives are read in name order and the rows land in that order"
    );
    assert_eq!(index.stats().workers, 2);
}

#[test]
fn declared_answers_every_declaration_in_archive_order_with_its_file() {
    let first: &[Chunk<'_>] = &[("data/a.bin", prop(&[("characters/shared", "Shared")]))];
    let second: &[Chunk<'_>] = &[("data/b.bin", prop(&[("characters/shared", "Shared")]))];
    let (_tmp, index) = build_named(
        &[("B.wad.client", second), ("A.wad.client", first)],
        2,
        TestNames::over(&["characters/shared"], &["Shared"]),
    );

    let declared = index
        .declared(BinHash::hash_str("characters/shared"))
        .unwrap();
    assert_eq!(declared.path, "characters/shared");
    let files: Vec<(&str, &str)> = declared
        .declarations
        .iter()
        .map(|declaration| {
            let wad = match &declaration.asset {
                AssetRef::GameChunk { wad, .. } => wad.as_str(),
                _ => "",
            };
            (declaration.file.as_str(), wad)
        })
        .collect();
    assert_eq!(
        files,
        [
            ("data/a.bin", "A.wad.client"),
            ("data/b.bin", "B.wad.client")
        ]
    );
    assert!(declared.declarations.iter().all(|declaration| {
        declaration.class == "Shared" && declaration.class_hash == hex(BinHash::hash_str("Shared"))
    }));
    assert_eq!(
        declared.declarations[0].asset,
        AssetRef::GameChunk {
            wad: "A.wad.client".to_owned(),
            path_hash: hex_name(WadHash::hash_str("data/a.bin")),
        }
    );

    assert!(
        index
            .declared(BinHash::hash_str("characters/ahri"))
            .is_none()
    );
}

#[test]
fn a_link_resolves_this_file_then_a_dependency_then_archive_order() {
    let chunk = |wad: &str, path: &str| ObjectDeclaration {
        asset: AssetRef::GameChunk {
            wad: wad.to_owned(),
            path_hash: hex_name(WadHash::hash_str(path)),
        },
        file: path.to_owned(),
        class_hash: "0x00000000".to_owned(),
        class: "Shared".to_owned(),
    };
    let files = |declared: &DeclaredObject| -> Vec<String> {
        declared
            .declarations
            .iter()
            .map(|declaration| declaration.file.clone())
            .collect()
    };
    let mut declared = DeclaredObject {
        path: "characters/shared".to_owned(),
        declarations: vec![
            chunk("A.wad.client", "data/a.bin"),
            chunk("B.wad.client", "data/b.bin"),
            chunk("C.wad.client", "data/c.bin"),
        ],
    };

    let this = AssetRef::GameChunk {
        wad: "C.wad.client".to_owned(),
        path_hash: hex_name(WadHash::hash_str("data/c.bin")),
    };
    declared.resolve_for(&this, &[WadHash::hash_str("data/b.bin")]);
    assert_eq!(files(&declared), ["data/c.bin", "data/b.bin", "data/a.bin"]);

    let layer = AssetRef::Layer {
        project: "C:/mods/skin".to_owned(),
        layer: "base".to_owned(),
        path: "data/x.bin".to_owned(),
    };
    declared.resolve_for(&layer, &[WadHash::hash_str("data/a.bin")]);
    assert_eq!(
        files(&declared),
        ["data/a.bin", "data/c.bin", "data/b.bin"],
        "a dependency leads and the rest keep their order"
    );
}
