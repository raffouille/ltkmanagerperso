import { RuneforgeSection } from "@/modules/runeforge";

import { AuthorProfilesSection } from "./AuthorProfilesSection";
import { ProjectEditorSection } from "./ProjectEditorSection";
import { SettingsGrid } from "./SettingsGrid";
import { WorkshopPathSection } from "./WorkshopPathSection";

export function WorkshopSection() {
  return (
    <SettingsGrid>
      <WorkshopPathSection />
      <ProjectEditorSection />
      <AuthorProfilesSection />
      <RuneforgeSection />
    </SettingsGrid>
  );
}
