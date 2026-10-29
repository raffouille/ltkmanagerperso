//! What an outline material gives the inking that a viewport draws around a skin's submeshes.
//!
//! The parameters are those of `Shaders/SkinnedMesh/Jade_CharacterOutline`, the shader that
//! every shipped outline material links.

use ltk_hash::BinHash;
use serde::Serialize;

use ltk_manager_bin::bin_document::{BinDocument, Locator, NamedAsset, hex};

use super::Reader;

/// `Outline_Color`.
const COLOR: &str = "Outline_Color";
/// `Outline_Width`.
const WIDTH: &str = "Outline_Width";
/// `Outline_CameraDistanceMod`.
const CAMERA_DISTANCE: &str = "Outline_CameraDistanceMod";
/// `Outline_CameraDepthOffset`.
const DEPTH_OFFSET: &str = "Outline_CameraDepthOffset";
/// `Outline_FinalAlphaMult`.
const ALPHA: &str = "Outline_FinalAlphaMult";
/// `Outline_Mask`, the texture.
const MASK: &str = "Outline_Mask";
/// `Outline_Mask_ChannelMask`.
const MASK_CHANNELS: &str = "Outline_Mask_ChannelMask";

/// The defaults that the shipped `Jade_CharacterOutline` def declares, for a read without the
/// shader defs.
const DEFAULT_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const DEFAULT_WIDTH: f32 = 0.25;
const DEFAULT_CAMERA_DISTANCE: f32 = 600.0;
const DEFAULT_DEPTH_OFFSET: f32 = 20.0;
const DEFAULT_ALPHA: f32 = 1.0;
const DEFAULT_MASK_CHANNELS: [f32; 4] = [1.0, 0.0, 0.0, 0.0];

/// An outline material, as a viewport draws the outline without the game's shader.
///
/// The shader moves each vertex of the submesh `depth_offset` units toward the camera and
/// `width * distance / camera_distance` units along its normal, with culling off. A negative
/// offset puts the shell behind the submesh, so the shell shows at the silhouette alone.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(specta::Type))]
pub struct OutlineMaterial {
    /// The material's path hash, `0x` and eight hex digits.
    pub hash: String,
    /// The material's path, where a table names it.
    pub name: Option<String>,
    /// `Outline_Color`. The shader does not read its alpha.
    pub color: [f32; 4],
    /// `Outline_Width`, in world units at `camera_distance` from the camera.
    pub width: f32,
    /// `Outline_CameraDistanceMod`, the camera distance at which the outline is `width` wide.
    pub camera_distance: f32,
    /// `Outline_CameraDepthOffset`, in world units toward the camera.
    pub depth_offset: f32,
    /// `Outline_FinalAlphaMult`, which scales the masked alpha.
    pub alpha: f32,
    /// `Outline_Mask`, and none if neither the material nor the shader names one.
    pub mask: Option<NamedAsset>,
    /// `Outline_Mask_ChannelMask`, the weight of each channel of the mask in the alpha.
    pub mask_channels: [f32; 4],
}

/// Reads the outline material at `hash` of `document`. Returns `None` if `document` declares
/// no object under `hash`.
///
/// A parameter that the material leaves out takes the shader def's default, and the shipped
/// default if `shaders` is none.
pub(crate) fn linked_outline(
    document: &BinDocument,
    hash: BinHash,
    locator: &Locator<'_>,
    shaders: Option<&BinDocument>,
) -> Option<OutlineMaterial> {
    let object = document.object_at(hash)?;
    let mut reader = Reader::new(hash, &object.properties, locator, shaders);
    let pass = reader.first_pass();
    let shader = reader.shader_def(pass);
    let samplers = reader.samplers(&shader);
    let params = reader.params(pass, &shader);

    let vector = |name: &str, default: [f32; 4]| params.first_of(&[name]).unwrap_or(default);
    let scalar = |name: &str, default: f32| vector(name, [default, 0.0, 0.0, 0.0])[0];

    Some(OutlineMaterial {
        hash: hex(hash),
        name: locator.names.entry_name(hash),
        color: vector(COLOR, DEFAULT_COLOR),
        width: scalar(WIDTH, DEFAULT_WIDTH),
        camera_distance: scalar(CAMERA_DISTANCE, DEFAULT_CAMERA_DISTANCE),
        depth_offset: scalar(DEPTH_OFFSET, DEFAULT_DEPTH_OFFSET),
        alpha: scalar(ALPHA, DEFAULT_ALPHA),
        mask: samplers
            .get(MASK)
            .and_then(|sampler| sampler.texture.clone()),
        mask_channels: vector(MASK_CHANNELS, DEFAULT_MASK_CHANNELS),
    })
}
