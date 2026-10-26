import { useQueries, useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { m } from "@/i18n";
import type { BinDocumentId, SkinModel } from "@/lib/tauri";
import { createPose, viewportQueries } from "@/modules/viewport";
import { usePreviewEffectsQuality } from "@/stores";

import { Notice } from "../../shared/preview/Notice";
import { skinQueries } from "../../skin/api/skinQueries";
import { type GraphSource, useSkinGraphSource } from "../../skin/hooks/useGraphSource";
import { clipFrameSeconds } from "../../skin/utils/clipEvents";
import { systemModel } from "../../skin/utils/skinScene";
import { abilityQueries } from "../api/abilityQueries";
import type { AbilityRecipe } from "../utils/abilityRecipe";
import { abilitySteps, oncePose } from "../utils/abilitySequence";
import type { AbilitySource } from "./AbilityLibrary";
import { AbilityScene } from "./AbilityScene";

export default function AbilityPreview({
  source,
  recipe,
}: {
  source: AbilitySource;
  recipe: AbilityRecipe;
}) {
  const read = useSkinGraphSource(source.document, source.entry);
  if (read.skin.isError) return <Notice text={m.workshop_bin_preview_failed_empty()} />;
  return (
    <>
      {read.skin.data === undefined && <Notice text={m.workshop_spells_loading_label()} />}
      {read.skin.data !== undefined && (
        <AbilityRead
          document={source.document}
          skin={read.skin.data}
          graph={read.source}
          recipe={recipe}
        />
      )}
    </>
  );
}

function AbilityRead({
  document,
  skin,
  graph: source,
  recipe,
}: {
  document: BinDocumentId;
  skin: SkinModel;
  graph: GraphSource;
  recipe: AbilityRecipe;
}) {
  const graph = useQuery(skinQueries.graph(source.document, source.graph));
  const chosen = graph.data?.clips.find((clip) => clip.hash === recipe.clip);
  const mesh = useQuery(viewportQueries.mesh(skin.mesh?.asset ?? null));
  const skeleton = useQuery(viewportQueries.skeleton(skin.skeleton?.asset ?? null));
  const clip = useQuery(viewportQueries.clip(chosen?.animation?.asset ?? null));
  const pose = useMemo(
    () =>
      skeleton.data === undefined || (recipe.clip !== null && clip.data === undefined)
        ? null
        : oncePose(
            createPose(
              skeleton.data,
              clip.data ?? null,
              clipFrameSeconds(chosen, clip.data?.fps ?? null),
            ),
          ),
    [skeleton.data, clip.data, recipe.clip, chosen],
  );
  const effects = useMemo(
    () =>
      [
        ...new Set(
          [recipe.castEffect, recipe.projectileEffect, recipe.impactEffect].filter(
            (key): key is string => key !== null,
          ),
        ),
      ].map((key) => skin.effectSystems.filter((effect) => effect.key === key)),
    [recipe, skin.effectSystems],
  );
  const quality = usePreviewEffectsQuality();
  const reads = useQueries({
    queries: effects.flatMap((matches) =>
      matches.length === 1 ? [abilityQueries.effect(document, matches[0])] : [],
    ),
    combine: (results) => ({
      data: results.map((result) => result.data),
      failed: results.some((result) => result.isError),
    }),
  });
  const { data: systems, failed: effectsFailed } = reads;
  const steps = useMemo(() => {
    if (
      pose === null ||
      effects.some((matches) => matches.length !== 1) ||
      systems.some((system) => system === undefined)
    )
      return null;
    if (recipe.bone !== "" && pose.jointNamed(recipe.bone) < 0) return null;
    return abilitySteps(recipe, pose, skin.scale ?? 1).map((step) => ({
      ...step,
      system: systemModel(
        systems[effects.findIndex((matches) => matches[0].key === step.key)]!,
        quality,
      ),
    }));
  }, [recipe, pose, skin.scale, effects, systems, quality]);
  const missing =
    (recipe.clip !== null && source.graph === null) ||
    skin.mesh?.asset == null ||
    skin.skeleton?.asset == null ||
    (recipe.clip !== null && graph.data !== undefined && chosen?.animation?.asset == null) ||
    effects.some((matches) => matches.length !== 1) ||
    (pose !== null && recipe.bone !== "" && pose.jointNamed(recipe.bone) < 0);
  if (missing) return <Notice text={m.workshop_ability_missing_hint()} />;
  if (graph.isError || mesh.isError || skeleton.isError || clip.isError || effectsFailed)
    return <Notice text={m.workshop_bin_preview_failed_empty()} />;
  if (mesh.data === undefined || pose === null || steps === null)
    return <Notice text={m.workshop_spells_loading_label()} />;
  return (
    <AbilityScene
      key={JSON.stringify(recipe)}
      skin={skin}
      mesh={mesh.data}
      pose={pose}
      recipe={recipe}
      steps={steps}
    />
  );
}
