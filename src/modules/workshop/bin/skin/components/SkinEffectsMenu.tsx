import { ArrowSquareOutIcon } from "@phosphor-icons/react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { MouseEvent } from "react";

import { IconButton, Menu, Tooltip } from "@/components";
import { m } from "@/i18n";
import type { AssetRef, BinDocumentId } from "@/lib/tauri";
import type { Pose } from "@/modules/viewport";

import { useOptionalProjectContext } from "../../../projects/state/ProjectContext";
import { useSandbox } from "../../../sandbox/state/SandboxContext";
import { clickIntent, type HostHint } from "../../../state";
import { declaringQueries } from "../../documents/api/declaringQueries";
import { spellQueries } from "../../spells/api/spellQueries";
import { type EffectTarget, useOpenEffect } from "../../vfx/context/OpenEffect";
import type { RigModel } from "../../vfx/engine/model/rig";
import type { ParticleCue } from "../utils/clipEvents";
import type { WornEffect } from "../utils/conditionScene";
import { cueRig, jointRig } from "../utils/skinScene";

export interface SkinEffectsMenuProps {
  /** The skin's document and what it was read from, which declare its idle effects' systems. */
  readonly document: BinDocumentId;
  readonly asset: AssetRef;
  /** The skin's object, which the effect's tab poses. */
  readonly skin: string;
  /** The clip playing, its hash and name, and an empty hash for the bind pose. */
  readonly clip: string;
  readonly clipName: string | null;
  readonly idle: readonly WornEffect[];
  readonly cues: readonly ParticleCue[];
  /** The pose the viewport plays, which the handed rigs ride, and null while it loads. */
  readonly pose: Pose | null;
  readonly scale: number;
}

/**
 * The skin preview's Open effect menu: each idle effect, and each particle event of the clip
 * playing, opened in its own tab on the joint it rides here. Nothing draws outside a project,
 * where no tab opens. "The rig picks itself" in docs/plans/vfx-templates.md.
 */
export function SkinEffectsMenu(props: SkinEffectsMenuProps) {
  const project = useOptionalProjectContext();
  if (project === null || props.pose === null) return null;
  if (props.idle.length + props.cues.length === 0) return null;

  return <EffectsMenu {...props} pose={props.pose} />;
}

function EffectsMenu({
  document,
  asset,
  skin,
  clip,
  clipName,
  idle,
  cues,
  pose,
  scale,
}: SkinEffectsMenuProps & { readonly pose: Pose }) {
  const sandbox = useSandbox();
  const hashes = [...idle.map((each) => each.system), ...cues.map((cue) => cue.system)];
  const names = useQuery(spellQueries.effects(sandbox, document, hashes));
  const openEffect = useOpenEffect();
  const client = useQueryClient();

  /* The tab opens the bin that declares the system, which the read of `document` found. */
  const targetOf = async (system: string): Promise<EffectTarget> => {
    const declaring = await client
      .fetchQuery(declaringQueries.object(document, system))
      .catch(() => null);
    const declared = names.data?.objects[system];
    return {
      asset: declaring ?? asset,
      entry: system,
      path: declared?.path ?? system,
      file: declared?.declarations[0]?.file ?? system,
    };
  };
  const leafOf = (system: string) => names.data?.objects[system]?.path.split("/").at(-1) ?? system;

  const openIdle = ({ effect, system }: WornEffect, event: MouseEvent) => {
    const rig: RigModel = { ...jointRig(pose, effect, scale), life: "continuous" };
    const host: HostHint = { skin, clip, offset: 0 };
    const label = m.workshop_bin_vfx_context_idle_label({ bone: effect.bone });
    const intent = clickIntent(event);
    void targetOf(system).then((target) => openEffect(target, rig, label, intent, host));
  };
  const openCue = (cue: ParticleCue, event: MouseEvent) => {
    const base = cueRig(pose, cue, scale);
    const motion =
      base.motion.kind === "bone" ? { ...base.motion, period: pose.duration } : base.motion;
    const rig: RigModel = { ...base, motion, life: "loop" };
    const host: HostHint = { skin, clip, offset: cue.at };
    const label = m.workshop_bin_vfx_context_cue_label({
      clip: clipName ?? clip,
      time: cue.at.toFixed(2),
    });
    const intent = clickIntent(event);
    void targetOf(cue.system).then((target) => openEffect(target, rig, label, intent, host));
  };

  const action = m.workshop_bin_vfx_open_effect_action();
  return (
    <Menu.Root>
      <Tooltip content={action}>
        <Menu.Trigger render={<IconButton aria-label={action} icon={<ArrowSquareOutIcon />} />} />
      </Tooltip>
      <Menu.Content data-ui="SkinEffectsMenu" className="max-h-96 w-72 overflow-y-auto">
        {idle.length > 0 && (
          <Menu.Group>
            <Menu.GroupLabel>{m.workshop_bin_mesh_preview_idle_effects_label()}</Menu.GroupLabel>
            {idle.map((held) => (
              <Menu.Item key={held.key} onClick={(event) => openIdle(held, event)}>
                <EffectLine name={leafOf(held.system)} where={held.effect.bone} />
              </Menu.Item>
            ))}
          </Menu.Group>
        )}
        {idle.length > 0 && cues.length > 0 && <Menu.Separator />}
        {cues.length > 0 && (
          <Menu.Group>
            <Menu.GroupLabel>{clipName ?? clip}</Menu.GroupLabel>
            {cues.map((cue) => (
              <Menu.Item key={cue.key} onClick={(event) => openCue(cue, event)}>
                <EffectLine
                  name={leafOf(cue.system)}
                  where={m.workshop_bin_preview_time_label({ seconds: cue.at.toFixed(2) })}
                />
              </Menu.Item>
            ))}
          </Menu.Group>
        )}
      </Menu.Content>
    </Menu.Root>
  );
}

function EffectLine({ name, where }: { name: string; where: string }) {
  return (
    <span className="flex min-w-0 flex-col">
      <span className="truncate">{name}</span>
      <span className="text-meta text-surface-400">{where}</span>
    </span>
  );
}
