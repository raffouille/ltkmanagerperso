use std::collections::HashMap;
use std::io::Cursor;

use ltk_hash::Hash as _;
use ltk_meta::{Bin, BinObject};

use super::*;

fn h(text: &str) -> BinHash {
    BinHash::hash_str(text)
}

fn file(path: &str) -> AssetRef {
    AssetRef::File {
        path: path.to_lowercase(),
    }
}

/// Places every path as a file of that path.
struct Files;

impl AssetLookup for Files {
    fn locate(&self, path: &str) -> Option<AssetRef> {
        Some(file(path))
    }
}

fn bin(objects: &[&str], dependencies: &[&str]) -> BinDocument {
    let mut bin = Bin::builder().dependencies(dependencies.iter().copied());
    for object in objects {
        bin = bin.object(BinObject::builder(h(object), h("VfxSystemDefinitionData")).build());
    }
    let mut out = Cursor::new(Vec::new());
    bin.build().to_writer(&mut out).unwrap();
    BinDocument::parse(out.into_inner()).unwrap()
}

/// Bins by path, and the paths in the order `read` was asked for them.
struct Disk {
    bins: HashMap<AssetRef, (Vec<&'static str>, Vec<&'static str>)>,
    asked: Vec<AssetRef>,
}

impl Disk {
    fn new(bins: &[(&str, &[&'static str], &[&'static str])]) -> Self {
        Self {
            bins: bins
                .iter()
                .map(|(path, objects, links)| (file(path), (objects.to_vec(), links.to_vec())))
                .collect(),
            asked: Vec::new(),
        }
    }

    fn read(&mut self, asset: &AssetRef) -> Option<BinDocument> {
        self.asked.push(asset.clone());
        let (objects, links) = self.bins.get(asset)?;
        Some(bin(objects, links))
    }
}

#[test]
fn find_answers_open_bin_before_reading_links() {
    let open = bin(&["Shared"], &["a.bin"]);
    let mut disk = Disk::new(&[("a.bin", &["Shared"], &[])]);
    let mut read = |asset: &AssetRef| disk.read(asset);

    let mut lookup = ObjectLookup::new(&open, &(), &Files, None, &mut read);

    assert_eq!(lookup.find(h("Shared")), Some(Declared::Open));
    drop(lookup);
    assert!(disk.asked.is_empty());
}

#[test]
fn find_walks_links_breadth_first_and_reads_each_bin_once() {
    let open = bin(&[], &["a.bin", "b.bin"]);
    let mut disk = Disk::new(&[
        ("a.bin", &[], &["c.bin"]),
        ("b.bin", &["InB"], &["c.bin"]),
        ("c.bin", &["InC"], &[]),
    ]);
    let mut read = |asset: &AssetRef| disk.read(asset);
    let mut lookup = ObjectLookup::new(&open, &(), &Files, None, &mut read);

    let in_b = lookup.find(h("InB")).unwrap();
    let in_c = lookup.find(h("InC")).unwrap();

    assert_eq!(lookup.asset(in_b), Some(&file("b.bin")));
    assert_eq!(lookup.asset(in_c), Some(&file("c.bin")));
    assert!(lookup.document(in_c).object_at(h("InC")).is_some());
    drop(lookup);
    assert_eq!(disk.asked, [file("a.bin"), file("b.bin"), file("c.bin")]);
}

#[test]
fn find_answers_none_if_no_bin_within_reach_declares_entry() {
    let open = bin(&[], &["a.bin", "missing.bin"]);
    let mut disk = Disk::new(&[("a.bin", &[], &[])]);
    let mut read = |asset: &AssetRef| disk.read(asset);
    let mut lookup = ObjectLookup::new(&open, &(), &Files, None, &mut read);

    assert_eq!(lookup.find(h("Nowhere")), None);
    assert_eq!(lookup.find(h("Nowhere")), None);
    drop(lookup);
    assert_eq!(disk.asked, [file("a.bin"), file("missing.bin")]);
}

#[test]
fn find_opens_at_most_linked_cap_bins() {
    let names: Vec<String> = (0..LINKED_CAP + 4).map(|at| format!("{at}.bin")).collect();
    let links: Vec<&str> = names.iter().map(String::as_str).collect();
    let open = bin(&[], &links);
    let mut asked = 0;
    let mut read = |_: &AssetRef| {
        asked += 1;
        Some(bin(&[], &[]))
    };
    let mut lookup = ObjectLookup::new(&open, &(), &Files, None, &mut read);

    assert_eq!(lookup.find(h("Nowhere")), None);
    drop(lookup);
    assert_eq!(asked, LINKED_CAP);
}

#[test]
fn bin_reads_named_bin_and_queues_its_links() {
    let open = bin(&[], &[]);
    let mut disk = Disk::new(&[
        ("skin.bin", &[], &["particles.bin"]),
        ("particles.bin", &["Orb"], &[]),
    ]);
    let mut read = |asset: &AssetRef| disk.read(asset);
    let mut lookup = ObjectLookup::new(&open, &(), &Files, None, &mut read);

    let skin = lookup.bin(file("skin.bin")).unwrap();
    let orb = lookup.find(h("Orb")).unwrap();

    assert_eq!(lookup.bin(file("skin.bin")), Some(skin));
    assert_eq!(lookup.asset(orb), Some(&file("particles.bin")));
}
