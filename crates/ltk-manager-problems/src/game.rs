//! Installed game content, for a rule to compare a mod against.
//!
//! Compare against, never take parts out of. Reading the install for parts is a
//! separate decision, recorded where it is made.
//!
//! **The hash tables are not this.** Mimir's tables are a superset across
//! patches, so they still name a path Riot removed. Trusting them here deletes
//! a file whose backing is gone.

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::OnceLock;

use ltk_game_index::{BuildOptions, ObjectIndex};
use ltk_hash::BinHash;
use ltk_hashdb::LayeredHashDb;
use ltk_wad::WadHash;
use parking_lot::Mutex;
use sha2::{Digest, Sha256};

use ltk_manager_assets::game_wads::{ChunkIndex, ChunkPaths, GameArchives, WadCache};
use ltk_manager_assets::hashtables::HashtableCache;
use ltk_manager_base::budget::files_at_once;
use ltk_manager_base::config::Config;

/// The chunks of the installed game.
///
/// Taken by a run rather than built by one, so a library sweep shares one
/// index.
pub trait GameContent: std::fmt::Debug + Send + Sync {
    /// Whether any installed archive contains this chunk.
    fn contains(&self, path: WadHash) -> bool;

    /// The chunk's decompressed size, or `None` where no archive contains it.
    ///
    /// Read from the table of contents, without decompressing the chunk.
    fn size(&self, path: WadHash) -> Option<u64>;

    /// The chunk's decompressed bytes, or `None` where no archive contains it.
    ///
    /// For comparing what a mod ships against what it overrides. Not a parts
    /// source for a repair.
    ///
    /// # Errors
    ///
    /// An archive that would not mount, or a chunk that would not decompress,
    /// as one sentence a panel can draw.
    fn read(&self, path: WadHash) -> Result<Option<Vec<u8>>, String>;

    /// Returns the SHA-256 of the chunk's decompressed bytes, or `None` where no archive
    /// contains it.
    ///
    /// The default reads and hashes the chunk on every call.
    ///
    /// # Errors
    ///
    /// Fails where [`read`](Self::read) fails.
    fn digest(&self, path: WadHash) -> Result<Option<[u8; 32]>, String> {
        Ok(self.read(path)?.map(|bytes| Sha256::digest(&bytes).into()))
    }

    /// Returns the installed chunks that declare the bin object `object`.
    ///
    /// Returns `None` if the bin objects of the install cannot be indexed. The
    /// default implementation returns `None`.
    fn declaring_chunks(&self, object: BinHash) -> Option<Vec<WadHash>> {
        let _ = object;
        None
    }

    /// Build whatever the content keeps lazily, ahead of the first ask.
    ///
    /// The default builds nothing. Content that builds an index keeps it for
    /// later calls.
    fn warm(&self) {}
}

/// The installed game's archives, indexed by chunk hash on first use.
///
/// Lazy because most runs never ask, and the walk costs seconds.
#[derive(Debug)]
pub struct InstalledContent {
    archives: GameArchives,
    /// The chunk index of the install, or `None` if the archives cannot be indexed.
    chunks: OnceLock<Option<ChunkIndex>>,
    /// The bin objects of the install, or `None` if the build failed.
    objects: OnceLock<Option<ObjectIndex>>,
    mounts: WadCache,
    /// The SHA-256 of each chunk read through [`GameContent::digest`], keyed by the chunk
    /// and its table-of-contents checksum.
    digests: Mutex<HashMap<(WadHash, u64), [u8; 32]>>,
}

impl InstalledContent {
    /// The install `config` points at, or `None` for no install.
    #[must_use]
    pub fn resolve(config: &Config) -> Option<Self> {
        GameArchives::resolve(config).ok().map(Self::over)
    }

    /// An index over already-resolved archives.
    #[must_use]
    pub fn over(archives: GameArchives) -> Self {
        Self {
            archives,
            chunks: OnceLock::new(),
            objects: OnceLock::new(),
            mounts: WadCache::default(),
            digests: Mutex::default(),
        }
    }

    /// The install rooted at `game_dir`, the directory that contains `DATA`.
    #[must_use]
    pub fn at(game_dir: &Path) -> Self {
        Self::over(GameArchives::at(game_dir))
    }

    /// Returns the chunk index of the install, and builds it on the first call.
    ///
    /// Tables of contents only, so hundreds of gigabytes cost the few megabytes
    /// their chunk tables come to. An archive that will not mount holds no
    /// chunks: skipping one costs a removal refused, which is the safe
    /// direction. Returns `None` if the archives cannot be indexed.
    fn index(&self) -> Option<&ChunkIndex> {
        self.chunks
            .get_or_init(|| {
                let started = std::time::Instant::now();
                let index = self
                    .archives
                    .chunk_index(None)
                    .inspect_err(|e| {
                        tracing::warn!("Could not index the installed game's archives: {e}");
                    })
                    .ok()?;

                tracing::debug!(
                    "Indexed {} chunks across {} game archives in {:?}",
                    index.len(),
                    index.archives().len(),
                    started.elapsed()
                );
                Some(index)
            })
            .as_ref()
    }

    /// Returns the bin object index of the install, and builds it on the first call.
    ///
    /// Returns `None` if no hashtable is synced or if the build fails. The failure
    /// is logged once, and later calls return `None` without a new build.
    fn objects(&self) -> Option<&ObjectIndex> {
        self.objects
            .get_or_init(|| {
                let cache = HashtableCache::shared()
                    .inspect_err(|e| {
                        tracing::warn!(
                            "Could not open the hashtable cache, so bin objects are not indexed: {e}"
                        );
                    })
                    .ok()?;
                if cache.generation().is_none() {
                    tracing::warn!(
                        "Bin objects are not indexed because no hashtable is synced. Sync the hashtables in Settings."
                    );
                    return None;
                }

                let chunks = self.index()?;
                index_objects(chunks, &cache.wad_tables())
                    .inspect_err(|e| {
                        tracing::warn!("Could not index the installed game's bin objects: {e}");
                    })
                    .ok()
            })
            .as_ref()
    }
}

/// Builds the index of the bin objects that the bins of `chunks` declare.
///
/// `tables` resolves chunk hashes to paths. The build reads the first bytes of every chunk whose hash
/// `tables` does not resolve, so an empty `tables` causes a prefix read of
/// every chunk of the install.
///
/// Fails if the build fails.
fn index_objects(chunks: &ChunkIndex, tables: &LayeredHashDb) -> Result<ObjectIndex, String> {
    let paths = ChunkPaths(tables);
    let mut options = BuildOptions::default();
    options.resolver = Some(&paths);
    options.workers = NonZeroUsize::new(files_at_once());
    ObjectIndex::build_with(chunks, &options).map_err(|e| e.to_string())
}

impl GameContent for InstalledContent {
    fn contains(&self, path: WadHash) -> bool {
        self.index().is_some_and(|index| index.contains(path))
    }

    fn size(&self, path: WadHash) -> Option<u64> {
        self.index()?.row(path).map(|row| row.size())
    }

    fn declaring_chunks(&self, object: BinHash) -> Option<Vec<WadHash>> {
        Some(
            self.objects()?
                .declarations(object)
                .map(|(_, row)| row.chunk)
                .collect(),
        )
    }

    fn warm(&self) {
        self.index();
    }

    /// Read through a cached mount, from the first archive that contains the chunk.
    ///
    /// One table of contents parse per archive, not per chunk.
    fn read(&self, path: WadHash) -> Result<Option<Vec<u8>>, String> {
        let Some((index, row)) = self
            .index()
            .and_then(|index| Some((index, index.row(path)?)))
        else {
            return Ok(None);
        };

        self.mounts
            .read_chunk(
                &self.archives,
                &index.archive(row.first_holder()).name,
                path,
            )
            .map(Some)
            .map_err(|e| e.to_string())
    }

    /// Hashes the chunk once for each table-of-contents checksum, so a patch that changes
    /// the chunk also changes the key.
    fn digest(&self, path: WadHash) -> Result<Option<[u8; 32]>, String> {
        let Some(checksum) = self
            .index()
            .and_then(|index| index.row(path))
            .and_then(|row| row.copies().first())
            .map(|copy| copy.checksum)
        else {
            return Ok(None);
        };

        if let Some(digest) = self.digests.lock().get(&(path, checksum)) {
            return Ok(Some(*digest));
        }

        let Some(bytes) = self.read(path)? else {
            return Ok(None);
        };
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        self.digests.lock().insert((path, checksum), digest);
        Ok(Some(digest))
    }
}

/// An install that contains only the chunks a test gives it.
///
/// The second adapter, which is what makes [`GameContent`] a seam: a unit test
/// cannot depend on a League install.
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct FakeContent {
    chunks: HashMap<WadHash, Vec<u8>>,
    /// The chunks that declare each bin object, or `None` for an install that indexes nothing.
    objects: Option<HashMap<BinHash, Vec<WadHash>>>,
}

#[cfg(test)]
impl FakeContent {
    /// An install that contains `paths`, each with empty bytes.
    pub(crate) fn containing(paths: &[&str]) -> std::sync::Arc<dyn GameContent> {
        Self::of(paths.iter().map(|path| (*path, [].as_slice())))
    }

    /// An install that contains `entries`, each with the bytes a rule reads.
    pub(crate) fn containing_bytes(entries: &[(&str, &[u8])]) -> std::sync::Arc<dyn GameContent> {
        Self::of(entries.iter().copied())
    }

    /// An install with no chunks.
    pub(crate) fn empty() -> std::sync::Arc<dyn GameContent> {
        Self::of(std::iter::empty())
    }

    /// Returns an install that contains `bins`, each as `(chunk path, bytes)`, with its bin
    /// objects indexed.
    pub(crate) fn with_bins(bins: &[(&str, &[u8])]) -> std::sync::Arc<dyn GameContent> {
        use ltk_hash::Hash as _;
        let mut objects: HashMap<BinHash, Vec<WadHash>> = HashMap::new();
        let mut chunks = HashMap::new();

        for (path, bytes) in bins {
            let chunk = WadHash::hash_str(path);
            ltk_manager_assets::bin_source::for_each_declaration(
                std::io::Cursor::new(bytes),
                |declared| objects.entry(declared.object).or_default().push(chunk),
            )
            .expect("a fixture bin reads");
            chunks.insert(chunk, bytes.to_vec());
        }

        std::sync::Arc::new(Self {
            chunks,
            objects: Some(objects),
        })
    }

    fn of<'e>(
        entries: impl Iterator<Item = (&'e str, &'e [u8])>,
    ) -> std::sync::Arc<dyn GameContent> {
        use ltk_hash::Hash as _;
        std::sync::Arc::new(Self {
            chunks: entries
                .map(|(path, bytes)| (WadHash::hash_str(path), bytes.to_vec()))
                .collect(),
            objects: None,
        })
    }
}

#[cfg(test)]
impl GameContent for FakeContent {
    fn contains(&self, path: WadHash) -> bool {
        self.chunks.contains_key(&path)
    }

    fn size(&self, path: WadHash) -> Option<u64> {
        self.chunks.get(&path).map(|bytes| bytes.len() as u64)
    }

    fn read(&self, path: WadHash) -> Result<Option<Vec<u8>>, String> {
        Ok(self.chunks.get(&path).cloned())
    }

    fn declaring_chunks(&self, object: BinHash) -> Option<Vec<WadHash>> {
        let objects = self.objects.as_ref()?;
        Some(objects.get(&object).cloned().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests;
