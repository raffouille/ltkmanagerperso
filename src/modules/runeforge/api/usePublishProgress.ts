import { useState } from "react";

import type { RuneforgePublishProgress } from "@/lib/tauri";
import { useTauriEvent } from "@/lib/useTauriEvent";

/** The latest progress of the publish of `projectPath`, or `null` before the first event. */
export function usePublishProgress(projectPath: string | null): RuneforgePublishProgress | null {
  const [progress, setProgress] = useState<RuneforgePublishProgress | null>(null);

  useTauriEvent<RuneforgePublishProgress>(
    projectPath ? "runeforge-publish-progress" : null,
    (payload) => {
      if (payload.projectPath === projectPath) setProgress(payload);
    },
  );

  return progress;
}
