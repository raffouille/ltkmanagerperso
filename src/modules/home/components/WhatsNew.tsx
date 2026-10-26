import { CaretDownIcon, CaretUpIcon } from "@phosphor-icons/react";
import { useState } from "react";

import { Button } from "@/components";
import { m } from "@/i18n";
import { useAppInfo } from "@/modules/settings";
import { ReleaseHistory, useReleaseHistory } from "@/modules/updater";
import { useUpdaterSetDialogOpen, useUpdaterUpdate } from "@/stores";
import { twMerge } from "@/utils";

import { bundledReleaseNote, newerOfficialRelease, releaseDigest } from "../api";
import { Tile } from "./Tile";

/** The notes the build ships, read once: they cannot change while it runs. */
const BUNDLED = bundledReleaseNote();

const CHIP =
  "inline-flex shrink-0 items-center rounded-sm px-1.5 py-0.5 text-fine leading-tight font-medium";

/**
 * The release a digest is drawn for: the one on offer, else a newer official release, else the
 * one installed.
 */
interface HeadRelease {
  version: string;
  body: string | undefined;
  publishedAt: string | null;
  pending: boolean;
  /** A release of the official build newer than this one, which this build does not install. */
  official: boolean;
}

/**
 * The newest release as a digest of the areas it changed, over the full feed on demand.
 *
 * The pending release leads with its Update button, which opens the dialog as the title bar
 * cell does. The installed notes come from the build until the feed's own row arrives.
 */
export function WhatsNew() {
  const [expanded, setExpanded] = useState(false);

  const { data: appInfo } = useAppInfo();
  const update = useUpdaterUpdate();
  const setDialogOpen = useUpdaterSetDialogOpen();
  const { releases } = useReleaseHistory({ enabled: true, excludeVersion: update?.version });

  const installedVersion = appInfo?.version ?? BUNDLED?.version;
  const installed = releases.find((release) => release.version === installedVersion) ?? BUNDLED;
  const official = update ? null : newerOfficialRelease(releases, installedVersion);
  const head = headRelease(update, official, installed);

  const ToggleGlyph = expanded ? CaretUpIcon : CaretDownIcon;
  const toggleLabel = expanded ? m.home_changes_fewer_action() : m.home_changes_all_action();

  return (
    <Tile title={m.home_changes_title()} data-ui="WhatsNew">
      {head && (
        <div className="flex flex-col gap-4">
          <header className="flex flex-wrap items-center gap-x-2 gap-y-1 select-none">
            <h3 className="text-sm font-semibold text-surface-200 tabular-nums select-text">
              v{head.version}
            </h3>
            {head.pending && (
              <span className={twMerge(CHIP, "bg-accent-500/15 text-accent-400")}>
                {m.updater_release_pending_label()}
              </span>
            )}
            {head.official && (
              <span className={twMerge(CHIP, "bg-accent-500/15 text-accent-400")}>
                {m.home_release_official_label()}
              </span>
            )}
            {!head.pending && !head.official && (
              <span className={twMerge(CHIP, "bg-surface-700 text-surface-300")}>
                {m.updater_release_installed_label()}
              </span>
            )}
            <span className="ml-auto flex items-center gap-2">
              {head.publishedAt && (
                <time dateTime={head.publishedAt} className="text-xs text-surface-400 tabular-nums">
                  {shortDate(head.publishedAt)}
                </time>
              )}
              {head.pending && (
                <Button variant="filled" size="sm" onClick={() => setDialogOpen(true)}>
                  {m.home_release_update_action()}
                </Button>
              )}
            </span>
          </header>

          <Digest body={head.body} />
        </div>
      )}

      <Button
        variant="ghost"
        /* Puts the label, not the ghost fill, on the column edge. */
        className="-ml-3 self-start"
        aria-expanded={expanded}
        right={<ToggleGlyph weight="bold" className="size-3.5" />}
        onClick={() => setExpanded((open) => !open)}
      >
        {toggleLabel}
      </Button>

      {expanded && (
        <div className="select-none">
          <ReleaseHistory
            enabled
            excludeVersion={update?.version}
            installedVersion={installedVersion}
            placeholder={BUNDLED}
          />
        </div>
      )}
    </Tile>
  );
}

/** The areas one release changed, each with its count and what it says first. */
function Digest({ body }: { body: string | undefined }) {
  const sections = releaseDigest(body);

  return (
    <ul data-ui="WhatsNew:digest" className="flex max-w-[70ch] flex-col gap-4">
      {sections.map((section, index) => (
        <li key={`${section.title}-${index}`} className="flex min-w-0 flex-col gap-1">
          {section.title && (
            <div className="flex items-baseline gap-2 select-none">
              <span className="truncate text-sm font-semibold text-accent-300">
                {section.title}
              </span>
              <span className="shrink-0 text-xs text-surface-400 tabular-nums">
                {m.home_changes_count_label({ count: section.count })}
              </span>
            </div>
          )}
          <p className="line-clamp-2 text-sm leading-relaxed text-surface-300 select-text">
            {section.summary}
          </p>
        </li>
      ))}
    </ul>
  );
}

function headRelease(
  update: ReturnType<typeof useUpdaterUpdate>,
  official: { version: string; body: string; publishedAt: string | null } | null,
  installed: { version: string; body: string; publishedAt: string | null } | null,
): HeadRelease | null {
  if (update) {
    return {
      version: update.version,
      body: update.body ?? undefined,
      publishedAt: null,
      pending: true,
      official: false,
    };
  }

  if (official) {
    return {
      version: official.version,
      body: official.body,
      publishedAt: official.publishedAt,
      pending: false,
      official: true,
    };
  }

  if (!installed) return null;

  return { ...installed, pending: false, official: false };
}

/** The day a release shipped, in the reader's locale. */
function shortDate(publishedAt: string): string | null {
  const date = new Date(publishedAt);
  if (Number.isNaN(date.getTime())) return null;

  return date.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}
