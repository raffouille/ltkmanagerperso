import { CheckIcon, CircleIcon, SpinnerGapIcon } from "@phosphor-icons/react";

import { Dialog, Inline, ProgressBar, Stack } from "@/components";
import { m } from "@/i18n";
import type { RuneforgePublishProgress, RuneforgePublishStage } from "@/lib/tauri";
import { formatBytes } from "@/utils/formatBytes";

/** The stages a prepare runs through, in order. */
const STEPS = ["validating", "packing", "uploading", "inspecting"] as const;

type Step = (typeof STEPS)[number];

/** Where a step is relative to the stage the prepare reached. */
type StepState = "done" | "current" | "pending";

/**
 * The steps of a prepare as a checklist, with the bytes sent under the upload while it runs.
 */
export function PublishPreparing({ progress }: { progress: RuneforgePublishProgress | null }) {
  const current = currentStep(progress?.stage ?? "validating");

  return (
    <Dialog.Body>
      <Stack as="ol" gap={2.5}>
        {STEPS.map((step) => (
          <PreparingStep
            key={step}
            step={step}
            state={stepState(step, current)}
            progress={progress}
          />
        ))}
      </Stack>
    </Dialog.Body>
  );
}

interface PreparingStepProps {
  step: Step;
  state: StepState;
  progress: RuneforgePublishProgress | null;
}

function PreparingStep({ step, state, progress }: PreparingStepProps) {
  const uploading =
    step === "uploading" && state === "current" && progress !== null && progress.totalBytes > 0;

  return (
    <li className="select-none">
      <Stack gap={1.5}>
        <Inline gap={2}>
          <StepGlyph state={state} />
          <span className={STEP_TEXT[state]}>{stepLabel(step)}</span>
        </Inline>
        {uploading && (
          <Stack gap={1}>
            <ProgressBar value={(progress.bytesSent / progress.totalBytes) * 100} />
            <span className="text-meta text-surface-400 tabular-nums">
              {m.runeforge_publish_upload_bytes_label({
                sent: formatBytes(progress.bytesSent),
                total: formatBytes(progress.totalBytes),
              })}
            </span>
          </Stack>
        )}
      </Stack>
    </li>
  );
}

const STEP_TEXT: Record<StepState, string> = {
  done: "text-row text-surface-300",
  current: "text-row font-medium text-surface-100",
  pending: "text-row text-surface-500",
};

function StepGlyph({ state }: { state: StepState }) {
  if (state === "done") {
    return <CheckIcon weight="bold" className="size-4 shrink-0 text-success-text" />;
  }
  if (state === "current") {
    return (
      <SpinnerGapIcon weight="bold" className="size-4 shrink-0 animate-spin text-accent-400" />
    );
  }
  return <CircleIcon className="size-4 shrink-0 text-surface-600" />;
}

/** Returns the step that `stage` belongs to. A finished prepare reads as inspecting. */
function currentStep(stage: RuneforgePublishStage): Step {
  if (stage === "complete" || stage === "error") return "inspecting";
  return stage;
}

function stepState(step: Step, current: Step): StepState {
  const index = STEPS.indexOf(step);
  const currentIndex = STEPS.indexOf(current);
  if (index < currentIndex) return "done";
  if (index === currentIndex) return "current";
  return "pending";
}

function stepLabel(step: Step): string {
  switch (step) {
    case "validating":
      return m.runeforge_publish_stage_validating_label();
    case "packing":
      return m.runeforge_publish_stage_packing_label();
    case "uploading":
      return m.runeforge_publish_stage_uploading_label();
    case "inspecting":
      return m.runeforge_publish_stage_inspecting_label();
  }
}
