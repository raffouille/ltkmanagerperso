//! The open bin and the bins it links, which every read resolves an object through.
//!
//! A consumer names an object by its hash against the open document. Only the read that
//! resolves the object knows that a linked bin declares it (ADR-0068). Each domain module adds
//! its read to [`ObjectLookup`]: [`ObjectLookup::system`], [`ObjectLookup::skin`], [`ObjectLookup::graph`] and
//! [`ObjectLookup::programs`].

use std::collections::{HashSet, VecDeque};

use ltk_hash::BinHash;
use ltk_manager_assets::preview::AssetRef;
use ltk_manager_bin::bin_document::{AssetLookup, BinDocument, Locator, RowNames};
use ltk_manager_bin::object_index::parse_hash;

use crate::material::{MaterialPreview, linked_material};

/// The most linked bins one read opens, however deep the links run.
pub const LINKED_CAP: usize = 32;

/// Where [`ObjectLookup::find`] found an object: the open bin, or a linked bin it read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Declared {
    Open,
    /// The index of the linked bin among those the lookup read.
    Linked(usize),
}

/// Reads the bin at an asset, and answers `None` for one it cannot read.
pub type BinReader<'a> = Box<dyn FnMut(&AssetRef) -> Option<BinDocument> + 'a>;

/// The open bin and the bins it links, read breadth first as a read asks for objects, with the
/// names, the assets and the shader defs that every object is resolved against.
///
/// Each bin is read at most once per lookup, and at most [`LINKED_CAP`] bins are opened.
/// The links of a bin come in the order its header lists them, so the bin nearest the open
/// one is read first. A bin that `read` cannot parse is passed over.
pub struct ObjectLookup<'a> {
    open: &'a BinDocument,
    names: &'a dyn RowNames,
    assets: &'a dyn AssetLookup,
    shaders: Option<&'a BinDocument>,
    read: BinReader<'a>,
    linked: Vec<(AssetRef, BinDocument)>,
    queue: VecDeque<AssetRef>,
    seen: HashSet<AssetRef>,
    opened: usize,
}

impl<'a> ObjectLookup<'a> {
    /// Returns the lookup over `open`, which queues the bins it links that `assets` locates.
    ///
    /// `shaders` is `data/shaders/shaders.bin`, which a material's pass shader resolves in.
    pub fn new(
        open: &'a BinDocument,
        names: &'a dyn RowNames,
        assets: &'a dyn AssetLookup,
        shaders: Option<&'a BinDocument>,
        read: impl FnMut(&AssetRef) -> Option<BinDocument> + 'a,
    ) -> Self {
        let mut lookup = Self {
            open,
            names,
            assets,
            shaders,
            read: Box::new(read),
            linked: Vec::new(),
            queue: VecDeque::new(),
            seen: HashSet::new(),
            opened: 0,
        };
        lookup.queue_links(open);
        lookup
    }

    /// Returns the lookup over `open` alone, which reads no linked bin.
    pub fn single(
        open: &'a BinDocument,
        names: &'a dyn RowNames,
        assets: &'a dyn AssetLookup,
        shaders: Option<&'a BinDocument>,
    ) -> Self {
        Self::new(open, names, assets, shaders, |_| None)
    }

    /// The open bin.
    pub fn open(&self) -> &'a BinDocument {
        self.open
    }

    /// The names that hashes resolve to.
    pub fn names(&self) -> &'a dyn RowNames {
        self.names
    }

    /// Where the files that the bins name live.
    pub fn assets(&self) -> &'a dyn AssetLookup {
        self.assets
    }

    /// `data/shaders/shaders.bin`, and none where the read has no copy.
    pub fn shaders(&self) -> Option<&'a BinDocument> {
        self.shaders
    }

    /// Returns where `entry` is declared: the open bin first, then each linked bin in the
    /// order the lookup reads them. Returns `None` if no bin of the lookup declares it.
    pub fn find(&mut self, entry: BinHash) -> Option<Declared> {
        self.find_where(|document| document.object_at(entry).is_some())
    }

    /// Returns the first bin of the lookup that `wanted` accepts, in the order of
    /// [`ObjectLookup::find`].
    pub fn find_where(&mut self, mut wanted: impl FnMut(&BinDocument) -> bool) -> Option<Declared> {
        if wanted(self.open) {
            return Some(Declared::Open);
        }
        if let Some(at) = self.linked.iter().position(|(_, bin)| wanted(bin)) {
            return Some(Declared::Linked(at));
        }

        while let Some(at) = self.read_next() {
            if wanted(&self.linked[at].1) {
                return Some(Declared::Linked(at));
            }
        }
        None
    }

    /// Returns the bin at `found`.
    pub fn document(&self, found: Declared) -> &BinDocument {
        match found {
            Declared::Open => self.open,
            Declared::Linked(at) => &self.linked[at].1,
        }
    }

    /// Returns the linked bin at `found`, and `None` for the open bin.
    pub fn asset(&self, found: Declared) -> Option<&AssetRef> {
        match found {
            Declared::Open => None,
            Declared::Linked(at) => Some(&self.linked[at].0),
        }
    }

    /// Reads `asset` into the lookup, if it is not read yet, and queues the bins it links.
    ///
    /// A read that names a bin by convention, such as the skin bin of a system's path,
    /// starts from it as from a link. Returns `None` if the cap is reached or `read` cannot
    /// parse the bin.
    pub fn bin(&mut self, asset: AssetRef) -> Option<Declared> {
        if let Some(at) = self.linked.iter().position(|(read, _)| *read == asset) {
            return Some(Declared::Linked(at));
        }
        if !self.seen.insert(asset.clone()) {
            return None;
        }
        self.load(asset).map(Declared::Linked)
    }

    /// Reads the next queued bin, and returns its index. Returns `None` once the queue is
    /// empty or the cap is reached.
    fn read_next(&mut self) -> Option<usize> {
        while self.opened < LINKED_CAP {
            let asset = self.queue.pop_front()?;
            if let Some(at) = self.load(asset) {
                return Some(at);
            }
        }
        None
    }

    /// Reads `asset`, queues the bins it links and returns its index. Returns `None` if the
    /// cap is reached or `read` cannot parse the bin.
    fn load(&mut self, asset: AssetRef) -> Option<usize> {
        if self.opened == LINKED_CAP {
            return None;
        }
        self.opened += 1;

        let document = (self.read)(&asset)?;
        self.queue_links(&document);
        self.linked.push((asset, document));
        Some(self.linked.len() - 1)
    }

    fn queue_links(&mut self, document: &BinDocument) {
        for next in document
            .dependencies()
            .iter()
            .filter_map(|path| self.assets.locate(path))
        {
            if self.seen.insert(next.clone()) {
                self.queue.push_back(next);
            }
        }
    }
}

/// Reads each of `missing` from the first bin of the lookup that declares it.
///
/// A material that no bin of the lookup declares stays missing.
pub(crate) fn find_linked_materials(
    mut missing: Vec<&mut MaterialPreview>,
    lookup: &mut ObjectLookup<'_>,
) {
    missing.retain(|material| material.missing);
    let locator = Locator {
        names: lookup.names(),
        assets: lookup.assets(),
    };

    for material in missing {
        let Some(hash) = parse_hash(&material.hash) else {
            continue;
        };
        if let Some(found) = lookup.find(hash) {
            *material = linked_material(lookup.document(found), hash, &locator, lookup.shaders());
        }
    }
}

#[cfg(test)]
mod tests;
