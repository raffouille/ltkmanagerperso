//! The built-in mod that restores the skin the local player picked in champion select, above
//! base skins.
//!
//! The pick is a mod of its own so that a new pick rewrites a few files rather than the base
//! skins project, whose thousands of files the overlay then reads again.

use super::game_skins::GameSkins;
use super::mod_skins::ModSkins;
use super::overrides::Overrides;
use super::skin_bin::SkinBin;
use super::{BuiltinMod, Context};
use ltk_manager_base::config::{BaseSkinsScope, BuiltinModSettings};
use ltk_manager_base::error::AppResult;

/// The game's own bins of one skin, which outrank the stand-ins base skins writes for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PickedSkin {
    /// The champion's alias, such as `Kaisa`, which its archive is named after.
    champion: String,
    skin: u32,
}

impl PickedSkin {
    /// The mod `settings` turn on, and none when base skins is off or no skin past the base is
    /// picked.
    pub(super) fn of(settings: &BuiltinModSettings) -> Option<Self> {
        if settings.base_skins == BaseSkinsScope::Off {
            return None;
        }

        let (champion, skin) = settings.picked_skin.clone()?;
        (skin > 0).then_some(Self { champion, skin })
    }
}

impl BuiltinMod for PickedSkin {
    fn slug(&self) -> &'static str {
        "picked-skin"
    }

    fn display_name(&self) -> &'static str {
        "Picked skin"
    }

    /// The game's bytes of each bin of the picked skin, the champion's and its companions', in
    /// the archives that hold them. A bin a mod ships stays that mod's.
    fn generate(&self, cx: &mut Context<'_>) -> AppResult<Overrides> {
        let mut game = GameSkins::champions(cx.game_dir, cx.tables)?;
        let mods = ModSkins::load(cx.mods, &mut game, cx.cache_dir);

        let mut targets: Vec<(String, SkinBin)> = game
            .past_base_in(&self.champion)
            .into_iter()
            .chain(game.past_base_of(&self.champion))
            .filter(|(_, bin)| bin.id == self.skin && !mods.ships(bin))
            .map(|(archive, bin)| (archive.to_owned(), bin))
            .collect();
        targets.sort();
        targets.dedup();

        let mut overrides = Overrides::default();
        for (archive, bin) in targets {
            if let Some(bytes) = game.read(&bin) {
                overrides.insert(&archive, &bin.path(), bytes);
            }
        }
        Ok(overrides)
    }
}
