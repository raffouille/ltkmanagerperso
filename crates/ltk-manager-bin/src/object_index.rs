//! Every bin object the install declares, and the file that declares it.
//!
//! A [`GameSearch`] of `ltk_game_index` holds the declarations, the terms of every
//! declaration and the name sheets that resolve their hashes. This module keeps the
//! palette's ranking, the full search, the objects browser and the reference walk over it.
//!
//! Per "The bin object index" in `docs/ux/PROJECT_EDITOR.md`, and section 10 of
//! `docs/research/bin-object-index.md`.

use std::borrow::Cow;
use std::collections::HashMap;

use ltk_game_index::{Declaration, GameSearch};
use ltk_hash::BinHash;
use ltk_wad::{WadHash, hex_name};

use crate::bin_document::hex;
use ltk_manager_assets::preview::AssetRef;

mod browse;
mod build;
mod find;
mod references;
mod search;
mod state;
mod types;
mod walk;

pub use ltk_game_index::{NameSource, NameTable, ObjectStats, TermIndex};
pub use state::{
    BuildTicket, IndexSnapshot, IndexState, ObjectFindGeneration, ObjectIndexSnapshot,
    ObjectIndexState, ObjectReferenceGeneration, ObjectSearchGeneration,
};
pub use types::{
    DeclaredObject, ObjectClassHit, ObjectDeclaration, ObjectDirListing, ObjectFindHit,
    ObjectFindResult, ObjectNodeEntry, ObjectPrefixEntry, ObjectSearchHit, ObjectSearchResult,
    ReferenceGroup, ReferenceHit, ReferenceProperty, ReferenceResult,
};
pub use walk::{FileTarget, LayerBin, WalkRequest, WalkTarget, layer_bins};

use ltk_manager_base::generation::STALE_CHECK_INTERVAL;

/// The prefix of the group holding the objects no table names.
///
/// A resolved object path never holds `?`. The group collides with no path the
/// game ships.
pub const UNNAMED_PREFIX: &str = "?";

/// The entry of an object the `Entries` sheet does not name.
const NO_ENTRY: u32 = u32::MAX;

/// One bin chunk that declares an object, which a declaration's chunk resolves to.
#[derive(Debug)]
struct DeclaringFile {
    path_hash: WadHash,
    /// Index into [`Declarations::wads`].
    wad: u32,
}

/// The declaring files of the install and the archives they sit in.
#[derive(Debug)]
struct Declarations {
    /// Every declaring chunk in archive order, by path within one archive, the unnamed last.
    files: Vec<DeclaringFile>,
    /// A declaring chunk's index in `files`, by its path hash.
    by_file: HashMap<WadHash, u32>,
    /// Archive names, in the chunk index's order.
    wads: Vec<String>,
}

/// A name source that names nothing, for an index built before any table is synced.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoNames;

impl NameSource for NoNames {
    fn version(&self, _table: NameTable) -> Option<String> {
        None
    }

    fn for_each_name(&self, _table: NameTable, _visit: &mut dyn FnMut(u64, &str)) {}
}

/// Every bin object the install declares, searchable by path or by hash.
///
/// Built once a session behind the Objects switch. The names are the sheets the
/// build cut from the hash tables, so a hashtable sync builds the index again.
#[derive(Debug)]
pub struct ObjectIndex {
    /// The term index of the install and the name sheets that resolve its hashes.
    search: GameSearch,
    declared: Declarations,
    /// Every object the install declares, ascending.
    objects: Vec<BinHash>,
    /// Per object, the `Entries` sheet entry naming it, or [`NO_ENTRY`].
    entries: Vec<u32>,
    /// Per object, the letters of its lowercased path, for the mask to reject on.
    masks: Vec<u32>,
    /// Per row, the position of its object in `objects`.
    row_objects: Vec<u32>,
    /// The name of every class a row declares that a table names.
    classes: HashMap<BinHash, Box<str>>,
    /// How many objects the `Entries` sheet names.
    named: usize,
}

impl ObjectIndex {
    /// Every declaration of `object` in archive order, serialized.
    fn declarations_of(&self, object: BinHash) -> Vec<ObjectDeclaration> {
        self.index()
            .declarations(object)
            .map(|(_, row)| self.declaration(row))
            .collect()
    }

    /// What the build measured.
    pub fn stats(&self) -> ObjectStats {
        *self.index().stats()
    }

    /// The term index of the install, which a reference walk reads its postings from.
    #[must_use]
    pub fn terms(&self) -> &TermIndex {
        self.search.terms()
    }

    /// Whether any file of the install declares `object`.
    #[must_use]
    pub fn declares(&self, object: BinHash) -> bool {
        self.index().declares(object)
    }

    /// Whether any file of the install declares `object` as an instance of `class`.
    #[must_use]
    pub fn declares_class(&self, object: BinHash, class: BinHash) -> bool {
        self.index()
            .declarations(object)
            .any(|(_, row)| row.class == class)
    }

    /// Returns the chunk of each declaration of `object`, in archive order.
    pub fn declaring_chunks(&self, object: BinHash) -> impl Iterator<Item = WadHash> + '_ {
        self.index().declarations(object).map(|(_, row)| row.chunk)
    }

    /// Every object a table names, with its path as the table writes it, in path order.
    pub fn named_objects(&self) -> impl Iterator<Item = (BinHash, Cow<'_, str>)> + '_ {
        self.named_under("")
    }

    /// Every object a table names at or under `prefix`, with its path, in path order.
    ///
    /// `prefix` is compared without regard to ASCII case. An empty prefix yields every
    /// named object.
    pub fn named_under<'a>(
        &'a self,
        prefix: &str,
    ) -> impl Iterator<Item = (BinHash, Cow<'a, str>)> + 'a {
        self.search
            .under(prefix)
            .filter_map(|(_, object)| Some((object, self.object_path(object)?)))
    }

    /// The objects no table names, by hash.
    pub fn unnamed_objects(&self) -> impl Iterator<Item = BinHash> + '_ {
        self.objects
            .iter()
            .zip(&self.entries)
            .filter(|(_, entry)| **entry == NO_ENTRY)
            .map(|(object, _)| *object)
    }

    /// Every declaration of `object` in archive order, or `None` where nothing declares it.
    #[must_use]
    pub fn declared(&self, object: BinHash) -> Option<DeclaredObject> {
        let declarations = self.declarations_of(object);
        if declarations.is_empty() {
            return None;
        }
        Some(DeclaredObject {
            path: self.object_name(object),
            declarations,
        })
    }

    /// `row` as a serialized declaration.
    fn declaration(&self, row: &Declaration) -> ObjectDeclaration {
        let (file, wad) = self.declared.file(row.chunk).map_or_else(
            || (hex_name(row.chunk), String::new()),
            |file| {
                (
                    self.file_name(file.path_hash),
                    self.declared.wads[file.wad as usize].clone(),
                )
            },
        );
        ObjectDeclaration {
            asset: AssetRef::GameChunk {
                wad,
                path_hash: hex_name(row.chunk),
            },
            file,
            class_hash: hex(row.class),
            class: self.class_name(row.class),
        }
    }

    /// The declarations of the install, in storage order. A declaration's position is its row.
    fn rows(&self) -> &[Declaration] {
        self.index().rows()
    }

    /// The object index of the install.
    fn index(&self) -> &ltk_game_index::ObjectIndex {
        self.search.terms().objects()
    }

    /// The position of `object` in `objects`, or `None` where nothing declares it.
    fn position(&self, object: BinHash) -> Option<usize> {
        self.objects.binary_search(&object).ok()
    }

    /// The `Entries` sheet entry naming the object at `at` in `objects`.
    fn entry_at(&self, at: usize) -> Option<u32> {
        Some(self.entries[at]).filter(|&entry| entry != NO_ENTRY)
    }

    /// The object's path as the table writes it, or `None` where no table names it.
    fn object_path(&self, object: BinHash) -> Option<Cow<'_, str>> {
        let entry = self.entry_at(self.position(object)?)?;
        Some(self.entries_sheet().entry(entry))
    }

    /// The object's path, or its hex when no table names it.
    fn object_name(&self, object: BinHash) -> String {
        self.object_path(object)
            .map_or_else(|| hex(object), Cow::into_owned)
    }

    /// The class's name, or its hex when no table names it.
    fn class_name(&self, class: BinHash) -> String {
        self.classes
            .get(&class)
            .map_or_else(|| hex(class), ToString::to_string)
    }

    /// The path the `Chunks` sheet gives a declaring chunk, else its hex.
    fn file_name(&self, path_hash: WadHash) -> String {
        self.search
            .name(NameTable::Chunks, path_hash.0)
            .map_or_else(|| hex_name(path_hash), Cow::into_owned)
    }

    /// The sheet of object paths.
    fn entries_sheet(&self) -> &ltk_game_index::Sheet {
        self.search.names().sheet(NameTable::Entries)
    }

    /// Every row as `(object, class, declaring file name)`, file by file in the order of
    /// `files`.
    #[cfg(test)]
    fn test_rows(&self) -> impl Iterator<Item = (BinHash, BinHash, String)> {
        self.declared.files.iter().flat_map(|file| {
            self.index()
                .chunk_declarations(file.path_hash)
                .map(|(_, row)| (row.object, row.class, self.file_name(file.path_hash)))
        })
    }
}

impl Declarations {
    fn file(&self, path_hash: WadHash) -> Option<&DeclaringFile> {
        self.by_file
            .get(&path_hash)
            .map(|&at| &self.files[at as usize])
    }
}

/// The object hash a query of eight hex digits names, `0x` or not.
#[must_use]
pub fn parse_hash(query: &str) -> Option<BinHash> {
    let digits = query.trim();
    let digits = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
        .unwrap_or(digits);
    if digits.len() != 8 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(digits, 16).ok().map(BinHash)
}

#[cfg(test)]
mod tests;
