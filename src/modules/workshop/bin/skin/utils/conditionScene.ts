import type { EffectSystem, JointEffect, PersistentCondition } from "@/lib/tauri";

import { nameHash } from "../../shared/utils/binHash";

/** One effect the preview draws on a joint, and the system its key resolves to. */
export interface WornEffect {
  /** Stable across renders while the effect's list keeps its order. */
  readonly key: string;
  readonly effect: JointEffect;
  /** The system, `0x` and eight hex digits. */
  readonly system: string;
}

/**
 * Returns `effects` with the system that each key resolves to through `systems`, each keyed
 * under `prefix`.
 *
 * Skips an effect whose key no file within reach maps to a system.
 */
export function wornEffects(
  effects: readonly JointEffect[],
  systems: readonly EffectSystem[],
  prefix: string,
): WornEffect[] {
  const byKey = new Map(systems.map((each) => [each.key, each]));

  return effects.flatMap((effect, index) => {
    const mapped = byKey.get(effect.effectKey);
    if (mapped === undefined) return [];

    return [{ key: `${prefix}:${index}:${effect.effectKey}`, effect, system: mapped.system }];
  });
}

/**
 * Returns the effects of the conditions whose index `active` holds, in list order, each
 * resolved through `systems` as [`wornEffects`] resolves one.
 */
export function conditionEffects(
  conditions: readonly PersistentCondition[],
  active: ReadonlySet<number>,
  systems: readonly EffectSystem[],
): WornEffect[] {
  return conditions.flatMap((condition, at) =>
    active.has(at) ? wornEffects(condition.effects, systems, `condition${at}`) : [],
  );
}

/**
 * Returns `hidden` with the submesh changes of the conditions in `active` applied.
 *
 * The show sets of the active conditions are joined, and every submesh in a hide set is
 * hidden, which is how the engine combines them. A hash that matches no name of `submeshes`
 * is skipped.
 */
export function conditionedHidden(
  hidden: readonly string[],
  submeshes: readonly string[],
  conditions: readonly PersistentCondition[],
  active: ReadonlySet<number>,
): readonly string[] {
  const applied = conditions.filter((_, at) => active.has(at));
  if (applied.length === 0) return hidden;

  const shown = new Set(applied.flatMap((condition) => condition.shown));
  const hides = new Set(applied.flatMap((condition) => condition.hidden));
  const kept = hidden.filter((name) => {
    const hash = nameHash(name);
    return !shown.has(hash) || hides.has(hash);
  });

  for (const name of submeshes) {
    const hash = nameHash(name);
    if (hides.has(hash) && !kept.some((each) => nameHash(each) === hash)) kept.push(name);
  }
  return kept;
}
