//! The bytes of an asset, read through the shared WAD cache under the current settings.

use std::cell::OnceCell;

use ltk_hash::{BinHash, WadHash};
use ltk_manager_assets::game_wads::WadCache;
use ltk_manager_assets::preview::AssetRef;
use ltk_manager_bin::bin_document::{AssetLookup, BinDocument};
use ltk_manager_bin::sandbox::SandboxRef;
use tauri::{AppHandle, Manager};

use crate::error::AppResult;
use crate::services::objects::index::ready_object_index;
use crate::services::shared::installed::ProjectGame;
use crate::state::SettingsState;

/// The bytes of `asset`.
pub(crate) fn read_asset(app: &AppHandle, asset: &AssetRef) -> AppResult<Vec<u8>> {
    asset.read(
        &app.state::<SettingsState>().config(),
        &app.state::<WadCache>(),
    )
}

/// The bin document `asset` holds.
pub(crate) fn read_bin(app: &AppHandle, asset: &AssetRef) -> AppResult<BinDocument> {
    Ok(BinDocument::parse(read_asset(app, asset)?)?)
}

/// A reader of many assets, under the settings as they are now.
pub(crate) fn asset_reader(app: &AppHandle) -> impl FnMut(&AssetRef) -> AppResult<Vec<u8>> + '_ {
    let config = app.state::<SettingsState>().config();
    let wads = app.state::<WadCache>();

    move |asset| asset.read(&config, &wads)
}

/// A reader of the bins a document of `sandbox` links, which passes over one it cannot read.
///
/// A game chunk is read as the sandbox's project builds it, with the project's game data
/// declarations applied, so an object that a declaration creates in a linked bin resolves.
pub(crate) fn linked_reader<'a>(
    app: &'a AppHandle,
    sandbox: &SandboxRef,
) -> impl FnMut(&AssetRef) -> Option<BinDocument> + 'a {
    let sandbox = sandbox.clone();
    /* The declarations load on the first linked bin, and a read that links none skips them. */
    let mut game: Option<ProjectGame> = None;

    move |asset| {
        parse_linked(
            game.get_or_insert_with(|| ProjectGame::new(app, &sandbox)),
            asset,
        )
    }
}

/// The bins that a read of a document of `sandbox` reads beside the document: the bins it
/// links, and the bins that may declare an object that no linked bin declares.
///
/// Both read the game as the project builds it, and its declarations load once, on first use.
pub(crate) struct GameBins<'a> {
    app: &'a AppHandle,
    sandbox: SandboxRef,
    game: OnceCell<ProjectGame>,
}

impl<'a> GameBins<'a> {
    pub(crate) fn new(app: &'a AppHandle, sandbox: &SandboxRef) -> Self {
        Self {
            app,
            sandbox: sandbox.clone(),
            game: OnceCell::new(),
        }
    }

    fn game(&self) -> &ProjectGame {
        self.game
            .get_or_init(|| ProjectGame::new(self.app, &self.sandbox))
    }

    /// Returns a reader of the bins a document links, as [`linked_reader`] reads them.
    pub(crate) fn reader(&self) -> impl FnMut(&AssetRef) -> Option<BinDocument> + '_ {
        move |asset| parse_linked(self.game(), asset)
    }

    /// Returns the bins that may declare an object, for `ObjectLookup::declared_by`.
    ///
    /// The targets of the project's modules that create the object come first, and `assets`
    /// locates them. Else every bin that the object index finds declaring it, and a read that
    /// needs the index builds it.
    pub(crate) fn declaring<'b>(
        &'b self,
        assets: &'b dyn AssetLookup,
    ) -> impl FnMut(BinHash) -> Vec<AssetRef> + 'b {
        move |entry| {
            let created: Vec<AssetRef> = self
                .game()
                .creating_targets(entry)
                .into_iter()
                .filter_map(|target| {
                    assets
                        .locate(target.as_str())
                        .or_else(|| assets.locate_chunk(WadHash(target.chunk_hash())))
                })
                .collect();
            if !created.is_empty() {
                return created;
            }

            ready_object_index(self.app)
                .and_then(|index| index.declared(entry))
                .map(|declared| {
                    declared
                        .declarations
                        .into_iter()
                        .map(|declaration| declaration.asset)
                        .collect()
                })
                .unwrap_or_default()
        }
    }
}

/// Reads the linked bin `asset` through `game`, and passes over one it cannot read.
fn parse_linked(game: &ProjectGame, asset: &AssetRef) -> Option<BinDocument> {
    match game
        .read(asset)
        .and_then(|bytes| Ok(BinDocument::parse(bytes)?))
    {
        Ok(bin) => Some(bin),
        Err(e) => {
            tracing::debug!(?asset, "Passed over a linked bin: {e}");
            None
        }
    }
}
