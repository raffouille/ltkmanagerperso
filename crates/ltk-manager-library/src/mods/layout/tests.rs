use super::*;

#[test]
fn is_library_entry_matches_root_entries_and_index_backups() {
    for name in [
        "library.json",
        "mods",
        "profiles",
        "builtin",
        "library.v1.json.bak",
    ] {
        assert!(is_library_entry(name), "{name}");
    }

    for name in [
        "settings.json",
        "logs",
        "library.vx.json.bak",
        "library.v.json.bak",
    ] {
        assert!(!is_library_entry(name), "{name}");
    }
}
