//! Mapping a repaired project file to its target in the archive it was unpacked
//! from, over the three forms of chunk name a lossless unpack writes: a path, a
//! bare hash and a path with a `.ltk` suffix.

use super::*;
use ltk_hash::Hash as _;

fn chunk(layer: &str, path: &str) -> (String, WadHash) {
    match DeltaTarget::of(layer, path) {
        Some(DeltaTarget::Chunk { wad, hash }) => (wad, hash),
        other => panic!("expected a chunk for {layer}/{path}, got {other:?}"),
    }
}

fn entry(layer: &str, path: &str) -> String {
    match DeltaTarget::of(layer, path) {
        Some(DeltaTarget::Entry { path }) => path,
        other => panic!("expected an entry for {layer}/{path}, got {other:?}"),
    }
}

#[test]
fn a_file_of_a_wad_directory_is_a_chunk_keyed_by_its_path() {
    assert_eq!(
        chunk("base", "Aatrox.wad.client/data/skin0.bin"),
        (
            "Aatrox.wad.client".to_owned(),
            WadHash::hash_str("data/skin0.bin")
        )
    );
}

/// An unnamed chunk is written as sixteen hex digits with no extension, which
/// parses as the hash itself rather than as a path to hash.
#[test]
fn a_nameless_chunk_keeps_the_hash_it_was_written_under() {
    assert_eq!(
        chunk("base", "Aatrox.wad.client/0123456789abcdef").1,
        WadHash(0x0123456789abcdef)
    );
}

/// A path two chunks share gets a `.ltk` suffix, which the mapping strips.
#[test]
fn a_collided_path_addresses_the_chunk_it_was_renamed_from() {
    assert_eq!(
        chunk("base", "Aatrox.wad.client/data/skin0.bin.ltk").1,
        chunk("base", "Aatrox.wad.client/data/skin0.bin").1
    );
}

#[test]
fn a_wad_directory_is_recognised_in_any_casing() {
    assert_eq!(
        chunk("base", "Aatrox.WAD.Client/data/skin0.bin"),
        (
            "Aatrox.WAD.Client".to_owned(),
            WadHash::hash_str("data/skin0.bin")
        )
    );
}

#[test]
fn a_raw_file_is_the_entry_it_was_unpacked_from() {
    assert_eq!(entry("base", "raw/config.ini"), "RAW/config.ini");
}

#[test]
fn a_loose_file_of_the_base_layer_is_a_wad_entry() {
    assert_eq!(entry("base", "notes.txt"), "WAD/notes.txt");
    assert_eq!(
        entry("base", "Aatrox.wad/data/skin0.bin"),
        "WAD/Aatrox.wad/data/skin0.bin"
    );
}

#[test]
fn a_file_of_another_layers_wad_directory_is_a_chunk_of_that_layer() {
    assert_eq!(
        chunk("high-res", "Aatrox.wad.client/data/skin0.bin"),
        (
            "Aatrox.wad.client".to_owned(),
            WadHash::hash_str("data/skin0.bin")
        )
    );
}

/// Only the base layer has a `RAW/` directory, so another layer's `raw`
/// directory is an ordinary directory under its `WAD_<layer>/`.
#[test]
fn a_loose_file_of_another_layer_is_an_entry_of_its_wad_directory() {
    assert_eq!(entry("high-res", "notes.txt"), "WAD_high-res/notes.txt");
    assert_eq!(
        entry("high-res", "raw/config.ini"),
        "WAD_high-res/raw/config.ini"
    );
}

#[test]
fn a_layer_no_wad_directory_can_hold_maps_nowhere() {
    assert_eq!(DeltaTarget::of("high res", "X.wad.client/f.bin"), None);
    assert_eq!(DeltaTarget::of("../x", "X.wad.client/f.bin"), None);
}

#[test]
fn a_declared_table_keeps_its_name_under_the_archives_hashes() {
    assert_eq!(
        archive_table_path("hashes/game.hashes.txt").as_deref(),
        Some("META/hashes/game.hashes.txt")
    );
}

/// A table declared anywhere other than directly under `hashes/` follows the
/// routing rules in `ltk_mod_project`, which only the repack applies.
#[test]
fn a_table_declared_elsewhere_maps_nowhere() {
    assert_eq!(archive_table_path("tables/game.hashes.txt"), None);
    assert_eq!(archive_table_path("hashes/nested/game.hashes.txt"), None);
    assert_eq!(archive_table_path("hashes/"), None);
}

/// A file the repair deleted becomes a removal in the delta, and no bytes are
/// read for it, since the staged tree no longer holds the file.
#[test]
fn a_removal_is_written_as_an_edit() {
    let report = ltk_manager_problems::FixReport {
        applied: 1,
        skipped: 0,
        names_kept: 0,
        tables: Vec::new(),
        remaining: Vec::new(),
        files: vec![ltk_manager_problems::FileOutcome {
            layer: "base".to_owned(),
            path: "Aatrox.wad.client/data/skin0.bin".to_owned(),
            applied: 1,
            skipped: 0,
            change: ltk_manager_problems::FileChange::Removed,
        }],
        failed: Vec::new(),
    };

    let tmp = tempfile::tempdir().unwrap();
    let archive = camino::Utf8Path::new("mod.fantome");
    let edit = RepairEdit::read(tmp.path(), archive, &report);

    assert!(edit.is_ok(), "{:?}", edit.err());
}

/// Edits from a held run, applied to the archive the run read.
mod held {
    use super::*;

    use ltk_manager_assets::test_util::{
        STALE_BIN_IN_WAD, make_packed_bin_fantome_zip, resolver_naming, stale_bin,
    };
    use ltk_manager_base::budget::Budget;
    use ltk_manager_base::config::Config;
    use ltk_manager_problems::{FixRun, Preserved, ProjectFiles};

    const CHUNK: &str = "Aatrox.wad.client/data/skin0.bin";
    const ICON: &str = "ASSETS/Characters/Smolder/HUD/Smolder_Circle.dds";
    const OTHER: &str = "ASSETS/Characters/Smolder/HUD/Smolder_Square.dds";
    const TABLE: &str = "META/hashes/game.hashes.txt";

    fn read_where_it_lies(archive: &Path, names: &[&str]) -> ProjectFiles {
        ProjectFiles::in_archive(
            archive,
            &Config::default(),
            Budget::repair(),
            &resolver_naming(names),
            None,
        )
        .expect("the archive")
    }

    fn tables_in(archive: &Path) -> Vec<(ltk_hashtable::HashtableEntry, ltk_hashtable::Hashtable)> {
        FantomeReader::new(fs::File::open(archive).unwrap())
            .unwrap()
            .read_hashtables()
            .unwrap()
    }

    /// A fantome declaring one game table holding `names`, and nothing else.
    fn declaring_fantome(archive: &Path, names: &[&str]) {
        let (algorithm, width) = Category::Game.default_shape().unwrap();
        let mut info = ltk_fantome::FantomeInfo::default();
        info.name = "declaring-mod".to_owned();
        info.author = "Author".to_owned();
        info.version = "1.0.0".to_owned();
        info.hashtables = vec![FantomeHashtable {
            path: TABLE.to_owned(),
            category: Category::Game,
            algorithm,
            bits: width.bits(),
        }];
        let mut table = Vec::new();
        ltk_hashtable::Hashtable::from_names(names.iter().copied())
            .unwrap()
            .write_to(&mut table)
            .unwrap();

        let mut zip = zip::ZipWriter::new(fs::File::create(archive).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("META/info.json", options).unwrap();
        std::io::Write::write_all(&mut zip, &serde_json::to_vec_pretty(&info).unwrap()).unwrap();
        zip.start_file(TABLE, options).unwrap();
        std::io::Write::write_all(&mut zip, &table).unwrap();
        zip.finish().unwrap();
    }

    /// Story: the repair writes one chunk and keeps one name, and the edit
    /// writes both into the archive with no unpacked tree.
    #[test]
    fn a_held_write_and_its_kept_name_land_in_the_archive() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = camino::Utf8PathBuf::from_path_buf(tmp.path().join("mod.fantome")).unwrap();
        make_packed_bin_fantome_zip(
            archive.as_std_path(),
            "packed-mod",
            &stale_bin(),
            zip::CompressionMethod::Stored,
        );
        let files = read_where_it_lies(archive.as_std_path(), &[STALE_BIN_IN_WAD]);
        let mut run = FixRun::held(files, Vec::new(), Vec::new(), None, Config::default(), None);
        run.write("base", CHUNK, b"repaired", 1, 0).unwrap();
        assert_eq!(run.kept_names().keep(ICON), Preserved::Kept);
        let (report, held) = run.finish_held().unwrap();

        let written = RepairEdit::held(&held, &archive, &report)
            .unwrap()
            .apply(&archive)
            .unwrap();

        assert_eq!(written.chunks_replaced, 1);
        assert_eq!(
            written.entries_replaced, 2,
            "the table, and the metadata that now declares it"
        );
        let files = read_where_it_lies(archive.as_std_path(), &[STALE_BIN_IN_WAD]);
        let chunk = files.files().find(|handle| handle.path() == CHUNK).unwrap();
        assert_eq!(chunk.bytes().unwrap(), b"repaired");
        let tables = tables_in(archive.as_std_path());
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].0.path().as_str(), TABLE);
        assert!(tables[0].1.names().any(|name| name == ICON));
    }

    /// Story: the archive already declares a game table, and the run's names
    /// are added to it instead of replacing it or declaring a second table.
    #[test]
    fn a_kept_name_joins_the_table_the_archive_already_declares() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = camino::Utf8PathBuf::from_path_buf(tmp.path().join("mod.fantome")).unwrap();
        declaring_fantome(archive.as_std_path(), &[OTHER]);
        let declared = tables_in(archive.as_std_path());
        let files = read_where_it_lies(archive.as_std_path(), &[]);
        let mut run = FixRun::held(files, declared, Vec::new(), None, Config::default(), None);
        assert_eq!(run.kept_names().keep(ICON), Preserved::Kept);
        let (report, held) = run.finish_held().unwrap();

        let written = RepairEdit::held(&held, &archive, &report)
            .unwrap()
            .apply(&archive)
            .unwrap();

        assert_eq!(
            written.entries_replaced, 1,
            "the table alone, which the metadata already declares"
        );
        let tables = tables_in(archive.as_std_path());
        assert_eq!(tables.len(), 1);
        let names: Vec<&str> = tables[0].1.names().collect();
        assert!(names.contains(&ICON) && names.contains(&OTHER), "{names:?}");
    }

    /// A file the report lists as written but the run holds no bytes for is a
    /// bug, and the edit returns an error for it.
    #[test]
    fn a_written_file_the_run_does_not_hold_is_refused() {
        let report = ltk_manager_problems::FixReport {
            applied: 1,
            skipped: 0,
            names_kept: 0,
            tables: Vec::new(),
            remaining: Vec::new(),
            files: vec![ltk_manager_problems::FileOutcome {
                layer: "base".to_owned(),
                path: CHUNK.to_owned(),
                applied: 1,
                skipped: 0,
                change: ltk_manager_problems::FileChange::Written,
            }],
            failed: Vec::new(),
        };

        let edit = RepairEdit::held(
            &ltk_manager_problems::HeldWrites::default(),
            camino::Utf8Path::new("mod.fantome"),
            &report,
        );

        assert!(edit.is_err());
    }
}
