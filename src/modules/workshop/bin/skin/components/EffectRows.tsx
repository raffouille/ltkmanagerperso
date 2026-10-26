import { ArrowRightIcon, XIcon } from "@phosphor-icons/react";
import { createContext, use } from "react";

import { Button, type DataTableColumn, Tooltip } from "@/components";
import { m } from "@/i18n";
import type { BinRow } from "@/lib/tauri";

import {
  Cell,
  fieldsOf,
  type LayoutPages,
  TableRows,
  TextCell,
  textOf,
} from "../../classes/components/ClassCells";
import { EFFECT } from "../../classes/utils/classLayouts";
import { ObjectChip } from "../../links/components/LinkChip";
import { nameHash } from "../../shared/utils/binHash";
import type { LeafEdit } from "../../tree/hooks/useLeafEdit";
import { RowValue } from "../../values/components/RowValue";
import { JointPicker } from "./JointPicker";
import type { ResourceTarget } from "./ResolverScope";

/**
 * The system an effect names, as a chip reading its last segment.
 *
 * An effect is keyed by `effectKey`, or by the hash of its `effectName` where it carries
 * no key. A name the map does not answer for draws as its text, and a key as its hash.
 * Every effect of a skin shares the folder its systems sit in, so the chip reads the name.
 */
function Resource({
  effect,
  name,
  resources,
}: {
  effect: BinRow | undefined;
  name: BinRow | undefined;
  resources: ReadonlyMap<string, ResourceTarget>;
}) {
  const named = textOf(name);
  const hash =
    effect?.value.type === "hash"
      ? effect.value.hash
      : named === undefined
        ? null
        : nameHash(named);
  const system = hash === null ? undefined : resources.get(hash);
  if (system !== undefined) {
    return <ObjectChip hash={system.hash} name={system.name} kind="link" reading="name" />;
  }
  if (effect !== undefined) return <RowValue row={effect} />;
  if (named !== undefined) return <span className="truncate select-text">{named}</span>;
  return null;
}

export interface EffectRowsContextValue {
  pages: LayoutPages;
  resources: ReadonlyMap<string, ResourceTarget>;
  /** The view's edits, and null where the effects take none. */
  edit: LeafEdit | null;
  /** The joint names of the skin's skeleton, and none while it loads. */
  joints: readonly string[];
}
const EffectRowsContext = createContext<EffectRowsContextValue | null>(null);

/** One row of the table, as a cell receives it. */
interface EffectCellProps {
  row: { id: string; original: BinRow };
}

const EFFECT_COLUMNS: DataTableColumn<BinRow>[] = [
  { id: "effect", cell: EffectCell },
  { id: "bone", cell: BoneCell },
  { id: "target", cell: TargetCell },
];
const EDIT_COLUMNS: DataTableColumn<BinRow>[] = [
  ...EFFECT_COLUMNS,
  { id: "remove", cell: RemoveCell },
];

/**
 * A row per effect: the system it resolves to, the bone it sits on, and the bone it aims at.
 * An editable row picks its bone from the skeleton and carries a remove button.
 */
export function EffectRows({
  effects,
  pages,
  resources,
  edit,
  joints,
}: EffectRowsContextValue & { effects: readonly BinRow[] }) {
  return (
    <EffectRowsContext value={{ pages, resources, edit, joints }}>
      <TableRows rows={effects} columns={edit === null ? EFFECT_COLUMNS : EDIT_COLUMNS} />
    </EffectRowsContext>
  );
}

function EffectCell({ row }: EffectCellProps) {
  const context = use(EffectRowsContext);
  if (context === null) return null;
  const fields = fieldsOf(context.pages.get(row.id));
  const key = fields(EFFECT.key);
  const name = fields(EFFECT.name);
  return (
    <Cell row={key ?? name} className="flex min-w-0 flex-1 items-center gap-2">
      <Resource effect={key} name={name} resources={context.resources} />
    </Cell>
  );
}

const BONE_CELL = "w-32 shrink-0 text-surface-400";

function BoneCell({ row }: EffectCellProps) {
  const context = use(EffectRowsContext);
  if (context === null) return null;

  const bone = fieldsOf(context.pages.get(row.id))(EFFECT.bone);
  const editProperty = context.edit?.editProperty;
  if (editProperty === undefined || context.joints.length === 0) {
    return <TextCell row={bone} className={BONE_CELL} />;
  }

  return (
    <Cell row={bone} className={BONE_CELL}>
      <JointPicker
        label={m.workshop_bin_skin_effect_bone_label()}
        joints={context.joints}
        value={textOf(bone) ?? null}
        onPick={(joint) =>
          void editProperty(row.original, EFFECT.bone, [
            { type: "setLeaf", path: "", value: { type: "string", value: joint } },
          ])
        }
      />
    </Cell>
  );
}

function TargetCell({ row }: EffectCellProps) {
  const context = use(EffectRowsContext);
  if (context === null) return null;
  const target = fieldsOf(context.pages.get(row.id))(EFFECT.targetBone);
  if (!textOf(target)) return null;
  return (
    <span className="flex w-32 shrink-0 items-center gap-1 text-surface-400">
      <ArrowRightIcon aria-hidden className="size-3 shrink-0" />
      <TextCell row={target} className="min-w-0" />
    </span>
  );
}

function RemoveCell({ row }: EffectCellProps) {
  const removeItem = use(EffectRowsContext)?.edit?.removeItem;
  if (removeItem === undefined) return null;

  const label = m.workshop_bin_skin_effect_remove_action();
  return (
    <Tooltip content={label}>
      <Button
        variant="ghost"
        size="xs"
        aria-label={label}
        left={<XIcon weight="bold" />}
        onClick={() => void removeItem(row.original)}
      />
    </Tooltip>
  );
}
