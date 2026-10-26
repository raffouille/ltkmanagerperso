import type { AssetRef, MaterialPreview, VfxValue } from "@/lib/tauri";

import { nameHash } from "../../../shared/utils/binHash";
import { field, flag, text } from "../../engine/parsing/readValue";
import {
  type ShimmerComponents,
  shimmerComponentsOf,
} from "../../engine/shimmer/shimmerComponents";

const SHIMMER_LIST = nameHash("shimmerEmitterDefinitionData");
const COMPLEX_LIST = nameHash("complexEmitterDefinitionData");
const STATIC_MATERIAL = nameHash("StaticMaterialDef");

/** The `Material` field of a render component and of the container it holds a material in. */
const MATERIAL = nameHash("Material");
const MATERIAL_CONTAINER = nameHash("MaterialContainer");

const EMITTER = {
  name: nameHash("emitterName"),
  disabled: nameHash("disabled"),
  components: nameHash("VfxComponents"),
} as const;

const SLOT = {
  render: nameHash("RenderComponent"),
  geometry: nameHash("GeometryComponent"),
} as const;

const MESH_EXTENSIONS = [".gmesh", ".tmesh", ".scb"] as const;
const TEXTURE_EXTENSIONS = [".tex", ".dds"] as const;

/** The depth past which the walk for assets and materials stops. */
const MAX_DEPTH = 24;

/** Where a material embedded in an emitter sits: its object and its property path there. */
export interface EmbeddedMaterialAt {
  readonly entry: string;
  readonly path: string;
}

/** A material a render component links to, which the system's read resolved. */
export interface LinkedMaterial {
  readonly hash: string;
}

/** One shimmer emitter's mesh, and the components that spawn and move its particles. */
export interface ShimmerMesh {
  /** The list it is read from: the shimmer list, or the complex list's component emitters. */
  readonly list: "shimmer" | "complex";
  /** Its place in its list. */
  readonly index: number;
  readonly name: string;
  readonly disabled: boolean;
  readonly mesh: { readonly asset: AssetRef; readonly path: string };
  readonly texture: AssetRef | null;
  readonly components: ShimmerComponents;
  /** The `StaticMaterialDef` its render component embeds, and null for none. */
  readonly material: EmbeddedMaterialAt | null;
  /** The material its render component links to in another bin, and null for none. */
  readonly linked: LinkedMaterial | null;
}

/**
 * Every shimmer emitter of a resolved system that names a mesh.
 *
 * The geometry component's primitive holds a mesh path and a texture name under fields no
 * table names reliably, so each is the first asset under it with the extension the engine
 * tests. The lifetime, physics and render components are read for `shimmerParticles`.
 *
 * The complex list's emitters that hold components are the ones the game draws, and the
 * shimmer list keeps disabled copies of them, so a copy named as a complex one is left out.
 */
export function shimmerMeshesOf(
  root: VfxValue,
  materials: readonly MaterialPreview[] = [],
): ShimmerMesh[] {
  const previews = new Map(materials.map((material) => [material.hash, material]));
  const complex = meshesOf(root, "complex", previews);
  const drawn = new Set(complex.map((each) => each.name));
  const shimmer = meshesOf(root, "shimmer", previews).filter((each) => !drawn.has(each.name));
  return [...complex, ...shimmer];
}

function meshesOf(
  root: VfxValue,
  from: ShimmerMesh["list"],
  previews: ReadonlyMap<string, MaterialPreview>,
): ShimmerMesh[] {
  const listHash = from === "shimmer" ? SHIMMER_LIST : COMPLEX_LIST;
  const list = field(root, listHash);
  if (list?.type !== "container") return [];
  const entry = root.type === "struct" ? (root.object?.entry ?? null) : null;

  return list.items.flatMap((emitter, index) => {
    const components = field(emitter, EMITTER.components);
    const geometry = field(components, SLOT.geometry);
    const render = field(components, SLOT.render);

    const mesh = firstAsset(geometry, MESH_EXTENSIONS, 0);
    if (mesh === null || mesh.asset === null) return [];
    /* The game skips a disabled complex emitter, and the shimmer list is all disabled copies. */
    const disabled = flag(field(emitter, EMITTER.disabled));
    if (from === "complex" && disabled) return [];

    const at = entry === null ? null : { entry, path: `${segment(listHash)}[${index}]` };
    return [
      {
        list: from,
        index,
        name: text(field(emitter, EMITTER.name)) ?? `[${index}]`,
        disabled,
        mesh: { asset: mesh.asset, path: mesh.path },
        texture:
          firstAsset(geometry, TEXTURE_EXTENSIONS, 0)?.asset ??
          firstAsset(render, TEXTURE_EXTENSIONS, 0)?.asset ??
          null,
        components: shimmerComponentsOf(components),
        material: at === null ? null : materialAt(emitter, at, [EMITTER.components, SLOT.render]),
        linked: linkedMaterial(render, previews),
      },
    ];
  });
}

/**
 * The material `render` links to rather than embeds, and null where the link names nothing
 * any bin within reach declares.
 *
 * A link to an object of the system's own bin is inlined by the read, and `materialAt`
 * finds it as an embedded material.
 */
function linkedMaterial(
  render: VfxValue | null,
  previews: ReadonlyMap<string, MaterialPreview>,
): LinkedMaterial | null {
  const held = field(render, MATERIAL) ?? field(field(render, MATERIAL_CONTAINER), MATERIAL);
  if (held?.type !== "link") return null;

  const preview = previews.get(held.hash);
  if (preview === undefined || preview.missing) return null;
  return { hash: held.hash };
}

/**
 * Where the `StaticMaterialDef` under the fields `steps` of `value` sits, depth first, with
 * `at` the address of `value`. A struct resolved from another object starts that object's
 * own address.
 */
function materialAt(
  value: VfxValue | null,
  at: EmbeddedMaterialAt,
  steps: readonly string[],
): EmbeddedMaterialAt | null {
  let held = value;
  let place = at;
  for (const step of steps) {
    if (held?.type === "struct" && held.object !== null) {
      place = { entry: held.object.entry, path: "" };
    }
    held = field(held, step);
    place = { entry: place.entry, path: joined(place.path, segment(step)) };
  }
  return firstMaterial(held, place, 0);
}

function firstMaterial(
  value: VfxValue | null,
  at: EmbeddedMaterialAt,
  depth: number,
): EmbeddedMaterialAt | null {
  if (value === null || depth > MAX_DEPTH) return null;

  if (value.type === "container") {
    for (const [index, item] of value.items.entries()) {
      const found = firstMaterial(item, { ...at, path: `${at.path}[${index}]` }, depth + 1);
      if (found !== null) return found;
    }
    return null;
  }
  if (value.type !== "struct") return null;

  const place = value.object === null ? at : { entry: value.object.entry, path: "" };
  if (value.classHash === STATIC_MATERIAL) return place;

  for (const each of value.fields) {
    const next = { ...place, path: joined(place.path, segment(each.hash)) };
    const found = firstMaterial(each.value, next, depth + 1);
    if (found !== null) return found;
  }
  return null;
}

/** A field hash as a property path writes it: its eight hex digits alone. */
function segment(hash: string): string {
  return hash.slice(2);
}

function joined(path: string, next: string): string {
  return path === "" ? next : `${path}.${next}`;
}

/** The first asset under `value` whose path ends in one of `extensions`, depth first. */
function firstAsset(
  value: VfxValue | null,
  extensions: readonly string[],
  depth: number,
): { path: string; asset: AssetRef | null } | null {
  if (value === null || depth > MAX_DEPTH) return null;

  if (value.type === "asset" || value.type === "string") {
    const path = value.type === "asset" ? value.path : value.value;
    const lower = path.toLowerCase();
    if (!extensions.some((extension) => lower.endsWith(extension))) return null;
    return { path, asset: value.type === "asset" ? value.asset : null };
  }

  const children =
    value.type === "struct"
      ? value.fields.map((each) => each.value)
      : value.type === "container"
        ? value.items
        : value.type === "map"
          ? value.entries.map((entry) => entry.value)
          : [];
  for (const child of children) {
    const found = firstAsset(child, extensions, depth + 1);
    if (found !== null) return found;
  }
  return null;
}
