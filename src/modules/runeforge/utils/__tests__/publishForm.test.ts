import { describe, expect, it } from "vitest";

import type { PublishReview } from "@/lib/tauri";

import { initialPublishForm, publishRequest } from "../publishForm";

function review(overrides: Partial<PublishReview> = {}): PublishReview {
  return {
    projectPath: "C:/projects/my-mod",
    fileName: "my-mod_1.0.1.modpkg",
    size: 1024,
    warnings: [],
    owned: [],
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
        post: {
          name: "My Mod",
          shortDescription: null,
          category: "skin",
          thumbnailImageId: null,
        },
        release: { tag: "v1.0.1", notes: "Fixes", gameVersionId: "gv-2", artifactLabel: "My Mod" },
        sources: { release: { tag: "package", notes: "package", gameVersionId: "default" } },
        findings: [],
        suggestedTag: null,
        changedPostFields: [{ field: "description" }, { field: "name" }],
      },
    },
    ...overrides,
  };
}

describe("initialPublishForm", () => {
  it("targets the stamped post with the prefilled release", () => {
    const form = initialPublishForm(review());

    expect(form.target).toEqual({ kind: "existing", modId: "mod-1" });
    expect(form.tag).toBe("v1.0.1");
    expect(form.notes).toBe("Fixes");
    expect(form.gameVersionId).toBe("gv-2");
    expect(form.markAsWorking).toBe(true);
    expect(form.refresh).toEqual([]);
    expect(form.publishNow).toBe(false);
  });

  it("targets a new post if no post matches", () => {
    const form = initialPublishForm(
      review({ choice: { target: { kind: "new" }, foreignStamp: null } }),
    );

    expect(form.target).toEqual({ kind: "new" });
    expect(form.name).toBe("My Mod");
    expect(form.category).toBe("skin");
  });

  it("keeps the tag of the package if the server suggests another", () => {
    const base = review();
    const form = initialPublishForm({
      ...base,
      review: { ...base.review, prefill: { ...base.review.prefill, suggestedTag: "v1.0.2" } },
    });

    expect(form.tag).toBe("v1.0.1");
  });
});

describe("publishRequest", () => {
  it("builds a release of the target post", () => {
    const form = { ...initialPublishForm(review()), refresh: ["description" as const] };

    expect(publishRequest(review(), form)).toEqual({
      kind: "release",
      modId: "mod-1",
      release: {
        uploadIds: ["upload-1"],
        tag: "v1.0.1",
        description: "Fixes",
        gameVersion: "gv-2",
        updatePost: ["description"],
        markAsWorking: true,
      },
    });
  });

  it("builds a new post that publishes when publish now is set", () => {
    const newPost = review({ choice: { target: { kind: "new" }, foreignStamp: null } });
    const form = { ...initialPublishForm(newPost), name: "Renamed", publishNow: true };

    expect(publishRequest(newPost, form)).toEqual({
      kind: "newMod",
      newMod: {
        uploadIds: ["upload-1"],
        post: { name: "Renamed", category: "skin" },
        release: { tag: "v1.0.1", notes: "Fixes", gameVersionId: "gv-2" },
      },
      publish: {},
    });
  });

  it("sends no publish for a new post without publish now", () => {
    const newPost = review({ choice: { target: { kind: "new" }, foreignStamp: null } });

    expect(publishRequest(newPost, initialPublishForm(newPost))).toMatchObject({
      kind: "newMod",
      publish: null,
    });
  });
});
