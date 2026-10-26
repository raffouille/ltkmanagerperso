import { useQueries } from "@tanstack/react-query";
import { useMemo } from "react";

import type { BinDocumentId, SkinModel, VfxSystem } from "@/lib/tauri";
import { usePreviewEffectsQuality } from "@/stores";

import type { SystemModel } from "../../vfx/engine/model/model";
import { vfxQueries } from "../../vfx/hooks/useVfxSystem";
import { type ParticleCue, particleCues, type TimedStep } from "../utils/clipEvents";
import { conditionEffects, type WornEffect, wornEffects } from "../utils/conditionScene";
import { systemModel } from "../utils/skinScene";

/** Every effect a skin's preview draws, and the system model each one runs. */
export interface WornEffects {
  /** The particle events of the playing pass. */
  readonly cues: readonly ParticleCue[];
  readonly cueModels: readonly (SystemModel | null)[];
  /** The idle effects, which the effects menu lists. */
  readonly idle: readonly WornEffect[];
  /** The idle effects and the effects of the applied conditions, in draw order. */
  readonly drawn: readonly WornEffect[];
  readonly drawnModels: readonly (SystemModel | null)[];
  /** Every model above, the drawn ones first. */
  readonly worn: readonly (SystemModel | null)[];
  /** One character per model, which changes when a model is read and the effects reseek. */
  readonly loaded: string;
}

/**
 * Returns every effect the skin of `document` draws on the pass `timed`, with the conditions in
 * `conditions` applied, and the system model of each.
 *
 * Each system is read through `document`, which reaches a system that a linked bin declares.
 */
export function useWornEffects(
  document: BinDocumentId,
  skin: SkinModel,
  timed: readonly TimedStep[],
  conditions: ReadonlySet<number>,
): WornEffects {
  const cues = useMemo(() => particleCues(timed, skin.effectSystems), [timed, skin.effectSystems]);
  const idle = useMemo(
    () => wornEffects(skin.idleEffects, skin.effectSystems, "idle"),
    [skin.idleEffects, skin.effectSystems],
  );
  const applied = useMemo(
    () => conditionEffects(skin.persistentConditions, conditions, skin.effectSystems),
    [skin.persistentConditions, conditions, skin.effectSystems],
  );
  const drawn = useMemo(() => [...idle, ...applied], [idle, applied]);

  const quality = usePreviewEffectsQuality();
  const systemQuery = ({ system }: { system: string }) => vfxQueries.system(document, system);
  const toModels = (results: { data?: VfxSystem }[]) =>
    results.map((result) => (result.data === undefined ? null : systemModel(result.data, quality)));
  const cueModels = useQueries({ queries: cues.map(systemQuery), combine: toModels });
  const drawnModels = useQueries({ queries: drawn.map(systemQuery), combine: toModels });

  const worn = drawnModels.concat(cueModels);
  const loaded = worn.map((model) => (model === null ? "-" : "+")).join("");
  return { cues, cueModels, idle, drawn, drawnModels, worn, loaded };
}
