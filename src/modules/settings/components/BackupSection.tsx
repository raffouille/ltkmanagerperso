import { DownloadSimpleIcon, UploadSimpleIcon, VaultIcon } from "@phosphor-icons/react";

import { Button, Inline, SectionCard } from "@/components";
import { m } from "@/i18n";

import { useSettingsBackup } from "../hooks";

/** The card that exports the settings to a file and restores them from one. */
export function BackupSection() {
  const backup = useSettingsBackup();

  return (
    <SectionCard
      title={m.settings_backup_title()}
      icon={<VaultIcon className="size-5" />}
      description={m.settings_backup_description()}
    >
      <Inline gap={2} wrap>
        <Button
          variant="outline"
          loading={backup.exporting}
          left={<UploadSimpleIcon weight="bold" className="size-4" />}
          onClick={() => void backup.exportTo()}
        >
          {m.settings_backup_export_action()}
        </Button>
        <Button
          variant="outline"
          loading={backup.reading}
          left={<DownloadSimpleIcon weight="bold" className="size-4" />}
          onClick={() => void backup.restoreFrom()}
        >
          {m.settings_backup_restore_action()}
        </Button>
      </Inline>
    </SectionCard>
  );
}
