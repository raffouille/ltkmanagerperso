import type { BinEdit, BinRow, NewItem, ValueEdit } from "@/lib/tauri";

import type { FieldsOf } from "../../classes/components/ClassCells";
import { CONDITION } from "../../classes/utils/classLayouts";
import type { EffectListPath } from "./effectEdits";

/** The class of one element of `PersistentEffectConditions`. */
const CONDITION_CLASS = "PersistentEffectConditionData";

/** The class of one element of `PersistentVfxs`. */
const EFFECT_CLASS = "PersistentVfxData";

/** The class of the owner condition that an added condition starts with. */
const FIRST_DRIVER = "HasBuffDynamicMaterialBoolDriver";

/** Returns the hash path of the condition at `index`, relative to the skin object. */
function conditionPath(index: number): string {
  return `${CONDITION.list.slice(2)}[${index}]`;
}

/** Returns `PersistentVfxs` of the condition at `index`, which an effect add appends to. */
export function conditionList(index: number): EffectListPath {
  return { holder: conditionPath(index), field: CONDITION.effects, class: EFFECT_CLASS };
}

/** Returns the index of the list element `row`, and null if `row` is not one. */
export function itemIndex(row: BinRow): number | null {
  const step = /\[(\d+)\]$/.exec(row.path)?.[1];
  return step === undefined ? null : Number(step);
}

/**
 * Returns the calls that append a condition to the skin `skin`, whose list holds `count`, with
 * a `HasBuffDynamicMaterialBoolDriver` owner condition.
 *
 * A document that saves its own bytes takes one call, which is one undo step. A declared
 * document writes the whole value of the property that an `editProperty` names, so it takes an
 * insert and then a write of the owner condition, and the game's conditions stay out of the
 * layer. A list that holds no condition is written whole.
 */
export function conditionAdd(skin: string, count: number, declares: boolean): BinEdit[] {
  const item: NewItem = { index: count, key: null, class: CONDITION_CLASS };

  if (!declares || count === 0) {
    const at = `[${count}]`;
    const edits: ValueEdit[] = [
      { type: "insertItem", path: "", item },
      { type: "ensureProperty", path: at, field: CONDITION.owner },
      { type: "ensurePointer", path: `${at}.${CONDITION.owner.slice(2)}`, class: FIRST_DRIVER },
    ];
    return [{ kind: "editProperty", entry: skin, holder: "", field: CONDITION.list, edits }];
  }

  return [
    { kind: "insertItem", entry: skin, path: CONDITION.list.slice(2), item },
    {
      kind: "editProperty",
      entry: skin,
      holder: conditionPath(count),
      field: CONDITION.owner,
      edits: [{ type: "ensurePointer", path: "", class: FIRST_DRIVER }],
    },
  ];
}

/** Returns the edits of a submesh list that append the submesh `name`, at `count`. */
export function submeshAppend(count: number, name: string): ValueEdit[] {
  return [
    { type: "insertItem", path: "", item: { index: count, key: null, class: null } },
    { type: "setLeaf", path: `[${count}]`, value: { type: "hash", text: name } },
  ];
}

/** One driver of a condition, as its row in the inspector names it. */
export interface DriverSummary {
  readonly classHash: string;
  readonly className: string | null;
  /** The value that tells two drivers of one class apart, and null where none does. */
  readonly detail: string | null;
}

/**
 * Returns the summary of the driver that the field row `driver` points to, and null if the
 * pointer is null or missing. `fields` reads the driver's own fields.
 *
 * The detail is the last segment of `Spell` where the driver has one, and the number of
 * `mDrivers` where the driver has those.
 */
export function driverSummary(driver: BinRow | undefined, fields: FieldsOf): DriverSummary | null {
  if (driver?.value.type !== "struct") return null;

  const spell = fields(CONDITION.spell);
  const drivers = fields(CONDITION.drivers);
  let detail: string | null = null;
  if (spell?.value.type === "hash") {
    detail = spell.value.name?.split("/").at(-1) ?? spell.value.hash;
  } else if (drivers?.value.type === "container") {
    detail = `(${drivers.value.len})`;
  }

  return { classHash: driver.value.classHash, className: driver.value.class, detail };
}
