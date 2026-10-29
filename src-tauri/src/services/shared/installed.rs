//! The installed game as the bin editor and the viewers read it.

use std::sync::Arc;

use super::document_assets::with_cache_names;
use super::read_asset;
use crate::error::AppResult;
use crate::services::objects::index::ObjectIndexState;
use crate::state::SettingsState;
use ltk_game_data::Target;
use ltk_hash::BinHash;
use ltk_manager_assets::preview::AssetRef;
use ltk_manager_base::game_build::GameBuild;
use ltk_manager_bin::bin_document::{GameCopy, ProjectDeclarations, RowNames};
use ltk_manager_bin::meta_schema::{self, MetaSchema, PatchSchema};
use ltk_manager_bin::object_index::ObjectIndexSnapshot;
use ltk_manager_bin::sandbox::SandboxRef;
use ltk_manager_workshop::ProjectDir;
use tauri::{AppHandle, Manager};

/// The installed game as a declared document reads it: the shared tables for names, and
/// the object index for an entry a reference names.
pub(crate) struct InstalledGame(pub(crate) AppHandle);

impl InstalledGame {
    /// The first chunk the object index finds declaring `entry`. An index that is not ready
    /// answers none.
    fn declaring_asset(&self, entry: BinHash) -> Option<AssetRef> {
        let ObjectIndexSnapshot::Ready(index) = self.0.state::<ObjectIndexState>().snapshot()
        else {
            return None;
        };
        index
            .declared(entry)
            .and_then(|declared| declared.declarations.into_iter().next())
            .map(|first| first.asset)
    }

    fn read(&self, asset: &AssetRef) -> AppResult<Vec<u8>> {
        read_asset(&self.0, asset)
    }
}

impl GameCopy for InstalledGame {
    fn declaring_chunk(&self, entry: BinHash) -> AppResult<Option<Vec<u8>>> {
        self.declaring_asset(entry)
            .map(|asset| self.read(&asset))
            .transpose()
    }

    fn with_names(&self, read: &mut dyn FnMut(&dyn RowNames)) {
        with_cache_names(&self.0, |names| read(names));
    }
}

/// The installed game as a project's build writes it: each game chunk a preview reads with the
/// project's game data declarations applied, as ADR-0042 applies them. The game sandbox, and a
/// project whose declarations cannot be read, read the installed game alone.
pub(crate) struct ProjectGame {
    installed: InstalledGame,
    declarations: Option<ProjectDeclarations>,
}

impl ProjectGame {
    /// The game as the project of `sandbox` builds it.
    pub(crate) fn new(app_handle: &AppHandle, sandbox: &SandboxRef) -> Self {
        let declarations = sandbox.project().and_then(|project| {
            let (schema, build) = installed_schema(app_handle);
            ProjectDir::open(project)
                .and_then(|project| {
                    ProjectDeclarations::load(
                        &project,
                        PatchSchema::new(schema, build),
                        Arc::new(InstalledGame(app_handle.clone())),
                    )
                })
                .inspect_err(|error| {
                    tracing::warn!(%project, %error, "A preview reads no game data declarations");
                })
                .ok()
        });

        Self {
            installed: InstalledGame(app_handle.clone()),
            declarations,
        }
    }

    /// Returns the target of each module of the project that creates the object `entry`, in
    /// build order, and none for the game sandbox.
    pub(crate) fn creating_targets(&self, entry: BinHash) -> Vec<&Target> {
        self.declarations
            .as_ref()
            .map(|declarations| declarations.creating_targets(entry))
            .unwrap_or_default()
    }

    /// The file `asset`, a bin chunk of the game with the declarations that reach it applied.
    pub(crate) fn read(&self, asset: &AssetRef) -> AppResult<Vec<u8>> {
        let bytes = self.installed.read(asset)?;
        Ok(match &self.declarations {
            Some(declarations) => declarations.declared_asset(asset, bytes),
            None => bytes,
        })
    }
}

impl GameCopy for ProjectGame {
    fn declaring_chunk(&self, entry: BinHash) -> AppResult<Option<Vec<u8>>> {
        self.installed
            .declaring_asset(entry)
            .map(|asset| self.read(&asset))
            .transpose()
    }

    fn with_names(&self, read: &mut dyn FnMut(&dyn RowNames)) {
        self.installed.with_names(read);
    }
}

/// The shared meta schema and the installed game's content build, which keys every
/// answer read out of it.
pub(crate) fn installed_schema(app_handle: &AppHandle) -> (Arc<MetaSchema>, Option<GameBuild>) {
    let config = app_handle.state::<SettingsState>().config();
    let build = GameBuild::installed(&config);
    (meta_schema::shared(build), build)
}
