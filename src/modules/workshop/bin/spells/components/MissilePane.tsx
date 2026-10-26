import { PathIcon } from "@phosphor-icons/react";
import { useQuery } from "@tanstack/react-query";
import { lazy, Suspense, useMemo, useState } from "react";

import { Button, Select } from "@/components";
import { errorSummary, m } from "@/i18n";
import {
  type AssetRef,
  type BinDocumentId,
  type DeclaredObjects,
  type EffectSystem,
  type SpellPreview,
} from "@/lib/tauri";

import { useSandbox } from "../../../sandbox/state/SandboxContext";
import { declaringQueries } from "../../documents/api/declaringQueries";
import { useBinDocument } from "../../documents/hooks/useBinDocument";
import { Notice } from "../../shared/preview/Notice";
import { skinQueries } from "../../skin/api/skinQueries";
import { type EffectTarget, OpenEffectButton } from "../../vfx/context/OpenEffect";
import { GROUND_RIG } from "../../vfx/engine/model/rig";
import { spellQueries } from "../api/spellQueries";
import { compileFlight } from "../utils/flight";
import { spellEffect, spellImpact } from "../utils/spellSuggestions";
import { MissileOptions } from "./MissileOptions";

const MissileViewport = lazy(() => import("./MissileViewport"));

export interface SkinSource {
  readonly document: BinDocumentId;
  readonly entry: string;
}

export interface MissilePaneProps {
  readonly asset: AssetRef;
  readonly entry: string;
  readonly skin: SkinSource;
}

/** One selected spell declaration, held open for the isolated missile preview. */
export function MissilePane({ asset, entry, skin }: MissilePaneProps) {
  const { state, reopen } = useBinDocument(asset, entry);
  if (state.status === "failed")
    return <Failure message={errorSummary(state.error)} retry={reopen} />;
  if (state.status !== "open") return <Notice text={m.workshop_spells_loading_label()} />;
  return <MissileRead document={state.handle.document} entry={entry} skin={skin} />;
}

function MissileRead({ document, entry, skin }: SkinSource & { skin: SkinSource }) {
  const read = useQuery(spellQueries.preview(document, entry));
  const skinRead = useQuery(skinQueries.skin(skin.document, skin.entry));
  const error = read.error ?? skinRead.error;
  if (error !== null)
    return (
      <Failure
        message={errorSummary(error)}
        retry={() => {
          void read.refetch();
          void skinRead.refetch();
        }}
      />
    );
  if (read.data === undefined || skinRead.data === undefined)
    return <Notice text={m.workshop_spells_loading_label()} />;
  return <MissileSetup preview={read.data} effects={skinRead.data.effectSystems} skin={skin} />;
}

function effectId(effect: EffectSystem): string {
  return `${effect.key}:${effect.system}`;
}

function MissileSetup({
  preview,
  effects,
  skin,
}: {
  preview: SpellPreview;
  effects: readonly EffectSystem[];
  skin: SkinSource;
}) {
  const [picked, setPicked] = useState<string | null>(null);
  const [anchors, setAnchors] = useState([-400, 0, 400, 0, 100]);
  const sandbox = useSandbox();
  const names = useQuery(
    spellQueries.effects(
      sandbox,
      skin.document,
      effects.map((item) => item.system),
    ),
  );
  const automatic = spellEffect(
    effects,
    preview.effectKey,
    preview.effectName,
    names.data?.objects,
  );
  const suggested = automatic === null ? null : effectId(automatic);
  const effect = effects.find((effect) => effectId(effect) === (picked ?? suggested));
  const [fromX, fromZ, toX, toZ, height] = anchors;
  const flight = useMemo(
    () =>
      preview.missile === null
        ? null
        : compileFlight(preview.missile, [fromX, height, fromZ], [toX, height, toZ]),
    [preview, fromX, fromZ, toX, toZ, height],
  );
  const invalid = preview.issues.some((issue) => issue.kind === "invalid");
  const items = effects
    .map((item) => ({
      value: effectId(item),
      label: names.data?.objects[item.system]?.path.split("/").at(-1) ?? item.system,
    }))
    .sort((a, b) => a.label.localeCompare(b.label) || a.value.localeCompare(b.value));
  const selected = items.find((item) => item.value === (picked ?? suggested));
  const hit = spellImpact(preview, effects, names.data?.objects);
  const declaring = useQuery(declaringQueries.object(skin.document, effect?.system ?? null));
  const hitDeclaring = useQuery(declaringQueries.object(skin.document, hit?.system ?? null));
  const target = effectTarget(effect, declaring.data ?? null, names.data?.objects);
  const hitTarget = effectTarget(hit ?? undefined, hitDeclaring.data ?? null, names.data?.objects);
  const movement = preview.missile?.movement;
  let blocked: { title: string; description: string } | null = null;
  if (invalid)
    blocked = {
      title: m.workshop_missile_data_title(),
      description: m.workshop_missile_data_hint(),
    };
  else if (preview.missile === null)
    blocked = {
      title: m.workshop_missile_missing_title(),
      description: m.workshop_missile_missing_empty(),
    };
  else if (movement?.kind === "unsupported" || movement?.kind === "missing")
    blocked = {
      title: m.workshop_missile_motion_title(),
      description: m.workshop_missile_motion_hint(),
    };
  else if (flight === null)
    blocked = {
      title: m.workshop_missile_invalid_title(),
      description: m.workshop_missile_invalid_hint(),
    };
  else if (effect === undefined)
    blocked = {
      title: m.workshop_missile_choose_title(),
      description: m.workshop_missile_choose_hint(),
    };

  return (
    <div data-ui="MissilePane" className="flex min-h-0 flex-1 flex-col overflow-auto scrollbar-md">
      <div className="flex shrink-0 flex-col gap-2 border-b border-surface-700/40 px-3 py-3">
        <div className="flex items-center justify-between gap-3 text-meta">
          <span className="font-medium text-surface-200">
            {m.workshop_missile_effect_short_label()}
          </span>
          <span className="text-surface-400">{m.workshop_missile_isolated_label()}</span>
        </div>
        <div className="flex min-w-0 items-center gap-1">
          <Select.Root
            items={items}
            value={selected?.value ?? null}
            onValueChange={setPicked}
            disabled={effects.length === 0}
          >
            <Select.Trigger
              aria-label={m.workshop_missile_effect_label()}
              className="min-w-0 flex-1 bg-surface-900"
            >
              <Select.Value
                className="min-w-0 truncate"
                placeholder={m.workshop_missile_effect_label()}
              >
                {selected?.label}
              </Select.Value>
              <Select.Icon />
            </Select.Trigger>
            <Select.Content>
              {items.map((item) => (
                <Select.Item key={item.value} value={item.value}>
                  {item.label}
                </Select.Item>
              ))}
            </Select.Content>
          </Select.Root>
          <OpenEffectButton
            target={target}
            rig={flight?.rig ?? null}
            label={m.workshop_bin_vfx_context_flight_label()}
          />
        </div>
        {picked !== null && (
          <p className="text-meta text-surface-400">{m.workshop_missile_effect_manual_hint()}</p>
        )}
        {hitTarget !== null && (
          <div className="flex min-w-0 items-center gap-2 text-meta">
            <span className="shrink-0 text-surface-400">
              {m.workshop_missile_hit_effect_label()}
            </span>
            <span className="min-w-0 flex-1 truncate text-surface-200 select-text">
              {hitTarget.path.split("/").at(-1)}
            </span>
            <OpenEffectButton
              target={hitTarget}
              rig={GROUND_RIG}
              label={m.workshop_bin_vfx_context_impact_label()}
            />
          </div>
        )}
      </div>
      {blocked !== null && (
        <div className="flex min-h-64 flex-1 flex-col items-center justify-center gap-3 px-6 py-10 text-center">
          <PathIcon weight="duotone" className="size-8 text-surface-400" />
          <div className="flex max-w-80 flex-col gap-2">
            <h3 className="text-sm font-medium text-surface-200">{blocked.title}</h3>
            <p className="text-meta leading-relaxed text-surface-400">{blocked.description}</p>
          </div>
        </div>
      )}
      {blocked === null && flight !== null && effect !== undefined && (
        <div className="flex flex-1 shrink-0 flex-col p-3">
          <Suspense fallback={<Notice text={m.workshop_spells_loading_label()} />}>
            <EffectPreview
              key={`${effectId(effect)}:${JSON.stringify(flight)}`}
              effect={effect}
              document={skin.document}
              flight={flight}
            />
          </Suspense>
        </div>
      )}
      <MissileOptions preview={preview} anchors={anchors} setAnchors={setAnchors} effect={effect} />
    </div>
  );
}

/**
 * The tab target of the picked effect's system, in `asset`, the bin that declares it, and null
 * while that bin is unknown. `objects` names the system and its file.
 */
function effectTarget(
  effect: EffectSystem | undefined,
  asset: AssetRef | null,
  objects: DeclaredObjects["objects"] | undefined,
): EffectTarget | null {
  if (effect === undefined || asset === null) return null;

  const declared = objects?.[effect.system];
  return {
    asset,
    entry: effect.system,
    path: declared?.path ?? effect.system,
    file: declared?.declarations[0]?.file ?? effect.system,
  };
}

function EffectPreview({
  effect,
  document,
  flight,
}: {
  effect: EffectSystem;
  document: BinDocumentId;
  flight: NonNullable<ReturnType<typeof compileFlight>>;
}) {
  return <MissileViewport document={document} entry={effect.system} flight={flight} />;
}

function Failure({ message, retry }: { message: string; retry: () => void }) {
  return (
    <div className="flex flex-col gap-2 p-2 text-meta">
      <p className="text-danger-text select-text">{message}</p>
      <Button size="sm" onClick={retry}>
        {m.workshop_objects_retry_action()}
      </Button>
    </div>
  );
}
