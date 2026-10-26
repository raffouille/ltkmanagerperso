//! The effects a skin draws on its joints, its persistent conditions and the keys its resolver
//! maps to systems.

use std::collections::HashMap;

use ltk_hash::BinHash;
use ltk_manager_base::hashing::named;
use ltk_meta::PropertyValueEnum;
use ltk_meta::walk::Leaf;
use serde::Serialize;

use ltk_manager_bin::bin_document::{
    BinDocument, Fields, RowNames, fields_of, hex, items, leaf, text,
};

use crate::resolver::{EFFECT_KEY, resolver_entries};

/// `SkinCharacterDataProperties.PersistentEffectConditions`.
pub(super) const PERSISTENT_CONDITIONS: BinHash = named("PersistentEffectConditions");
/// `PersistentEffectConditionData.PersistentVfxs`.
pub(super) const PERSISTENT_VFXS: BinHash = named("PersistentVfxs");
/// `PersistentEffectConditionData.SubmeshesToShow`.
pub(super) const SUBMESHES_TO_SHOW: BinHash = named("SubmeshesToShow");
/// `PersistentEffectConditionData.SubmeshesToHide`.
pub(super) const SUBMESHES_TO_HIDE: BinHash = named("SubmeshesToHide");
/// `SkinCharacterDataProperties_CharacterIdleEffect.effectName`, whose hash keys an idle effect
/// that has no `effectKey`.
pub(super) const EFFECT_NAME: BinHash = named("effectName");
/// `SkinCharacterDataProperties_CharacterIdleEffect.boneName`.
pub(super) const BONE_NAME: BinHash = named("boneName");
/// `SkinCharacterDataProperties_CharacterIdleEffect.targetBoneName`.
pub(super) const TARGET_BONE_NAME: BinHash = named("targetBoneName");
/// `SkinCharacterDataProperties_CharacterIdleEffect.Position`.
pub(super) const POSITION: BinHash = named("Position");

/// One key of the skin's resolver, and the system it stands for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct EffectSystem {
    /// The key, `0x` and eight hex digits.
    pub key: String,
    /// The `VfxSystemDefinitionData` object, `0x` and eight hex digits.
    pub system: String,
    /// The system's path, where a table names it.
    pub name: Option<String>,
}

/// One effect a skin draws on a joint: an idle effect, or an effect of a persistent condition.
///
/// `PersistentVfxData` declares no `Position`, so the effect of a condition has a zero offset.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct JointEffect {
    /// `effectKey`, `0x` and eight hex digits, or the hash of `effectName` if the effect has
    /// no key. [`SkinModel::effect_systems`](super::SkinModel::effect_systems) maps it to its system.
    pub effect_key: String,
    /// `boneName`, the joint the effect rides.
    pub bone: String,
    /// `targetBoneName`, the joint it aims at, and empty for one that aims at none.
    pub target_bone: String,
    /// `Position`, the effect's offset from its joint.
    pub position: [f32; 3],
}

/// One entry of `PersistentEffectConditions`, which the engine applies while its conditions
/// are true.
///
/// The conditions themselves are left out, because the preview does not evaluate them.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct PersistentCondition {
    /// `PersistentVfxs`, in the order the entry lists them.
    pub effects: Vec<JointEffect>,
    /// `SubmeshesToShow`, each the hash of a submesh name, `0x` and eight hex digits.
    pub shown: Vec<String>,
    /// `SubmeshesToHide`, each the hash of a submesh name, `0x` and eight hex digits.
    pub hidden: Vec<String>,
}

/// One key of the resolver and the system it maps to, named by `names`.
pub(super) fn effect_system(key: BinHash, system: BinHash, names: &dyn RowNames) -> EffectSystem {
    EffectSystem {
        key: hex(key),
        system: hex(system),
        name: names.entry_name(system),
    }
}

/// The skin's persistent conditions, in the order the skin lists them.
pub(super) fn persistent_conditions(skin: &Fields) -> Vec<PersistentCondition> {
    items(skin.get(&PERSISTENT_CONDITIONS))
        .iter()
        .filter_map(|item| {
            let fields = fields_of(Some(item))?;

            Some(PersistentCondition {
                effects: effects_in(fields.get(&PERSISTENT_VFXS)),
                shown: hashes_in(fields.get(&SUBMESHES_TO_SHOW)),
                hidden: hashes_in(fields.get(&SUBMESHES_TO_HIDE)),
            })
        })
        .collect()
}

/// The effects the list `list` holds.
///
/// An element of `idleParticlesEffects` and one of `PersistentVfxs` share the field names
/// read here.
pub(super) fn effects_in(list: Option<&PropertyValueEnum>) -> Vec<JointEffect> {
    items(list)
        .iter()
        .filter_map(|item| {
            let fields = fields_of(Some(item))?;
            let key = match leaf(fields.get(&EFFECT_KEY)) {
                Some(Leaf::Hash(key)) => key.try_as_bin_hash().unwrap_or(BinHash(0)),
                _ => BinHash(0),
            };
            let key = match text(fields.get(&EFFECT_NAME)) {
                Some(name) if key.0 == 0 => named(name),
                _ => key,
            };

            Some(JointEffect {
                effect_key: hex(key),
                bone: text(fields.get(&BONE_NAME)).unwrap_or_default().to_owned(),
                target_bone: text(fields.get(&TARGET_BONE_NAME))
                    .unwrap_or_default()
                    .to_owned(),
                position: match leaf(fields.get(&POSITION)) {
                    Some(Leaf::Vector3(position)) => position.to_array(),
                    _ => [0.0; 3],
                },
            })
        })
        .collect()
}

/// The hashes the list `list` holds, `0x` and eight hex digits each, in its order.
fn hashes_in(list: Option<&PropertyValueEnum>) -> Vec<String> {
    items(list)
        .iter()
        .filter_map(|item| match leaf(Some(item)) {
            Some(Leaf::Hash(hash)) => hash.try_as_bin_hash().map(hex),
            _ => None,
        })
        .collect()
}

/// The effect keys the resolver object `resolver` maps, each with the system it names, in
/// key order.
///
/// The first entry for a key wins, which is the order the engine probes a map in.
pub(super) fn resolver_systems(
    document: &BinDocument,
    resolver: Option<BinHash>,
) -> Vec<(BinHash, BinHash)> {
    let mut systems = HashMap::new();
    let Some(resolver) = resolver.and_then(|resolver| document.object_at(resolver)) else {
        return Vec::new();
    };
    for (key, target) in resolver_entries(resolver) {
        systems.entry(key).or_insert(target);
    }
    let mut mapped: Vec<(BinHash, BinHash)> = systems.into_iter().collect();
    mapped.sort_by_key(|(key, _)| key.0);
    mapped
}
