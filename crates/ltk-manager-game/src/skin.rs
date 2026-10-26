//! What a skin gives a viewport: the files its character is built from, the effects it
//! wears, and the animation graph its clips come out of.
//!
//! Rust resolves the references and TypeScript draws them, the split
//! docs/plans/vfx-particle-renderer.md makes for a particle system. A skin reads a
//! handful of named fields rather than walking its subtree, because the resolver it links
//! maps every system its file declares and a walk would inline all of them.

mod colliders;
mod dynamics;
mod effects;
mod shipped;
mod tangents;

pub use colliders::{ColliderCapsule, ColliderShapes, ColliderSphere, save_colliders};
pub use dynamics::{
    ChainProperties, CurveKeys, ExtraJointChain, JointTree, JointTreeGroup, PoseModifier,
    ScaledCurve, Socket,
};
pub use effects::{EffectSystem, JointEffect, PersistentCondition};
pub use shipped::{ShippedSkin, shipped_skin};
pub use tangents::bake_mesh_tangents;

use std::collections::HashSet;

use ltk_hash::BinHash;
use ltk_manager_base::hashing::named;
use ltk_meta::PropertyValueEnum;
use ltk_meta::walk::Leaf;
use serde::Serialize;

use ltk_manager_bin::bin_document::{
    AssetLookup, BinDocument, BinDocumentError, Fields, Locator, RowNames, fields_of, hash_hex,
    hex, items, leaf, link, object_at, text,
};
pub use ltk_manager_bin::bin_document::{NamedAsset, boolean, float, struct_entries};

use crate::lookup::{ObjectLookup, find_linked_materials};
use crate::material::{MaterialPreview, linked_material};
use effects::{effect_system, effects_in, persistent_conditions, resolver_systems};

/// `SkinCharacterDataProperties.skinMeshProperties`.
const MESH_PROPERTIES: BinHash = named("skinMeshProperties");
/// `SkinMeshDataProperties.simpleSkin`, the `.skn`.
const SIMPLE_SKIN: BinHash = named("simpleSkin");
/// `SkinMeshDataProperties.skeleton`, the `.skl`.
const SKELETON: BinHash = named("skeleton");
/// `texture`, on the mesh properties and on each material override.
const TEXTURE: BinHash = named("texture");
/// `SkinMeshDataProperties.skinScale`.
const SKIN_SCALE: BinHash = named("skinScale");
/// `SkinMeshDataProperties.selfIllumination`.
const SELF_ILLUMINATION: BinHash = named("selfIllumination");
/// `SkinMeshDataProperties.emissiveTexture`.
const EMISSIVE_TEXTURE: BinHash = named("emissiveTexture");
/// `SkinMeshDataProperties.initialSubmeshToHide`.
const HIDDEN_SUBMESHES: BinHash = named("initialSubmeshToHide");
/// `SkinMeshDataProperties.materialOverride`.
const MATERIAL_OVERRIDE: BinHash = named("materialOverride");
/// `SkinMeshDataProperties_MaterialOverride.submesh`.
const SUBMESH: BinHash = named("submesh");
/// `Material`, the `StaticMaterialDef` link on the mesh properties and on each override.
const MATERIAL: BinHash = named("Material");
/// `SkinCharacterDataProperties.skinAnimationProperties`.
const ANIMATION_PROPERTIES: BinHash = named("skinAnimationProperties");
/// `SkinAnimationProperties.animationGraphData`.
const ANIMATION_GRAPH: BinHash = named("animationGraphData");
/// `SkinCharacterDataProperties.idleParticlesEffects`.
const IDLE_EFFECTS: BinHash = named("idleParticlesEffects");
/// `SkinCharacterDataProperties.mResourceResolver`.
const RESOURCE_RESOLVER: BinHash = named("mResourceResolver");
/// `AnimationGraphData.mClipDataMap`.
const CLIP_DATA_MAP: BinHash = named("mClipDataMap");
/// `AnimationGraphData.mTrackDataMap`.
const TRACK_DATA_MAP: BinHash = named("mTrackDataMap");
/// `AnimationGraphData.mMaskDataMap`.
const MASK_DATA_MAP: BinHash = named("mMaskDataMap");
/// `AnimationGraphData.mSyncGroupDataMap`.
const SYNC_GROUP_DATA_MAP: BinHash = named("mSyncGroupDataMap");
/// `AtomicClipData.mAnimationResourceData`.
const ANIMATION_RESOURCE: BinHash = named("mAnimationResourceData");
/// `AnimationResourceData.mAnimationFilePath`.
const ANIMATION_FILE: BinHash = named("mAnimationFilePath");
/// `AtomicClipData.mTickDuration`.
const TICK_DURATION: BinHash = named("mTickDuration");
/// `BlendableClipData.mTrackDataName`.
const TRACK_DATA_NAME: BinHash = named("mTrackDataName");
/// `BlendableClipData.mMaskDataName`.
const MASK_DATA_NAME: BinHash = named("mMaskDataName");
/// `BlendableClipData.mSyncGroupDataName`.
const SYNC_GROUP_DATA_NAME: BinHash = named("mSyncGroupDataName");
/// `mEventDataMap`, on a blendable clip and on a sequencer.
const EVENT_DATA_MAP: BinHash = named("mEventDataMap");
/// `ClipBaseData.mAnimationInterruptionGroupNames`.
const INTERRUPTION_GROUPS: BinHash = named("mAnimationInterruptionGroupNames");
/// `ClipBaseData.mFlags`.
const FLAGS: BinHash = named("mFlags");
/// `TrackData.mPriority`.
const TRACK_PRIORITY: BinHash = named("mPriority");
/// `TrackData.mBlendMode`.
const TRACK_BLEND_MODE: BinHash = named("mBlendMode");
/// `TrackData.mBlendWeight`.
const TRACK_BLEND_WEIGHT: BinHash = named("mBlendWeight");
/// `MaskData.mId`.
const MASK_ID: BinHash = named("mId");
/// `MaskData.mWeightList`.
const MASK_WEIGHTS: BinHash = named("mWeightList");
/// `SyncGroupData.mType`.
const SYNC_GROUP_TYPE: BinHash = named("mType");
/// `ParametricClipData.mParametricPairDataList`.
const PARAMETRIC_PAIRS: BinHash = named("mParametricPairDataList");
/// `ParametricPairData.mClipName`, which the other `m`-prefixed pair kinds share.
const PAIR_CLIP: BinHash = named("mClipName");
/// `ParametricPairData.mValue`, the parameter the pair's clip plays at.
const PAIR_VALUE: BinHash = named("mValue");
/// `BaseEventData.mStartFrame`.
const EVENT_START_FRAME: BinHash = named("mStartFrame");
/// `BaseEventData.mEndFrame`.
const EVENT_END_FRAME: BinHash = named("mEndFrame");
const PARAMETRIC_CLIP: BinHash = named("ParametricClipData");
const SEQUENCER_CLIP: BinHash = named("SequencerClipData");
/// The flag of a parametric clip that turns its own events on, which no table names.
const CLIP_OWN_EVENTS: BinHash = BinHash(0x69de_8fca);
const SUBMESH_VISIBILITY_EVENT: BinHash = named("SubmeshVisibilityEventData");
/// `SubmeshVisibilityEventData.mShowSubmeshList`.
const EVENT_SHOW_SUBMESHES: BinHash = named("mShowSubmeshList");
/// `SubmeshVisibilityEventData.mHideSubmeshList`.
const EVENT_HIDE_SUBMESHES: BinHash = named("mHideSubmeshList");
const PARTICLE_EVENT: BinHash = named("ParticleEventData");
/// `ParticleEventData.mEffectKey`, which is not the idle effect's `effectKey`.
const EVENT_EFFECT_KEY: BinHash = named("mEffectKey");
/// `ParticleEventData.mEffectName`.
const EVENT_EFFECT_NAME: BinHash = named("mEffectName");
/// `ParticleEventData.mParticleEventDataPairList`.
const EVENT_PAIRS: BinHash = named("mParticleEventDataPairList");
/// `ParticleEventData.mIsLoop`.
const EVENT_IS_LOOP: BinHash = named("mIsLoop");
/// `ParticleEventData.mIsKillEvent`.
const EVENT_IS_KILL: BinHash = named("mIsKillEvent");
/// `ParticleEventData.scale`.
const EVENT_SCALE: BinHash = named("scale");
/// `ParticleEventDataPair.mBoneName`.
const EVENT_BONE: BinHash = named("mBoneName");
/// `ParticleEventDataPair.mTargetBoneName`.
const EVENT_TARGET_BONE: BinHash = named("mTargetBoneName");
const JOINT_SNAP_EVENT: BinHash = named("JointSnapEventData");
/// `JointSnapEventData.mJointNameToOverride`.
const EVENT_JOINT: BinHash = named("mJointNameToOverride");
/// `JointSnapEventData.mJointNameToSnapTo`.
const EVENT_SNAP_TO: BinHash = named("mJointNameToSnapTo");
/// `JointSnapEventData.offset`.
const EVENT_OFFSET: BinHash = named("offset");
const CONFORM_EVENT: BinHash = named("ConformToPathEventData");
/// `ConformToPathEventData.mBlendInTime`.
const EVENT_BLEND_IN: BinHash = named("mBlendInTime");
/// `ConformToPathEventData.mBlendOutTime`.
const EVENT_BLEND_OUT: BinHash = named("mBlendOutTime");
const DYNAMICS_CHAIN_BLEND_EVENT: BinHash = named("DynamicsChainBlendEventData");
/// `BlendFromDefaultDuration`, on a dynamics chain blend event and on a joint orientation's blend.
const EVENT_BLEND_FROM_DEFAULT: BinHash = named("BlendFromDefaultDuration");
/// `BlendToDefaultDuration`, on the same two.
const EVENT_BLEND_TO_DEFAULT: BinHash = named("BlendToDefaultDuration");
const SPRING_PHYSICS_EVENT: BinHash = named("SpringPhysicsEventData");
/// `SpringPhysicsEventData.SpringToAffect`.
const EVENT_SPRING: BinHash = named("SpringToAffect");
/// `BlendOutTime`, on a spring event and on a lock root orientation event.
const EVENT_BLEND_OUT_TIME: BinHash = named("BlendOutTime");
const JOINT_ORIENTATION_EVENT: BinHash = named("JointOrientationEventData");
/// `JointOrientationEventData.BlendData`.
const EVENT_BLEND_DATA: BinHash = named("BlendData");
/// The source a joint orientation event puts in place of the modifier's, which no table names.
const EVENT_SOURCE_OVERRIDE: BinHash = BinHash(0x3e20_ae96);
const LOCK_ROOT_ORIENTATION_EVENT: BinHash = named("LockRootOrientationEventData");
/// `LockRootOrientationEventData.JointName`.
const EVENT_JOINT_NAME: BinHash = named("JointName");

/// Where a clip names the clips it plays: a `Hash` field, a list of them, or a list of
/// pairs each naming one.
///
/// Every kind of `ClipBaseData` keeps its children under one of these, "What the data
/// is" in docs/plans/animation-graph-table.md. A clip holds the fields of its own kind
/// alone, so one list read in this order is each kind's own field order.
const CHILD_FIELDS: &[Children] = &[
    /* ParametricClipData.mParametricPairDataList[].mClipName */
    Children::Pairs(PARAMETRIC_PAIRS, PAIR_CLIP),
    /* SelectorClipData.mSelectorPairDataList[].mClipName */
    Children::Pairs(BinHash(0x512c_9525), PAIR_CLIP),
    /* ConditionBoolClipData.mTrueConditionClipName */
    Children::One(BinHash(0x4d7a_54c0)),
    /* ConditionBoolClipData.mFalseConditionClipName */
    Children::One(BinHash(0x24af_5ac1)),
    /* ConditionFloatClipData.mConditionFloatPairDataList[].mClipName */
    Children::Pairs(BinHash(0x2329_eec5), PAIR_CLIP),
    /* SequencerClipData.mClipNameList, and ParallelClipData's */
    Children::Many(BinHash(0x078c_afd9)),
    /* EventControlledSelectorClipData.SelectorPairDataList[].ClipName */
    Children::Pairs(BinHash(0xd188_b400), BinHash(0x68c1_4f60)),
    /* EventControlledSelectorClipData.DefaultClipName */
    Children::One(BinHash(0x9a7f_92cb)),
    /* StateAnimClipData.ChildClipName */
    Children::One(BinHash(0x8e5e_6618)),
    /* StateAnimClipData.Transitions[].TargetClipName */
    Children::Pairs(BinHash(0x2132_8a43), BinHash(0xc6f2_91ed)),
    /* SwitchIntClipData.SwitchIntPairDataList[].ClipName */
    Children::Pairs(BinHash(0x778d_6dee), BinHash(0x68c1_4f60)),
];

/// One field a clip names its children through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Children {
    /// A `Hash` field naming one clip.
    One(BinHash),
    /// A `List<Hash>` naming clips in order.
    Many(BinHash),
    /// A list of structs, each naming one clip in the second field.
    Pairs(BinHash, BinHash),
}

/// A skin, as a viewport draws it.
///
/// A submesh picks what it draws with in the engine's order: its override's `Material`,
/// else its override's `texture`, else the skin's `Material`, else the skin's `texture`.
/// Section 1.3 of docs/research/static-material-studio-rendering.md.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct SkinModel {
    /// The `.skn`, `skinMeshProperties.simpleSkin`.
    pub mesh: Option<NamedAsset>,
    /// The `.skl`, `skinMeshProperties.skeleton`.
    pub skeleton: Option<NamedAsset>,
    /// The texture a submesh draws with where no override names its own.
    pub texture: Option<NamedAsset>,
    /// `emissiveTexture`, the emissive mask of a submesh with no material.
    pub emissive_texture: Option<NamedAsset>,
    /// The `Material` a submesh draws with where no override names its own.
    pub material: Option<MaterialPreview>,
    /// The submeshes a `materialOverride` gives a texture or a material of their own.
    pub overrides: Vec<SubmeshOverride>,
    /// The submeshes `initialSubmeshToHide` names, which the character starts without.
    pub hidden: Vec<String>,
    /// `skinScale`, which the character is drawn at.
    pub scale: f32,
    /// `selfIllumination`, added to the character's ambient light. Zero by default.
    pub self_illumination: f32,
    /// `skinAnimationProperties.animationGraphData`, `0x` and eight hex digits.
    pub animation_graph: Option<String>,
    /// `idleParticlesEffects`, in the order the skin lists them.
    pub idle_effects: Vec<JointEffect>,
    /// `PersistentEffectConditions`, in the order the skin lists them.
    pub persistent_conditions: Vec<PersistentCondition>,
    /// Every effect key `mResourceResolver` maps to a system that a bin the read reached
    /// declares, in key order.
    ///
    /// A particle event of the graph names a key of this map, and the graph is read from
    /// another file, so the map crosses with the skin for the viewport to look the key up.
    /// [`ObjectLookup::skin`] reads the resolver and its systems from the bins of the lookup.
    pub effect_systems: Vec<EffectSystem>,
    /// `skinMeshProperties.rigPoseModifierData`, in the order the skin lists them.
    pub pose_modifiers: Vec<PoseModifier>,
    /// `skinMeshProperties.SocketDefinitions`, in the order the skin lists them.
    pub sockets: Vec<Socket>,
}

/// One submesh a material override gives its own texture or material.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct SubmeshOverride {
    /// The submesh's name as the `.skn` spells it.
    pub submesh: String,
    /// The override's `texture`, which the submesh draws with in place of the skin's own.
    pub texture: Option<NamedAsset>,
    /// The override's `Material`, which wins over every texture.
    pub material: Option<MaterialPreview>,
}

/// An animation graph, as a clip table and a viewport read it.
///
/// "The model" in docs/plans/animation-graph-table.md. Every list keeps the order its
/// map holds, and every key is named by the tables or written as its hex.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct AnimationGraph {
    /// `mClipDataMap`.
    pub clips: Vec<GraphClip>,
    /// `mTrackDataMap`.
    pub tracks: Vec<Track>,
    /// `mMaskDataMap`.
    pub masks: Vec<Mask>,
    /// `mSyncGroupDataMap`.
    pub sync_groups: Vec<SyncGroup>,
}

/// One entry of `mClipDataMap`, of any kind of `ClipBaseData`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct GraphClip {
    /// The clip's key as the tables name it, and its hash where none does.
    pub name: String,
    /// The key, `0x` and eight hex digits.
    pub hash: String,
    /// The clip's class as the tables name it, and its hash where none does.
    pub class: String,
    /// `mAnimationResourceData.mAnimationFilePath`, which an atomic clip alone names.
    pub animation: Option<NamedAsset>,
    /// `mTrackDataName`.
    pub track: Option<KeyRef>,
    /// `mMaskDataName`.
    pub mask: Option<KeyRef>,
    /// `mSyncGroupDataName`.
    pub sync_group: Option<KeyRef>,
    /// `mTickDuration`, seconds per tick, which an atomic clip alone sets.
    pub tick_duration: Option<f32>,
    /// `mEventDataMap`, in map order.
    pub events: Vec<ClipEvent>,
    /// The clips this one plays, in its kind's field order.
    pub children: Vec<KeyRef>,
    /// `mValue` of each pair of a parametric clip, one per child in the same order, and
    /// empty for every other kind.
    pub parameters: Vec<f32>,
    /// `mAnimationInterruptionGroupNames`.
    pub interruption_groups: Vec<String>,
    /// `mFlags`.
    pub flags: u32,
    /// The clip fires the events of its own `mEventDataMap` while it plays other clips: a
    /// sequencer clip, and a parametric clip whose own-events flag is set.
    pub own_events: bool,
}

/// One entry of `mEventDataMap`, of any kind of `BaseEventData`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct ClipEvent {
    /// The event's key as the tables name it, and its hash where none does.
    pub name: String,
    /// The key, `0x` and eight hex digits.
    pub hash: String,
    /// The event's class as the tables name it, and its hash where none does.
    pub class: String,
    /// `mStartFrame`, the frame of the clip the event fires on.
    pub start_frame: f32,
    /// `mEndFrame`, and none for an event that ends on its own, which the meta writes as -1.
    pub end_frame: Option<f32>,
    /// What the event does, for the kinds a viewport plays.
    pub kind: EventKind,
}

/// What a clip event does, for the kinds a viewport plays, and nothing for the rest.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub enum EventKind {
    /// `SubmeshVisibilityEventData`: submeshes shown and hidden from the start frame on.
    SubmeshVisibility {
        /// `mShowSubmeshList`.
        show: Vec<HashRef>,
        /// `mHideSubmeshList`.
        hide: Vec<HashRef>,
    },
    /// `ParticleEventData`: a system spawned on a joint at the start frame.
    #[serde(rename_all = "camelCase")]
    Particle {
        /// `mEffectKey`, `0x` and eight hex digits, which the skin's resolver maps.
        effect_key: String,
        /// `mEffectName`, what the author called it.
        effect_name: String,
        /// `mParticleEventDataPairList`, one spawn per pair.
        spawns: Vec<EventSpawn>,
        /// `mIsLoop`.
        is_loop: bool,
        /// `mIsKillEvent`, which stops the effect of the key rather than spawning one.
        is_kill: bool,
        /// `scale`, which the meta defaults to one.
        scale: f32,
    },
    /// `JointSnapEventData`: one joint stands where another does, from the start frame on.
    #[serde(rename_all = "camelCase")]
    JointSnap {
        /// `mJointNameToOverride`, the joint moved, and none for an event naming no joint.
        joint: Option<HashRef>,
        /// `mJointNameToSnapTo`, the joint it snaps to, and none for an event naming no joint.
        snap_to: Option<HashRef>,
        /// `offset`, in the frame of the joint it snaps to.
        offset: [f32; 3],
    },
    /// `ConformToPathEventData`: the joints a mask weighs follow the unit's path over the span.
    #[serde(rename_all = "camelCase")]
    ConformToPath {
        /// `mMaskDataName`, the joints that conform, and none for an event naming no mask.
        mask: Option<KeyRef>,
        /// `mBlendInTime`, seconds the conforming eases in over.
        blend_in: f32,
        /// `mBlendOutTime`, seconds it eases out over.
        blend_out: f32,
    },
    /// `DynamicsChainBlendEventData`: every dynamics chain leaves its default state over the span.
    #[serde(rename_all = "camelCase")]
    DynamicsChainBlend {
        /// `BlendFromDefaultDuration`, seconds the change takes at the start frame.
        blend_from_default: f32,
        /// `BlendToDefaultDuration`, seconds the change back takes at the end frame.
        blend_to_default: f32,
    },
    /// `SpringPhysicsEventData`: a spring is turned off over the span.
    #[serde(rename_all = "camelCase")]
    SpringPhysics {
        /// `SpringToAffect`, the spring's `name`, and none for every spring of the skin.
        spring: Option<HashRef>,
        /// `BlendOutTime`, seconds.
        blend_out: f32,
    },
    /// `JointOrientationEventData`: every joint orientation leaves its default state over
    /// the span, or follows another source over it.
    #[serde(rename_all = "camelCase")]
    JointOrientation {
        /// `BlendData.BlendFromDefaultDuration`, seconds the change takes at the start frame,
        /// and none for an event with no blend, which changes no weight.
        blend_from_default: Option<f32>,
        /// `BlendData.BlendToDefaultDuration`, seconds the change back takes at the end frame.
        blend_to_default: Option<f32>,
        /// The event names a source of its own, which the modifier follows over the span.
        overrides_source: bool,
    },
    /// `LockRootOrientationEventData`: a joint keeps facing where it did while the unit turns.
    #[serde(rename_all = "camelCase")]
    LockRootOrientation {
        /// `JointName`, the joint held, and none for an event naming no joint.
        joint: Option<HashRef>,
        /// `BlendOutTime`, seconds the joint takes to follow the unit again.
        blend_out: f32,
    },
    /// Any other kind, which the viewport draws nothing for.
    Other,
}

/// One pair of a particle event: the joint the system rides, and the joint it aims at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct EventSpawn {
    /// `mBoneName`, and none for a pair riding the skeleton's own origin.
    pub bone: Option<HashRef>,
    /// `mTargetBoneName`, and none for a pair aiming at nothing.
    pub target_bone: Option<HashRef>,
}

/// A hash a bin names something outside the graph by, such as a submesh or a joint.
///
/// The tables name a few of them. A viewport matches the hash against the names the `.skn`
/// or the `.skl` spells, which is how the engine reaches them too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct HashRef {
    /// The hash as the tables name it, and its hex where none does.
    pub name: String,
    /// The hash, `0x` and eight hex digits.
    pub hash: String,
}

/// A key one clip names into a map of the graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct KeyRef {
    /// The key as the tables name it, and its hash where none does.
    pub name: String,
    /// The key, `0x` and eight hex digits.
    pub hash: String,
    /// The map holds an entry under the key.
    pub declared: bool,
}

/// One entry of `mTrackDataMap`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct Track {
    pub name: String,
    /// The key, `0x` and eight hex digits.
    pub hash: String,
    /// `mPriority`.
    pub priority: u8,
    /// `mBlendMode`.
    pub blend_mode: u8,
    /// `mBlendWeight`.
    pub blend_weight: f32,
}

/// One entry of `mMaskDataMap`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct Mask {
    pub name: String,
    /// The key, `0x` and eight hex digits.
    pub hash: String,
    /// `mId`.
    pub id: u32,
    /// `mWeightList`, one weight per joint of the skeleton in the skeleton's order.
    pub weights: Vec<f32>,
}

/// One entry of `mSyncGroupDataMap`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct SyncGroup {
    pub name: String,
    /// The key, `0x` and eight hex digits.
    pub hash: String,
    /// `mType`.
    pub kind: u32,
}

/// The skin object at `entry`, as a viewport draws it.
///
/// A field the skin leaves out answers the meta default: no file, a scale of one, no graph
/// and no effects. `shaders` is `data/shaders/shaders.bin`, which a material's slots
/// take their defaults from, and none leaves every material on its own fields.
///
/// # Errors
///
/// Fails with [`BinDocumentError::NodeNotFound`] where `entry` is no object of the
/// document.
pub(crate) fn resolve_skin(
    document: &BinDocument,
    entry: BinHash,
    names: &dyn RowNames,
    assets: &dyn AssetLookup,
    shaders: Option<&BinDocument>,
) -> Result<SkinModel, BinDocumentError> {
    let skin = &object_at(document, entry)?.properties;
    let locator = Locator { names, assets };
    let mesh = fields_of(skin.get(&MESH_PROPERTIES));
    let mesh_field = |field: BinHash| mesh.and_then(|mesh| mesh.get(&field));
    let material =
        |value| link(value).map(|hash| linked_material(document, hash, &locator, shaders));
    let systems = resolver_systems(document, link(skin.get(&RESOURCE_RESOLVER)));

    Ok(SkinModel {
        mesh: locator.asset(mesh_field(SIMPLE_SKIN)),
        skeleton: locator.asset(mesh_field(SKELETON)),
        texture: locator.asset(mesh_field(TEXTURE)),
        emissive_texture: locator.asset(mesh_field(EMISSIVE_TEXTURE)),
        material: material(mesh_field(MATERIAL)),
        overrides: items(mesh_field(MATERIAL_OVERRIDE))
            .iter()
            .filter_map(|item| {
                let fields = fields_of(Some(item))?;
                let texture = locator.asset(fields.get(&TEXTURE));
                let material = material(fields.get(&MATERIAL));
                /* An override naming neither draws as no override at all. */
                if texture.is_none() && material.is_none() {
                    return None;
                }
                Some(SubmeshOverride {
                    submesh: text(fields.get(&SUBMESH))?.to_owned(),
                    texture,
                    material,
                })
            })
            .collect(),
        hidden: text(mesh_field(HIDDEN_SUBMESHES))
            .map(submesh_names)
            .unwrap_or_default(),
        scale: match leaf(mesh_field(SKIN_SCALE)) {
            Some(Leaf::F32(scale)) => scale,
            _ => 1.0,
        },
        self_illumination: float(mesh_field(SELF_ILLUMINATION)).unwrap_or(0.0),
        animation_graph: fields_of(skin.get(&ANIMATION_PROPERTIES))
            .and_then(|animation| link(animation.get(&ANIMATION_GRAPH)))
            .map(hex),
        idle_effects: effects_in(skin.get(&IDLE_EFFECTS)),
        persistent_conditions: persistent_conditions(skin),
        effect_systems: systems
            .iter()
            .filter(|(_, system)| document.object_at(*system).is_some())
            .map(|(key, system)| effect_system(*key, *system, names))
            .collect(),
        pose_modifiers: dynamics::pose_modifiers(mesh, &locator),
        sockets: dynamics::sockets(mesh, &locator),
    })
}

/// The animation graph at `entry`: its clips of every kind, and the maps they key into.
///
/// # Errors
///
/// Fails with [`BinDocumentError::NodeNotFound`] where `entry` is no object of the
/// document.
pub(crate) fn resolve_graph(
    document: &BinDocument,
    entry: BinHash,
    names: &dyn RowNames,
    assets: &dyn AssetLookup,
) -> Result<AnimationGraph, BinDocumentError> {
    let graph = &object_at(document, entry)?.properties;
    let keys = GraphKeys::of(graph, names, assets);
    let named = |hash: BinHash| keys.named(hash);

    let tracks: Vec<Track> = map_entries(graph.get(&TRACK_DATA_MAP))
        .map(|(hash, fields)| Track {
            name: named(hash),
            hash: hex(hash),
            priority: u8_of(fields.get(&TRACK_PRIORITY)),
            blend_mode: u8_of(fields.get(&TRACK_BLEND_MODE)),
            blend_weight: match leaf(fields.get(&TRACK_BLEND_WEIGHT)) {
                Some(Leaf::F32(weight)) => weight,
                _ => 0.0,
            },
        })
        .collect();
    let masks: Vec<Mask> = map_entries(graph.get(&MASK_DATA_MAP))
        .map(|(hash, fields)| Mask {
            name: named(hash),
            hash: hex(hash),
            id: u32_of(fields.get(&MASK_ID)),
            weights: items(fields.get(&MASK_WEIGHTS))
                .iter()
                .map(|item| match leaf(Some(item)) {
                    Some(Leaf::F32(weight)) => weight,
                    _ => 0.0,
                })
                .collect(),
        })
        .collect();
    let sync_groups: Vec<SyncGroup> = map_entries(graph.get(&SYNC_GROUP_DATA_MAP))
        .map(|(hash, fields)| SyncGroup {
            name: named(hash),
            hash: hex(hash),
            kind: u32_of(fields.get(&SYNC_GROUP_TYPE)),
        })
        .collect();

    let clips = struct_entries(graph.get(&CLIP_DATA_MAP))
        .map(|(hash, class, fields)| GraphClip {
            name: named(hash),
            hash: hex(hash),
            class: keys.class(class),
            animation: fields_of(fields.get(&ANIMATION_RESOURCE))
                .and_then(|resource| keys.locator.asset(resource.get(&ANIMATION_FILE))),
            track: keys.keyed(fields.get(&TRACK_DATA_NAME), GraphMap::Tracks),
            mask: keys.keyed(fields.get(&MASK_DATA_NAME), GraphMap::Masks),
            sync_group: keys.keyed(fields.get(&SYNC_GROUP_DATA_NAME), GraphMap::SyncGroups),
            tick_duration: match leaf(fields.get(&TICK_DURATION)) {
                Some(Leaf::F32(seconds)) => Some(seconds),
                _ => None,
            },
            events: struct_entries(fields.get(&EVENT_DATA_MAP))
                .map(|(hash, class, fields)| ClipEvent {
                    name: named(hash),
                    hash: hex(hash),
                    class: keys.class(class),
                    start_frame: float(fields.get(&EVENT_START_FRAME)).unwrap_or(0.0),
                    end_frame: Some(float(fields.get(&EVENT_END_FRAME)).unwrap_or(-1.0))
                        .filter(|frame| *frame >= 0.0),
                    kind: event_kind(class, fields, &keys),
                })
                .collect(),
            children: children_of(fields)
                .map(|child| keys.key_ref(child, GraphMap::Clips))
                .collect(),
            parameters: parameters_of(fields),
            interruption_groups: items(fields.get(&INTERRUPTION_GROUPS))
                .iter()
                .filter_map(|item| match leaf(Some(item)) {
                    Some(Leaf::Hash(hash)) => hash.try_as_bin_hash().map(named),
                    _ => None,
                })
                .collect(),
            flags: u32_of(fields.get(&FLAGS)),
            own_events: match class {
                PARAMETRIC_CLIP => boolean(fields.get(&CLIP_OWN_EVENTS)).unwrap_or(false),
                SEQUENCER_CLIP => true,
                _ => false,
            },
        })
        .collect();

    Ok(AnimationGraph {
        clips,
        tracks,
        masks,
        sync_groups,
    })
}

/// The maps of a graph a key names an entry of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphMap {
    Clips,
    Tracks,
    Masks,
    SyncGroups,
}

/// How a graph's hashes are named, and which keys each of its maps declares.
///
/// Every key and hash a graph carries is written through this, so a clip's key, a track
/// it names and a joint an event names take one spelling.
struct GraphKeys<'a> {
    locator: Locator<'a>,
    clips: HashSet<BinHash>,
    tracks: HashSet<BinHash>,
    masks: HashSet<BinHash>,
    sync_groups: HashSet<BinHash>,
}

impl<'a> GraphKeys<'a> {
    /// The keys `graph` declares under each of its maps, named through `names`.
    fn of(graph: &Fields, names: &'a dyn RowNames, assets: &'a dyn AssetLookup) -> Self {
        let keys_of = |field: BinHash| -> HashSet<BinHash> {
            map_entries(graph.get(&field))
                .map(|(hash, _)| hash)
                .collect()
        };
        Self {
            locator: Locator { names, assets },
            clips: keys_of(CLIP_DATA_MAP),
            tracks: keys_of(TRACK_DATA_MAP),
            masks: keys_of(MASK_DATA_MAP),
            sync_groups: keys_of(SYNC_GROUP_DATA_MAP),
        }
    }

    /// A hash as the tables name it, and its hex where none does.
    fn named(&self, hash: BinHash) -> String {
        self.locator
            .names
            .value_name(hash)
            .unwrap_or_else(|| hex(hash))
    }

    /// A class as the tables name it, and its hex where none does.
    fn class(&self, hash: BinHash) -> String {
        class_name(hash, &self.locator)
    }

    /// A `Hash` field naming something outside the graph, such as a submesh or a joint, and
    /// none for a zero or absent one.
    fn hash_at(&self, value: Option<&PropertyValueEnum>) -> Option<HashRef> {
        hash_at(value, &self.locator)
    }

    /// A key into `map`, marked for whether the map declares it.
    fn key_ref(&self, hash: BinHash, map: GraphMap) -> KeyRef {
        let keys = match map {
            GraphMap::Clips => &self.clips,
            GraphMap::Tracks => &self.tracks,
            GraphMap::Masks => &self.masks,
            GraphMap::SyncGroups => &self.sync_groups,
        };
        KeyRef {
            name: self.named(hash),
            hash: hex(hash),
            declared: keys.contains(&hash),
        }
    }

    /// A `Hash` field keying into `map`, and none for a zero or absent one.
    fn keyed(&self, value: Option<&PropertyValueEnum>, map: GraphMap) -> Option<KeyRef> {
        match leaf(value) {
            Some(Leaf::Hash(hash)) => hash
                .try_as_bin_hash()
                .filter(|hash| hash.0 != 0)
                .map(|hash| self.key_ref(hash, map)),
            _ => None,
        }
    }
}

/// A `Hash` field as the tables name it, and none for a zero or absent one.
fn hash_at(value: Option<&PropertyValueEnum>, locator: &Locator) -> Option<HashRef> {
    match leaf(value) {
        Some(Leaf::Hash(hash)) => {
            let hash = hash.try_as_bin_hash().filter(|hash| hash.0 != 0)?;

            Some(HashRef {
                name: locator.names.value_name(hash).unwrap_or_else(|| hex(hash)),
                hash: hex(hash),
            })
        }
        _ => None,
    }
}

/// A class as the tables name it, and its hex where none does.
fn class_name(class: BinHash, locator: &Locator) -> String {
    locator
        .names
        .class_name(class)
        .unwrap_or_else(|| hex(class))
}

/// The entries of a `Map<Hash, Struct>`, each as its key and its fields.
///
/// An entry keyed by anything but a hash, or holding no struct, is passed over.
fn map_entries(value: Option<&PropertyValueEnum>) -> impl Iterator<Item = (BinHash, &Fields)> + '_ {
    struct_entries(value).map(|(hash, _, fields)| (hash, fields))
}

/// The clips `fields` names as children, through every field of [`CHILD_FIELDS`] it holds.
fn children_of(fields: &Fields) -> impl Iterator<Item = BinHash> + '_ {
    let hash_of = |value: Option<&PropertyValueEnum>| match leaf(value) {
        Some(Leaf::Hash(hash)) => hash.try_as_bin_hash().filter(|hash| hash.0 != 0),
        _ => None,
    };
    CHILD_FIELDS.iter().flat_map(move |children| {
        let named: Vec<BinHash> = match *children {
            Children::One(field) => hash_of(fields.get(&field)).into_iter().collect(),
            Children::Many(field) => items(fields.get(&field))
                .iter()
                .filter_map(|item| hash_of(Some(item)))
                .collect(),
            Children::Pairs(field, clip) => items(fields.get(&field))
                .iter()
                .filter_map(|item| hash_of(fields_of(Some(item))?.get(&clip)))
                .collect(),
        };
        named
    })
}

/// The parameter each pair of a parametric clip plays at, for the pairs `children_of` keeps.
///
/// A pair naming no clip is passed over as `children_of` passes it over, so the two lists
/// line up. A pair with no `mValue` plays at zero.
fn parameters_of(fields: &Fields) -> Vec<f32> {
    items(fields.get(&PARAMETRIC_PAIRS))
        .iter()
        .filter_map(|item| {
            let pair = fields_of(Some(item))?;
            match leaf(pair.get(&PAIR_CLIP)) {
                Some(Leaf::Hash(hash))
                    if hash.try_as_bin_hash().is_some_and(|hash| hash.0 != 0) => {}
                _ => return None,
            }
            Some(match leaf(pair.get(&PAIR_VALUE)) {
                Some(Leaf::F32(value)) => value,
                _ => 0.0,
            })
        })
        .collect()
}

/// What an event of `class` does, read off `fields`, and nothing for a kind no viewport plays.
fn event_kind(class: BinHash, fields: &Fields, keys: &GraphKeys) -> EventKind {
    let hash_refs = |value: Option<&PropertyValueEnum>| -> Vec<HashRef> {
        items(value)
            .iter()
            .filter_map(|item| keys.hash_at(Some(item)))
            .collect()
    };

    match class {
        SUBMESH_VISIBILITY_EVENT => EventKind::SubmeshVisibility {
            show: hash_refs(fields.get(&EVENT_SHOW_SUBMESHES)),
            hide: hash_refs(fields.get(&EVENT_HIDE_SUBMESHES)),
        },
        PARTICLE_EVENT => EventKind::Particle {
            effect_key: hash_hex(match leaf(fields.get(&EVENT_EFFECT_KEY)) {
                Some(Leaf::Hash(key)) => key,
                _ => BinHash(0).into(),
            }),
            effect_name: text(fields.get(&EVENT_EFFECT_NAME))
                .unwrap_or_default()
                .to_owned(),
            spawns: items(fields.get(&EVENT_PAIRS))
                .iter()
                .filter_map(|item| {
                    let pair = fields_of(Some(item))?;
                    Some(EventSpawn {
                        bone: keys.hash_at(pair.get(&EVENT_BONE)),
                        target_bone: keys.hash_at(pair.get(&EVENT_TARGET_BONE)),
                    })
                })
                .collect(),
            is_loop: boolean(fields.get(&EVENT_IS_LOOP)).unwrap_or(false),
            is_kill: boolean(fields.get(&EVENT_IS_KILL)).unwrap_or(false),
            scale: float(fields.get(&EVENT_SCALE)).unwrap_or(1.0),
        },
        JOINT_SNAP_EVENT => EventKind::JointSnap {
            joint: keys.hash_at(fields.get(&EVENT_JOINT)),
            snap_to: keys.hash_at(fields.get(&EVENT_SNAP_TO)),
            offset: match leaf(fields.get(&EVENT_OFFSET)) {
                Some(Leaf::Vector3(offset)) => offset.to_array(),
                _ => [0.0; 3],
            },
        },
        CONFORM_EVENT => EventKind::ConformToPath {
            mask: keys.keyed(fields.get(&MASK_DATA_NAME), GraphMap::Masks),
            blend_in: float(fields.get(&EVENT_BLEND_IN)).unwrap_or(0.0),
            blend_out: float(fields.get(&EVENT_BLEND_OUT)).unwrap_or(0.0),
        },
        DYNAMICS_CHAIN_BLEND_EVENT => EventKind::DynamicsChainBlend {
            blend_from_default: float(fields.get(&EVENT_BLEND_FROM_DEFAULT)).unwrap_or(0.0),
            blend_to_default: float(fields.get(&EVENT_BLEND_TO_DEFAULT)).unwrap_or(0.0),
        },
        SPRING_PHYSICS_EVENT => EventKind::SpringPhysics {
            spring: keys.hash_at(fields.get(&EVENT_SPRING)),
            blend_out: float(fields.get(&EVENT_BLEND_OUT_TIME)).unwrap_or(0.0),
        },
        JOINT_ORIENTATION_EVENT => {
            let blend = fields_of(fields.get(&EVENT_BLEND_DATA));
            let seconds =
                |field: BinHash| blend.map(|blend| float(blend.get(&field)).unwrap_or(0.0));

            EventKind::JointOrientation {
                blend_from_default: seconds(EVENT_BLEND_FROM_DEFAULT),
                blend_to_default: seconds(EVENT_BLEND_TO_DEFAULT),
                overrides_source: fields_of(fields.get(&EVENT_SOURCE_OVERRIDE)).is_some(),
            }
        }
        LOCK_ROOT_ORIENTATION_EVENT => EventKind::LockRootOrientation {
            joint: keys.hash_at(fields.get(&EVENT_JOINT_NAME)),
            blend_out: float(fields.get(&EVENT_BLEND_OUT_TIME)).unwrap_or(0.2),
        },
        _ => EventKind::Other,
    }
}

/// A `U8` field's value, and zero for one the struct leaves out.
fn u8_of(value: Option<&PropertyValueEnum>) -> u8 {
    match leaf(value) {
        Some(Leaf::U8(byte)) => byte,
        _ => 0,
    }
}

/// A `U32` field's value, and zero for one the struct leaves out.
fn u32_of(value: Option<&PropertyValueEnum>) -> u32 {
    match leaf(value) {
        Some(Leaf::U32(word)) => word,
        _ => 0,
    }
}

impl ObjectLookup<'_> {
    /// Resolves the skin object at `entry` of the open bin, with its materials, its resolver and
    /// its effect systems read from the first bin of the lookup that declares each.
    ///
    /// A skin keeps its particle systems and its resolver in the bins it links as often as in
    /// its own. A material or a system that no bin of the lookup declares is skipped: the
    /// material stays missing, and the key is left out of [`SkinModel::effect_systems`].
    ///
    /// # Errors
    ///
    /// Fails with [`BinDocumentError::NodeNotFound`] if `entry` is no object of the open bin.
    pub fn skin(&mut self, entry: BinHash) -> Result<SkinModel, BinDocumentError> {
        let open = self.open();
        let mut model = resolve_skin(open, entry, self.names(), self.assets(), self.shaders())?;

        let missing: Vec<&mut MaterialPreview> = model
            .material
            .iter_mut()
            .chain(
                model
                    .overrides
                    .iter_mut()
                    .filter_map(|o| o.material.as_mut()),
            )
            .collect();
        find_linked_materials(missing, self);

        let resolver = object_at(open, entry)
            .ok()
            .and_then(|skin| link(skin.properties.get(&RESOURCE_RESOLVER)));
        let mapped = match resolver.and_then(|resolver| self.find(resolver)) {
            Some(found) => resolver_systems(self.document(found), resolver),
            None => Vec::new(),
        };
        let names = self.names();
        model.effect_systems = mapped
            .into_iter()
            .filter(|(_, system)| self.find(*system).is_some())
            .map(|(key, system)| effect_system(key, system, names))
            .collect();

        Ok(model)
    }

    /// Resolves the animation graph at `entry` from the first bin of the lookup that declares
    /// it.
    ///
    /// A skin's graph is usually declared in the animations bin that its own file links, which
    /// is where the engine resolves it from too.
    ///
    /// # Errors
    ///
    /// As `resolve_graph`, against the bin that declares `entry`. Fails with
    /// [`BinDocumentError::NodeNotFound`] if no bin of the lookup declares it.
    pub fn graph(&mut self, entry: BinHash) -> Result<AnimationGraph, BinDocumentError> {
        let found = self.find(entry);
        let document = found.map_or(self.open(), |found| self.document(found));
        resolve_graph(document, entry, self.names(), self.assets())
    }
}

/// The submesh names `initialSubmeshToHide` lists, apart on spaces and commas.
fn submesh_names(list: &str) -> Vec<String> {
    list.split(|character: char| character.is_whitespace() || character == ',')
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests;
