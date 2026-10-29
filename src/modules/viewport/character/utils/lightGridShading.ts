import {
  DoubleSide,
  type Material,
  MeshLambertMaterial,
  MeshStandardMaterial,
  type Texture,
  type WebGLProgramParametersWithUniforms,
} from "three";

import { CUBE_FACES, type LightGrid, sceneCubeAt } from "../../assets/parsing/lightGridBuffer";
import { blackTexel } from "../../hexshade/neutralTextures";

/**
 * The uniforms a lit character material reads its map's baked ambient from.
 *
 * `cube` is in the scene's space. Its +X and -X faces are the engine's, swapped across
 * the mirrored axis. `on` is 0 where no grid lights the character, and ThreeJS's lights
 * shade it there. `selfIllumination` is the skin's, which `scale.y` weighs.
 */
export interface LightGridUniforms {
  readonly cube: { value: Float32Array };
  readonly scale: { value: [number, number] };
  readonly selfIllumination: { value: number };
  readonly on: { value: number };
}

/** The key every patched material shares. All of them draw with one program. */
const PROGRAM_KEY = "light-grid-3";

/** The key every patched physically based material shares. */
const PBR_PROGRAM_KEY = "light-grid-pbr-2";

const VERTEX_DECLARATIONS = /* glsl */ `
uniform vec3 lightGridCube[${CUBE_FACES}];
varying vec3 vLightGrid;
`;

/* The ambient cube as the game's character vertex shader weighs it: the squared world
   normal, each axis taking the face its sign points at. The bones are world space there,
   and here the model matrix is a mirror and a uniform scale, so it turns a normal true. */
const VERTEX_AMBIENT = /* glsl */ `
vec3 gridNormal = normalize( mat3( modelMatrix ) * objectNormal );
vec3 gridWeight = gridNormal * gridNormal;
vLightGrid = gridWeight.x * lightGridCube[ gridNormal.x < 0.0 ? 1 : 0 ]
  + gridWeight.y * lightGridCube[ gridNormal.y < 0.0 ? 3 : 2 ]
  + gridWeight.z * lightGridCube[ gridNormal.z < 0.0 ? 5 : 4 ];
`;

const FRAGMENT_DECLARATIONS = /* glsl */ `
uniform vec2 lightGridScale;
uniform float lightGridSelfIllumination;
uniform float lightGridOn;
uniform sampler2D lightGridEmissive;
varying vec3 vLightGrid;
`;

/* `EMISSIVE_MAP`'s green channel, which `LIT_UBER` samples at the colour texture's uv. */
const FRAGMENT_EMISSIVE = /* glsl */ `
float emissiveMask = 0.0;
#ifdef USE_MAP
  emissiveMask = texture2D( lightGridEmissive, vMapUv ).g;
#endif
`;

/* The game's light is `clamp(ambient * (1 - emissive) * LIGHTGRID_SCALE.x + emissive +
   SELF_ILLUMINATION * LIGHTGRID_SCALE.y)`, multiplied by a gamma albedo. The light is in
   gamma space here too, and is converted to linear before it multiplies the linear albedo.
   Without a grid the emissive mask blends three's lit colour toward the albedo. */
const FRAGMENT_LIGHT = /* glsl */ `${FRAGMENT_EMISSIVE}
if ( lightGridOn > 0.5 ) {
  vec3 gridLight = clamp(
    vLightGrid * ( 1.0 - emissiveMask ) * lightGridScale.x
      + emissiveMask
      + lightGridSelfIllumination * lightGridScale.y,
    0.0,
    1.0
  );
  outgoingLight = diffuseColor.rgb * sRGBTransferEOTF( vec4( gridLight, 1.0 ) ).rgb;
} else {
  outgoingLight = mix( outgoingLight, diffuseColor.rgb, emissiveMask );
}
`;

/* The physically based light blends toward the unlit albedo by the emissive mask, which is
   `LIT_UBER`'s base rule in linear space. Its PBR permutation samples no emissive map. */
const PBR_EMISSIVE = /* glsl */ `${FRAGMENT_EMISSIVE}
outgoingLight = mix( outgoingLight, diffuseColor.rgb, emissiveMask );
`;

/** Each patched material's emissive mask, which a binding sets apart from the shared uniforms. */
const EMISSIVE_MASKS = new WeakMap<Material, { value: Texture }>();

function emissiveMaskOf(material: Material): { value: Texture } {
  const held = EMISSIVE_MASKS.get(material);
  if (held !== undefined) return held;

  const made = { value: blackTexel() };
  EMISSIVE_MASKS.set(material, made);
  return made;
}

/**
 * Binds `mask`, an `emissiveTexture`, to a material that `gridLitMaterial` or
 * `gridPbrMaterial` made. Null binds a black texel, which leaves the light unchanged.
 */
export function applyEmissiveMask(material: Material, mask: Texture | null): void {
  emissiveMaskOf(material).value = mask ?? blackTexel();
}

/* `RoughnessMetallicAoTexture` packs roughness in red, metalness in green and occlusion in
   blue. Three reads an ORM map as occlusion, roughness and metalness, so one sample of
   `roughnessMap` replaces both of three's reads, and `metalnessMap` stays unset. */
const PBR_SURFACE = /* glsl */ `
float roughnessFactor = roughness;
float metalnessFactor = metalness;
float rmaOcclusion = 1.0;
#ifdef USE_ROUGHNESSMAP
  vec4 rmaTexel = texture2D( roughnessMap, vRoughnessMapUv );
  roughnessFactor *= rmaTexel.r;
  metalnessFactor *= rmaTexel.g;
  rmaOcclusion = rmaTexel.b;
#endif
`;

/* `LIT_UBER` reads x and y of its normal map and rebuilds z, so a two-channel map draws. */
const PBR_NORMAL = /* glsl */ `
#ifdef USE_NORMALMAP_TANGENTSPACE
  vec3 mapN = texture2D( normalMap, vNormalMapUv ).xyz * 2.0 - 1.0;
  mapN.z = sqrt( saturate( 1.0 - dot( mapN.xy, mapN.xy ) ) );
  normal = normalize( tbn * mapN );
#endif
`;

/* The ambient light is the environment a metal reflects: the grid's light where a grid holds
   one, else the hemisphere's. As image-based light it splits between diffuse and specular
   by the Fresnel term, and a dielectric reads at the grid-lit Lambert's brightness. */
const PBR_AMBIENT = /* glsl */ `
if ( lightGridOn > 0.5 ) {
  vec3 gridLight = clamp(
    vLightGrid * lightGridScale.x + lightGridSelfIllumination * lightGridScale.y,
    0.0,
    1.0
  );
  irradiance = PI * sRGBTransferEOTF( vec4( gridLight, 1.0 ) ).rgb;
}
iblIrradiance += irradiance;
radiance += irradiance * RECIPROCAL_PI;
irradiance = vec3( 0.0 );
`;

/* `LIT_UBER` multiplies its image-based light alone by the occlusion channel. */
const PBR_OCCLUSION = /* glsl */ `
reflectedLight.indirectDiffuse *= rmaOcclusion;
reflectedLight.indirectSpecular *= rmaOcclusion;
`;

/** Uniforms for one material, unlit by any grid until `lightFrom` fills them. */
export function lightGridUniforms(): LightGridUniforms {
  return {
    cube: { value: new Float32Array(CUBE_FACES * 3) },
    scale: { value: [0, 0] },
    selfIllumination: { value: 0 },
    on: { value: 0 },
  };
}

/**
 * Light `material` from the ambient cube in `uniforms`, where a grid sets one.
 *
 * The cube replaces the whole lit colour. The game's character shader reads no sun and no
 * other light under a grid.
 */
export function patchLightGrid(material: MeshLambertMaterial, uniforms: LightGridUniforms): void {
  material.onBeforeCompile = (shader: WebGLProgramParametersWithUniforms) => {
    shader.uniforms.lightGridCube = uniforms.cube;
    shader.uniforms.lightGridScale = uniforms.scale;
    shader.uniforms.lightGridSelfIllumination = uniforms.selfIllumination;
    shader.uniforms.lightGridOn = uniforms.on;
    shader.uniforms.lightGridEmissive = emissiveMaskOf(material);
    shader.vertexShader = inject(
      shader.vertexShader,
      "#include <defaultnormal_vertex>",
      VERTEX_DECLARATIONS,
      VERTEX_AMBIENT,
    );
    shader.fragmentShader = inject(
      shader.fragmentShader,
      "#include <opaque_fragment>",
      FRAGMENT_DECLARATIONS,
      FRAGMENT_LIGHT,
      true,
    );
  };
  material.customProgramCacheKey = () => PROGRAM_KEY;
}

/** A double-sided, vertex-coloured character material lit by the grid in `uniforms`. */
export function gridLitMaterial(uniforms: LightGridUniforms): MeshLambertMaterial {
  const material = new MeshLambertMaterial({ side: DoubleSide, vertexColors: true });
  patchLightGrid(material, uniforms);
  return material;
}

/**
 * A double-sided, vertex-coloured physically based character material, which reads a
 * `RoughnessMetallicAoTexture` and a two-channel normal map as `LIT_UBER` does.
 *
 * The ambient light, from the grid in `uniforms` or else from the scene's hemisphere, is
 * image-based light that the occlusion channel darkens. The scene's sun stays a direct
 * light, as in the engine's physically based shading model.
 */
export function gridPbrMaterial(uniforms: LightGridUniforms): MeshStandardMaterial {
  const material = new MeshStandardMaterial({
    side: DoubleSide,
    vertexColors: true,
    roughness: 1,
    metalness: 1,
  });
  material.onBeforeCompile = (shader: WebGLProgramParametersWithUniforms) => {
    shader.uniforms.lightGridCube = uniforms.cube;
    shader.uniforms.lightGridScale = uniforms.scale;
    shader.uniforms.lightGridSelfIllumination = uniforms.selfIllumination;
    shader.uniforms.lightGridOn = uniforms.on;
    shader.uniforms.lightGridEmissive = emissiveMaskOf(material);
    shader.vertexShader = inject(
      shader.vertexShader,
      "#include <defaultnormal_vertex>",
      VERTEX_DECLARATIONS,
      VERTEX_AMBIENT,
    );
    const fragment = shader.fragmentShader
      .replace("#include <roughnessmap_fragment>", PBR_SURFACE)
      .replace("#include <metalnessmap_fragment>", "")
      .replace("#include <normal_fragment_maps>", PBR_NORMAL)
      .replace("#include <lights_fragment_end>", `${PBR_AMBIENT}#include <lights_fragment_end>`)
      .replace("#include <aomap_fragment>", `#include <aomap_fragment>${PBR_OCCLUSION}`)
      .replace("#include <opaque_fragment>", `${PBR_EMISSIVE}#include <opaque_fragment>`);
    shader.fragmentShader = `${FRAGMENT_DECLARATIONS}${fragment}`;
  };
  material.customProgramCacheKey = () => PBR_PROGRAM_KEY;
  return material;
}

/**
 * Binds `rma` and `normal` to `material`, and flags the program stale where a map arrives or
 * leaves, since that changes its defines.
 */
export function applyPbrMaps(
  material: MeshStandardMaterial,
  rma: Texture,
  normal: Texture | null,
): void {
  const moved =
    material.roughnessMap === null || (material.normalMap === null) !== (normal === null);
  material.roughnessMap = rma;
  material.normalMap = normal;
  if (moved) material.needsUpdate = true;
}

/**
 * Fill `uniforms` from the grid cell under `x`, `z` in the scene's space, and turn them
 * off where there is no grid. Answers the cell, so a caller can skip a frame that did
 * not change it.
 */
export function lightFrom(
  grid: LightGrid | null,
  x: number,
  z: number,
  uniforms: LightGridUniforms,
): number {
  if (grid === null) {
    uniforms.on.value = 0;
    return -1;
  }
  const cell = sceneCubeAt(grid, x, z, uniforms.cube.value);
  uniforms.scale.value = [1, grid.fullBright];
  uniforms.on.value = 1;
  return cell;
}

function inject(
  source: string,
  anchor: string,
  declarations: string,
  body: string,
  before = false,
): string {
  const placed = before ? `${body}\n${anchor}` : `${anchor}\n${body}`;
  return `${declarations}\n${source.replace(anchor, placed)}`;
}
