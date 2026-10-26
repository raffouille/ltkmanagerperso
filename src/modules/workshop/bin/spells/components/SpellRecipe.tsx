import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo } from "react";

import { m } from "@/i18n";
import type { AssetRef, BinDocumentId } from "@/lib/tauri";
import { createPose, jointAnchor, viewportQueries } from "@/modules/viewport";

import { useSandbox } from "../../../sandbox/state/SandboxContext";
import { useBinDocument } from "../../documents/hooks/useBinDocument";
import { Notice } from "../../shared/preview/Notice";
import { skinQueries } from "../../skin/api/skinQueries";
import { useSkinGraphSource } from "../../skin/hooks/useGraphSource";
import { clipFrameSeconds } from "../../skin/utils/clipEvents";
import { spellQueries } from "../api/spellQueries";
import type { AbilityRecipe } from "../utils/abilityRecipe";
import { oncePose } from "../utils/abilitySequence";
import { compileFlight } from "../utils/flight";
import { spellAnimation } from "../utils/spellAnimation";
import { castTiming, spellEffect, spellGuides, spellImpact } from "../utils/spellSuggestions";
import type { AbilitySource } from "./AbilityLibrary";

export interface SpellSelection {
  readonly asset: AssetRef;
  readonly entry: string;
  readonly name: string;
}

export function SpellRecipe({
  selected,
  source,
  character,
  onReady,
}: {
  selected: SpellSelection;
  source: AbilitySource;
  character: string;
  onReady: (recipe: AbilityRecipe) => void;
}) {
  const { state } = useBinDocument(selected.asset, selected.entry);
  if (state.status === "failed") return <Notice text={m.workshop_bin_preview_failed_empty()} />;
  if (state.status !== "open") return <Notice text={m.workshop_spells_loading_label()} />;
  return (
    <ReadRecipe
      document={state.handle.document}
      selected={selected}
      source={source}
      character={character}
      onReady={onReady}
    />
  );
}

function ReadRecipe({
  document,
  selected,
  source,
  character,
  onReady,
}: {
  document: BinDocumentId;
  selected: SpellSelection;
  source: AbilitySource;
  character: string;
  onReady: (recipe: AbilityRecipe) => void;
}) {
  const read = useQuery(spellQueries.preview(document, selected.entry));
  const skin = useSkinGraphSource(source.document, source.entry);
  const graph = useQuery(skinQueries.graph(skin.source.document, skin.source.graph));
  const animation = spellAnimation(read.data?.animationName ?? null, graph.data?.clips ?? []);
  const skeleton = useQuery(viewportQueries.skeleton(skin.skin.data?.skeleton?.asset ?? null));
  const clipRead = useQuery(viewportQueries.clip(animation?.animation?.asset ?? null));
  const sandbox = useSandbox();
  const names = useQuery(
    spellQueries.effects(
      sandbox,
      source.document,
      skin.skin.data?.effectSystems.map((effect) => effect.system) ?? [],
    ),
  );
  const recipe = useMemo<AbilityRecipe | null>(() => {
    if (
      read.data === undefined ||
      (Boolean(read.data.effectName || read.data.hitEffectName) && names.isFetching) ||
      skin.skin.data === undefined ||
      (skin.source.graph !== null && graph.data === undefined) ||
      (skin.skin.data.skeleton?.asset != null && skeleton.data === undefined) ||
      (animation !== null && clipRead.data === undefined)
    )
      return null;
    const preview = read.data;
    const pose =
      skeleton.data === undefined
        ? null
        : oncePose(
            createPose(
              skeleton.data,
              clipRead.data ?? null,
              clipFrameSeconds(animation, clipRead.data?.fps ?? null),
            ),
          );
    const bone = preview.missile?.startBone ?? "";
    const slot = bone === "" ? -1 : (pose?.jointNamed(bone) ?? -1);
    const timing = castTiming(preview);
    const release = timing.release + (preview.missile?.startDelay ?? 0);
    const from =
      pose === null
        ? ([0, 0, 0] as const)
        : jointAnchor(pose, slot, [0, 0, 0], skin.skin.data.scale ?? 1).originAt(release);
    const guides = spellGuides(preview);
    const target: AbilityRecipe["target"] = [guides.range || 500, from[1], 0];
    const effect = spellEffect(
      skin.skin.data.effectSystems,
      preview.effectKey,
      preview.effectName,
      names.data?.objects,
    );
    const hit = spellImpact(preview, skin.skin.data.effectSystems, names.data?.objects);
    const flight = preview.missile === null ? null : compileFlight(preview.missile, from, target);
    const projectile = flight !== null && effect !== null;
    return {
      version: 1,
      id: crypto.randomUUID(),
      name: selected.name.split("/").at(-1) ?? selected.name,
      character,
      clip: animation?.hash ?? (preview.animationName ? "" : null),
      bone,
      ...timing,
      missileDelay: flight?.delay ?? 0,
      guides,
      castEffect: null,
      projectileEffect: projectile ? effect.key : null,
      flightDuration: flight?.duration || 0.5,
      impactEffect: hit?.key ?? null,
      impactDuration: 0.5,
      target,
    };
  }, [
    read.data,
    names.data,
    names.isFetching,
    skin.skin.data,
    skin.source.graph,
    graph.data,
    selected,
    character,
    skeleton.data,
    clipRead.data,
    animation,
  ]);
  useEffect(() => {
    if (recipe !== null) onReady(recipe);
  }, [recipe, onReady]);
  return (
    <>
      <Notice
        text={
          read.isError || skin.skin.isError || graph.isError || skeleton.isError || clipRead.isError
            ? m.workshop_bin_preview_failed_empty()
            : m.workshop_spells_loading_label()
        }
      />
    </>
  );
}
