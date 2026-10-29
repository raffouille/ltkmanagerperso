// @vitest-environment happy-dom

import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StrictMode } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import type { PublishOutcome, PublishReview, WorkshopProject } from "@/lib/tauri";
import { commandNames } from "@/test/commandNames";
import { mockInvoke } from "@/test/mocks/tauri";
import { renderWithProviders } from "@/test/utils";

import { usePublishDialog } from "../../state";
import { PublishDialog } from "../PublishDialog";

const PROJECT = {
  path: "C:/projects/my-mod",
  displayName: "My Mod",
} as WorkshopProject;

const REVIEW: PublishReview = {
  projectPath: PROJECT.path,
  fileName: "my-mod_1.0.1.modpkg",
  size: 1024,
  warnings: [],
  owned: [
    {
      id: "mod-1",
      name: "My Mod",
      visibility: "private",
      isPublished: false,
      updatedAt: "2026-10-27T00:00:00Z",
      latestRelease: { id: "rel-1", tag: "v1.0.0", createdAt: "2026-10-26T00:00:00Z" },
    },
  ],
  choice: { target: { kind: "stamped", modId: "mod-1" }, foreignStamp: null },
  review: {
    upload: {
      id: "upload-1",
      filename: "my-mod_1.0.1.modpkg",
      status: "inspected",
      format: "modpkg",
    },
    findings: [],
    suggestions: { modId: "mod-1", matchedBy: "distributor", nextTag: "v1.0.1" },
    prefill: {
      post: { name: "My Mod", shortDescription: null, category: "skin", thumbnailImageId: null },
      release: { tag: "v1.0.1", notes: "Fixes", gameVersionId: null, artifactLabel: "My Mod" },
      sources: { release: { tag: "package", notes: "package", gameVersionId: "default" } },
      findings: [],
      suggestedTag: null,
      changedPostFields: [],
    },
  },
};

const OUTCOME: PublishOutcome = {
  modId: "mod-1",
  releaseId: "rel-2",
  modUrl: "https://runeforge.dev/mods/mod-1",
  publish: {
    kind: "refused",
    checklist: {
      path: "review",
      missing: ["thumbnail"],
      aiContentDisclosureVersion: "3",
      aiModeratorEnforced: false,
      reviewPath: "/mods/mod-1",
    },
    reviewUrl: "https://runeforge.dev/mods/mod-1",
  },
  stamp: { kind: "written" },
  postUpdateError: null,
};

type Answer = { ok: true; value: unknown } | { ok: false; error: unknown };

/** Answers each runeforge command from `answers`, and every other command with `null`. */
function mockBackend(answers: Partial<Record<string, () => Promise<Answer>>>) {
  mockInvoke.mockImplementation((command: string) => {
    const answer = answers[command];
    if (answer) return answer();
    if (command === commandNames.runeforge.runeforgeStatus) {
      return Promise.resolve({
        ok: true,
        value: { account: null, rejected: false, siteUrl: "", tokenPageUrl: "" },
      });
    }
    if (command === commandNames.runeforge.runeforgeContext) {
      return Promise.resolve({
        ok: true,
        value: { ownedPosts: [], gameVersions: [], canPublishWithoutReview: false, categories: [] },
      });
    }
    return Promise.resolve({ ok: true, value: null });
  });
}

function ok(value: unknown): () => Promise<Answer> {
  return () => Promise.resolve({ ok: true, value });
}

/* StrictMode as `main.tsx` mounts the app, so a dev build's double mount is what the tests run. */
function openDialog() {
  renderWithProviders(
    <StrictMode>
      <PublishDialog />
    </StrictMode>,
  );
  act(() => usePublishDialog.getState().open({ project: PROJECT, format: "modpkg" }));
}

function invoked(command: string) {
  return mockInvoke.mock.calls.filter(([name]) => name === command);
}

describe("PublishDialog", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  afterEach(() => {
    act(() => usePublishDialog.getState().close());
  });

  it("shows the preparing stage while the project packs", async () => {
    mockBackend({ [commandNames.runeforge.runeforgePrepare]: () => new Promise(() => {}) });

    openDialog();

    expect(await screen.findByText("Publishing My Mod")).toBeInTheDocument();
    expect(screen.getByText("Validating")).toBeInTheDocument();
    expect(invoked(commandNames.runeforge.runeforgePrepare)).toHaveLength(1);
  });

  it("shows the review with the prefilled tag", async () => {
    mockBackend({ [commandNames.runeforge.runeforgePrepare]: ok(REVIEW) });

    openDialog();

    expect(await screen.findByText("Review My Mod")).toBeInTheDocument();
    expect(screen.getByLabelText("Tag")).toHaveValue("v1.0.1");
  });

  it("fills the suggested tag if the tag is taken", async () => {
    mockBackend({
      [commandNames.runeforge.runeforgePrepare]: ok(REVIEW),
      [commandNames.runeforge.runeforgeSubmit]: () =>
        Promise.resolve({
          ok: false,
          error: { code: "RUNEFORGE", error: { kind: "TAG_TAKEN", suggestedTag: "v1.0.2" } },
        }),
    });
    const user = userEvent.setup();

    openDialog();
    await user.click(await screen.findByRole("button", { name: "Create release" }));
    await user.click(await screen.findByRole("button", { name: "Use v1.0.2" }));

    expect(screen.getByLabelText("Tag")).toHaveValue("v1.0.2");
  });

  it("shows the checklist if the publish is refused", async () => {
    mockBackend({
      [commandNames.runeforge.runeforgePrepare]: ok(REVIEW),
      [commandNames.runeforge.runeforgeSubmit]: ok(OUTCOME),
      [commandNames.runeforge.runeforgeRelease]: ok({ id: "rel-2", tag: "v1.0.1", artifacts: [] }),
    });
    const user = userEvent.setup();

    openDialog();
    await user.click(await screen.findByRole("button", { name: "Create release" }));

    expect(await screen.findByText("Before it goes public")).toBeInTheDocument();
    expect(screen.getByText("A thumbnail")).toBeInTheDocument();
    expect(screen.getByText("Open post page")).toBeInTheDocument();
  });

  it("deletes the upload if the review is closed", async () => {
    mockBackend({ [commandNames.runeforge.runeforgePrepare]: ok(REVIEW) });
    const user = userEvent.setup();

    openDialog();
    await screen.findByText("Review My Mod");
    await user.click(screen.getByRole("button", { name: "Cancel" }));

    await waitFor(() => {
      expect(invoked(commandNames.runeforge.runeforgeDiscard)).toEqual([
        [commandNames.runeforge.runeforgeDiscard, { uploadId: "upload-1" }],
      ]);
    });
  });
});
