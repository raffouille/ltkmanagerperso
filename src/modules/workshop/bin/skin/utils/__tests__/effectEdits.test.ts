import { describe, expect, it } from "vitest";

import { nameHash } from "../../../shared/utils/binHash";
import {
  declaredAdd,
  groupedAdd,
  IDLE_EFFECTS,
  IDLE_LIST,
  type NewEffect,
  resolverKey,
} from "../effectEdits";

const SKIN = "0x2a1f3c7d";
const RESOLVER = "0x11223344";
const SYSTEM = "0x99887766";
const OTHER = "0x55667788";
const MAP = nameHash("resourceMap").slice(2);
const LIST = IDLE_EFFECTS.slice(2);

function entry(key: string, target: string): [string, { hash: string }] {
  return [key, { hash: target }];
}

const SPARK = { entry: SYSTEM, name: "Characters/Ahri/Skins/Skin0/Particles/Ahri_Spark" };

function add(over: Partial<NewEffect>): NewEffect {
  return {
    skin: SKIN,
    path: IDLE_LIST,
    count: 2,
    resolver: RESOLVER,
    key: { text: "Ahri_Spark", hash: nameHash("Ahri_Spark"), fresh: true },
    system: SYSTEM,
    bone: "Root",
    ...over,
  };
}

describe("resolverKey", () => {
  it("returns the key that maps to the system if the map holds one", () => {
    const resources = new Map([entry("0xaabbccdd", SYSTEM)]);

    expect(resolverKey(resources, SPARK)).toEqual({
      text: "0xaabbccdd",
      hash: "0xaabbccdd",
      fresh: false,
    });
  });

  it("returns a fresh key named after the system's last segment if no key maps to it", () => {
    expect(resolverKey(new Map(), SPARK)).toEqual({
      text: "Ahri_Spark",
      hash: nameHash("Ahri_Spark"),
      fresh: true,
    });
  });

  it("numbers the fresh key if another system holds its name", () => {
    const resources = new Map([entry(nameHash("Ahri_Spark"), OTHER)]);

    expect(resolverKey(resources, SPARK).text).toBe("Ahri_Spark2");
  });

  it("returns the system's hash as the key if no table names the system", () => {
    expect(resolverKey(new Map(), { entry: SYSTEM, name: null })).toEqual({
      text: SYSTEM,
      hash: SYSTEM,
      fresh: true,
    });
  });
});

describe("groupedAdd", () => {
  it("returns the key and the effect as one edit if the key is fresh", () => {
    const edit = groupedAdd(add({}));
    if (edit.kind !== "editProperties") throw new Error("not one grouped edit");

    expect(edit.edits.map((each) => each.entry)).toEqual([RESOLVER, SKIN]);
    expect(edit.edits[0].edits).toEqual([
      { type: "insertItem", path: "", item: { index: null, key: "Ahri_Spark", class: null } },
      {
        type: "setLeaf",
        path: `{${nameHash("Ahri_Spark").slice(2)}}`,
        value: { type: "objectLink", text: SYSTEM },
      },
    ]);
    expect(edit.edits[1].edits[0]).toEqual({
      type: "insertItem",
      path: "",
      item: { index: 2, key: null, class: "SkinCharacterDataProperties_CharacterIdleEffect" },
    });
    expect(edit.edits[1].edits).toContainEqual({
      type: "setLeaf",
      path: `[2].${nameHash("boneName").slice(2)}`,
      value: { type: "string", value: "Root" },
    });
  });

  it("returns the effect alone if the key is in the map", () => {
    const edit = groupedAdd(add({ key: { text: "0xaabbccdd", hash: "0xaabbccdd", fresh: false } }));
    if (edit.kind !== "editProperties") throw new Error("not one grouped edit");

    expect(edit.edits.map((each) => each.entry)).toEqual([SKIN]);
  });
});

describe("declaredAdd", () => {
  it("returns an insert and a write for the key, then for the effect", () => {
    const calls = declaredAdd(add({}));

    expect(calls.map((call) => call.kind)).toEqual([
      "insertItem",
      "patch",
      "insertItem",
      "editProperty",
      "editProperty",
    ]);
    expect(calls[1]).toMatchObject({ path: `${MAP}{${nameHash("Ahri_Spark").slice(2)}}` });
    expect(calls[3]).toMatchObject({ holder: `${LIST}[2]`, field: nameHash("effectKey") });
  });

  it("writes the whole list if it holds no effect", () => {
    const calls = declaredAdd(
      add({ count: 0, key: { text: "K", hash: nameHash("K"), fresh: false } }),
    );

    expect(calls).toHaveLength(1);
    expect(calls[0]).toMatchObject({ kind: "editProperty", holder: "", field: IDLE_EFFECTS });
  });

  it("addresses a list under a holder by the holder's path", () => {
    const field = nameHash("PersistentVfxs");
    const list = { holder: "87b1d303[1]", field, class: "PersistentVfxData" };
    const calls = declaredAdd(
      add({ path: list, key: { text: "K", hash: nameHash("K"), fresh: false } }),
    );
    const vfxs = `87b1d303[1].${field.slice(2)}`;

    expect(calls[0]).toMatchObject({
      kind: "insertItem",
      path: vfxs,
      item: { index: 2, key: null, class: "PersistentVfxData" },
    });
    expect(calls[1]).toMatchObject({ holder: `${vfxs}[2]`, field: nameHash("effectKey") });
  });
});
