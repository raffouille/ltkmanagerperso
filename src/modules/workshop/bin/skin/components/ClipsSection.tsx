import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { viewportQueries } from "@/modules/viewport";
import { twMerge } from "@/utils";

import type { ViewContext, WidgetProps } from "../../classes/components/ClassCells";
import { nameHash } from "../../shared/utils/binHash";
import { useSkinGraphSource } from "../hooks/useGraphSource";
import { ClipsPane, ClipTabs } from "./ClipTable";

/** The graph's own class, whose layout reads the graph out of the object itself. */
const ANIMATION_GRAPH = nameHash("AnimationGraphData");

/** The tallest a stack's clip table stands, so no section owns the page. */
const BOX_HEIGHT = "max-h-72";

interface ClipsHostProps {
  readonly view: ViewContext;
  /** The skin object, or the graph object where the view is the graph's own. */
  readonly entry: string;
  readonly className?: string;
}

/**
 * The clips pane over whichever object the view holds: a skin's graph, found through the
 * skin, or the graph itself.
 */
export function ClipsHost({ view, entry, className }: ClipsHostProps) {
  return (
    <div className={className}>
      {view.classHash === ANIMATION_GRAPH && (
        <ClipsPane source={{ document: view.document, graph: entry }} joints={null} />
      )}
      {view.classHash !== ANIMATION_GRAPH && <SkinClips view={view} entry={entry} />}
    </div>
  );
}

/**
 * The pane over the graph a skin names, read through the source the skin's own read
 * finds, with the skin's skeleton naming the joints a mask weighs.
 */
function SkinClips({ view, entry }: { view: ViewContext; entry: string }) {
  const { skin, source } = useSkinGraphSource(view.document, entry);
  const skeleton = useQuery(viewportQueries.skeleton(skin.data?.skeleton?.asset ?? null));
  const joints = useMemo(
    () => skeleton.data?.joints.map((joint) => joint.name) ?? null,
    [skeleton.data],
  );
  return <ClipsPane source={source} joints={joints} />;
}

/**
 * The Clips section of a stack: the tabs over the tables.
 *
 * "The clips pane" in docs/ux/SKIN_EDITOR.md. A shell draws the same in a pane of its
 * own and leaves this section out.
 */
export function ClipsSection({ section, view }: WidgetProps) {
  const entry = section.rows[0]?.entry;
  if (entry === undefined) return null;

  return (
    <div className="flex flex-col gap-1.5">
      <ClipTabs />
      <ClipsHost
        view={view}
        entry={entry}
        /* DS-GROUND, DS-RADIUS */
        className={twMerge(
          "flex flex-col overflow-hidden rounded-md border border-surface-700/50 bg-surface-900",
          BOX_HEIGHT,
        )}
      />
    </div>
  );
}
