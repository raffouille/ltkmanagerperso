import { describe, expect, it } from "vitest";

import type { BinRow, BinValue } from "@/lib/tauri";

import { fieldsIn } from "../../../classes/components/ClassCells";
import { nameHash } from "../../../shared/utils/binHash";
import { conditionAdd, conditionList, driverSummary, itemIndex } from "../conditionEdits";

const SKIN = "0x2a1f3c7d";
const LIST = nameHash("PersistentEffectConditions");
const OWNER = nameHash("OwnerCondition");

function row(path: string, value: BinValue): BinRow {
  return {
    entry: SKIN,
    path,
    label: path,
    node: "property",
    name: path,
    unnamed: false,
    kind: null,
    value,
    declared: null,
  };
}

const struct = (className: string): BinValue => ({
  type: "struct",
  classHash: nameHash(className),
  class: className,
  len: 1,
});

describe("conditionAdd", () => {
  it("returns one edit that appends a condition with a HasBuff owner condition", () => {
    const [edit, ...rest] = conditionAdd(SKIN, 2, false);

    expect(rest).toEqual([]);
    expect(edit).toEqual({
      kind: "editProperty",
      entry: SKIN,
      holder: "",
      field: LIST,
      edits: [
        {
          type: "insertItem",
          path: "",
          item: { index: 2, key: null, class: "PersistentEffectConditionData" },
        },
        { type: "ensureProperty", path: "[2]", field: OWNER },
        {
          type: "ensurePointer",
          path: `[2].${OWNER.slice(2)}`,
          class: "HasBuffDynamicMaterialBoolDriver",
        },
      ],
    });
  });

  it("returns an insert and a write of the owner condition for a declared document", () => {
    const calls = conditionAdd(SKIN, 2, true);

    expect(calls.map((call) => call.kind)).toEqual(["insertItem", "editProperty"]);
    expect(calls[0]).toMatchObject({ path: LIST.slice(2), item: { index: 2 } });
    expect(calls[1]).toMatchObject({ holder: `${LIST.slice(2)}[2]`, field: OWNER });
  });

  it("writes the whole list in a declared document if it holds no condition", () => {
    expect(conditionAdd(SKIN, 0, true).map((call) => call.kind)).toEqual(["editProperty"]);
  });
});

describe("conditionList", () => {
  it("returns the effect list of the condition at the index", () => {
    expect(conditionList(3)).toEqual({
      holder: `${LIST.slice(2)}[3]`,
      field: nameHash("PersistentVfxs"),
      class: "PersistentVfxData",
    });
  });
});

describe("itemIndex", () => {
  it("returns the index of a list element", () => {
    expect(itemIndex(row(`${LIST.slice(2)}[4]`, struct("PersistentEffectConditionData")))).toBe(4);
  });

  it("returns null if the row is no list element", () => {
    expect(itemIndex(row(LIST.slice(2), { type: "container", len: 1, itemKind: "pointer" }))).toBe(
      null,
    );
  });
});

describe("driverSummary", () => {
  const owner = row("owner", struct("HasBuffDynamicMaterialBoolDriver"));

  it("returns the last segment of the driver's Spell", () => {
    const spell = row("spell", {
      type: "hash",
      hash: nameHash("Characters/Ahri/Spells/AhriR"),
      name: "Characters/Ahri/Spells/AhriR",
    });
    const fields = fieldsIn([{ ...spell, path: `owner.${nameHash("Spell").slice(2)}` }]);

    expect(driverSummary(owner, fields)).toEqual({
      classHash: nameHash("HasBuffDynamicMaterialBoolDriver"),
      className: "HasBuffDynamicMaterialBoolDriver",
      detail: "AhriR",
    });
  });

  it("returns the number of the driver's mDrivers", () => {
    const all = row("owner", struct("AllTrueMaterialDriver"));
    const drivers = row(`owner.${nameHash("mDrivers").slice(2)}`, {
      type: "container",
      len: 2,
      itemKind: "pointer",
    });

    expect(driverSummary(all, fieldsIn([drivers]))?.detail).toBe("(2)");
  });

  it("returns null if the pointer is null", () => {
    expect(driverSummary(row("owner", { type: "null" }), fieldsIn([]))).toBeNull();
  });
});
