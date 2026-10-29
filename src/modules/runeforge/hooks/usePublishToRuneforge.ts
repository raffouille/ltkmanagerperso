import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";

import type { PackFormat, WorkshopProject } from "@/lib/tauri";

import { runeforgeQueries } from "../api";
import { usePublishDialog } from "../state";

/**
 * Returns the action that publishes a project to RuneForge.
 *
 * Opens the publish dialog if an account is connected, else the Settings tab that connects one.
 */
export function usePublishToRuneforge(): (project: WorkshopProject, format: PackFormat) => void {
  const client = useQueryClient();
  const navigate = useNavigate();
  const open = usePublishDialog((s) => s.open);

  return (project, format) => {
    void client.fetchQuery(runeforgeQueries.status()).then((status) => {
      if (status.account) {
        open({ project, format });
        return;
      }

      void navigate({ to: "/settings", search: { tab: "workshop" } });
    });
  };
}
