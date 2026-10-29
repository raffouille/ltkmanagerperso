import {
  AdditiveBlending,
  type Blending as ThreeBlending,
  CustomBlending,
  BackSide,
  DoubleSide,
  FrontSide,
  type Material,
  type MeshBasicMaterial,
  type MeshLambertMaterial,
  type MeshStandardMaterial,
  NormalBlending,
  OneMinusSrcAlphaFactor,
  OneMinusSrcColorFactor,
  type Side,
  SrcAlphaFactor,
  ZeroFactor,
} from "three";

import type { Blending, RenderState } from "@/lib/tauri";

/** The stock material of each shading model, all of which take the same slots. */
export type SubmeshMaterial = MeshBasicMaterial | MeshLambertMaterial | MeshStandardMaterial;

/**
 * The blend mode each class reaches three.js as.
 *
 * A record rather than a chain of comparisons, so a class added to `Blending` is a type
 * error here rather than a pass that silently draws as `NormalBlending`.
 */
const BLENDING: Record<Blending, ThreeBlending> = {
  opaque: NormalBlending,
  normal: NormalBlending,
  additive: AdditiveBlending,
  modulate: CustomBlending,
};

/** The program parameters a binding can move, so a change of them compiles again. */
const PROGRAMS = new WeakMap<Material, string>();

/**
 * `material` set to draw the way `state` says the pass does.
 *
 * Every field is written on every call, because a stock material is reused between
 * bindings and a field left alone keeps what the last one put there.
 */
export function applyRenderState(material: SubmeshMaterial, state: RenderState): void {
  /* A cutout still blends the fringe its filtered alpha leaves above the threshold, as the
     game's pass does, and drawn opaque that fringe is the dark edge of a black transparent
     texel. It writes depth, so a missorted neighbour costs a fringe and one pass per side
     is enough. */
  material.transparent = state.blending !== "opaque";
  material.forceSinglePass = state.cutout;
  material.blending = BLENDING[state.blending];
  /* Read only under CustomBlending, and set unconditionally so a material that stops
     modulating stops carrying the factors of the one that did. */
  material.blendSrc = state.blending === "modulate" ? OneMinusSrcColorFactor : SrcAlphaFactor;
  material.blendDst = state.blending === "modulate" ? ZeroFactor : OneMinusSrcAlphaFactor;
  material.premultipliedAlpha = state.premultiplied;
  material.side = sideOf(state);
  material.depthWrite = state.depthWrite;
  material.depthTest = state.depthTest;
}

/**
 * The face a pass keeps.
 *
 * Three flips its front face under the mirrored axis of world.ts on its own, so the
 * engine's default winding is `FrontSide` here. Unjudged on screen against the game.
 */
export function sideOf(state: RenderState): Side {
  if (state.doubleSided) return DoubleSide;
  return state.inverted ? BackSide : FrontSide;
}

/** Flag the program stale where a parameter it was compiled on has moved. */
export function recompileIfMoved(material: SubmeshMaterial): void {
  const key = [
    material.map !== null,
    material.alphaTest > 0,
    material.premultipliedAlpha,
    material.side,
  ].join(":");
  if (PROGRAMS.get(material) === key) return;
  PROGRAMS.set(material, key);
  material.needsUpdate = true;
}
