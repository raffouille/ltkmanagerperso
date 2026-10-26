# ADR-0069: The data location is one folder, and a move restarts the app

- **Status:** Accepted (2026-10-23)
- **Date:** 2026-10-23
- **Crates:** `src-tauri`, where `data_location` plans and runs the move, and
  `ltk-manager-library`, where `is_library_entry` names the entries of the library
- **Related:** "Data location" in `docs/ux/SETTINGS.md`

## Context and problem statement

The manager writes its data under `%APPDATA%\dev.leaguetoolkit.manager`, on the system drive. A
library with a few hundred mods takes several gigabytes there, with its overlay builds beside it.
`Config::mod_storage_path` re-rooted the library, but a change of it moved no file, so the library
read as empty and the old data stayed on the system drive. The mod previews, the translated shaders,
the patcher delivery state and the incidents stayed in the app data directory whatever the setting
said.

The WebView2 user data folder in `%LOCALAPPDATA%` holds the HTTP cache and the code cache of the
webview, which reach a gigabyte with no cap.

## Decision

**One data directory.** `data_location::data_dir` resolves `mod_storage_path`, or the app data
directory if it is unset. The library, the overlay builds, `mod-previews`, `shaders`, `patcher` and
`incidents` resolve through it. The setting keeps its `modStoragePath` key, so an existing
`settings.json` reads unchanged.

**The bootstrap files stay in the app data directory.** `settings.json` holds the location, so it
cannot live inside it. The logs start before the settings load. The telemetry spool, the workshop
project list and the window state are kilobytes and stay beside them.

**A change of the location moves the data.** `plan_data_move` lists the entries that the library
and the shell write, with their sizes, and refuses a folder that already holds one of them. If the
new folder holds a library and the current one holds none, the plan adopts the new folder and
moves nothing. `move_data` renames each entry, and copies it across volumes. A copied entry leaves
the old folder only after every entry reached the new one, and a failure returns the moved entries.
Only the named entries move, so a folder that also holds other files keeps them.

**A move restarts the app.** The library watcher, the incident store, the patcher delivery and the
WAD report cache read the data directory once at startup. A restart is the one point at which all
of them read the new folder, so no state needs a re-root of its own. The library index stays
locked from the move until the settings name the new folder, so no reconcile writes an index into
the old one.

**The webview cache is capped, not moved.** The window passes `--disk-cache-size=104857600` (100
MiB) to WebView2, beside the default arguments that Tauri sets only when none are given. Moving the
user data folder would need the location before the window exists.

## Consequences

- **Positive:** a reader moves every large file off the system drive with one setting, and an
  existing library on another drive is adopted after a reinstall.
- **Positive:** no state has a re-root path that only a move exercises.
- **Negative:** a move restarts the app, and a running patcher blocks it.
- **Negative:** the shared hashtable cache in `%LOCALAPPDATA%\LeagueToolkit` stays where it is,
  because other LeagueToolkit tools read it there.
- **Negative:** a new entry that the library or the shell writes under the data directory has to
  join `is_library_entry` or `SHELL_ENTRIES`, or a move leaves it behind.
