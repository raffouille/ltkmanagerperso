//! The preview service: the viewers' reads of a map, a skin, a spell and a VFX system, the
//! programs they draw with, and one asset's preview and its copy out.
//!
//! Its state is managed in `setup`, beside the shader caches and the sandboxes it reads through.

mod assets;
mod locate;
mod map;
mod map_geometry;
mod map_import;
pub(crate) mod material;
mod ritobin;
mod skin;
mod spell;
mod vfx;

pub use assets::*;
pub use locate::*;
pub use map::*;
pub use map_geometry::*;
pub use map_import::*;
pub use material::*;
pub use ritobin::*;
pub use skin::*;
pub use spell::*;
pub use vfx::*;

use tauri::plugin::TauriPlugin;
use tauri::Wry;

/// The preview service's row of `services/table.rs`.
pub struct Table;

/// The plugin answering the viewers' commands.
pub fn plugin() -> TauriPlugin<Wry> {
    super::plugin::<Table>().build()
}
