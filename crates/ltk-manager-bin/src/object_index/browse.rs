//! The path-ordered view the objects browser reads, one prefix at a time.
//!
//! "Objects browser" in `docs/ux/PROJECT_EDITOR.md`.

use ltk_game_index::{Dir, ListOptions, ObjectNode, TreeEntry};
use ltk_hash::BinHash;

use crate::bin_document::hex;
use ltk_manager_base::utils::natural_order::compare_names;

use super::{ObjectDirListing, ObjectIndex, ObjectNodeEntry, ObjectPrefixEntry, UNNAMED_PREFIX};

impl ObjectIndex {
    /// What one prefix of the object tree holds, or `None` where no object path runs
    /// through it.
    ///
    /// `prefix` is `""` for the root, [`UNNAMED_PREFIX`] for the objects no table names,
    /// and otherwise a path a listing gave. An object at the prefix itself is the node's
    /// own row and not a child. A run of prefixes each holding one prefix and no object
    /// folds into one row, the game index's rule.
    #[must_use]
    pub fn object_dir(&self, prefix: &str) -> Option<ObjectDirListing> {
        let dir = match prefix {
            "" => Dir::Root,
            UNNAMED_PREFIX => Dir::Unnamed,
            path => Dir::Path(path),
        };
        let mut options = ListOptions::default();
        options.limit = usize::MAX;
        options.fold_single = true;
        let Some(listing) = self.search.list(dir, &options) else {
            /* An object with nothing under it is a path that runs through the tree. */
            return self.search.node(prefix).map(|_| ObjectDirListing {
                prefixes: Vec::new(),
                objects: Vec::new(),
            });
        };

        let mut prefixes = Vec::new();
        let mut objects = Vec::new();
        let mut unnamed = None;
        for entry in listing.entries {
            match entry {
                TreeEntry::Dir {
                    path,
                    name,
                    objects: count,
                    own: None,
                } => prefixes.push(ObjectPrefixEntry { path, name, count }),
                TreeEntry::Dir {
                    objects: count,
                    own: Some(node),
                    ..
                } => objects.push(self.node_entry(node, count)),
                TreeEntry::Object(node) if prefix == UNNAMED_PREFIX => {
                    objects.push(self.unnamed_entry(node.object));
                }
                TreeEntry::Object(node) => objects.push(self.node_entry(node, 0)),
                TreeEntry::Unnamed { objects: count } => {
                    unnamed = Some(ObjectPrefixEntry {
                        path: UNNAMED_PREFIX.to_owned(),
                        name: UNNAMED_PREFIX.to_owned(),
                        count,
                    });
                }
                _ => {}
            }
        }

        if prefix != UNNAMED_PREFIX {
            prefixes.sort_by(|a, b| compare_names(&a.name, &b.name));
            objects.sort_by(|a, b| compare_names(&a.name, &b.name));
        }

        /* Last, and only at the root. The unnamed push no named path down. */
        prefixes.extend(unnamed);

        Some(ObjectDirListing { prefixes, objects })
    }

    /// The row of a named object with `count` objects below it.
    fn node_entry(&self, node: ObjectNode, count: u32) -> ObjectNodeEntry {
        ObjectNodeEntry {
            object_hash: hex(node.object),
            declarations: self.declarations_of(node.object),
            path: node.path,
            name: node.name,
            count,
        }
    }

    /// The row of an object no table names, its hex for a path and a name.
    fn unnamed_entry(&self, object: BinHash) -> ObjectNodeEntry {
        let text = hex(object);
        ObjectNodeEntry {
            object_hash: text.clone(),
            path: text.clone(),
            name: text,
            declarations: self.declarations_of(object),
            count: 0,
        }
    }
}
