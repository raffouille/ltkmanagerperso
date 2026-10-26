//! The particle renderer's read: one system's whole value tree in one call.
//!
//! The tree is the open document's (ADR-0026), and this answers a subtree of it with
//! every reference resolved rather than a window of rows.

use crate::error::IpcResult;
use crate::services::shared::document_assets::{parse_entry, read_lookup, read_resolved};
use crate::services::shared::linked_reader;
use crate::services::shared::off_thread;
use ltk_manager_bin::bin_document::BinDocumentId;
use ltk_manager_bin::vfx::{vfx_templates as catalog, VfxTemplate};
use ltk_manager_game::spell::{missile_spells, MissileSpell};
use ltk_manager_game::vfx::{child_targets, VfxObject, VfxSystem};
use tauri::AppHandle;

/// One particle system of an open document, with every reference resolved.
///
/// `entry` is the object's hash as `0x` and eight hex digits. A class or field the hash
/// tables leave unnamed takes the meta schema's name. The system, a child system or a custom
/// material that the document does not declare is looked for through the files it links, and
/// a linked file that cannot be read is passed over.
#[tauri::command]
#[specta::specta]
pub async fn read_vfx_system(
    document: BinDocumentId,
    entry: String,
    app_handle: AppHandle,
) -> IpcResult<VfxSystem> {
    off_thread(move || {
        let entry = parse_entry(&entry)?;
        read_lookup(&app_handle, document, |lookup| Ok(lookup.system(entry)?))
    })
    .await
}

/// The spells that fire one particle system of an open document as their missile.
///
/// `entry` is the system's hash as `0x` and eight hex digits. The spells are read from the
/// bin of the character that the system's path names. Returns none for a system that no
/// table names or whose path names no character.
#[tauri::command]
#[specta::specta]
pub async fn vfx_missile_spells(
    document: BinDocumentId,
    entry: String,
    app_handle: AppHandle,
) -> IpcResult<Vec<MissileSpell>> {
    off_thread(move || {
        let entry = parse_entry(&entry)?;
        read_resolved(&app_handle, document, |open, names, assets| {
            let Some(path) = names.entry_name(entry) else {
                return Ok(Vec::new());
            };
            let mut read = linked_reader(&app_handle);
            Ok(missile_spells(open, entry, &path, names, assets, &mut read))
        })
    })
    .await
}

/// The systems that a child particle set of an open document can link.
#[tauri::command]
#[specta::specta]
pub async fn vfx_child_targets(
    document: BinDocumentId,
    app_handle: AppHandle,
) -> IpcResult<Vec<VfxObject>> {
    off_thread(move || {
        read_resolved(&app_handle, document, |open, names, _| {
            Ok(child_targets(open, names))
        })
    })
    .await
}

/// Every VFX template of the catalog, which the Graph pane and the inspector offer. ADR-0058.
#[tauri::command]
#[specta::specta]
pub async fn vfx_templates() -> IpcResult<Vec<VfxTemplate>> {
    off_thread(|| Ok(catalog())).await
}
