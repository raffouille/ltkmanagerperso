import {
  BooksIcon,
  DatabaseIcon,
  GearIcon,
  InfoIcon,
  KeyboardIcon,
  PaletteIcon,
  PlugsConnectedIcon,
  PuzzlePieceIcon,
  SpinnerGapIcon,
} from "@phosphor-icons/react";
import { getRouteApi } from "@tanstack/react-router";
import { type ReactNode } from "react";

import { LootIcon, PatcherIcon, ReadingColumn, Tabs } from "@/components";
import { m } from "@/i18n";
import { MigrationPanel } from "@/modules/migration";
import {
  AboutSection,
  AppearanceSection,
  BackupSection,
  BuiltinModsSection,
  CacheSection,
  DiskUsageSection,
  DEFAULT_SETTINGS_TAB,
  GeneralSection,
  HotkeySection,
  IntegrationsSection,
  LibrarySection,
  PatchingSection,
  SettingFocusProvider,
  SETTINGS_TAB_LABELS,
  type SettingsTab,
  useAppInfo,
  useSettings,
  WorkshopSection,
} from "@/modules/settings";

const routeApi = getRouteApi("/settings");

const TABS: { value: SettingsTab; icon: ReactNode }[] = [
  { value: "general", icon: <GearIcon className="size-5 shrink-0" /> },
  { value: "library", icon: <BooksIcon className="size-5 shrink-0" /> },
  { value: "workshop", icon: <LootIcon className="size-5 shrink-0" /> },
  { value: "builtins", icon: <PuzzlePieceIcon className="size-5 shrink-0" /> },
  { value: "integrations", icon: <PlugsConnectedIcon className="size-5 shrink-0" /> },
  { value: "patching", icon: <PatcherIcon className="size-5 shrink-0" /> },
  { value: "cache", icon: <DatabaseIcon className="size-5 shrink-0" /> },
  { value: "hotkeys", icon: <KeyboardIcon className="size-5 shrink-0" /> },
  { value: "appearance", icon: <PaletteIcon className="size-5 shrink-0" /> },
  { value: "about", icon: <InfoIcon className="size-5 shrink-0" /> },
];

export function Settings() {
  const { firstRun, tab } = routeApi.useSearch();
  const navigate = routeApi.useNavigate();
  const { data: settings, isLoading } = useSettings();
  const { data: appInfo } = useAppInfo();

  if (isLoading || !settings) {
    return (
      <div className="flex h-full items-center justify-center">
        <SpinnerGapIcon className="size-8 animate-spin text-accent-500" />
      </div>
    );
  }

  function selectTab(value: unknown) {
    /* Replace, because a tab is not a place a reader wants Back to walk through.
       Back leaves settings. */
    void navigate({ search: (prev) => ({ ...prev, tab: value as SettingsTab }), replace: true });
  }

  return (
    <div className="flex h-full flex-col">
      <Tabs.Root
        value={tab ?? DEFAULT_SETTINGS_TAB}
        onValueChange={selectTab}
        orientation="vertical"
        className="flex min-h-0 flex-1 flex-row"
      >
        <Tabs.List
          variant="rail"
          className="w-52 shrink-0 border-r border-surface-700/50 bg-surface-800/40 p-3"
        >
          {TABS.map((item) => (
            <Tabs.Tab key={item.value} value={item.value}>
              {item.icon}
              {SETTINGS_TAB_LABELS[item.value]}
            </Tabs.Tab>
          ))}
        </Tabs.List>

        <div className="min-h-0 flex-1 overflow-auto">
          <SettingFocusProvider>
            <Tabs.Panel value="general">
              <ReadingColumn gap={8}>
                {firstRun && !settings.leaguePath && (
                  <div className="flex items-start gap-3 rounded-xl border border-accent-500/30 bg-accent-500/10 p-5">
                    <InfoIcon className="mt-0.5 size-5 shrink-0 text-accent-400" />
                    <div>
                      <h3 className="font-medium text-accent-300">{m.settings_welcome_title()}</h3>
                      <p className="mt-1 text-sm text-surface-400">
                        {m.settings_welcome_description()}
                      </p>
                    </div>
                  </div>
                )}
                <GeneralSection migration={<MigrationPanel />} />
                <BackupSection />
              </ReadingColumn>
            </Tabs.Panel>

            <Tabs.Panel value="library">
              <ReadingColumn>
                <LibrarySection />
              </ReadingColumn>
            </Tabs.Panel>

            <Tabs.Panel value="workshop">
              <ReadingColumn>
                <WorkshopSection />
              </ReadingColumn>
            </Tabs.Panel>

            <Tabs.Panel value="builtins">
              <ReadingColumn>
                <BuiltinModsSection />
              </ReadingColumn>
            </Tabs.Panel>

            <Tabs.Panel value="integrations">
              <ReadingColumn>
                <IntegrationsSection />
              </ReadingColumn>
            </Tabs.Panel>

            <Tabs.Panel value="patching">
              <ReadingColumn>
                <PatchingSection />
              </ReadingColumn>
            </Tabs.Panel>

            <Tabs.Panel value="cache">
              <ReadingColumn>
                <DiskUsageSection />
                <CacheSection />
              </ReadingColumn>
            </Tabs.Panel>

            <Tabs.Panel value="hotkeys">
              <ReadingColumn>
                <HotkeySection />
              </ReadingColumn>
            </Tabs.Panel>

            <Tabs.Panel value="appearance">
              <ReadingColumn>
                <AppearanceSection />
              </ReadingColumn>
            </Tabs.Panel>

            <Tabs.Panel value="about">
              <ReadingColumn>
                <AboutSection appInfo={appInfo} />
              </ReadingColumn>
            </Tabs.Panel>
          </SettingFocusProvider>
        </div>
      </Tabs.Root>
    </div>
  );
}
