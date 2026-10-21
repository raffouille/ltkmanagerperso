import { describe, expect, it } from "vitest";

import type { BuiltinModSettings } from "@/lib/tauri";
import { createMockSettings } from "@/test/fixtures";

import { hasBuiltinMods, keptSkinsOf, withDecoration, withKeptSkins } from "../builtinMods";

function withBuiltinMods(builtinMods: Partial<BuiltinModSettings>) {
  const defaults: BuiltinModSettings = {
    defaultWardSkins: false,
    baseSkins: "off",
    mapSkin: "game",
    forcedMapSkin: "",
    mapDecorations: {},
  };
  return createMockSettings({ builtinMods: { ...defaults, ...builtinMods } as BuiltinModSettings });
}

describe("hasBuiltinMods", () => {
  it("counts default ward skins, which change the game alone", () => {
    expect(hasBuiltinMods(withBuiltinMods({ defaultWardSkins: true }))).toBe(true);
  });

  it("counts base skins for every champion, which change the game alone", () => {
    expect(hasBuiltinMods(withBuiltinMods({ baseSkins: "allChampions" }))).toBe(true);
  });

  it("leaves out base skins for modded champions, which only rework other mods", () => {
    expect(hasBuiltinMods(withBuiltinMods({ baseSkins: "moddedChampions" }))).toBe(false);
  });

  it("counts the classic map skin", () => {
    expect(hasBuiltinMods(withBuiltinMods({ mapSkin: "classic" }))).toBe(true);
  });

  it("counts a chosen map skin only once one is chosen", () => {
    expect(hasBuiltinMods(withBuiltinMods({ mapSkin: "forced", forcedMapSkin: "Bloom" }))).toBe(
      true,
    );
    expect(hasBuiltinMods(withBuiltinMods({ mapSkin: "forced" }))).toBe(false);
  });

  it("leaves out the game's own map skin, however a chosen one is remembered", () => {
    expect(hasBuiltinMods(withBuiltinMods({ mapSkin: "game", forcedMapSkin: "Bloom" }))).toBe(
      false,
    );
  });

  it("counts a map decoration forced off or on", () => {
    expect(
      hasBuiltinMods(withBuiltinMods({ mapDecorations: { SR_Hall_Of_Legends: "hide" } })),
    ).toBe(true);
    expect(hasBuiltinMods(withBuiltinMods({ mapDecorations: {} }))).toBe(false);
  });
});

describe("withDecoration", () => {
  it("drops a decoration set back to the game", () => {
    expect(
      withDecoration({ MSITrophy: "show", SR_Hall_Of_Legends: "hide" }, "MSITrophy", "game"),
    ).toEqual({
      SR_Hall_Of_Legends: "hide",
    });
  });

  it("sets a decoration forced off or on", () => {
    expect(withDecoration({}, "MSITrophy", "show")).toEqual({ MSITrophy: "show" });
  });
});

describe("withKeptSkins", () => {
  it("sorts the skins of a champion and drops repeats", () => {
    expect(withKeptSkins({}, "Ahri", [7, 1, 7])).toEqual({ Ahri: [1, 7] });
  });

  it("replaces another spelling of the champion", () => {
    expect(withKeptSkins({ ahri: [2], Annie: [5] }, "Ahri", [7])).toEqual({
      Annie: [5],
      Ahri: [7],
    });
  });

  it("drops the champion for null", () => {
    expect(withKeptSkins({ Ahri: [7], Annie: [5] }, "Ahri", null)).toEqual({ Annie: [5] });
  });
});

describe("keptSkinsOf", () => {
  it("reads the skins of a champion in any case", () => {
    expect(keptSkinsOf({ MonkeyKing: [3] }, "monkeyking")).toEqual([3]);
  });

  it("returns no skin for a champion the setting does not hold", () => {
    expect(keptSkinsOf({ Ahri: [7] }, "Annie")).toEqual([]);
  });
});
