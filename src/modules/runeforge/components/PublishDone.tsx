import {
  ArrowClockwiseIcon,
  ArrowSquareOutIcon,
  CheckCircleIcon,
  CircleDashedIcon,
  WarningCircleIcon,
} from "@phosphor-icons/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-shell";
import { type ReactNode, useState } from "react";

import {
  AlertBox,
  Badge,
  type BadgeTone,
  Button,
  Code,
  Dialog,
  IconButton,
  Inline,
  OVERLINE,
  Properties,
  Property,
  RuneforgeIcon,
  Stack,
  Tooltip,
} from "@/components";
import { describeError, errorSummary, m } from "@/i18n";
import type {
  HealthStatus,
  PublishChecklist,
  PublishOutcome,
  PublishRequirement,
  PublishResult,
  RuneforgeError,
} from "@/lib/tauri";

import { runeforgeMutations, runeforgeQueries } from "../api";
import { FindingList } from "./FindingList";

/** What the done view names: the post, the tag of its release, and what the create made. */
export interface PublishSummary {
  postName: string;
  tag: string;
  /** `release` for a release of a post that existed, `post` for a new post. */
  created: "post" | "release";
  /** Whether the post was public before the publish. */
  wasPublic: boolean;
}

interface PublishDoneProps {
  outcome: PublishOutcome;
  summary: PublishSummary;
  onClose: () => void;
}

/**
 * The created post or release, in the findings-panel anatomy: a toned header off the RuneForge
 * mark, the facts of the post in an inset panel, what publishing still needs, and a confirm band.
 * The scan is read once and on "Check again". DS-REPORT-PANEL.
 */
export function PublishDone({ outcome, summary, onClose }: PublishDoneProps) {
  const [publish, setPublish] = useState<PublishResult | null>(outcome.publish);
  const isPublic = summary.wasPublic || publish?.kind === "published";

  return (
    <Dialog.Frame
      open
      onClose={onClose}
      size="xl"
      data-ui="PublishDone"
      aria-label={doneTitle(summary, isPublic)}
      className="overflow-hidden"
    >
      <header className="relative flex shrink-0 items-center gap-3 bg-linear-to-r from-success/15 to-success/0 px-4 py-3.5 select-none">
        <RuneforgeIcon className="size-10 shrink-0" />
        <Stack gap={1}>
          <h2 className="text-sm font-medium text-surface-100">{doneTitle(summary, isPublic)}</h2>
          <p className="text-meta text-surface-300 select-text">
            {summary.postName} <Code>{summary.tag}</Code>
          </p>
        </Stack>
        <span
          aria-hidden="true"
          className="pointer-events-none absolute inset-x-0 bottom-0 h-px bg-linear-to-r from-success/50 to-success/0"
        />
      </header>

      <div className="mx-2 mt-2 flex min-h-0 flex-col gap-4 overflow-y-auto rounded-xl border border-surface-700 bg-surface-950/30 px-3.5 py-3 scrollbar-md">
        <Properties className="items-center gap-y-2 text-row">
          <Property label={m.runeforge_publish_visibility_label()}>
            <VisibilityBadge isPublic={isPublic} />
          </Property>
          <ScanProperty modId={outcome.modId} releaseId={outcome.releaseId} />
          <Property label={m.runeforge_publish_stamp_label()}>
            <StampValue written={outcome.stamp.kind === "written"} />
          </Property>
        </Properties>

        {publish && publish.kind !== "published" && (
          <PublishTodo modId={outcome.modId} result={publish} onResult={setPublish} />
        )}

        {outcome.postUpdateError && (
          <AlertBox tone="warning">{m.runeforge_publish_post_update_failed_description()}</AlertBox>
        )}
      </div>

      <div className="flex shrink-0 justify-end gap-2 px-3 py-2.5 select-none">
        <Button size="lg" variant="ghost" onClick={onClose}>
          {m.runeforge_publish_close_action()}
        </Button>
        <Button
          size="lg"
          variant="filled"
          right={<ArrowSquareOutIcon weight="bold" className="size-4" />}
          onClick={() => void open(outcome.modUrl)}
        >
          {m.runeforge_publish_open_action()}
        </Button>
      </div>
    </Dialog.Frame>
  );
}

function doneTitle(summary: PublishSummary, isPublic: boolean): string {
  if (summary.created === "release") return m.runeforge_publish_done_release_title();
  if (isPublic) return m.runeforge_publish_done_published_title();
  return m.runeforge_publish_done_draft_title();
}

function VisibilityBadge({ isPublic }: { isPublic: boolean }) {
  if (isPublic) {
    return <Badge tone="success">{m.runeforge_publish_visibility_public_label()}</Badge>;
  }

  return <Badge>{m.runeforge_publish_visibility_private_label()}</Badge>;
}

function StampValue({ written }: { written: boolean }) {
  if (written) {
    return (
      <Inline gap={1.5}>
        <CheckCircleIcon weight="fill" className="size-4 shrink-0 text-success-text" />
        <span>{m.runeforge_publish_stamp_written_label()}</span>
      </Inline>
    );
  }

  return (
    <Tooltip content={m.runeforge_publish_stamp_failed_description()}>
      <Inline gap={1.5}>
        <WarningCircleIcon weight="fill" className="size-4 shrink-0 text-warning-text" />
        <span>{m.runeforge_publish_stamp_failed_label()}</span>
      </Inline>
    </Tooltip>
  );
}

function ScanProperty({ modId, releaseId }: { modId: string; releaseId: string }) {
  const release = useQuery(runeforgeQueries.release(modId, releaseId));
  const artifact = release.data?.artifacts[0];
  const status: HealthStatus = artifact?.health?.status ?? "pending";

  return (
    <Property label={m.runeforge_publish_scan_label()}>
      <Stack gap={2}>
        <Inline gap={1.5}>
          <Badge tone={HEALTH_TONE[status]}>{healthLabel(status)}</Badge>
          <Tooltip content={m.runeforge_publish_check_again_action()}>
            <IconButton
              size="xs"
              variant="ghost"
              icon={
                <ArrowClockwiseIcon
                  weight="bold"
                  className={release.isFetching ? "animate-spin" : undefined}
                />
              }
              aria-label={m.runeforge_publish_check_again_action()}
              onClick={() => void release.refetch()}
            />
          </Tooltip>
          {release.error && (
            <span className="text-meta text-danger-text select-text">
              {errorSummary(release.error)}
            </span>
          )}
        </Inline>
        <FindingList
          label={m.runeforge_publish_findings_label()}
          findings={artifact?.health?.findings ?? []}
        />
      </Stack>
    </Property>
  );
}

interface PublishTodoProps {
  modId: string;
  result: Exclude<PublishResult, { kind: "published" }>;
  onResult: (result: PublishResult) => void;
}

/** What the post still needs before it goes public, one row per requirement. */
function PublishTodo({ modId, result, onResult }: PublishTodoProps) {
  return (
    <Stack gap={2}>
      <span className={`${OVERLINE} select-none`}>{m.runeforge_publish_todo_title()}</span>
      {result.kind === "failed" && <PublishFailed error={result.error} />}
      {result.kind === "refused" && (
        <Stack as="ul" gap={2.5}>
          {result.checklist.missing.map((requirement) => (
            <RequirementRow
              key={requirement}
              modId={modId}
              requirement={requirement}
              checklist={result.checklist}
              onResult={onResult}
            />
          ))}
          {result.checklist.path === "review" && (
            <TodoRow
              title={m.runeforge_publish_review_row_title()}
              detail={m.runeforge_publish_refused_review_description()}
              action={result.reviewUrl && <OpenButton url={result.reviewUrl} />}
            />
          )}
        </Stack>
      )}
    </Stack>
  );
}

function PublishFailed({ error }: { error: RuneforgeError }) {
  const appError = { code: "RUNEFORGE" as const, error };

  return (
    <AlertBox tone="danger" title={describeError(appError).title}>
      {errorSummary(appError)}
    </AlertBox>
  );
}

interface RequirementRowProps {
  modId: string;
  requirement: PublishRequirement;
  checklist: PublishChecklist;
  onResult: (result: PublishResult) => void;
}

function RequirementRow({ modId, requirement, checklist, onResult }: RequirementRowProps) {
  if (requirement === "ai_disclosure") {
    return <AiDisclosureRow modId={modId} checklist={checklist} onResult={onResult} />;
  }

  return <TodoRow title={requirementLabel(requirement)} detail={requirementDetail(requirement)} />;
}

function AiDisclosureRow({ modId, checklist, onResult }: Omit<RequirementRowProps, "requirement">) {
  const client = useQueryClient();
  const publish = useMutation(runeforgeMutations.publish(client));
  const pendingAnswer = publish.isPending
    ? publish.variables?.answer.hasAiGeneratedPostContent
    : undefined;

  function answer(hasAiGeneratedPostContent: boolean) {
    publish.mutate(
      {
        modId,
        answer: {
          hasAiGeneratedPostContent,
          expectedVersion: checklist.aiContentDisclosureVersion,
        },
      },
      { onSuccess: onResult },
    );
  }

  const hint = checklist.aiModeratorEnforced
    ? m.runeforge_publish_ai_enforced_hint()
    : m.runeforge_publish_ai_description();

  return (
    <TodoRow
      title={m.runeforge_publish_missing_ai_disclosure_label()}
      detail={publish.error ? errorSummary(publish.error) : hint}
      action={
        <Inline gap={1}>
          <Button
            size="sm"
            variant="tonal"
            loading={pendingAnswer === true}
            disabled={publish.isPending}
            onClick={() => answer(true)}
          >
            {m.runeforge_publish_ai_yes_action()}
          </Button>
          {!checklist.aiModeratorEnforced && (
            <Button
              size="sm"
              variant="tonal"
              loading={pendingAnswer === false}
              disabled={publish.isPending}
              onClick={() => answer(false)}
            >
              {m.runeforge_publish_ai_no_action()}
            </Button>
          )}
        </Inline>
      }
    />
  );
}

interface TodoRowProps {
  title: string;
  detail: string;
  action?: ReactNode;
}

/** One thing the post still needs, with what the creator does about it. */
function TodoRow({ title, detail, action }: TodoRowProps) {
  return (
    <li className="flex items-center gap-3">
      <CircleDashedIcon weight="bold" className="size-4 shrink-0 text-warning-text" />
      <div className="min-w-0 flex-1">
        <p className="text-row font-medium text-surface-100 select-none">{title}</p>
        <p className="text-meta text-surface-400 select-text">{detail}</p>
      </div>
      {action}
    </li>
  );
}

function OpenButton({ url }: { url: string }) {
  return (
    <Button
      size="sm"
      variant="tonal"
      right={<ArrowSquareOutIcon weight="bold" className="size-3.5" />}
      onClick={() => void open(url)}
    >
      {m.runeforge_publish_open_post_action()}
    </Button>
  );
}

const HEALTH_TONE: Record<HealthStatus, BadgeTone> = {
  ok: "success",
  warnings: "warning",
  errors: "danger",
  pending: "neutral",
};

function healthLabel(status: HealthStatus): string {
  switch (status) {
    case "ok":
      return m.runeforge_publish_scan_ok_label();
    case "warnings":
      return m.runeforge_publish_scan_warnings_label();
    case "errors":
      return m.runeforge_publish_scan_errors_label();
    case "pending":
      return m.runeforge_publish_scan_pending_label();
  }
}

function requirementLabel(requirement: Exclude<PublishRequirement, "ai_disclosure">): string {
  switch (requirement) {
    case "thumbnail":
      return m.runeforge_publish_missing_thumbnail_label();
    case "release":
      return m.runeforge_publish_missing_release_label();
    case "license":
      return m.runeforge_publish_missing_license_label();
    case "unknown":
      return m.runeforge_publish_missing_unknown_label();
  }
}

function requirementDetail(requirement: Exclude<PublishRequirement, "ai_disclosure">): string {
  switch (requirement) {
    case "thumbnail":
      return m.runeforge_publish_missing_thumbnail_description();
    case "release":
      return m.runeforge_publish_missing_release_description();
    case "license":
      return m.runeforge_publish_missing_license_description();
    case "unknown":
      return m.runeforge_publish_missing_unknown_description();
  }
}
