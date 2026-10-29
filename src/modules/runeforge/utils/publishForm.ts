import type { PostField, PublishRequest, PublishReview } from "@/lib/tauri";

/** The post that a publish goes to: one of the account, or a new one. */
export type PublishTargetChoice = { kind: "existing"; modId: string } | { kind: "new" };

/** What the review lets the creator change before the post or the release is created. */
export interface PublishForm {
  target: PublishTargetChoice;
  tag: string;
  notes: string;
  gameVersionId: string | null;
  /** The post fields that a release refreshes from the package. */
  refresh: PostField[];
  /** Sets the status of the post to working. A release only. */
  markAsWorking: boolean;
  /** The name of a new post. */
  name: string;
  /** The category of a new post. */
  category: string;
  /** Publishes a new post once it exists. */
  publishNow: boolean;
}

/** Returns the form that the review opens with, from what the server proposed. */
export function initialPublishForm(review: PublishReview): PublishForm {
  const { prefill } = review.review;
  const target = review.choice.target;

  return {
    target: target.kind === "new" ? { kind: "new" } : { kind: "existing", modId: target.modId },
    tag: prefill.release.tag,
    notes: prefill.release.notes,
    gameVersionId: prefill.release.gameVersionId,
    refresh: [],
    markAsWorking: true,
    name: prefill.post.name,
    category: prefill.post.category,
    publishNow: false,
  };
}

/** Returns the request that `runeforge_submit` sends for `form`. */
export function publishRequest(review: PublishReview, form: PublishForm): PublishRequest {
  const uploadIds = [review.review.upload.id];

  if (form.target.kind === "existing") {
    return {
      kind: "release",
      modId: form.target.modId,
      release: {
        uploadIds,
        tag: form.tag,
        description: form.notes,
        gameVersion: form.gameVersionId,
        updatePost: form.refresh,
        markAsWorking: form.markAsWorking,
      },
    };
  }

  return {
    kind: "newMod",
    newMod: {
      uploadIds,
      post: { name: form.name, category: form.category },
      release: { tag: form.tag, notes: form.notes, gameVersionId: form.gameVersionId },
    },
    publish: form.publishNow ? {} : null,
  };
}

/** The post fields that a release may refresh, as the server lists them. */
const REFRESHABLE: readonly PostField[] = [
  "shortDescription",
  "description",
  "category",
  "license",
  "thumbnailImageId",
  "championIds",
  "mapIds",
];

/** Returns the fields of `changed` that a release may refresh from its package. */
export function refreshableFields(changed: readonly { field: PostField }[]): PostField[] {
  return changed.map(({ field }) => field).filter((field) => REFRESHABLE.includes(field));
}
