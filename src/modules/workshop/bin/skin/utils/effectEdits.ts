import type { BinEdit, BinRow, LeafValue, PropertyEdit, VfxObject } from "@/lib/tauri";

import { EFFECT } from "../../classes/utils/classLayouts";
import { setField } from "../../material/utils/entryEdits";
import { nameHash } from "../../shared/utils/binHash";
import { childCount } from "../../tree/utils/binRows";
import { systemLeaf } from "../../vfx/children/childModel";

/** `idleParticlesEffects`, the skin's list of idle effects. */
export const IDLE_EFFECTS = nameHash("idleParticlesEffects");

/** A list of effects of the skin, which an add appends to. */
export interface EffectListPath {
  /** The path of the struct that holds the list, and empty for the skin itself. */
  readonly holder: string;
  /** The list's field, `0x` and eight hex digits. */
  readonly field: string;
  /** The class of one element. */
  readonly class: string;
}

/** `idleParticlesEffects`, the list of effects the skin draws at all times. */
export const IDLE_LIST: EffectListPath = {
  holder: "",
  field: IDLE_EFFECTS,
  class: "SkinCharacterDataProperties_CharacterIdleEffect",
};

/** Returns the number of items the list row `row` holds, and zero where the skin lacks it. */
export function listLength(row: BinRow | undefined): number {
  return row === undefined ? 0 : childCount(row);
}

/** A key of the resolver's `resourceMap`. */
export interface ResolverKey {
  /** The key as an edit types it: a name, or `0x` and eight hex digits. */
  readonly text: string;
  readonly hash: string;
  /** `true` if the map does not hold the key yet. */
  readonly fresh: boolean;
}

/** One effect to add, and the resolver key it is keyed by. */
export interface NewEffect {
  /** The skin object, `0x` and eight hex digits. */
  readonly skin: string;
  readonly path: EffectListPath;
  /** The number of effects the list holds, which is the new effect's index. */
  readonly count: number;
  /** The resolver object, `0x` and eight hex digits. */
  readonly resolver: string;
  readonly key: ResolverKey;
  /** The system the key maps to, `0x` and eight hex digits. */
  readonly system: string;
  readonly bone: string;
}

/** Returns the hash of a key as an edit types it. */
function keyHash(text: string): string {
  return /^0x[0-9a-f]{8}$/i.test(text) ? text.toLowerCase() : nameHash(text);
}

/**
 * Returns the key of `resources` that maps to `system`, the systems keyed by key hash.
 *
 * If no key maps to it, returns a fresh key named after the last segment of the system's path,
 * numbered from 2 if another system holds that name.
 */
export function resolverKey(
  resources: ReadonlyMap<string, { readonly hash: string }>,
  system: VfxObject,
): ResolverKey {
  for (const [hash, target] of resources) {
    if (target.hash === system.entry) return { text: hash, hash, fresh: false };
  }

  const base = systemLeaf(system.name ?? system.entry);
  let text = base;
  for (let at = 2; resources.has(keyHash(text)); at += 1) {
    text = `${base}${at}`;
  }
  return { text, hash: keyHash(text), fresh: true };
}

/** Returns the hash path of `field` under the struct at `holder`. */
export function fieldPath(holder: string, field: string): string {
  return holder === "" ? field.slice(2) : `${holder}.${field.slice(2)}`;
}

/** Returns the hash path of the `resourceMap` entry under `hash`. */
function entryPath(hash: string): string {
  return `${EFFECT.resourceMap.slice(2)}{${hash.slice(2)}}`;
}

function link(add: NewEffect): LeafValue {
  return { type: "objectLink", text: add.system };
}

/** Returns the edit of the effect list that appends the effect, with its key and bone. */
function effectEdit(add: NewEffect): PropertyEdit {
  const at = `[${add.count}]`;
  return {
    entry: add.skin,
    holder: add.path.holder,
    field: add.path.field,
    edits: [
      {
        type: "insertItem",
        path: "",
        item: { index: add.count, key: null, class: add.path.class },
      },
      ...setField(at, EFFECT.key, { type: "hash", text: add.key.text }),
      ...setField(at, EFFECT.bone, { type: "string", value: add.bone }),
    ],
  };
}

/**
 * Returns `add` as one `editProperties`, which a document that saves its own bytes records as
 * one undo step. The resolver's map takes the key first if the key is fresh.
 */
export function groupedAdd(add: NewEffect): BinEdit {
  const effect = effectEdit(add);
  if (!add.key.fresh) return { kind: "editProperties", edits: [effect] };

  const key: PropertyEdit = {
    entry: add.resolver,
    holder: "",
    field: EFFECT.resourceMap,
    edits: [
      { type: "insertItem", path: "", item: { index: null, key: add.key.text, class: null } },
      { type: "setLeaf", path: `{${add.key.hash.slice(2)}}`, value: link(add) },
    ],
  };
  return { kind: "editProperties", edits: [key, effect] };
}

/**
 * Returns `add` as the narrow calls a declared document takes, in order.
 *
 * A declared document writes the whole value of the property that an `editProperty` names, so
 * the key and the effect are each an insert and then a write of each field. The game's keys and
 * effects then stay the game's. A list that holds no effect is written whole.
 */
export function declaredAdd(add: NewEffect): BinEdit[] {
  const calls: BinEdit[] = [];
  if (add.key.fresh) {
    calls.push(
      {
        kind: "insertItem",
        entry: add.resolver,
        path: EFFECT.resourceMap.slice(2),
        item: { index: null, key: add.key.text, class: null },
      },
      { kind: "patch", entry: add.resolver, path: entryPath(add.key.hash), value: link(add) },
    );
  }

  if (add.count === 0) {
    const { entry, holder, field, edits } = effectEdit(add);
    calls.push({ kind: "editProperty", entry, holder, field, edits });
    return calls;
  }

  const list = fieldPath(add.path.holder, add.path.field);
  const item = `${list}[${add.count}]`;
  const write = (field: string, value: LeafValue): BinEdit => ({
    kind: "editProperty",
    entry: add.skin,
    holder: item,
    field,
    edits: [{ type: "setLeaf", path: "", value }],
  });
  calls.push(
    {
      kind: "insertItem",
      entry: add.skin,
      path: list,
      item: { index: add.count, key: null, class: add.path.class },
    },
    write(EFFECT.key, { type: "hash", text: add.key.text }),
    write(EFFECT.bone, { type: "string", value: add.bone }),
  );
  return calls;
}
