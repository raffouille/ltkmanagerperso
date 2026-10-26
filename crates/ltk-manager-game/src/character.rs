//! The characters of the install, and what a `CharacterRecord` links: the spells of a
//! champion's abilities.

use std::collections::HashSet;

use ltk_hash::BinHash;
use ltk_manager_base::hashing::named;
use ltk_manager_base::utils::natural_order::compare_names;
use ltk_manager_bin::bin_document::{Fields, items, leaf};
use ltk_manager_bin::object_index::ObjectIndex;
use ltk_meta::PropertyValueEnum;
use ltk_meta::walk::Leaf;

/// How many abilities a champion casts, Q, W, E and R.
pub const ABILITY_COUNT: usize = 4;

/// The record's links to its ability spells.
const SPELLS: BinHash = named("spells");
/// The record's links to its `AbilityObject`s, for a record that links no spells.
const ABILITIES: BinHash = named("mAbilities");
/// The spell an `AbilityObject` casts first.
const ROOT_SPELL: BinHash = named("mRootSpell");

/// Every character a named `Characters/{character}/CharacterRecords/Root` object in `index`
/// names, as the first path naming it spells it, in natural order.
///
/// Segments match without regard to ASCII case, as the tables name one character's paths in
/// more than one casing.
#[must_use]
pub fn characters(index: &ObjectIndex) -> Vec<String> {
    characters_named(index.named_under("Characters").map(|(_, path)| path))
}

/// The path of the `CharacterRecord` of `character`.
#[must_use]
pub fn record_path(character: &str) -> String {
    format!("Characters/{character}/CharacterRecords/Root")
}

/// The path of skin `skin` of `character`.
#[must_use]
pub fn skin_path(character: &str, skin: u32) -> String {
    format!("Characters/{character}/Skins/Skin{skin}")
}

/// The spells of the four abilities `record` links, each read through `at`: its `spells`
/// links, else each of its `mAbilities` through the ability's root spell. An ability that
/// does not read is `None`, and so is every ability of an absent record.
pub fn ability_spells(
    record: Option<&Fields>,
    mut at: impl FnMut(BinHash) -> Option<Fields>,
) -> Vec<Option<Fields>> {
    let Some(record) = record else {
        return vec![None; ABILITY_COUNT];
    };

    let linked = |field| {
        items(record.get(&field))
            .iter()
            .filter_map(|item| object(Some(item)))
            .take(ABILITY_COUNT)
            .collect::<Vec<_>>()
    };
    let spells = linked(SPELLS);
    let mut found: Vec<Option<Fields>> = if spells.is_empty() {
        linked(ABILITIES)
            .into_iter()
            .map(|ability| {
                let root = object(at(ability)?.get(&ROOT_SPELL))?;
                at(root)
            })
            .collect()
    } else {
        spells.into_iter().map(at).collect()
    };

    found.resize(ABILITY_COUNT, None);
    found
}

/// The characters among the record paths in `paths`, in natural order.
fn characters_named(paths: impl Iterator<Item = impl AsRef<str>>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut characters = Vec::new();

    for path in paths {
        let mut segments = path.as_ref().split('/');
        let (Some(root), Some(owner), Some(records), Some(record), None) = (
            segments.next(),
            segments.next(),
            segments.next(),
            segments.next(),
            segments.next(),
        ) else {
            continue;
        };

        if root.eq_ignore_ascii_case("Characters")
            && records.eq_ignore_ascii_case("CharacterRecords")
            && record.eq_ignore_ascii_case("Root")
            && !owner.is_empty()
            && seen.insert(owner.to_ascii_lowercase())
        {
            characters.push(owner.to_owned());
        }
    }

    characters.sort_by(|a, b| compare_names(a, b));
    characters
}

/// The object `value` names, by link or by hash, and none for a null one.
fn object(value: Option<&PropertyValueEnum>) -> Option<BinHash> {
    let hash = match leaf(value)? {
        Leaf::Link(hash) => hash,
        Leaf::Hash(hash) => hash.try_as_bin_hash()?,
        _ => return None,
    };

    (hash.0 != 0).then_some(hash)
}

#[cfg(test)]
mod tests;
