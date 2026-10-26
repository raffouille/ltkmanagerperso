// @vitest-environment happy-dom

import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";

import type { EffectSystem } from "@/lib/tauri";
import { commandNames } from "@/test/commandNames";
import { mockInvoke } from "@/test/mocks/tauri";
import { createTestQueryClient } from "@/test/utils";

import { SpellRecipe } from "../SpellRecipe";

let effects: EffectSystem[] = [];
vi.mock("../../../documents/hooks/useBinDocument", () => ({
  useBinDocument: () => ({ state: { status: "open", handle: { document: 2 } } }),
}));
vi.mock("../../../skin/hooks/useGraphSource", () => ({
  useSkinGraphSource: () => ({
    skin: { data: { skeleton: null, effectSystems: effects, scale: 1 } },
    source: { document: 1, graph: null },
    opener: null,
  }),
}));
afterEach(cleanup);

function mount(patch: Record<string, unknown> = {}) {
  mockInvoke.mockImplementation(async (command) => {
    if (command === commandNames.preview.readSpell)
      return {
        ok: true,
        value: {
          animationName: null,
          spellCastTime: 0.25,
          castTime: null,
          missile: { movement: { kind: "fixedSpeed", speed: 1000 }, startDelay: 0.1 },
          castRangeDisplay: [825, 825, 825, 825, 825, 825, 825],
          effectKey: null,
          effectName: "Flight",
          hitEffectKey: "hit",
          haveHitEffect: true,
          issues: [],
          ...patch,
        },
      };
    if (command === commandNames.objects.declaredObjects)
      return {
        ok: true,
        value: {
          index: { status: "ready" },
          objects: { "0x1": { path: "Skin/Particles/Flight", declarations: [] } },
        },
      };
    throw new Error(command);
  });
  const onReady = vi.fn();
  render(
    <QueryClientProvider client={createTestQueryClient()}>
      <SpellRecipe
        selected={{ asset: { kind: "file", path: "spell.bin" }, entry: "spell", name: "Q" }}
        source={{ asset: { kind: "file", path: "skin.bin" }, document: 1, entry: "skin" }}
        character="Galio"
        onReady={onReady}
      />
    </QueryClientProvider>,
  );
  return onReady;
}

it("builds the initial recipe from timing, range and both effect references", async () => {
  effects = [
    { key: "flight", system: "0x1", name: null },
    { key: "hit", system: "0x2", name: null },
  ];
  const ready = mount();
  await waitFor(() => expect(ready).toHaveBeenCalled());
  expect(ready.mock.lastCall![0]).toMatchObject({
    release: 0.25,
    missileDelay: 0.1,
    target: [825, 0, 0],
    flightDuration: 0.825,
    projectileEffect: "flight",
    impactEffect: "hit",
  });
});

it("keeps timing conflicts editable and suppresses disabled impacts", async () => {
  effects = [{ key: "hit", system: "0x2", name: null }];
  const ready = mount({ castTime: 0, haveHitEffect: false });
  await waitFor(() => expect(ready).toHaveBeenCalled());
  expect(ready.mock.lastCall![0]).toMatchObject({ timingConflict: [0.25, 0], impactEffect: null });
});

it("finishes with manual effect choices when the skin has no resolvers", async () => {
  effects = [];
  const ready = mount();
  await waitFor(() => expect(ready).toHaveBeenCalled());
  expect(ready.mock.lastCall![0]).toMatchObject({ projectileEffect: null, impactEffect: null });
});
