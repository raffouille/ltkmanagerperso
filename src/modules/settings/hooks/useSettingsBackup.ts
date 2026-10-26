import { useMutation, useQueryClient } from "@tanstack/react-query";
import { open, save } from "@tauri-apps/plugin-dialog";

import { useConfirm, useToast } from "@/components";
import { m } from "@/i18n";
import type { Settings } from "@/lib/tauri";

import { settingsKeys, useUpdateSettings } from "../api";
import { settingsMutations } from "../api/mutations";
import { revertPatch } from "../backupPatch";

const BACKUP_FILE_NAME = "ltk-manager-settings.json";

/**
 * Export the settings to a file, and restore them from one.
 *
 * A restore reads the backup over the current settings, asks for confirmation with the count of
 * settings it changes, and saves through the usual settings save. Its toast offers an undo that
 * writes back only the keys the restore changed. See "Backup" in docs/ux/SETTINGS.md.
 */
export function useSettingsBackup() {
  const queryClient = useQueryClient();
  const update = useUpdateSettings();
  const confirm = useConfirm();
  const toast = useToast();
  const exportSettings = useMutation(settingsMutations.exportSettings());
  const readBackup = useMutation(settingsMutations.readSettingsBackup());

  const filters = [{ name: m.settings_backup_file_filter_label(), extensions: ["json"] }];

  async function exportTo() {
    const path = await save({ defaultPath: BACKUP_FILE_NAME, filters });
    if (!path) return;

    exportSettings.mutate(path, {
      onSuccess: () => toast.success(m.settings_backup_exported_title(), path),
    });
  }

  async function restoreFrom() {
    const path = await open({ multiple: false, directory: false, filters });
    if (typeof path !== "string") return;

    const next = await readBackup.mutateAsync(path).catch(() => null);
    const current = queryClient.getQueryData<Settings>(settingsKeys.settings());
    if (!next || !current) return;

    const undo = revertPatch(current, next);
    const count = Object.keys(undo).length;
    if (count === 0) {
      toast.toast({ title: m.settings_backup_unchanged_title(), type: "info" });
      return;
    }

    const accepted = await confirm({
      title: m.settings_backup_confirm_title(),
      heading: m.settings_backup_confirm_heading({ count }),
      description: m.settings_backup_confirm_description(),
      confirmLabel: m.settings_backup_restore_action(),
      tone: "warning",
    });
    if (!accepted) return;

    update(next, {
      onSuccess: () =>
        toast.toast({
          title: m.settings_backup_restored_title({ count }),
          type: "success",
          action: { label: m.settings_backup_undo_action(), onClick: () => update(undo) },
        }),
    });
  }

  return {
    exportTo,
    restoreFrom,
    exporting: exportSettings.isPending,
    reading: readBackup.isPending,
  };
}
