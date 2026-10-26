// @vitest-environment happy-dom

import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import type { EffectSystem, SpellPreview } from "@/lib/tauri";
import { commandNames } from "@/test/commandNames";
import { mockInvoke } from "@/test/mocks/tauri";
import { createTestQueryClient } from "@/test/utils";

import { MissilePane } from "../MissilePane";

vi.mock("../MissileViewport", () => ({
  default: ({ document, entry }: { document: number; entry: string }) => (
    <div>{`Flight ${document} ${entry}`}</div>
  ),
}));

const PREVIEW: SpellPreview = {
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
  missileSpeed: null,
  missile: {
    movement: { kind: "fixedSpeed", speed: 5000 },
    startDelay: null,
    startBone: null,
    targetBone: "r_hand",
    targetHeight: 100,
    initialTargetHeight: 100,
    heightOffsetAtEnd: null,
    height: null,
    facing: null,
  },
  effectKey: "0x00000001",
  effectName: null,
  issues: [],
};
const EFFECT: EffectSystem = { key: "0x00000001", system: "0x12345678", name: null };

function mount(preview: SpellPreview = PREVIEW, effects: EffectSystem[] = [EFFECT]) {
  mockInvoke.mockImplementation(async (command, args) => {
    if (command === commandNames.bin.binOpen)
      return { ok: true, value: { document: args.asset.kind === "file" ? 9 : 2 } };
    if (command === commandNames.preview.readSpell) return { ok: true, value: preview };
    if (command === commandNames.objects.declaredObjects)
      return {
        ok: true,
        value: {
          index: { status: "ready" },
          objects: {
            "0x12345678": {
              path: "Characters/Sejuani/Skins/Skin0/Particles/Sejuani_Flight",
              declarations: [],
            },
          },
        },
      };
    if (command === commandNames.preview.readSkin)
      return { ok: true, value: { effectSystems: effects } };
    if (command === commandNames.bin.binClose) return { ok: true, value: null };
    throw new Error(`Unexpected command ${command}`);
  });
  render(
    <QueryClientProvider client={createTestQueryClient()}>
      <MissilePane
        asset={{ kind: "gameChunk", wad: "Sejuani.wad.client", pathHash: "1234" }}
        entry="0x859d7934"
        skin={{ document: 1, entry: "0x98765432" }}
      />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  mockInvoke.mockReset();
});
afterEach(cleanup);

it("resolves the flight key through the selected skin rather than treating it as a system", async () => {
  mount();
  expect(await screen.findByText("Flight 1 0x12345678")).toBeInTheDocument();
  expect(mockInvoke).toHaveBeenCalledWith(commandNames.preview.readSpell, {
    document: 2,
    entry: "0x859d7934",
  });
  expect(mockInvoke).toHaveBeenCalledWith(commandNames.preview.readSkin, {
    document: 1,
    entry: "0x98765432",
  });
});

it("waits for a manual choice when the spell has no flight effect", async () => {
  mount({ ...PREVIEW, effectKey: null });
  const user = userEvent.setup();
  await user.click(await screen.findByRole("combobox", { name: "Flight effect from this skin" }));
  await user.click(await screen.findByRole("option", { name: "Sejuani_Flight" }));
  expect(await screen.findByText("Flight 1 0x12345678")).toBeInTheDocument();
  expect(screen.getByText("The selected effect is a manual preview override.")).toBeInTheDocument();
});

it("refuses invalid motion without substituting a default speed", async () => {
  mount({
    ...PREVIEW,
    missile: { ...PREVIEW.missile!, movement: { kind: "fixedSpeed", speed: null } },
  });
  expect(
    await screen.findByText(/A valid fixed speed or fixed duration is required/),
  ).toBeInTheDocument();
  expect(screen.queryByText(/Flight 1/)).not.toBeInTheDocument();
});

it("shows a movement limitation instead of blaming valid coordinates", async () => {
  mount({
    ...PREVIEW,
    missile: { ...PREVIEW.missile!, movement: { kind: "unsupported", class_hash: "0x775dfd10" } },
  });
  expect(
    await screen.findByRole("heading", { name: "Movement not supported" }),
  ).toBeInTheDocument();
  expect(screen.queryByRole("spinbutton")).not.toBeInTheDocument();
  expect(await screen.findByRole("combobox")).toHaveTextContent("Sejuani_Flight");
  expect(screen.queryByText(/game:.*wad/)).not.toBeInTheDocument();
  const user = userEvent.setup();
  await user.click(screen.getByRole("button", { name: "Source details" }));
  expect(await screen.findByText("0x775dfd10")).toBeInTheDocument();
});

it("resolves an exact missile name when its key is missing", async () => {
  mount({ ...PREVIEW, effectKey: null, effectName: "Sejuani_Flight" });
  expect(await screen.findByText("Flight 1 0x12345678")).toBeInTheDocument();
});
