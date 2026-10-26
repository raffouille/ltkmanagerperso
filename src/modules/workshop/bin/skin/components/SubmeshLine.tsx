import { XIcon } from "@phosphor-icons/react";
import { useQuery } from "@tanstack/react-query";
import { use, useMemo } from "react";

import { Button, Tooltip } from "@/components";
import { m } from "@/i18n";
import type { BinRow } from "@/lib/tauri";
import { viewportQueries } from "@/modules/viewport";
import { twMerge } from "@/utils";

import type { ViewContext } from "../../classes/components/ClassCells";
import { nameHash } from "../../shared/utils/binHash";
import { LeafEditContext } from "../../tree/hooks/useLeafEdit";
import { rowKey } from "../../tree/utils/binRows";
import { skinQueries } from "../api/skinQueries";
import { useListElements } from "../hooks/useListElements";
import { submeshAppend } from "../utils/conditionEdits";
import { listLength } from "../utils/effectEdits";
import { JointPicker } from "./JointPicker";

const NO_NAMES: readonly string[] = [];

export interface SubmeshLineProps {
  /** The condition that holds the list, which an add writes under. */
  condition: BinRow;
  field: string;
  /** The submesh list, and undefined where the condition lacks it. */
  list: BinRow | undefined;
  label: string;
  view: ViewContext;
}

/**
 * One submesh list of a condition as a line of names, each removable, and a picker of the
 * mesh's submeshes that appends one.
 *
 * A hash that matches no submesh of the mesh shows as its hash.
 */
export function SubmeshLine({ condition, field, list, label, view }: SubmeshLineProps) {
  const edit = use(LeafEditContext);
  const submeshes = useSubmeshes(view);
  const { elements } = useListElements(view.document, list);
  const spelled = useMemo(
    () => new Map(submeshes.map((name) => [nameHash(name), name])),
    [submeshes],
  );
  const named = (row: BinRow) =>
    row.value.type === "hash"
      ? (spelled.get(row.value.hash) ?? row.value.name ?? row.value.hash)
      : row.name;
  const listed = new Set(elements.map((row) => (row.value.type === "hash" ? row.value.hash : "")));
  const addable = submeshes.filter((name) => !listed.has(nameHash(name)));
  const editProperty = edit?.editProperty;
  const removeItem = edit?.removeItem;

  if (elements.length === 0 && editProperty === undefined) return null;
  return (
    <div className="flex min-h-6 flex-wrap items-center gap-1 px-1.5">
      <span className="w-12 shrink-0 text-surface-400">{label}</span>
      {elements.map((row) => (
        <Chip
          key={rowKey(row)}
          row={row}
          name={named(row)}
          onRemove={removeItem === undefined ? null : () => void removeItem(row)}
        />
      ))}
      {editProperty !== undefined && addable.length > 0 && (
        <span className="w-32 shrink-0">
          <JointPicker
            label={m.workshop_bin_skin_condition_submesh_add_label()}
            placeholder={m.workshop_bin_skin_condition_submesh_add_label()}
            joints={addable}
            value={null}
            onPick={(name) =>
              void editProperty(condition, field, submeshAppend(listLength(list), name))
            }
          />
        </span>
      )}
    </div>
  );
}

/** Returns the submesh names of the skin's `.skn`, and none while the mesh loads. */
function useSubmeshes(view: ViewContext): readonly string[] {
  const skin = useQuery(skinQueries.skin(view.document, view.entry));
  const mesh = useQuery(viewportQueries.mesh(skin.data?.mesh?.asset ?? null));
  return useMemo(() => mesh.data?.ranges.map((range) => range.name) ?? NO_NAMES, [mesh.data]);
}

/** One submesh of a list, with its remove where the view edits. */
function Chip({
  row,
  name,
  onRemove,
}: {
  row: BinRow;
  name: string;
  onRemove: (() => void) | null;
}) {
  const remove = m.workshop_bin_skin_condition_submesh_remove_action({ submesh: name });
  return (
    <span
      data-row-key={rowKey(row)}
      /* DS-RADIUS */
      className={twMerge(
        "flex h-5 items-center gap-0.5 rounded-sm bg-surface-veil-soft pl-1.5 text-surface-200",
        onRemove === null && "pr-1.5",
      )}
    >
      <span className="truncate select-text">{name}</span>
      {onRemove !== null && (
        <Tooltip content={remove}>
          <Button
            variant="ghost"
            size="xs"
            aria-label={remove}
            left={<XIcon weight="bold" />}
            onClick={onRemove}
          />
        </Tooltip>
      )}
    </span>
  );
}
