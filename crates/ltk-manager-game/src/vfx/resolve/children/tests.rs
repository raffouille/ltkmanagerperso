//! Unit tests for the child search: which bin a child is read from and when it stays a link.

use std::collections::HashMap;
use std::io::Cursor;

use ltk_hash::{Hash as _, WadHash};
use ltk_meta::property::{Kind, values};
use ltk_meta::{Bin, BinObject};

use super::*;
use ltk_manager_assets::preview::AssetRef;
use ltk_manager_bin::bin_document::{AssetLookup, BinDocument, RowNames, hex};

fn h(text: &str) -> BinHash {
    BinHash::hash_str(text)
}

const PARENT: &str = "Characters/Aatrox/Skins/Skin11/Particles/Aatrox_Skin11_Parent";
const CHILD: &str = "Characters/Aatrox/Skins/Skin0/Particles/Aatrox_Base_Child";
const SKIN_BIN: &str = "data/characters/aatrox/skins/skin11.bin";
const LINKED_BIN: &str = "data/characters/aatrox/aatrox_shared.bin";

/// Tables that name the two systems and nothing else.
struct Tables;

impl RowNames for Tables {
    fn for_each_entry(&self, hashes: &[BinHash], visit: &mut dyn FnMut(usize, &str)) {
        for (at, hash) in hashes.iter().enumerate() {
            for name in [PARENT, CHILD] {
                if *hash == h(name) {
                    visit(at, name);
                }
            }
        }
    }

    fn for_each_class(&self, _: &[BinHash], _: &mut dyn FnMut(usize, &str)) {}

    fn for_each_field(&self, _: &[BinHash], _: &mut dyn FnMut(usize, &str)) {}

    fn for_each_value(&self, _: &[BinHash], _: &mut dyn FnMut(usize, &str)) {}

    fn for_each_chunk(&self, _: &[WadHash], _: &mut dyn FnMut(usize, &str)) {}
}

/// A lookup that places every bin path at a layer path of the same spelling.
struct Bins;

impl AssetLookup for Bins {
    fn locate(&self, path: &str) -> Option<AssetRef> {
        path.ends_with(".bin").then(|| asset(path))
    }
}

fn asset(path: &str) -> AssetRef {
    AssetRef::Layer {
        project: "C:/projects/aatrox".to_owned(),
        layer: "base".to_owned(),
        path: path.to_lowercase(),
    }
}

fn document_of(objects: Vec<BinObject>) -> BinDocument {
    document_linking(objects, &[])
}

fn document_linking(objects: Vec<BinObject>, dependencies: &[&str]) -> BinDocument {
    let mut bin = Bin::builder().dependencies(dependencies.iter().copied());
    for object in objects {
        bin = bin.object(object);
    }
    let mut out = Cursor::new(Vec::new());
    bin.build().to_writer(&mut out).unwrap();
    BinDocument::parse(out.into_inner()).unwrap()
}

/// A system at `path` whose one child identifier writes `field` as `value`.
fn parent(path: &str, field: &str, value: PropertyValue) -> BinObject {
    let identifier = values::Embedded(values::Struct {
        class_hash: h("VfxChildIdentifier"),
        properties: [(h(field), value)].into_iter().collect(),
    });
    let set = values::Embedded(values::Struct {
        class_hash: h("VfxChildParticleSetDefinitionData"),
        properties: [(
            h("childrenIdentifiers"),
            values::Container::from(vec![identifier]).into(),
        )]
        .into_iter()
        .collect(),
    });

    BinObject::builder(h(path), h("VfxSystemDefinitionData"))
        .property(h("childParticleSetDefinition"), set)
        .build()
}

type PropertyValue = ltk_meta::PropertyValueEnum;

fn system(path: &str) -> BinObject {
    BinObject::builder(h(path), h("VfxSystemDefinitionData"))
        .property(h("particleName"), values::String::from("child"))
        .build()
}

fn resolver(key: &str, target: &str) -> BinObject {
    let map = values::Map::new(
        Kind::Hash,
        Kind::ObjectLink,
        vec![(
            values::Hash::new(h(key)).into(),
            values::ObjectLink::new(h(target)).into(),
        )],
    )
    .unwrap();
    BinObject::builder(
        h("Characters/Aatrox/Skins/Skin11/Resources"),
        h("ResourceResolver"),
    )
    .property(h("resourceMap"), map)
    .build()
}

/// The identifier's `field` after a search of `open` for `PARENT`, with `bins` readable by
/// path, and the paths the search read.
fn searched(
    open: Vec<BinObject>,
    field: &str,
    bins: HashMap<&str, Vec<BinObject>>,
) -> (VfxValue, Vec<AssetRef>) {
    let open = document_linking(open, &[LINKED_BIN]);
    let mut system = resolve_system(&open, h(PARENT), &Tables, &Bins, None).unwrap();
    let mut documents: HashMap<AssetRef, Vec<BinObject>> = bins
        .into_iter()
        .map(|(path, objects)| (asset(path), objects))
        .collect();
    let mut read = Vec::new();

    {
        let mut reading = |asset: &AssetRef| {
            read.push(asset.clone());
            documents.remove(asset).map(document_of)
        };
        let mut lookup = ObjectLookup::new(&open, &Tables, &Bins, None, &mut reading);
        search_linked_children(&mut system, Declared::Open, &mut lookup);
    }

    let VfxValue::Struct { fields, .. } = &system.root else {
        panic!("the root is a struct");
    };
    let set = &fields[0].value;
    let VfxValue::Struct { fields, .. } = set else {
        panic!("the set is a struct");
    };
    let VfxValue::Container { items } = &fields[0].value else {
        panic!("the identifiers are a container");
    };
    let VfxValue::Struct { fields, .. } = &items[0] else {
        panic!("the identifier is a struct");
    };
    let value = fields
        .iter()
        .find(|each| parse_hash(&each.hash) == Some(h(field)))
        .map(|each| each.value.clone())
        .unwrap();
    (value, read)
}

fn inlined_entry(value: &VfxValue) -> Option<String> {
    match value {
        VfxValue::Struct {
            object: Some(object),
            ..
        } => Some(object.entry.clone()),
        _ => None,
    }
}

#[test]
fn search_linked_children_inlines_effect_link_from_linked_bin() {
    let (effect, _) = searched(
        vec![parent(
            PARENT,
            "effect",
            values::ObjectLink::new(h(CHILD)).into(),
        )],
        "effect",
        HashMap::from([(LINKED_BIN, vec![system(CHILD)])]),
    );

    assert_eq!(inlined_entry(&effect), Some(hex(h(CHILD))));
}

#[test]
fn search_linked_children_inlines_effect_key_from_open_resolver_and_linked_system() {
    let (key, _) = searched(
        vec![
            parent(
                PARENT,
                "effectKey",
                values::Hash::new(h("Child_Key")).into(),
            ),
            resolver("Child_Key", CHILD),
        ],
        "effectKey",
        HashMap::from([(LINKED_BIN, vec![system(CHILD)])]),
    );

    assert_eq!(inlined_entry(&key), Some(hex(h(CHILD))));
}

#[test]
fn search_linked_children_resolves_effect_key_through_skin_bin_of_system_path() {
    let (key, read) = searched(
        vec![
            parent(
                PARENT,
                "effectKey",
                values::Hash::new(h("Child_Key")).into(),
            ),
            system(CHILD),
        ],
        "effectKey",
        HashMap::from([(SKIN_BIN, vec![resolver("Child_Key", CHILD)])]),
    );

    assert_eq!(inlined_entry(&key), Some(hex(h(CHILD))));
    assert!(read.contains(&asset(SKIN_BIN)));
}

#[test]
fn search_linked_children_leaves_link_to_open_ancestor() {
    let (effect, _) = searched(
        vec![parent(
            PARENT,
            "effect",
            values::ObjectLink::new(h(CHILD)).into(),
        )],
        "effect",
        HashMap::from([(
            LINKED_BIN,
            vec![parent(
                CHILD,
                "effect",
                values::ObjectLink::new(h(PARENT)).into(),
            )],
        )]),
    );

    let VfxValue::Struct { fields, .. } = &effect else {
        panic!("the child inlined: {effect:?}");
    };
    let nested = format!("{fields:?}");
    assert!(nested.contains("Link"), "{nested}");
}

#[test]
fn search_linked_children_reads_no_bin_if_every_child_resolves() {
    let (effect, read) = searched(
        vec![
            parent(PARENT, "effect", values::ObjectLink::new(h(CHILD)).into()),
            system(CHILD),
        ],
        "effect",
        HashMap::new(),
    );

    assert_eq!(inlined_entry(&effect), Some(hex(h(CHILD))));
    assert!(read.is_empty());
}

#[test]
fn skin_bin_names_bin_of_skin_in_path() {
    assert_eq!(skin_bin(PARENT).as_deref(), Some(SKIN_BIN));
    assert_eq!(skin_bin("Maps/Shipping/Map11/Particles/Foo"), None);
    assert_eq!(skin_bin("Characters/Aatrox/Skins/Skin11"), None);
}
