import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { AlertBox, Button, Dialog, Stack } from "@/components";
import { describeError, m } from "@/i18n";
import { api, type AppError, type PublishRequest, type PublishReview } from "@/lib/tauri";
import { useDialog } from "@/stores/createDialogStore";

import { runeforgeMutations, runeforgeQueries, usePublishProgress } from "../api";
import { type PublishTarget, usePublishDialog } from "../state";
import { FindingList } from "./FindingList";
import { PublishDone, type PublishSummary } from "./PublishDone";
import { PublishPreparing } from "./PublishPreparing";
import { PublishReviewForm } from "./PublishReviewForm";

/**
 * Publishes a workshop project to RuneForge, in four states: preparing, review, submitting and done.
 *
 * The prepare starts when the dialog opens. A close while it runs cancels it, and a close in the
 * review deletes the upload. Per "Publishing to RuneForge" in `docs/ux/WORKSHOP.md`.
 */
export function PublishDialog() {
  const { isOpen, payload, close } = useDialog(usePublishDialog);
  if (!isOpen || !payload) return null;

  return <PublishFlow key={payload.project.path} target={payload} onClose={close} />;
}

function PublishFlow({ target, onClose }: { target: PublishTarget; onClose: () => void }) {
  const { project, format } = target;
  const client = useQueryClient();
  const [attempt, setAttempt] = useState(0);
  const prepare = useQuery(runeforgeQueries.prepare(client, project.path, format, attempt));
  const submit = useMutation(runeforgeMutations.submit(client));
  const progress = usePublishProgress(project.path);

  const review = prepare.data;

  function close() {
    if (prepare.isPending) {
      void api.runeforge.cancel(project.path);
    } else if (review && !submit.data && !submit.isPending) {
      void api.runeforge.discard(review.review.upload.id);
    }
    onClose();
  }

  function retry() {
    submit.reset();
    setAttempt((current) => current + 1);
  }

  if (submit.data && review && submit.variables) {
    return (
      <PublishDone
        outcome={submit.data}
        summary={publishSummary(review, submit.variables.request)}
        onClose={close}
      />
    );
  }

  if (review) {
    return (
      <Dialog.Shell
        open
        onClose={close}
        title={m.runeforge_publish_review_title({ name: project.displayName })}
        description={review.fileName}
        size="xl"
        closable={!submit.isPending}
      >
        <PublishReviewForm
          review={review}
          submitting={submit.isPending}
          submitError={submit.error}
          onSubmit={(request) => submit.mutate({ projectPath: project.path, request })}
          onCancel={close}
        />
      </Dialog.Shell>
    );
  }

  return (
    <Dialog.Shell
      open
      onClose={close}
      title={m.runeforge_publish_preparing_title({ name: project.displayName })}
      size="xl"
    >
      {prepare.error && <PrepareFailed error={prepare.error} onRetry={retry} onClose={close} />}
      {!prepare.error && (
        <>
          <PublishPreparing progress={progress} />
          <Dialog.Footer>
            <Button size="lg" variant="ghost" onClick={close}>
              {m.runeforge_publish_cancel_action()}
            </Button>
          </Dialog.Footer>
        </>
      )}
    </Dialog.Shell>
  );
}

interface PrepareFailedProps {
  error: AppError;
  onRetry: () => void;
  onClose: () => void;
}

function PrepareFailed({ error, onRetry, onClose }: PrepareFailedProps) {
  const copy = describeError(error);
  const findings =
    error.code === "RUNEFORGE" && error.error.kind === "PACKAGE_REJECTED"
      ? error.error.findings
      : [];

  return (
    <>
      <Dialog.Body>
        <Stack gap={3}>
          <AlertBox tone="danger" title={copy.title}>
            {copy.description}
            {copy.detail && (
              <span className="block font-mono text-meta select-text">{copy.detail}</span>
            )}
          </AlertBox>
          <FindingList label={m.runeforge_publish_findings_label()} findings={findings} />
        </Stack>
      </Dialog.Body>
      <Dialog.Footer>
        <Button size="lg" variant="ghost" onClick={onClose}>
          {m.runeforge_publish_close_action()}
        </Button>
        <Button size="lg" onClick={onRetry}>
          {m.runeforge_publish_retry_action()}
        </Button>
      </Dialog.Footer>
    </>
  );
}

/** Returns the post, the tag and what `request` created. */
function publishSummary(review: PublishReview, request: PublishRequest): PublishSummary {
  const { prefill } = review.review;

  if (request.kind === "release") {
    const post = review.owned.find((owned) => owned.id === request.modId);
    return {
      postName: post?.name ?? prefill.post.name,
      tag: request.release.tag ?? prefill.release.tag,
      created: "release",
      wasPublic: post?.isPublished ?? false,
    };
  }

  return {
    postName: request.newMod.post?.name ?? prefill.post.name,
    tag: request.newMod.release?.tag ?? prefill.release.tag,
    created: "post",
    wasPublic: false,
  };
}
