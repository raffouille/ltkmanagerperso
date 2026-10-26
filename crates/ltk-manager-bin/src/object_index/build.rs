//! The read of every bin chunk of the install, through the term index of `ltk_game_index`,
//! and the name sheets that resolve it.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::Path;

use ltk_game_index::{
    BuildOptions, Dir, GameSearch, ListOptions, NameSheets, NameSource, NameTable,
    ObjectBuildError, ResolveWadPath, TermIndex, path_cmp,
};
use ltk_hash::BinHash;
use ltk_wad::WadHash;

use ltk_manager_assets::game_wads::ChunkIndex;
use ltk_manager_base::error::{AppError, AppResult, Utf8PathRefExt as _};
use ltk_manager_base::matcher::letter_mask;

use super::{Declarations, DeclaringFile, NO_ENTRY, ObjectIndex};

/// The file under the cache directory that holds the term index.
const TERMS_FILE: &str = "terms.bin";

/// The file under the cache directory that holds the name sheets.
const NAMES_FILE: &str = "names.bin";

impl ObjectIndex {
    /// Reads every bin chunk of `chunks` for its declarations and terms, and names them
    /// through `names`.
    ///
    /// One job per archive in the chunk index's order, each mounting its archive itself,
    /// on at most `workers` threads. `paths` names the chunks for the read. A named chunk
    /// is a bin by its extension. One with a bare name, and one `paths` does not name, is
    /// decoded far enough to read its magic, and read whole only when the magic is a
    /// bin's. A chunk that will not read is skipped and counted.
    ///
    /// `called_off` is tested before each archive, so a build nobody wants stops at the
    /// next one rather than decompressing the rest.
    ///
    /// With `cache`, the term index loads from `terms.bin` under it when the cached
    /// fingerprint matches `chunks`, and the name sheets from `names.bin` for every table
    /// whose version `names` still reports. What does not load is built and written there.
    ///
    /// # Errors
    ///
    /// Fails when the build was called off, and if `cache` is not UTF-8. An archive that
    /// will not mount is skipped and logged.
    pub fn build(
        chunks: &ChunkIndex,
        paths: &dyn ResolveWadPath,
        names: &dyn NameSource,
        workers: usize,
        called_off: &(impl Fn() -> bool + Sync),
        cache: Option<&Path>,
    ) -> AppResult<Self> {
        let cache = cache
            .map(|dir| dir.try_as_utf8("object index cache"))
            .transpose()?;

        let mut options = BuildOptions::default();
        options.resolver = Some(paths);
        options.workers = NonZeroUsize::new(workers);
        options.called_off = Some(called_off);
        let terms = match cache {
            Some(dir) => TermIndex::load_or_build_with(chunks, &dir.join(TERMS_FILE), &options),
            None => TermIndex::build_with(chunks, &options),
        }
        .map_err(build_error)?;

        let sheets = match cache {
            Some(dir) => NameSheets::load_or_build(&dir.join(NAMES_FILE), names, chunks),
            None => NameSheets::build(names, chunks),
        };

        Ok(Self::over(chunks, GameSearch::new(terms, sheets)))
    }

    /// The index over `search`, with the archive names of `chunks`.
    ///
    /// Cuts the object tree the browser reads. The index reads no query of the crate, so the
    /// scan tables and the standing order stay uncut.
    fn over(chunks: &ChunkIndex, search: GameSearch) -> Self {
        let _ = search.list(Dir::Root, &ListOptions::default());

        let objects: Vec<BinHash> = search.terms().objects().objects().collect();
        let sheet = search.names().sheet(NameTable::Entries);
        let mut entries = vec![NO_ENTRY; objects.len()];
        for entry in 0..sheet.len() as u32 {
            let Ok(hash) = u32::try_from(sheet.value(entry)) else {
                continue;
            };
            if let Ok(at) = objects.binary_search(&BinHash(hash))
                && entries[at] == NO_ENTRY
            {
                entries[at] = entry;
            }
        }
        let masks = entries
            .iter()
            .map(|&entry| match entry {
                NO_ENTRY => 0,
                entry => letter_mask(sheet.entry_lower(entry)),
            })
            .collect();
        let named = entries.iter().filter(|&&entry| entry != NO_ENTRY).count();

        let rows = search.terms().objects().rows();
        let row_objects = rows
            .iter()
            .map(|row| objects.binary_search(&row.object).unwrap_or_default() as u32)
            .collect();

        let mut distinct: Vec<BinHash> = rows.iter().map(|row| row.class).collect();
        distinct.sort_unstable();
        distinct.dedup();
        let classes = distinct
            .into_iter()
            .filter_map(|class| {
                let name = search.name(NameTable::Types, u64::from(class.0))?;
                Some((class, name.into_owned().into_boxed_str()))
            })
            .collect();

        let declared = declarations(chunks, &search);
        Self {
            search,
            declared,
            objects,
            entries,
            masks,
            row_objects,
            classes,
            named,
        }
    }
}

/// The app error of a build that did not finish.
fn build_error(error: ObjectBuildError) -> AppError {
    match error {
        ObjectBuildError::CalledOff => {
            AppError::Other("The object index build was called off".to_owned())
        }
        other => AppError::Other(format!("Could not build the object index: {other}")),
    }
}

/// Every chunk that declares an object in `search`, and the archive names of `chunks`.
///
/// The files are in archive order, and within one archive in [`path_cmp`] order of the
/// path the `Chunks` sheet gives them, with the chunks it does not name last, by hash.
fn declarations(chunks: &ChunkIndex, search: &GameSearch) -> Declarations {
    let mut held: Vec<(WadHash, u32)> = search
        .terms()
        .objects()
        .rows()
        .chunk_by(|a, b| a.chunk == b.chunk)
        .map(|run| (run[0].chunk, run[0].archive.index() as u32))
        .collect();
    held.sort_unstable();
    held.dedup_by_key(|(chunk, _)| *chunk);

    let mut named: Vec<(DeclaringFile, Option<String>)> = held
        .into_iter()
        .map(|(path_hash, wad)| {
            let path = search
                .name(NameTable::Chunks, path_hash.0)
                .map(|path| path.to_ascii_lowercase());
            (DeclaringFile { path_hash, wad }, path)
        })
        .collect();
    named.sort_by(|(a, a_path), (b, b_path)| {
        a.wad
            .cmp(&b.wad)
            .then_with(|| match (a_path, b_path) {
                (Some(a), Some(b)) => path_cmp(a, b),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            })
            .then_with(|| a.path_hash.cmp(&b.path_hash))
    });

    let files: Vec<DeclaringFile> = named.into_iter().map(|(file, _)| file).collect();
    let by_file: HashMap<WadHash, u32> = files
        .iter()
        .enumerate()
        .map(|(at, file)| (file.path_hash, at as u32))
        .collect();
    let wads = chunks
        .archives()
        .iter()
        .map(|archive| archive.name.clone())
        .collect();

    Declarations {
        files,
        by_file,
        wads,
    }
}
