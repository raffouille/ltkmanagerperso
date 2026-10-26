import { useQuery } from "@tanstack/react-query";
import { type ReactNode, use, useMemo } from "react";

import type { BinDocumentId, BinRow, EffectSystem } from "@/lib/tauri";

import { AlsoCheck, type ViewContext } from "../../classes/components/ClassCells";
import { EFFECT } from "../../classes/utils/classLayouts";
import { useBinRead } from "../../documents/hooks/useBinRead";
import { useOwnRows } from "../../documents/hooks/useOwnRows";
import { type LeafEdit, LeafEditContext } from "../../tree/hooks/useLeafEdit";
import { childCount, entryKeyHash, fieldHash, rowKey } from "../../tree/utils/binRows";
import { skinQueries } from "../api/skinQueries";
import { useSkinBind } from "../hooks/useSkinBind";

/** The system one key of the resolver links to. */
export interface ResourceTarget {
  /** The `VfxSystemDefinitionData` object, `0x` and eight hex digits. */
  readonly hash: string;
  /** The system's path, where a table names it. */
  readonly name: string | null;
}

/** What the effects of a skin are drawn and edited against. */
export interface EffectScope {
  /** The system each key of the resolver's `resourceMap` links to, by key hash. */
  readonly resources: ReadonlyMap<string, ResourceTarget>;
  /** The view's edits, and null where the effects take none. */
  readonly edit: LeafEdit | null;
  /** The resolver object, and null where the effects take no edits. */
  readonly resolver: string | null;
  /** The joint names of the skin's skeleton, and none where the effects take no edits. */
  readonly joints: readonly string[];
}

interface ResolverScopeProps {
  /** The rows the section placed, of which the `mResourceResolver` link is one. */
  rows: readonly BinRow[];
  view: ViewContext;
  children: (scope: EffectScope) => ReactNode;
}

/**
 * The scope of the skin's resolver, which a widget over the skin's effects draws in.
 *
 * The resolver's rows are read through the view's document. If another bin declares the
 * resolver, the document answers no rows, and the keys come from the skin read, which reaches
 * that bin. The effects then take no edits, since their keys are written in that bin.
 */
export function ResolverScope({ rows, view, children }: ResolverScopeProps) {
  const resolver = rows.find((row) => fieldHash(row.path) === EFFECT.resolver);
  const entry = resolver?.value.type === "objectLink" ? resolver.value.hash : null;
  const map = useResourceMap(view.document, entry);

  if (map.elsewhere) return <ReachedResolver view={view} render={children} />;
  return <Resolved view={view} entry={entry} map={map} render={children} />;
}

interface ResolvedProps {
  view: ViewContext;
  /** The resolver's object hash, or null where the skin names none. */
  entry: string | null;
  map: ResourceMap;
  render: (scope: EffectScope) => ReactNode;
}

/** The scope of a resolver that the view's document declares. */
function Resolved({ view, entry, map, render }: ResolvedProps) {
  const resources = useMemo(() => {
    const byKey = new Map<string, ResourceTarget>();
    for (const row of map.rows) {
      const hash = entryKeyHash(row);
      if (hash !== null && row.value.type === "objectLink") {
        byKey.set(hash, { hash: row.value.hash, name: row.value.name });
      }
    }
    return byKey;
  }, [map.rows]);

  const edit = use(LeafEditContext);
  const body =
    edit !== null && entry !== null ? (
      <Editable view={view} edit={edit} resolver={entry} resources={resources} render={render} />
    ) : (
      render({ resources, edit: null, resolver: null, joints: NO_JOINTS })
    );

  if (map.key === null) return body;
  return (
    <AlsoCheck document={view.document} group={{ key: map.key, rows: map.rows }}>
      {body}
    </AlsoCheck>
  );
}

/** The scope of a resolver that another bin declares, as the skin read typed its keys. */
function ReachedResolver({
  view,
  render,
}: {
  view: ViewContext;
  render: (scope: EffectScope) => ReactNode;
}) {
  const systems = useQuery(skinQueries.skin(view.document, view.entry)).data?.effectSystems;
  const resources = useMemo(() => reachedResources(systems ?? NO_SYSTEMS), [systems]);
  const group = useMemo(
    () => ({
      key: `resolver:${view.entry}`,
      rows: NO_ROWS,
      links: [...resources.values()].map((target) => target.hash),
    }),
    [view.entry, resources],
  );

  return (
    <AlsoCheck document={view.document} group={group}>
      {render({ resources, edit: null, resolver: null, joints: NO_JOINTS })}
    </AlsoCheck>
  );
}

/** Returns the system each key of `systems` maps to, by key hash. */
export function reachedResources(
  systems: readonly EffectSystem[],
): ReadonlyMap<string, ResourceTarget> {
  return new Map(systems.map((each) => [each.key, { hash: each.system, name: each.name }]));
}

interface EditableProps {
  view: ViewContext;
  edit: LeafEdit;
  resolver: string;
  resources: ReadonlyMap<string, ResourceTarget>;
  render: (scope: EffectScope) => ReactNode;
}

/** The scope of a resolver this file declares, which reads the skeleton the pickers list. */
function Editable({ view, edit, resolver, resources, render }: EditableProps) {
  const joints = useJoints(view);
  return render({ resources, edit, resolver, joints });
}

const NO_JOINTS: readonly string[] = [];
const NO_ROWS: readonly BinRow[] = [];
const NO_SYSTEMS: readonly EffectSystem[] = [];

/** The entries of the resolver's `resourceMap`, as the view's document answers them. */
interface ResourceMap {
  readonly rows: readonly BinRow[];
  /** The map row's key, which the entries were checked under, and null before it answers. */
  readonly key: string | null;
  /** The document declares no object under the resolver's hash. */
  readonly elsewhere: boolean;
}

/**
 * Returns the entries of the resolver's `resourceMap` that `document` declares.
 *
 * The object's own rows come first, because the map's row carries how many entries
 * reading it costs. A resolver the skin names none of answers nothing.
 */
function useResourceMap(document: BinDocumentId, entry: string | null): ResourceMap {
  const { rows: roots, elsewhere } = useOwnRows(document, entry);
  const map = roots.find((row) => fieldHash(row.path) === EFFECT.resourceMap);
  const key = map === undefined ? null : rowKey(map);
  const entries = useBinRead(
    document,
    map === undefined ? [] : [{ key: rowKey(map), rows: childCount(map) }],
  );

  return {
    rows: key === null ? NO_ROWS : (entries.get(key)?.rows ?? NO_ROWS),
    key,
    elsewhere,
  };
}

/** Returns the joint names of the skin's skeleton, and none while the skeleton loads. */
function useJoints(view: ViewContext): readonly string[] {
  const { pose } = useSkinBind(view.document, view.entry);
  return useMemo(() => pose?.skeleton.joints.map((joint) => joint.name) ?? NO_JOINTS, [pose]);
}
