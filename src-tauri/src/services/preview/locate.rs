//! Where an object that an open document reaches is declared, for a tab or an edit of it.

use crate::error::IpcResult;
use crate::services::shared::document_assets::{parse_entry, read_lookup};
use crate::services::shared::off_thread;
use ltk_manager_assets::preview::AssetRef;
use ltk_manager_bin::bin_document::{BinDocumentId, BinDocuments};
use tauri::{AppHandle, Manager};

/// The bin that declares the object `entry`, found the way every read of `document` finds it:
/// the open document first, then the files it links.
///
/// `entry` is the object's hash as `0x` and eight hex digits. Returns the document's own asset
/// if it declares the object, and `None` if no bin of the lookup does.
///
/// # Errors
///
/// Fails if `entry` is not a hash or no document is open under `document`.
#[tauri::command]
#[specta::specta]
pub async fn locate_object(
    document: BinDocumentId,
    entry: String,
    app_handle: AppHandle,
) -> IpcResult<Option<AssetRef>> {
    off_thread(move || {
        let entry = parse_entry(&entry)?;
        let own = app_handle.state::<BinDocuments>().asset_of(document);

        read_lookup(&app_handle, document, |lookup| {
            Ok(lookup
                .find(entry)
                .and_then(|found| lookup.asset(found).cloned().or(own)))
        })
    })
    .await
}
