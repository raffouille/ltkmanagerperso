import {
  ClampToEdgeWrapping,
  type Color,
  MirroredRepeatWrapping,
  RepeatWrapping,
  SRGBColorSpace,
  type Texture,
  type Wrapping,
} from "three";

import type { MaterialPreview, RenderState, Wrap } from "@/lib/tauri";

import {
  applyRenderState,
  recompileIfMoved,
  type SubmeshMaterial,
} from "../../shared/utils/renderState";

/**
 * What one submesh is drawn with, in the engine's order of choosing it.
 *
 * Section 1.3 of docs/research/static-material-studio-rendering.md.
 */
export interface SubmeshBinding<T = Texture> {
  /** The material's slots, and null for a submesh drawn with a texture alone. */
  readonly material: MaterialPreview | null;
  /** The texture the material's base slot names, and null for none or one not held. */
  readonly base: T | null;
  /** The submesh's own skin texture, drawn where no material names a base. */
  readonly texture: T | null;
  /** The maps that draw a submesh with no material physically based, and null for none. */
  readonly pbr: PbrMaps<T> | null;
  /**
   * The skin's `emissiveTexture` for a submesh with no material, read without colour
   * decoding, and null for none, one not held or a submesh with a material. Its green channel
   * blends the lit colour toward the unlit albedo.
   */
  readonly emissive: T | null;
}

/**
 * The `LIT_UBER` maps of a submesh with no material, read without colour decoding.
 *
 * "RoughnessMetallicAoTexture" at https://meta-wiki.leaguetoolkit.dev/classes/skinmeshdataproperties.
 */
export interface PbrMaps<T = Texture> {
  /** `RoughnessMetallicAoTexture`: roughness in red, metalness in green, occlusion in blue. */
  readonly rma: T;
  /** `normalMapTexture`, and null for none or one not held. */
  readonly normal: T | null;
}

/** The colours a bound submesh falls back on. */
export interface FallbackColors {
  /** What a submesh no texture reaches is drawn in. */
  readonly untextured: Color;
  /** What a submesh whose material link resolves to nothing is drawn in. */
  readonly errored: Color;
}

/** Three has no border wrap, and a clamp is the nearest edge a border sits on. */
const WRAPPING: Record<Wrap, Wrapping> = {
  repeat: RepeatWrapping,
  clamp: ClampToEdgeWrapping,
  mirror: MirroredRepeatWrapping,
  border: ClampToEdgeWrapping,
};

export type { SubmeshMaterial };

/** What a submesh no usable material reaches draws as: flat, both faces, depth on. */
const UNBOUND: RenderState = {
  blending: "opaque",
  srcFactor: "one",
  dstFactor: "zero",
  premultiplied: false,
  cutout: false,
  doubleSided: true,
  inverted: false,
  depthWrite: true,
  depthTest: true,
};

/**
 * Whether the binding's shading model is the lit one.
 *
 * The game lights a body and blends a VFX layer unlit, and an additive pass is the one
 * sure sign of the latter. A missing material draws flat so its colour reads as a flag.
 */
export function lit({ material }: SubmeshBinding): boolean {
  if (material === null) return true;
  return !material.missing && material.renderState.blending !== "additive";
}

/**
 * `material` set to what `binding` names, and the scroll its map moves at per second.
 *
 * The slots land on a stock material as "10.5 Three.js" of the studio doc writes them.
 * A tint is set in sRGB, because the engine multiplies it onto the encoded texel and
 * the decode-multiply-encode round trip lands the same place.
 */
export function applyBinding(
  material: SubmeshMaterial,
  { material: slots, base, texture }: SubmeshBinding,
  colors: FallbackColors,
): readonly [number, number] | null {
  const map = mapOf(slots, base, texture);
  if (material.map !== map) material.map = map;

  if (slots === null || slots.missing) {
    material.opacity = 1;
    material.alphaTest = 0;
    applyRenderState(material, UNBOUND);
    if (slots !== null) material.color.copy(colors.errored);
    else if (map === null) material.color.copy(colors.untextured);
    else material.color.setRGB(1, 1, 1);
    recompileIfMoved(material);
    return null;
  }

  if (slots.tint !== null) {
    const [r, g, b] = slots.tint;
    material.color.setRGB(r ?? 1, g ?? 1, b ?? 1, SRGBColorSpace);
  } else if (map === null) material.color.copy(colors.untextured);
  else material.color.setRGB(1, 1, 1);
  material.opacity = slots.opacity ?? 1;
  material.alphaTest = slots.alphaTest ?? 0;
  applyRenderState(material, slots.renderState);
  if (base !== null && map === base) tile(base, slots);
  recompileIfMoved(material);
  return slots.uvScroll === null ? null : [slots.uvScroll[0] ?? 0, slots.uvScroll[1] ?? 0];
}

/**
 * The map a submesh draws: the material's base, or its own texture where an opaque
 * material names none, per "6.3 Defaults when a sampler is missing" of the studio note.
 */
function mapOf(
  slots: MaterialPreview | null,
  base: Texture | null,
  texture: Texture | null,
): Texture | null {
  if (slots === null) return texture;
  if (slots.missing) return null;
  if (slots.base !== null) return base;
  return slots.renderState.blending === "opaque" ? texture : null;
}

/** The base's wrap and repeat, which are the material's own and so safe to set on it. */
function tile(base: Texture, slots: MaterialPreview): void {
  const [u, v] = slots.uvRepeat ?? [1, 1];
  base.repeat.set(u ?? 1, v ?? 1);
  const wrapS =
    slots.uvRepeat === null ? WRAPPING[slots.base?.wrap[0] ?? "repeat"] : RepeatWrapping;
  const wrapT =
    slots.uvRepeat === null ? WRAPPING[slots.base?.wrap[1] ?? "repeat"] : RepeatWrapping;
  if (base.wrapS !== wrapS || base.wrapT !== wrapT) {
    base.wrapS = wrapS;
    base.wrapT = wrapT;
    /* A wrap mode is set at upload, so a change of one uploads again. */
    base.needsUpdate = true;
  }
}
