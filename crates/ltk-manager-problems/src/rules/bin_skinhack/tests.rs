//! Unit tests for the base skins the rule reports and the ones it leaves alone.

use std::sync::Arc;

use fs_err as fs;
use indexmap::IndexMap;
use ltk_meta::property::values;
use ltk_meta::{Bin, PropertyKind};

use super::*;
use crate::game::FakeContent;
use ltk_manager_base::config::Config;

const WAD: &str = "Ahri.wad.client";
const SKIN0: &str = "data/characters/ahri/skins/skin0.bin";
const SKIN7: &str = "data/characters/ahri/skins/skin7.bin";
const BASE_SKL: &str = "assets/characters/ahri/skins/base/ahri.skl";
const BASE_SKN: &str = "assets/characters/ahri/skins/base/ahri.skn";
const SKIN7_SKL: &str = "assets/characters/ahri/skins/skin07/ahri_skin07.skl";
const SKIN7_SKN: &str = "assets/characters/ahri/skins/skin07/ahri_skin07.skn";
const CUSTOM_SKL: &str = "assets/characters/ahri/skins/base/custom.skl";
const CUSTOM_SKN: &str = "assets/characters/ahri/skins/base/custom.skn";

const BASE_SKL_BYTES: &[u8] = b"base skeleton";
const BASE_SKN_BYTES: &[u8] = b"base simple skin";
const SKIN7_SKL_BYTES: &[u8] = b"skin 7 skeleton!";
const SKIN7_SKN_BYTES: &[u8] = b"skin 7 simple skin";
const CUSTOM_SKL_BYTES: &[u8] = b"custom skeleton";
const CUSTOM_SKN_BYTES: &[u8] = b"custom simple skin";

fn string(value: &str) -> PropertyValueEnum {
    PropertyValueEnum::String(values::String::new(value.to_owned()))
}

fn embed(class: &str, properties: Vec<(&str, PropertyValueEnum)>) -> PropertyValueEnum {
    let properties: IndexMap<_, _> = properties
        .into_iter()
        .map(|(name, value)| (named(name), value))
        .collect();
    PropertyValueEnum::Embedded(values::Embedded(values::Struct {
        class_hash: named(class),
        properties,
    }))
}

fn list(kind: PropertyKind, items: Vec<PropertyValueEnum>) -> PropertyValueEnum {
    PropertyValueEnum::Container(values::Container::new(kind, items).unwrap())
}

/// The bytes of skin `skin` of Ahri, with its meshes and the extra properties `extra`.
fn skin_bin(skin: u32, skl: &str, skn: &str, extra: Vec<(&str, PropertyValueEnum)>) -> Vec<u8> {
    let mut object = BinObject::builder(
        skin_object("ahri", skin),
        named("SkinCharacterDataProperties"),
    )
    .property(
        named("SkinMeshProperties"),
        embed(
            "SkinMeshProperties",
            vec![("Skeleton", string(skl)), ("SimpleSkin", string(skn))],
        ),
    );
    for (name, value) in extra {
        object = object.property(named(name), value);
    }

    let bin = Bin::new([object.build()], std::iter::empty::<&str>());
    let mut out = std::io::Cursor::new(Vec::new());
    bin.to_writer(&mut out).unwrap();
    out.into_inner()
}

fn base() -> Vec<u8> {
    skin_bin(0, BASE_SKL, BASE_SKN, Vec::new())
}

/// An install with Ahri's base skin and skin 7, with their meshes.
fn game() -> Arc<dyn GameContent> {
    FakeContent::containing_bytes(&[
        (SKIN0, &base()),
        (SKIN7, &skin_bin(7, SKIN7_SKL, SKIN7_SKN, Vec::new())),
        (BASE_SKL, BASE_SKL_BYTES),
        (BASE_SKN, BASE_SKN_BYTES),
        (SKIN7_SKL, SKIN7_SKL_BYTES),
        (SKIN7_SKN, SKIN7_SKN_BYTES),
    ])
}

/// A base skin that ships its own meshes, with the extra properties `extra`.
fn custom(extra: Vec<(&str, PropertyValueEnum)>) -> Vec<(String, Vec<u8>)> {
    vec![
        (in_layer(SKIN0), skin_bin(0, CUSTOM_SKL, CUSTOM_SKN, extra)),
        (in_layer(CUSTOM_SKL), CUSTOM_SKL_BYTES.to_vec()),
        (in_layer(CUSTOM_SKN), CUSTOM_SKN_BYTES.to_vec()),
    ]
}

fn in_layer(chunk: &str) -> String {
    format!("{WAD}/{chunk}")
}

fn tree(
    files: &[(String, Vec<u8>)],
    game: Option<Arc<dyn GameContent>>,
) -> (tempfile::TempDir, ProjectFiles) {
    let tmp = tempfile::tempdir().unwrap();
    for (path, bytes) in files {
        let at = tmp
            .path()
            .join("content")
            .join("base")
            .join(path.replace('/', std::path::MAIN_SEPARATOR_STR));
        fs::create_dir_all(at.parent().unwrap()).unwrap();
        fs::write(&at, bytes).unwrap();
    }

    let files = ProjectFiles::read(tmp.path(), &Config::default(), game).unwrap();
    (tmp, files)
}

fn found_in(files: &ProjectFiles) -> Vec<Problem> {
    let (problems, failed) = files.report(&[&BinSkinhack::new()]).finish();
    assert!(
        failed.is_empty(),
        "the fixture should read cleanly: {failed:?}"
    );
    problems
}

fn messages(problems: &[Problem]) -> Vec<&str> {
    problems
        .iter()
        .filter_map(|problem| problem.message.as_deref())
        .collect()
}

#[test]
fn base_skin_pointed_at_other_skin_meshes_is_reported() {
    let (_tmp, files) = tree(
        &[(
            in_layer(SKIN0),
            skin_bin(0, SKIN7_SKL, SKIN7_SKN, Vec::new()),
        )],
        Some(game()),
    );

    let problems = found_in(&files);

    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].rule, ID);
    assert_eq!(problems[0].severity, ProblemSeverity::Error);
    assert_eq!(problems[0].site.path, in_layer(SKIN0));
    assert_eq!(
        messages(&problems),
        ["SkinMeshProperties.Skeleton is the mesh of skin 7"]
    );
}

#[test]
fn base_skin_shipping_copy_of_other_skin_mesh_is_reported() {
    let (_tmp, files) = tree(
        &[
            (
                in_layer(SKIN0),
                skin_bin(0, CUSTOM_SKL, CUSTOM_SKN, Vec::new()),
            ),
            (in_layer(CUSTOM_SKL), CUSTOM_SKL_BYTES.to_vec()),
            (in_layer(CUSTOM_SKN), SKIN7_SKN_BYTES.to_vec()),
        ],
        Some(game()),
    );

    assert_eq!(
        messages(&found_in(&files)),
        ["SkinMeshProperties.SimpleSkin is the mesh of skin 7"]
    );
}

/* The patcher allows a base skin that still renders a stock mesh, whatever it names. */
#[test]
fn base_skin_keeping_base_skeleton_is_not_reported() {
    let (_tmp, files) = tree(
        &[(
            in_layer(SKIN0),
            skin_bin(
                0,
                BASE_SKL,
                SKIN7_SKN,
                vec![("ChampionSkinName", string("AhriSkin7"))],
            ),
        )],
        Some(game()),
    );

    assert!(found_in(&files).is_empty());
}

#[test]
fn base_skin_naming_other_skin_is_reported() {
    let (_tmp, files) = tree(
        &custom(vec![("ChampionSkinName", string("AhriSkin07"))]),
        Some(game()),
    );

    assert_eq!(
        messages(&found_in(&files)),
        ["ChampionSkinName names another skin: AhriSkin07"]
    );
}

#[test]
fn base_skin_with_other_skin_audio_tag_is_reported() {
    let audio = embed(
        "SkinAudioProperties",
        vec![(
            "TagEventList",
            list(PropertyKind::String, vec![string("AhriSkin7")]),
        )],
    );
    let (_tmp, files) = tree(&custom(vec![("SkinAudioProperties", audio)]), Some(game()));

    assert_eq!(
        messages(&found_in(&files)),
        ["SkinAudioProperties.TagEventList names another skin: AhriSkin7"]
    );
}

#[test]
fn base_skin_with_other_skin_bank_unit_is_reported() {
    let unit = embed("BankUnit", vec![("Name", string("Ahri_Skin07_SFX"))]);
    let audio = embed(
        "SkinAudioProperties",
        vec![("BankUnits", list(PropertyKind::Embedded, vec![unit]))],
    );
    let (_tmp, files) = tree(&custom(vec![("SkinAudioProperties", audio)]), Some(game()));

    assert_eq!(
        messages(&found_in(&files)),
        ["SkinAudioProperties.BankUnits.Name names another skin: Ahri_Skin07_SFX"]
    );
}

#[test]
fn custom_base_skin_is_not_reported() {
    let (_tmp, files) = tree(
        &custom(vec![("ChampionSkinName", string("AhriSkin0"))]),
        Some(game()),
    );

    assert!(found_in(&files).is_empty());
}

#[test]
fn unmodified_base_skin_is_not_reported() {
    let (_tmp, files) = tree(&[(in_layer(SKIN0), base())], Some(game()));

    assert!(found_in(&files).is_empty());
}

#[test]
fn base_skin_is_not_reported_without_game() {
    let (_tmp, files) = tree(
        &[(
            in_layer(SKIN0),
            skin_bin(0, SKIN7_SKL, SKIN7_SKN, Vec::new()),
        )],
        None,
    );

    assert!(found_in(&files).is_empty());
}

#[test]
fn base_skin_outside_its_champion_wad_is_not_reported() {
    let (_tmp, files) = tree(
        &[(
            format!("Common.wad.client/{SKIN0}"),
            skin_bin(0, SKIN7_SKL, SKIN7_SKN, Vec::new()),
        )],
        Some(game()),
    );

    assert!(found_in(&files).is_empty());
}

#[test]
fn names_other_skin_matches_nonzero_skin_number() {
    assert!(names_other_skin("AatroxSkin07", "aatroxskin", ""));
    assert!(names_other_skin("AatroxSkin7", "aatroxskin", ""));
    assert!(names_other_skin("AatroxSkin123", "aatroxskin", ""));
    assert!(names_other_skin("Aatrox_Skin07_SFX", "aatrox_skin", "_sfx"));
}

#[test]
fn names_other_skin_ignores_base_skin_and_missing_suffix() {
    assert!(!names_other_skin("AatroxSkin0", "aatroxskin", ""));
    assert!(!names_other_skin("Aatrox", "aatroxskin", ""));
    assert!(!names_other_skin("Aatrox_Skin07_VO", "aatrox_skin", "_sfx"));
    assert!(!names_other_skin(
        "AatroxSkin99999999999999999999",
        "aatroxskin",
        ""
    ));
}

#[test]
fn names_other_skin_searches_past_first_match() {
    assert!(names_other_skin(
        "AatroxSkin0 AatroxSkin12",
        "aatroxskin",
        ""
    ));
}
