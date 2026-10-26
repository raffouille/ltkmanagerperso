//! The mimir tables as the name source of the object index's sheets.

use ltk_game_index::{NameSource, NameTable};
use ltk_mimir_cache::{HashStore, Manifest, Table};

use super::WAD_TABLES;

/// The mimir tables behind one name table. The WAD tables name a chunk.
fn mimir_tables(table: NameTable) -> &'static [Table] {
    match table {
        NameTable::Entries => &[Table::BinEntries],
        NameTable::Types => &[Table::BinTypes],
        NameTable::Fields => &[Table::BinFields],
        NameTable::Hashes => &[Table::BinHashes],
        _ => &WAD_TABLES,
    }
}

/// The tables of one mimir cache, each versioned by the manifest's checksum of its files.
#[derive(Debug, Clone)]
pub struct MimirNames {
    store: HashStore,
    /// `None` for a cache never synced, which versions every table as `None`.
    manifest: Option<Manifest>,
}

impl MimirNames {
    pub(super) fn new(store: HashStore) -> Self {
        let manifest = store
            .manifest()
            .inspect_err(|e| tracing::debug!("No hashtable manifest to version names by: {e}"))
            .ok();
        Self { store, manifest }
    }
}

impl NameSource for MimirNames {
    /// The checksums of the table's files, joined by `+`, or `None` if one is not installed.
    fn version(&self, table: NameTable) -> Option<String> {
        let manifest = self.manifest.as_ref()?;
        let checksums = mimir_tables(table)
            .iter()
            .map(|&table| manifest.entry(table).map(|entry| entry.sha256.as_str()))
            .collect::<Option<Vec<&str>>>()?;
        Some(checksums.join("+"))
    }

    fn for_each_name(&self, table: NameTable, visit: &mut dyn FnMut(u64, &str)) {
        for &table in mimir_tables(table) {
            let db = match self.store.open_shared(table) {
                Ok(db) => db,
                Err(e) => {
                    tracing::debug!("Hashtable `{table}` unavailable for the name sheets: {e}");
                    continue;
                }
            };
            for (hash, name) in db.iter() {
                visit(hash, name.as_ref());
            }
        }
    }
}
