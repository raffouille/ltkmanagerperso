//! Every archive of an install, folded into one directory tree.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap};
use std::path::PathBuf;
use std::sync::Arc;

use ltk_game_index::ChunkRow;
use ltk_hashdb::LayeredHashDb;
use ltk_wad::{WadHash, hex_name};
use serde::{Deserialize, Serialize};

use crate::game_wads::{ChunkIndex, GameArchives, WadSource};
use ltk_manager_base::error::AppResult;
use ltk_manager_base::generation::{Generation, STALE_CHECK_INTERVAL, line};
use ltk_manager_base::matcher::{FindQuery, Query, Range, SearchHits, letter_mask, mask_covers};
use ltk_manager_base::utils::lazy_slot::LazySlot;
use ltk_manager_base::utils::natural_order::compare_names;

/// The directory id of the group holding chunks no hash table names.
///
/// A resolved WAD path never holds `?`, so the group cannot collide with a
/// directory the game ships.
pub const UNKNOWN_DIR: &str = "?";

/// What one directory of the folded index holds.
#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct GameDirListing {
    /// Subdirectories, sorted by name.
    pub dirs: Vec<GameDirEntry>,
    /// Files directly under this directory, sorted by name.
    pub files: Vec<GameFileEntry>,
}

/// One subdirectory, folded through any chain of single-child directories.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct GameDirEntry {
    /// What [`GameIndex::read_dir`] takes to open this row, forward slashes.
    pub path: String,
    /// What the row reads: the folded chain of segments joined by `/`.
    pub name: String,
    /// Files at or below the directory.
    pub file_count: u32,
}

/// One file of the folded index, in the shape a single archive reads back.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct GameFileEntry {
    /// Chunk path hash as 16 lowercase hex digits.
    pub path_hash: String,
    /// Resolved chunk path, or `None` when no hash table names it.
    pub path: Option<String>,
    /// Uncompressed chunk size.
    pub size_bytes: u64,
    /// The archive the chunk was read from, relative to its source's root.
    ///
    /// The fold drops every copy of a chunk after the first, so this names the
    /// archive that copy came from and not every archive that carries it.
    pub wad: String,
}

/// What a built index holds.
#[derive(Debug, Clone, Copy, Serialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct GameIndexStats {
    /// Archives merged, including any that failed to read.
    pub archives: u32,
    /// Files after deduplication.
    pub files: u32,
    /// Directories, not counting the root.
    pub dirs: u32,
}

/// One row a search matched, with the runs its two lines mark.
///
/// Marked runs are byte offsets into `name` and `path`, which the palette
/// slices to lift the matched characters out of the rest.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct GameSearchHit {
    /// Chunk path hash as 16 lowercase hex digits.
    pub path_hash: String,
    /// The path's basename, or the hash when no hash table names the chunk.
    pub name: String,
    /// The directory holding it, empty at the root and for an unnamed chunk.
    pub path: String,
    /// The archive the chunk was read from, relative to its source's root.
    pub wad: String,
    /// 0 is a name the query opens, 1 a name holding it, 2 a match reaching the directory.
    pub band: u8,
    pub score: f64,
    pub name_ranges: Vec<Range>,
    pub path_ranges: Vec<Range>,
}

/// What one search of the folded index found, best first, capped at [`SEARCH_LIMIT`].
pub type GameSearchResult = SearchHits<GameSearchHit>;

/// How many rows a search returns. Nothing sorts a million of them.
pub const SEARCH_LIMIT: usize = 100;

/// The files a path field wants ranked first in a search.
///
/// Files with an expected extension rank first, then files from the field's archive,
/// then the bands decide. A preference changes the order of the matches and adds no match.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SearchPreference {
    /// The extensions the field expects, without the dot. Empty means no preferred kind.
    pub extensions: Vec<String>,
    /// The file name of the field's archive, such as `Ahri.wad.client`.
    pub archive: Option<String>,
}

/// One file the full search matched, shaped as an entry a file tree can hold.
///
/// The pattern is matched over the full path, and the marked runs arrive
/// split at the basename: `name_ranges` are byte offsets into `name`, and
/// `path_ranges` are byte offsets into the directory prefix of `path`.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct GameFindHit {
    /// Chunk path hash as 16 lowercase hex digits.
    pub path_hash: String,
    /// Resolved path with forward slashes, or `None` when no hash table names the chunk.
    pub path: Option<String>,
    /// The path's basename, or the hash when no hash table names the chunk.
    pub name: String,
    /// The archive the chunk was read from, relative to its source's root.
    pub wad: String,
    /// Uncompressed chunk size.
    pub size_bytes: u64,
    pub name_ranges: Vec<Range>,
    pub path_ranges: Vec<Range>,
}

/// What one full search of the folded index found, in tree order, capped at [`FIND_LIMIT`].
pub type GameFindResult = SearchHits<GameFindHit>;

/// How many rows a full search returns.
///
/// The figure VS Code's own search stops at. A broader answer is not one a
/// reader scrolls, and past it the fix is a narrower pattern.
pub const FIND_LIMIT: usize = 20_000;

/// The newest search asked for, so a scan can see it has been overtaken.
///
/// Without this, a ten-character query runs ten full scans of the install and
/// only the last of them is one anybody wants.
pub type SearchGeneration = Generation<line::Palette>;

/// The newest full search asked for, on its own line apart from the palette's.
///
/// Separate from [`SearchGeneration`] so a keystroke in one box never gives up
/// a scan the other box is waiting on.
pub type FindGeneration = Generation<line::Find>;

/// The ticket counter for path field searches.
///
/// Separate from [`SearchGeneration`], so a path field search cancels only older path field
/// searches and never a palette search.
pub type PathSearchGeneration = Generation<line::PathField>;

/// Every archive of an install merged into one deduplicated directory tree.
///
/// A chunk several archives carry is one file here. The game ships the same
/// copy of a shared chunk in every archive that needs it, so the file keeps
/// the first archive that contains it.
///
/// Built once and read many times from a [`ChunkIndex`]. A build resolves the
/// path of every chunk of the install, while [`read_dir`](Self::read_dir) is a
/// lookup.
///
/// Every name sits in one string and every directory's files and children in
/// one array each, because an install holds close to a million chunks and an
/// allocation per name doubles what the index costs.
#[derive(Debug)]
pub struct GameIndex {
    /// Directory arena. Index 0 is the root.
    dirs: Vec<Dir>,
    /// Every named file, with the files of one directory together in natural order.
    files: Vec<File>,
    /// Every subdirectory, with the children of one directory together in byte order.
    children: Vec<Child>,
    /// Chunks no hash table names, which have no directory to sit in, in hash order.
    unknown: Vec<File>,
    /// The name of every file and directory, end to end.
    names: String,
    /// Archive names, in merge order. A file's `wad` indexes this.
    wads: Vec<String>,
}

/// A run of [`GameIndex::files`] or [`GameIndex::children`].
#[derive(Debug, Default, Clone, Copy)]
struct Span {
    start: u32,
    len: u32,
}

#[derive(Debug, Default)]
struct Dir {
    children: Span,
    files: Span,
    /// Files at or below this directory.
    file_count: u32,
    /// Every letter below this directory: its files' names and its descendants'.
    ///
    /// A query whose letters this does not cover cannot match anything under
    /// here, so one `AND` skips the whole subtree. The letters of the path
    /// *down to* this directory are not in it - the walk carries those, which
    /// is what keeps this to one word per directory.
    subtree_mask: u32,
}

/// One subdirectory: its name in [`GameIndex::names`] and its index in [`GameIndex::dirs`].
#[derive(Debug)]
struct Child {
    name_at: u32,
    name_len: u32,
    dir: u32,
}

#[derive(Debug)]
struct File {
    path_hash: u64,
    /// The offset of the name in [`GameIndex::names`].
    name_at: u32,
    size_bytes: u32,
    /// The letters of the name.
    mask: u32,
    name_len: u16,
    /// Index into [`GameIndex::wads`].
    wad: u16,
}

/// A [`GameIndex`] while the build inserts into it.
#[derive(Debug)]
struct Builder {
    dirs: Vec<BuildDir>,
    unknown: Vec<File>,
    names: String,
    file_count: usize,
}

#[derive(Debug, Default)]
struct BuildDir {
    children: BTreeMap<String, usize>,
    files: Vec<File>,
}

impl GameIndex {
    /// Merge every archive of one source into one tree.
    ///
    /// Builds the chunk index of `archives` without a cache, then calls
    /// [`over`](Self::over).
    ///
    /// # Errors
    ///
    /// Fails if [`GameArchives::chunk_index`] fails.
    pub fn build(archives: &GameArchives, resolver: &LayeredHashDb) -> AppResult<Self> {
        Ok(Self::over(&archives.chunk_index(None)?, resolver))
    }

    /// Merges every chunk of `chunks` into one tree, with the paths that `resolver` resolves.
    ///
    /// A chunk that several archives contain is one file, read from the first of them.
    pub fn over(chunks: &ChunkIndex, resolver: &LayeredHashDb) -> Self {
        let mut builder = Builder {
            dirs: vec![BuildDir::default()],
            unknown: Vec::new(),
            names: String::new(),
            file_count: 0,
        };

        let (hashes, rows): (Vec<u64>, Vec<&ChunkRow>) =
            chunks.chunks().map(|(hash, row)| (hash.0, row)).unzip();
        let mut unlisted = 0usize;
        resolver.for_each_batch(&hashes, |at, path_hash, path| {
            let row = rows[at];
            match u16::try_from(row.first_holder().index()) {
                Ok(wad) => builder.insert(path_hash, path, row.size(), wad),
                Err(_) => unlisted += 1,
            }
        });

        if unlisted > 0 {
            tracing::warn!(
                "Skipped {unlisted} game chunks: the install has more than {} archives",
                u16::MAX
            );
        }

        let wads = chunks
            .archives()
            .iter()
            .map(|archive| archive.name.clone())
            .collect();
        builder.finish(wads)
    }

    /// List one directory, or `None` when nothing in the index has that path.
    ///
    /// The root is `""` and the group of unnamed chunks is [`UNKNOWN_DIR`].
    pub fn read_dir(&self, path: &str) -> Option<GameDirListing> {
        if path == UNKNOWN_DIR {
            return Some(GameDirListing {
                dirs: Vec::new(),
                files: self.unnamed_entries(),
            });
        }

        let index = self.resolve(path)?;
        let dir = &self.dirs[index];

        let mut dirs: Vec<GameDirEntry> = self
            .children_of(dir)
            .iter()
            .map(|child| self.fold(path, self.child_name(child), child.dir as usize))
            .collect();
        /* The map keyed them by byte order, and a folded row is named for the
        whole run rather than the key. */
        dirs.sort_by(|a, b| compare_names(&a.name, &b.name));

        /* Last, and only at the root, so the junk drawer never pushes named
        paths down the tree. */
        if path.is_empty() && !self.unknown.is_empty() {
            dirs.push(GameDirEntry {
                path: UNKNOWN_DIR.to_owned(),
                name: "unknown".to_owned(),
                file_count: self.unknown.len() as u32,
            });
        }

        Some(GameDirListing {
            dirs,
            files: self
                .files_of(dir)
                .iter()
                .map(|file| self.entry(file, path))
                .collect(),
        })
    }

    /// The best rows of the whole install for one query, best first.
    ///
    /// `is_overtaken` is tested every few thousand files. A scan that has been
    /// overtaken returns what it has rather than finishing a walk nobody is
    /// waiting for, and says so through [`SearchHits::superseded`].
    ///
    /// An empty query matches nothing here. The install is not a list anybody
    /// wants handed to them unasked, and the palette only reaches this source
    /// once something is typed.
    pub fn search(&self, query: &str, is_overtaken: impl Fn() -> bool) -> GameSearchResult {
        self.search_preferring(query, &SearchPreference::default(), is_overtaken)
    }

    /// [`search`](Self::search), with the files `preference` names ranked first.
    pub fn search_preferring(
        &self,
        query: &str,
        preference: &SearchPreference,
        is_overtaken: impl Fn() -> bool,
    ) -> GameSearchResult {
        let unnamed = self.dirs[0].file_count == 0 && !self.unknown.is_empty();

        let Some(query) = Query::parse(query) else {
            return SearchHits::empty(unnamed);
        };

        let mut scan = Scan {
            index: self,
            mask: query.mask(),
            query,
            preferred: Preferred::new(self, preference),
            heap: BinaryHeap::with_capacity(SEARCH_LIMIT + 1),
            total: 0,
            path: String::with_capacity(128),
            full: String::with_capacity(160),
            since_check: 0,
            overtaken: false,
        };

        scan.walk(0, 0, &is_overtaken);
        scan.unknown(&is_overtaken);

        GameSearchResult {
            total: scan.total,
            superseded: scan.overtaken,
            unnamed,
            hits: scan
                .heap
                .into_sorted_vec()
                .into_iter()
                .map(|hit| hit.row)
                .collect(),
        }
    }

    /// Every file of the install the pattern matches, in tree order.
    ///
    /// Where [`search`](Self::search) ranks a bounded best-of for the palette,
    /// this returns the whole answer: each match in depth-first tree order,
    /// capped at [`FIND_LIMIT`] rows while the total counts on past it. The
    /// chunks no hash table names come last, matched by their hash.
    ///
    /// `is_overtaken` is tested every few thousand files, the contract
    /// [`search`](Self::search) sets.
    pub fn find(&self, query: &FindQuery, is_overtaken: impl Fn() -> bool) -> GameFindResult {
        self.find_capped(query, FIND_LIMIT, is_overtaken)
    }

    /// [`find`](Self::find) with the cap a test can afford to fill.
    fn find_capped(
        &self,
        query: &FindQuery,
        limit: usize,
        is_overtaken: impl Fn() -> bool,
    ) -> GameFindResult {
        let mut scan = FindScan {
            index: self,
            query,
            hits: Vec::new(),
            limit,
            total: 0,
            path: String::with_capacity(160),
            since_check: 0,
            overtaken: false,
        };

        scan.walk(0, &is_overtaken);
        scan.unknown(&is_overtaken);

        GameFindResult {
            hits: scan.hits,
            total: scan.total,
            superseded: scan.overtaken,
            unnamed: self.dirs[0].file_count == 0 && !self.unknown.is_empty(),
        }
    }

    /// What the index holds, for a caller that reports its size.
    pub fn stats(&self) -> GameIndexStats {
        GameIndexStats {
            archives: self.wads.len() as u32,
            files: self.dirs[0].file_count + self.unknown.len() as u32,
            dirs: (self.dirs.len() - 1) as u32,
        }
    }

    /// The archive a file was read from.
    ///
    /// # Panics
    ///
    /// Panics when the file's ordinal is not one this index handed out, which
    /// is a bug in the build rather than a condition a caller can hit.
    fn wad_name(&self, file: &File) -> &str {
        &self.wads[file.wad as usize]
    }

    /// The name of `file`: the last segment of its path, or its hash for an unnamed chunk.
    fn name_of(&self, file: &File) -> &str {
        let at = file.name_at as usize;
        &self.names[at..at + file.name_len as usize]
    }

    fn child_name(&self, child: &Child) -> &str {
        let at = child.name_at as usize;
        &self.names[at..at + child.name_len as usize]
    }

    fn files_of(&self, dir: &Dir) -> &[File] {
        let start = dir.files.start as usize;
        &self.files[start..start + dir.files.len as usize]
    }

    fn children_of(&self, dir: &Dir) -> &[Child] {
        let start = dir.children.start as usize;
        &self.children[start..start + dir.children.len as usize]
    }

    /// Returns the subdirectory `name` of the directory at `dir`, or `None` if it has none.
    fn child(&self, dir: usize, name: &str) -> Option<usize> {
        let children = self.children_of(&self.dirs[dir]);
        let at = children
            .binary_search_by(|child| self.child_name(child).cmp(name))
            .ok()?;
        Some(children[at].dir as usize)
    }

    /// The wire shape of `file`, with the path that its directory `dir` gives it.
    fn entry(&self, file: &File, dir: &str) -> GameFileEntry {
        let name = self.name_of(file);
        let path = if dir.is_empty() {
            name.to_owned()
        } else {
            format!("{dir}/{name}")
        };
        GameFileEntry {
            path_hash: hex_name(WadHash(file.path_hash)),
            path: Some(path),
            size_bytes: u64::from(file.size_bytes),
            wad: self.wad_name(file).to_owned(),
        }
    }

    /// The wire shape of a chunk no hash table names, which has no path.
    fn unnamed_entry(&self, file: &File) -> GameFileEntry {
        GameFileEntry {
            path_hash: hex_name(WadHash(file.path_hash)),
            path: None,
            size_bytes: u64::from(file.size_bytes),
            wad: self.wad_name(file).to_owned(),
        }
    }

    /// Every file at or below one directory, in tree order.
    ///
    /// `path` is what a listing gave, so a folded chain addresses the
    /// directory that holds the files rather than the first link of the
    /// chain. [`UNKNOWN_DIR`] gives the group of chunks no hash table names,
    /// and the root gives its named files plus that group, which is what the
    /// tree shows below the root row.
    ///
    /// Returns `None` when nothing in the index has that path.
    pub fn files_under(&self, path: &str) -> Option<Vec<GameFileEntry>> {
        if path == UNKNOWN_DIR {
            return Some(self.unnamed_entries());
        }

        let root = self.resolve(path)?;
        let mut out = Vec::new();
        self.collect_files(root, path, &mut out);
        if path.is_empty() {
            out.extend(self.unnamed_entries());
        }
        Some(out)
    }

    /// The file at one path of the tree, or `None` where the index names nothing there.
    ///
    /// `path` is spelled as the hash tables spell it, which is how a bin's `file` link
    /// resolves.
    #[must_use]
    pub fn file_at(&self, path: &str) -> Option<GameFileEntry> {
        let (dir, name) = path.rsplit_once('/').unwrap_or(("", path));
        let index = self.resolve(dir)?;
        let file = self
            .files_of(&self.dirs[index])
            .iter()
            .find(|file| self.name_of(file) == name)?;
        Some(self.entry(file, dir))
    }

    /// The chunk no hash table names at `path_hash`, or `None` where the index holds
    /// none under it.
    ///
    /// A bin names a file by the path's hash whether or not a table names it, so this
    /// is where a path the tree lacks is looked for next.
    #[must_use]
    pub fn unnamed_at(&self, path_hash: u64) -> Option<GameFileEntry> {
        let at = self
            .unknown
            .binary_search_by_key(&path_hash, |file| file.path_hash)
            .ok()?;
        Some(self.unnamed_entry(&self.unknown[at]))
    }

    /// The wire shape of every chunk no hash table names.
    fn unnamed_entries(&self) -> Vec<GameFileEntry> {
        self.unknown
            .iter()
            .map(|file| self.unnamed_entry(file))
            .collect()
    }

    /// Append this directory's files and every descendant's, depth first.
    fn collect_files(&self, index: usize, path: &str, out: &mut Vec<GameFileEntry>) {
        let dir = &self.dirs[index];
        for file in self.files_of(dir) {
            out.push(self.entry(file, path));
        }
        for child in self.children_of(dir) {
            let name = self.child_name(child);
            let child_path = if path.is_empty() {
                name.to_owned()
            } else {
                format!("{path}/{name}")
            };
            self.collect_files(child.dir as usize, &child_path, out);
        }
    }
    /// Visit every named file as `(path hash, path, archive ordinal)`, in tree order.
    ///
    /// For a build fed by this index, which reads the files it names and wants
    /// no owned row per file. The chunks no hash table names are not visited.
    /// The ordinal indexes [`wads`](Self::wads).
    pub fn for_each_named_file(&self, mut visit: impl FnMut(u64, &str, u32)) {
        let mut path = String::with_capacity(160);
        self.visit_files(0, &mut path, &mut visit);
    }

    /// Visit every chunk no hash table names as `(path hash, archive ordinal)`.
    ///
    /// The other half of [`for_each_named_file`](Self::for_each_named_file),
    /// in the order the unnamed group is kept: by hex name. The ordinal
    /// indexes [`wads`](Self::wads).
    pub fn for_each_unnamed_file(&self, mut visit: impl FnMut(u64, u32)) {
        for file in &self.unknown {
            visit(file.path_hash, u32::from(file.wad));
        }
    }

    /// Archive names in merge order, which a file's ordinal indexes.
    pub fn wads(&self) -> &[String] {
        &self.wads
    }

    /// Visit this directory's files and every descendant's, depth first.
    ///
    /// `path` is the directory the walk stands in, pushed and popped in place so
    /// a file's full path is built without allocating one per file.
    fn visit_files(&self, dir: usize, path: &mut String, visit: &mut impl FnMut(u64, &str, u32)) {
        let node = &self.dirs[dir];
        for file in self.files_of(node) {
            let restore = path.len();
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(self.name_of(file));
            visit(file.path_hash, path, u32::from(file.wad));
            path.truncate(restore);
        }
        for child in self.children_of(node) {
            let restore = path.len();
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(self.child_name(child));
            self.visit_files(child.dir as usize, path, visit);
            path.truncate(restore);
        }
    }

    /// The arena index of `path`, where `""` is the root.
    fn resolve(&self, path: &str) -> Option<usize> {
        let mut cursor = 0usize;
        for segment in path.split('/').filter(|s| !s.is_empty()) {
            cursor = self.child(cursor, segment)?;
        }
        Some(cursor)
    }

    /// One child row, walked down its chain of single-child directories.
    ///
    /// A run of directories that each hold nothing but the next one is one row,
    /// so a modder scanning a tree of asset paths spends no rows on chains that
    /// carry no choice.
    fn fold(&self, parent: &str, name: &str, child: usize) -> GameDirEntry {
        let mut path = if parent.is_empty() {
            name.to_owned()
        } else {
            format!("{parent}/{name}")
        };
        let mut display = name.to_owned();
        let mut cursor = child;

        loop {
            let dir = &self.dirs[cursor];
            if dir.files.len != 0 {
                break;
            }
            let [next] = self.children_of(dir) else {
                break;
            };
            let next_name = self.child_name(next);
            path.push('/');
            path.push_str(next_name);
            display.push('/');
            display.push_str(next_name);
            cursor = next.dir as usize;
        }

        GameDirEntry {
            path,
            name: display,
            file_count: self.dirs[cursor].file_count,
        }
    }
}

/// One walk of the index for one query.
///
/// Depth first, pushing and popping a segment on a single `String`, so a path
/// that survives the mask is built without allocating one per file.
struct Scan<'a> {
    index: &'a GameIndex,
    /// Split and lowercased once for every candidate that follows.
    query: Query,
    preferred: Preferred<'a>,
    mask: u32,
    /// The best rows so far, worst at the root so the cap knows what to drop.
    heap: BinaryHeap<Hit>,
    total: u32,
    /// The directory the walk stands in, without a trailing separator.
    path: String,
    /// `path/name`, rebuilt in place for a candidate whose name did not match.
    full: String,
    since_check: u32,
    overtaken: bool,
}

impl Scan<'_> {
    /// Walk one directory, its files first and then its children.
    ///
    /// `path_mask` is every letter of the path down to here. Held on the stack
    /// rather than in each directory, which is what keeps the index's own cost
    /// to one word per directory.
    fn walk(&mut self, dir: usize, path_mask: u32, is_overtaken: &impl Fn() -> bool) {
        if self.overtaken {
            return;
        }

        /* The index outlives the scan, so a directory borrowed from it does not
        borrow the scan and the walk can still take itself mutably. */
        let index = self.index;
        let node = &index.dirs[dir];
        if !mask_covers(path_mask | node.subtree_mask, self.mask) {
            return;
        }

        for file in index.files_of(node) {
            if self.tick(is_overtaken) {
                return;
            }
            if !mask_covers(path_mask | file.mask, self.mask) {
                continue;
            }
            self.consider(file);
        }

        for child in index.children_of(node) {
            let name = index.child_name(child);
            let restore = self.path.len();
            if !self.path.is_empty() {
                self.path.push('/');
            }
            self.path.push_str(name);

            self.walk(
                child.dir as usize,
                path_mask | letter_mask(name),
                is_overtaken,
            );

            self.path.truncate(restore);
            if self.overtaken {
                return;
            }
        }
    }

    /// The chunks no hash table names, which sit under no directory.
    ///
    /// Searchable by their hash, because reading one out of a log and asking
    /// the manager where it lives is the reason anybody types sixteen hex
    /// digits.
    fn unknown(&mut self, is_overtaken: &impl Fn() -> bool) {
        let index = self.index;
        self.path.clear();

        for file in &index.unknown {
            if self.tick(is_overtaken) {
                return;
            }
            if mask_covers(file.mask, self.mask) {
                self.consider(file);
            }
        }
    }

    /// Test the generation every so often, and report whether to stop.
    fn tick(&mut self, is_overtaken: &impl Fn() -> bool) -> bool {
        self.since_check += 1;
        if self.since_check >= STALE_CHECK_INTERVAL {
            self.since_check = 0;
            self.overtaken = is_overtaken();
        }
        self.overtaken
    }

    /// Score one file, and keep it when it beats the worst row held.
    ///
    /// The bands are the ranking rule the palette's own matcher obeys: a name
    /// the query opens, then a name that holds it, then a match reaching into
    /// the directory. A name match marks no part of the path, because it never
    /// read one.
    fn consider(&mut self, file: &File) {
        let name = self.index.name_of(file);
        let hit = match self.query.matches(name) {
            Some(matched) => {
                let band = u8::from(!self.query.starts(name));
                self.hit(file, band, matched.score, matched.ranges, Vec::new())
            }
            /* A name holding only some of the terms still gets its path read,
            because the rest of them may be in a directory above it. */
            None if self.path.is_empty() => return,
            None => {
                self.full.clear();
                self.full.push_str(&self.path);
                self.full.push('/');
                self.full.push_str(name);

                let Some(matched) = self.query.matches(&self.full) else {
                    return;
                };
                let boundary = self.path.len() as u32;
                let (path_ranges, name_ranges) = split_ranges(&matched.ranges, boundary);
                self.hit(file, 2, matched.score, name_ranges, path_ranges)
            }
        };

        self.total += 1;
        self.push(hit);
    }

    fn hit(
        &self,
        file: &File,
        band: u8,
        score: f64,
        name_ranges: Vec<Range>,
        path_ranges: Vec<Range>,
    ) -> Hit {
        let name = self.index.name_of(file);
        Hit {
            tier: self.preferred.tier(file, name),
            band,
            score,
            length: (self.path.len() + name.len()) as u32,
            row: GameSearchHit {
                path_hash: hex_name(WadHash(file.path_hash)),
                name: name.to_owned(),
                path: self.path.clone(),
                wad: self.index.wad_name(file).to_owned(),
                band,
                score,
                name_ranges,
                path_ranges,
            },
        }
    }

    /// Keep the row while the cap has room, and past that only when it beats
    /// the worst one held.
    fn push(&mut self, hit: Hit) {
        if self.heap.len() < SEARCH_LIMIT {
            self.heap.push(hit);
            return;
        }
        // `Hit` orders worst first, so the root is the row to drop.
        if self.heap.peek().is_some_and(|worst| hit < *worst) {
            self.heap.pop();
            self.heap.push(hit);
        }
    }
}

/// One walk of the index for one full-search pattern.
///
/// Depth first like [`Scan`], but yes-or-no over the full path rather than
/// ranked, so the rows land in tree order and nothing sorts them. The walk
/// pushes and pops each segment on a single `String`, and a candidate's name
/// rides the same buffer, so a path is never allocated to be rejected.
struct FindScan<'a> {
    index: &'a GameIndex,
    query: &'a FindQuery,
    /// Matching rows in the order the walk met them, full at `limit`.
    hits: Vec<GameFindHit>,
    limit: usize,
    total: u32,
    /// The path the walk stands in, without a trailing separator.
    path: String,
    since_check: u32,
    overtaken: bool,
}

impl FindScan<'_> {
    /// Walk one directory, its files first and then its children.
    fn walk(&mut self, dir: usize, is_overtaken: &impl Fn() -> bool) {
        if self.overtaken {
            return;
        }

        let index = self.index;
        let node = &index.dirs[dir];

        for file in index.files_of(node) {
            if self.tick(is_overtaken) {
                return;
            }
            self.consider(file, true);
        }

        for child in index.children_of(node) {
            let restore = self.path.len();
            if !self.path.is_empty() {
                self.path.push('/');
            }
            self.path.push_str(index.child_name(child));

            self.walk(child.dir as usize, is_overtaken);

            self.path.truncate(restore);
            if self.overtaken {
                return;
            }
        }
    }

    /// The chunks no hash table names, matched by their hash.
    fn unknown(&mut self, is_overtaken: &impl Fn() -> bool) {
        let index = self.index;
        self.path.clear();

        for file in &index.unknown {
            if self.tick(is_overtaken) {
                return;
            }
            self.consider(file, false);
        }
    }

    /// Match one file over its full path, and keep it while the cap has room.
    ///
    /// `named` says a hash table resolved the file, so an unnamed chunk's hash
    /// never masquerades as a path.
    fn consider(&mut self, file: &File, named: bool) {
        let name = self.index.name_of(file);
        let restore = self.path.len();
        if !self.path.is_empty() {
            self.path.push('/');
        }
        self.path.push_str(name);

        if let Some(ranges) = self.query.matches(&self.path) {
            self.total += 1;
            if self.hits.len() < self.limit {
                /* At the root the whole match is the name, and `split_ranges`
                cannot say so - it assumes a separator sits at the boundary. */
                let (path_ranges, name_ranges) = if restore == 0 {
                    (Vec::new(), ranges)
                } else {
                    split_ranges(&ranges, restore as u32)
                };
                self.hits.push(GameFindHit {
                    path_hash: hex_name(WadHash(file.path_hash)),
                    path: named.then(|| self.path.clone()),
                    name: name.to_owned(),
                    wad: self.index.wad_name(file).to_owned(),
                    size_bytes: u64::from(file.size_bytes),
                    name_ranges,
                    path_ranges,
                });
            }
        }

        self.path.truncate(restore);
    }

    /// Test the generation every so often, and report whether to stop.
    fn tick(&mut self, is_overtaken: &impl Fn() -> bool) -> bool {
        self.since_check += 1;
        if self.since_check >= STALE_CHECK_INTERVAL {
            self.since_check = 0;
            self.overtaken = is_overtaken();
        }
        self.overtaken
    }
}

/// A [`SearchPreference`] prepared for one index, checked against each candidate.
struct Preferred<'a> {
    extensions: &'a [String],
    /// Whether each archive, by ordinal, is the one preferred.
    archives: Vec<bool>,
}

impl<'a> Preferred<'a> {
    fn new(index: &GameIndex, preference: &'a SearchPreference) -> Self {
        let archives = index
            .wads
            .iter()
            .map(|wad| {
                preference.archive.as_deref().is_none_or(|archive| {
                    let name = wad.rsplit_once('/').map_or(wad.as_str(), |(_, name)| name);
                    name.eq_ignore_ascii_case(archive)
                })
            })
            .collect();

        Self {
            extensions: &preference.extensions,
            archives,
        }
    }

    /// The file's rank group, where 0 ranks first.
    ///
    /// A wrong kind adds 2 and a wrong archive adds 1, so a texture from another archive
    /// still ranks above a mesh from the field's archive.
    fn tier(&self, file: &File, name: &str) -> u8 {
        let kind = self.extensions.is_empty()
            || name.rsplit_once('.').is_some_and(|(_, extension)| {
                self.extensions
                    .iter()
                    .any(|expected| expected.eq_ignore_ascii_case(extension))
            });
        let archive = self
            .archives
            .get(file.wad as usize)
            .copied()
            .unwrap_or(true);

        u8::from(!kind) * 2 + u8::from(!archive)
    }
}

/// One kept row, ordered worst first so a bounded heap drops the right one.
#[derive(Debug)]
struct Hit {
    /// The rank group from `Preferred::tier`, where 0 ranks first.
    tier: u8,
    band: u8,
    score: f64,
    /// The length of `path/name`, so the shorter path wins a tie.
    length: u32,
    row: GameSearchHit,
}

impl Ord for Hit {
    /// Greater is worse: a higher tier, a higher band, a lower score, then a longer path.
    fn cmp(&self, other: &Self) -> Ordering {
        self.tier
            .cmp(&other.tier)
            .then_with(|| self.band.cmp(&other.band))
            .then_with(|| other.score.total_cmp(&self.score))
            .then_with(|| self.length.cmp(&other.length))
            .then_with(|| self.row.path.cmp(&other.row.path))
            .then_with(|| self.row.name.cmp(&other.row.name))
    }
}

impl PartialOrd for Hit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Hit {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Hit {}

/// Cut one match over `path/name` into the runs each of the two lines marks.
fn split_ranges(ranges: &[Range], boundary: u32) -> (Vec<Range>, Vec<Range>) {
    let mut path = Vec::new();
    let mut name = Vec::new();

    for &(start, end) in ranges {
        if end <= boundary {
            path.push((start, end));
        } else if start > boundary {
            name.push((start - boundary - 1, end - boundary - 1));
        } else {
            if start < boundary {
                path.push((start, boundary));
            }
            if end > boundary + 1 {
                name.push((0, end - boundary - 1));
            }
        }
    }
    (path, name)
}

/// Lazily-built, app-managed [`GameIndex`], one for each [`WadSource`].
#[derive(Debug, Default)]
pub struct GameIndexState {
    game: LazySlot<GameIndex>,
    lcu: LazySlot<GameIndex>,
    /// The file that caches the game's [`ChunkIndex`], or `None` for no cache.
    chunk_cache: Option<PathBuf>,
}

impl GameIndexState {
    /// Returns a state that caches the game's chunk index in the file `chunk_cache`.
    #[must_use]
    pub fn cached_at(chunk_cache: PathBuf) -> Self {
        Self {
            chunk_cache: Some(chunk_cache),
            ..Self::default()
        }
    }

    /// Return the index over `archives`, building it on first use.
    ///
    /// `resolver` is read only when a build happens. The lock is held across
    /// the build, so concurrent callers wait rather than each walking the whole
    /// install.
    ///
    /// # Errors
    ///
    /// Fails when the build fails.
    pub fn get_or_build(
        &self,
        archives: &GameArchives,
        resolver: &LayeredHashDb,
    ) -> AppResult<Arc<GameIndex>> {
        self.slot(archives.source())
            .get_or_try_init(|| Ok(GameIndex::over(&self.chunk_index(archives)?, resolver)))
    }

    /// Returns the chunk index of `archives`, from the cache file of the state if it matches.
    ///
    /// # Errors
    ///
    /// Fails if [`GameArchives::chunk_index`] fails.
    pub fn chunk_index(&self, archives: &GameArchives) -> AppResult<ChunkIndex> {
        archives.chunk_index(self.chunk_cache.as_deref())
    }

    /// Drop the built index of one source, so its next read reads the install again.
    pub fn clear(&self, source: WadSource) {
        self.slot(source).clear();
    }

    /// Drop every built index, for a change such as new hash tables that both read.
    pub fn clear_all(&self) {
        self.clear(WadSource::Game);
        self.clear(WadSource::Lcu);
    }

    fn slot(&self, source: WadSource) -> &LazySlot<GameIndex> {
        match source {
            WadSource::Game => &self.game,
            WadSource::Lcu => &self.lcu,
        }
    }
}

impl Builder {
    /// Adds one chunk under its resolved path, or to the unnamed group.
    ///
    /// `wad` is the ordinal of the archive the chunk was read from, which
    /// indexes [`GameIndex::wads`].
    fn insert(&mut self, path_hash: u64, path: Option<&str>, size_bytes: u64, wad: u16) {
        // A table of contents records a chunk's size in 32 bits.
        let size_bytes = u32::try_from(size_bytes).unwrap_or(u32::MAX);

        let Some(path) = path else {
            let name = hex_name(WadHash(path_hash));
            if let Some(file) = self.file(&name, path_hash, size_bytes, wad) {
                self.unknown.push(file);
            }
            return;
        };

        let mut segments = path.split('/').filter(|s| !s.is_empty()).peekable();
        let mut cursor = 0usize;
        let mut name = None;

        while let Some(segment) = segments.next() {
            if segments.peek().is_none() {
                name = Some(segment);
                break;
            }
            cursor = match self.dirs[cursor].children.get(segment) {
                Some(&child) => child,
                None => {
                    let child = self.dirs.len();
                    self.dirs.push(BuildDir::default());
                    self.dirs[cursor].children.insert(segment.to_owned(), child);
                    child
                }
            };
        }

        // A path of nothing but separators names no file.
        let Some(name) = name else { return };
        if let Some(file) = self.file(name, path_hash, size_bytes, wad) {
            self.dirs[cursor].files.push(file);
            self.file_count += 1;
        }
    }

    /// Appends `name` to the name arena and returns a file that reads it.
    ///
    /// Returns `None` if the name is longer than a file records.
    fn file(&mut self, name: &str, path_hash: u64, size_bytes: u32, wad: u16) -> Option<File> {
        let name_len = u16::try_from(name.len()).ok()?;
        let name_at = u32::try_from(self.names.len()).ok()?;
        self.names.push_str(name);

        Some(File {
            path_hash,
            name_at,
            size_bytes,
            mask: letter_mask(name),
            name_len,
            wad,
        })
    }

    /// Lays the tree out as a [`GameIndex`] over the archives `wads`.
    fn finish(mut self, wads: Vec<String>) -> GameIndex {
        let mut index = GameIndex {
            dirs: (0..self.dirs.len()).map(|_| Dir::default()).collect(),
            files: Vec::with_capacity(self.file_count),
            children: Vec::with_capacity(self.dirs.len() - 1),
            unknown: std::mem::take(&mut self.unknown),
            names: std::mem::take(&mut self.names),
            wads,
        };

        self.finalize(0, &mut index);

        // Every unnamed chunk's name is 16 hex digits, so hash order is name order.
        index.unknown.sort_unstable_by_key(|file| file.path_hash);
        index.unknown.shrink_to_fit();
        index.names.shrink_to_fit();
        index
    }

    /// Moves the files and children of the directory at `at`, and of every directory below
    /// it, into `index`.
    ///
    /// Returns the recursive file count and the union of every letter below
    /// the directory, which is what lets a search skip a subtree whole.
    fn finalize(&mut self, at: usize, index: &mut GameIndex) -> (u32, u32) {
        let mut files = std::mem::take(&mut self.dirs[at].files);
        files.sort_by(|a, b| compare_names(index.name_of(a), index.name_of(b)));

        let mut total = files.len() as u32;
        let mut mask = files.iter().fold(0, |mask, file| mask | file.mask);
        let file_span = Span {
            start: index.files.len() as u32,
            len: files.len() as u32,
        };
        index.files.extend(files);

        let children = std::mem::take(&mut self.dirs[at].children);
        let child_span = Span {
            start: index.children.len() as u32,
            len: children.len() as u32,
        };
        for (name, &dir) in &children {
            index.children.push(Child {
                name_at: index.names.len() as u32,
                name_len: name.len() as u32,
                dir: dir as u32,
            });
            index.names.push_str(name);
        }

        for (name, dir) in children {
            let (count, subtree) = self.finalize(dir, index);
            total += count;
            mask |= letter_mask(&name) | subtree;
        }

        index.dirs[at] = Dir {
            children: child_span,
            files: file_span,
            file_count: total,
            subtree_mask: mask,
        };
        (total, mask)
    }
}

#[cfg(test)]
mod tests;
