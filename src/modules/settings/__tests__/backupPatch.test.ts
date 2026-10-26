import { describe, expect, it } from "vitest";

import { createMockSettings } from "@/test/fixtures";

import { revertPatch } from "../backupPatch";

describe("revertPatch", () => {
  it("returns the current values of the keys that differ", () => {
    const current = createMockSettings();
    const next = { ...current, minimizeToTray: !current.minimizeToTray, theme: "light" as const };

    expect(revertPatch(current, next)).toEqual({
      minimizeToTray: current.minimizeToTray,
      theme: current.theme,
    });
  });

  it("compares nested values by content", () => {
    const current = createMockSettings();
    const next = { ...current, trustedDomains: [...current.trustedDomains] };

    expect(revertPatch(current, next)).toEqual({});
  });
});
