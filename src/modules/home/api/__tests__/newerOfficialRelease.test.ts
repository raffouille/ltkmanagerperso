import { describe, expect, it } from "vitest";

import type { ReleaseNote } from "@/lib/tauri";

import { newerOfficialRelease } from "../newerOfficialRelease";

function release(version: string, prerelease = false): ReleaseNote {
  return {
    version,
    tag: `v${version}`,
    body: "",
    publishedAt: null,
    prerelease,
    url: `https://github.com/LeagueToolkit/ltk-manager/releases/tag/v${version}`,
  };
}

describe("newerOfficialRelease", () => {
  it("returns the newest release later than the installed version", () => {
    const releases = [release("1.30.4"), release("1.30.5"), release("1.29.0")];

    expect(newerOfficialRelease(releases, "1.29.0")?.version).toBe("1.30.5");
  });

  it("compares versions by number rather than by text", () => {
    expect(newerOfficialRelease([release("1.10.0")], "1.9.3")?.version).toBe("1.10.0");
  });

  it("returns null if the installed version is the newest", () => {
    expect(newerOfficialRelease([release("1.30.5"), release("1.29.0")], "1.30.5")).toBeNull();
  });

  it("leaves out a pre-release", () => {
    expect(newerOfficialRelease([release("1.31.0-rc.1", true)], "1.30.5")).toBeNull();
  });
});
