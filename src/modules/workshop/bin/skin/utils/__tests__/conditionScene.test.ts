import { describe, expect, it } from "vitest";

import type { EffectSystem, JointEffect, PersistentCondition } from "@/lib/tauri";

import { nameHash } from "../../../shared/utils/binHash";
import { conditionedHidden, conditionEffects, wornEffects } from "../conditionScene";

function effect(key: string): JointEffect {
  return { effectKey: key, bone: "Root", targetBone: "", position: [0, 0, 0] };
}

function condition(over: Partial<PersistentCondition> = {}): PersistentCondition {
  return { effects: [], shown: [], hidden: [], ...over };
}

const SYSTEMS: EffectSystem[] = [
  { key: "0x1", system: "0xa", name: null },
  { key: "0x2", system: "0xb", name: null },
  { key: "0x3", system: "0xc", name: null },
];

const SUBMESHES = ["Body", "Cape", "Wings"];

describe("wornEffects", () => {
  it("resolves each key through the systems", () => {
    expect(wornEffects([effect("0x2")], SYSTEMS, "idle")).toEqual([
      { key: "idle:0:0x2", effect: effect("0x2"), system: "0xb" },
    ]);
  });

  it("skips an effect whose key no file maps to a system", () => {
    expect(
      wornEffects([effect("0x9"), effect("0x1")], SYSTEMS, "idle").map((e) => e.system),
    ).toEqual(["0xa"]);
  });
});

describe("conditionEffects", () => {
  it("returns the effects of the active conditions alone, in list order", () => {
    const conditions = [
      condition({ effects: [effect("0x1")] }),
      condition({ effects: [effect("0x2"), effect("0x3")] }),
    ];

    expect(conditionEffects(conditions, new Set([1]), SYSTEMS).map((each) => each.system)).toEqual([
      "0xb",
      "0xc",
    ]);
  });
});

describe("conditionedHidden", () => {
  it("returns `hidden` unchanged if no condition is active", () => {
    const hidden = ["Wings"];
    const conditions = [condition({ shown: [nameHash("Wings")] })];

    expect(conditionedHidden(hidden, SUBMESHES, conditions, new Set())).toBe(hidden);
  });

  it("shows the submeshes of an active show set and hides those of a hide set", () => {
    const conditions = [condition({ shown: [nameHash("wings")], hidden: [nameHash("Cape")] })];

    expect(conditionedHidden(["Wings"], SUBMESHES, conditions, new Set([0]))).toEqual(["Cape"]);
  });

  it("hides a submesh that one active condition shows and another hides", () => {
    const conditions = [
      condition({ shown: [nameHash("Wings")] }),
      condition({ hidden: [nameHash("Wings")] }),
    ];

    expect(conditionedHidden(["Wings"], SUBMESHES, conditions, new Set([0, 1]))).toEqual(["Wings"]);
  });

  it("skips a hash that matches no submesh of the mesh", () => {
    const conditions = [condition({ hidden: [nameHash("Hat")] })];

    expect(conditionedHidden([], SUBMESHES, conditions, new Set([0]))).toEqual([]);
  });
});
