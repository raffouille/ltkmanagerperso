import { EyeIcon, EyeSlashIcon, PlusIcon, XIcon } from "@phosphor-icons/react";
import { use, useCallback, useMemo, useState } from "react";

import { Button, IconButton, Tooltip } from "@/components";
import { m } from "@/i18n";
import type { BinRow } from "@/lib/tauri";

import { ClassCard } from "../../classes/components/ClassCard";
import {
  elementsOf,
  FieldRows,
  fieldsOf,
  type LayoutPages,
  None,
  type ViewContext,
  type WidgetProps,
} from "../../classes/components/ClassCells";
import { CONDITION } from "../../classes/utils/classLayouts";
import { FoldCaret } from "../../shared/components/FoldCaret";
import { LeafEditContext } from "../../tree/hooks/useLeafEdit";
import { useHeldRows } from "../../tree/state/rowRegistry";
import { fieldHash, rowKey } from "../../tree/utils/binRows";
import { useSkinCalls } from "../hooks/useEffectAdd";
import { useListElements } from "../hooks/useListElements";
import { SkinChoiceContext } from "../state/skinChoice";
import {
  conditionAdd,
  conditionList,
  type DriverSummary,
  driverSummary,
  itemIndex,
} from "../utils/conditionEdits";
import { listLength } from "../utils/effectEdits";
import { EffectList } from "./EffectList";
import { type EffectScope, ResolverScope } from "./ResolverScope";
import { SubmeshLine } from "./SubmeshLine";

/** The fields a condition's row and its effect and submesh lists draw, apart from the rest. */
const OWN_FIELDS: ReadonlySet<string> = new Set([
  CONDITION.effects,
  CONDITION.shown,
  CONDITION.hidden,
]);

const NO_ROWS: readonly BinRow[] = [];

/**
 * The skin's persistent conditions, one row each, which unfolds to the condition's effects,
 * its submesh lists and its other fields.
 *
 * "The Conditions section" in docs/ux/SKIN_EDITOR.md. The effects resolve through the
 * resolver as the idle effects do, and take edits under the same rule.
 */
export function ConditionRows({ section, pages, view }: WidgetProps) {
  const list = section.rows.find((row) => fieldHash(row.path) === CONDITION.list);
  const conditions = elementsOf(list === undefined ? [] : [list], pages);
  const [open, setOpen] = useState<ReadonlySet<number>>(() => new Set());
  const toggle = useCallback((index: number) => {
    setOpen((held) => {
      const next = new Set(held);
      if (!next.delete(index)) next.add(index);
      return next;
    });
  }, []);

  const count = listLength(list);
  const sendCalls = useSkinCalls();
  const add =
    sendCalls === null
      ? null
      : () => {
          setOpen((held) => new Set(held).add(count));
          sendCalls((declares) => conditionAdd(view.entry, count, declares));
        };

  return (
    <div data-ui="ConditionRows" className="flex flex-col">
      {conditions.length === 0 && <None />}
      <ResolverScope rows={section.rows} view={view}>
        {(scope) =>
          conditions.map((condition) => {
            const index = itemIndex(condition);
            if (index === null) return null;
            return (
              <ConditionRow
                key={rowKey(condition)}
                condition={condition}
                index={index}
                pages={pages}
                view={view}
                scope={scope}
                open={open.has(index)}
                onToggle={toggle}
              />
            );
          })
        }
      </ResolverScope>
      {add !== null && (
        <div data-ui="ConditionRows:add" className="flex min-h-6 items-center px-1.5 pt-1">
          <Button variant="ghost" size="xs" left={<PlusIcon />} onClick={add}>
            {m.workshop_bin_skin_condition_add_action()}
          </Button>
        </div>
      )}
    </div>
  );
}

interface ConditionRowProps {
  condition: BinRow;
  /** The condition's index in `PersistentEffectConditions`. */
  index: number;
  pages: LayoutPages;
  view: ViewContext;
  scope: EffectScope;
  open: boolean;
  onToggle: (index: number) => void;
}

/** One condition's row: its fold, its preview switch, its drivers, its effect count and remove. */
function ConditionRow({ condition, index, pages, view, scope, open, onToggle }: ConditionRowProps) {
  const choice = use(SkinChoiceContext);
  const removeItem = use(LeafEditContext)?.removeItem;
  const fields = fieldsOf(pages.get(rowKey(condition)));
  const driverOf = (field: string) => {
    const driver = fields(field);
    return driverSummary(driver, fieldsOf(driver && pages.get(rowKey(driver))));
  };
  const owner = driverOf(CONDITION.owner);
  const source = driverOf(CONDITION.source);
  const effects = listLength(fields(CONDITION.effects));

  const applied = choice?.conditions.has(index) ?? false;
  const Eye = applied ? EyeIcon : EyeSlashIcon;
  const remove = m.workshop_bin_skin_condition_remove_action();

  return (
    <div data-ui="ConditionRows:condition" className="flex flex-col">
      {/* DS-VEIL, DS-RADIUS */}
      <div
        data-row-key={rowKey(condition)}
        className="flex min-h-6 items-center gap-1 rounded-sm px-1.5 hover:bg-surface-veil-soft"
      >
        <FoldCaret
          open={open}
          onToggle={() => onToggle(index)}
          label={m.workshop_bin_row_fields_action()}
        />
        {choice !== null && (
          <IconButton
            size="row"
            pressed={applied}
            icon={<Eye />}
            label={m.workshop_bin_skin_condition_preview_label()}
            onClick={() => choice.toggleCondition(index)}
          />
        )}
        <span className="flex min-w-0 flex-1 items-center gap-1.5">
          {owner === null && source === null && (
            <span className="text-surface-400">{m.workshop_bin_skin_condition_always_label()}</span>
          )}
          {owner !== null && <Driver summary={owner} />}
          {source !== null && (
            <>
              <span className="shrink-0 text-surface-400">
                {m.workshop_bin_skin_condition_source_label()}
              </span>
              <Driver summary={source} />
            </>
          )}
        </span>
        <span className="shrink-0 text-meta text-surface-400">
          {m.workshop_bin_skin_condition_effects_value({ count: effects })}
        </span>
        {removeItem !== undefined && (
          <Tooltip content={remove}>
            <Button
              variant="ghost"
              size="xs"
              aria-label={remove}
              left={<XIcon weight="bold" />}
              onClick={() => void removeItem(condition)}
            />
          </Tooltip>
        )}
      </div>
      {open && (
        <ConditionBody
          condition={condition}
          index={index}
          pages={pages}
          view={view}
          scope={scope}
        />
      )}
    </div>
  );
}

/** A driver as its class, and the value that tells it apart from another of its class. */
function Driver({ summary }: { summary: DriverSummary }) {
  return (
    <span className="flex min-w-0 items-center gap-1.5">
      <ClassCard classHash={summary.classHash} name={summary.className} />
      {summary.detail !== null && (
        <span className="min-w-0 truncate text-surface-200 select-text">{summary.detail}</span>
      )}
    </span>
  );
}

interface ConditionBodyProps {
  condition: BinRow;
  index: number;
  pages: LayoutPages;
  view: ViewContext;
  scope: EffectScope;
}

/** An unfolded condition: its effects, its submesh lists and the rest of its fields. */
function ConditionBody({ condition, index, pages, view, scope }: ConditionBodyProps) {
  const rows = pages.get(rowKey(condition))?.rows ?? NO_ROWS;
  const fields = fieldsOf(pages.get(rowKey(condition)));
  const vfxs = fields(CONDITION.effects);
  const rest = rows.filter((row) => !OWN_FIELDS.has(fieldHash(row.path)));
  const owner = condition.value.type === "struct" ? condition.value.classHash : null;

  return (
    <div data-ui="ConditionRows:body" className="flex flex-col gap-0.5 pb-1 pl-5">
      <ConditionEffects view={view} vfxs={vfxs} scope={scope} index={index} />
      <SubmeshLine
        condition={condition}
        field={CONDITION.shown}
        list={fields(CONDITION.shown)}
        label={m.workshop_bin_skin_condition_shown_label()}
        view={view}
      />
      <SubmeshLine
        condition={condition}
        field={CONDITION.hidden}
        list={fields(CONDITION.hidden)}
        label={m.workshop_bin_skin_condition_hidden_label()}
        view={view}
      />
      {rest.length > 0 && <FieldRows rows={rest} owner={owner} />}
    </div>
  );
}

interface ConditionEffectsProps {
  view: ViewContext;
  /** The condition's `PersistentVfxs`, and undefined where the condition lacks it. */
  vfxs: BinRow | undefined;
  scope: EffectScope;
  index: number;
}

/** The condition's effects, drawn and edited as the idle effects are. */
function ConditionEffects({ view, vfxs, scope, index }: ConditionEffectsProps) {
  const { elements, pages } = useListElements(view.document, vfxs);
  const fields = useMemo(() => [...pages.values()].flatMap((page) => page.rows), [pages]);
  useHeldRows(fields);
  const path = useMemo(() => conditionList(index), [index]);

  return (
    <EffectList
      view={view}
      scope={scope}
      path={path}
      effects={elements}
      pages={pages}
      count={listLength(vfxs)}
    />
  );
}
