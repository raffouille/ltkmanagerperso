import type { PackFormat, WorkshopProject } from "@/lib/tauri";
import { createDialogStore } from "@/stores/createDialogStore";

/** The project that a publish packs, and the format it packs it in. */
export interface PublishTarget {
  project: WorkshopProject;
  format: PackFormat;
}

export const usePublishDialog = createDialogStore<PublishTarget>();
