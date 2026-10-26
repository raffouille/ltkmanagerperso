import { expect, it } from "vitest";

import type { EffectSystem, SpellPreview } from "@/lib/tauri";

import { nameHash } from "../../../shared/utils/binHash";
import { guideLines } from "../guideLines";
import { castTiming, spellEffect, spellGuides, spellImpact } from "../spellSuggestions";

const spell = (patch: Partial<SpellPreview>): SpellPreview => ({
  spellCastTime: null,
  castTime: null,
  haveHitEffect: null,
  hitEffectKey: null,
  hitEffectName: null,
  castRangeDisplay: null,
  castRange: null,
  castRangeValues: null,
  castRadius: null,
  castRadiusSecondary: null,
  castConeAngle: null,
  castConeDistance: null,
  animationName: null,
  missile: null,
  missileSpeed: null,
  effectKey: null,
  effectName: null,
  issues: [],
  ...patch,
});
const ranks = (value: number) => [999, value, 2000, 3000, 4000, 5000, 6000];

it("keeps explicit zero timing and requires a choice for disagreeing fields", () => {
  expect(castTiming(spell({}))).toEqual({ release: 0 });
  expect(castTiming(spell({ spellCastTime: 0 }))).toEqual({ release: 0 });
  expect(castTiming(spell({ castTime: 0.25 }))).toEqual({ release: 0.25 });
  expect(castTiming(spell({ spellCastTime: 0.25, castTime: 0.250000001 }))).toEqual({
    release: 0.25,
  });
  expect(castTiming(spell({ spellCastTime: 0.25, castTime: 0 }))).toEqual({
    release: 0.25,
    timingConflict: [0.25, 0],
  });
});

it("prefers rank-one display range and falls back through the two cast ranges", () => {
  const fields = {
    castRangeDisplay: ranks(825),
    castRangeValues: ranks(1200),
    castRange: ranks(25000),
  };
  expect(spellGuides(spell(fields)).range).toBe(825);
  expect(spellGuides(spell({ ...fields, castRangeDisplay: ranks(0) })).range).toBe(1200);
  expect(spellGuides(spell({ castRange: ranks(700), castRangeValues: ranks(-1) })).range).toBe(700);
  expect(spellGuides(spell({ castRange: ranks(0) })).range).toBe(0);
});

it("resolves exact skin names without accepting ambiguous keys or leaf names", () => {
  const effect: EffectSystem = { key: nameHash("Flight"), system: "0x1", name: null };
  const other: EffectSystem = { key: "0x2", system: "0x2", name: null };
  const names = { "0x1": { path: "Skin/Particles/Q" }, "0x2": { path: "Other/Particles/Q" } };
  expect(spellEffect([effect], null, "Flight")).toBe(effect);
  expect(spellEffect([effect], null, "q", names)).toBe(effect);
  expect(spellEffect([effect], null, "Skin/Particles/Q", names)).toBe(effect);
  expect(spellEffect([effect], null, "Particles", names)).toBeNull();
  expect(spellEffect([effect, other], null, "Q", names)).toBeNull();
  expect(
    spellEffect([effect, { ...other, key: effect.key }], effect.key, "Skin/Particles/Q", names),
  ).toBeNull();
  expect(spellEffect([effect, other], effect.key, "Other/Particles/Q", names)).toBe(effect);
});

it("centers area rings on the target and aims the cone toward it", () => {
  const guides = spellGuides(
    spell({
      castRadius: ranks(100),
      castRadiusSecondary: ranks(200),
      castConeAngle: 90,
      castConeDistance: 400,
    }),
  );
  const [primary, secondary, cone] = guideLines(guides, [0, 30, 500]);
  expect(primary[0]).toEqual([100, 1, 500]);
  expect(secondary[0]).toEqual([200, 1, 500]);
  expect(cone[33][0]).toBeCloseTo(0);
  expect(cone[33][2]).toBeCloseTo(400);
  expect(cone[1][0]).toBeCloseTo(Math.sqrt(0.5) * 400);
  expect(guideLines(spellGuides(spell({})), [0, 0, 0])).toEqual([]);
});

it("suppresses explicitly disabled or malformed hit effects", () => {
  const effect = { key: "hit", system: "0x1", name: null };
  expect(spellImpact(spell({ hitEffectKey: "hit" }), [effect])).toBe(effect);
  expect(spellImpact(spell({ haveHitEffect: true, hitEffectKey: "hit" }), [effect])).toBe(effect);
  expect(spellImpact(spell({ haveHitEffect: false, hitEffectKey: "hit" }), [effect])).toBeNull();
  expect(
    spellImpact(
      spell({ hitEffectKey: "hit", issues: [{ path: "mSpell.bHaveHitEffect", kind: "invalid" }] }),
      [effect],
    ),
  ).toBeNull();
});
