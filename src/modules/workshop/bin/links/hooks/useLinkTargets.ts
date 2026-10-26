import { useQueries, type UseQueryOptions, type UseQueryResult } from "@tanstack/react-query";
import { createContext, use, useCallback, useEffect, useMemo, useState } from "react";

import {
  api,
  type AppError,
  type AssetRef,
  type BinDocumentId,
  type BinRow,
  type DeclaredObject,
  type DeclaredObjects,
  type ObjectIndexStatus,
  type SandboxRef,
} from "@/lib/tauri";
import { unwrapForQuery } from "@/utils/query";

import { layerTitle } from "../../../documents/utils/contentDocument";
/* The leaves rather than the browser barrel, which pulls the documents that route back here. */
import { gameKeys } from "../../../gameBrowser/api/keys";
import { useWarmObjectIndex } from "../../../objectsBrowser/api/useObjectIndex";
import type { OpenIntent } from "../../../palette/utils/types";
import { useOptionalProjectContext } from "../../../projects/state/ProjectContext";
import { useSandbox } from "../../../sandbox/state/SandboxContext";
import { sandboxKey } from "../../../sandbox/utils/sandboxRef";
import { pollWhileBuilding } from "../../../shared/api/indexQueries";
import { useOpenDocumentAs } from "../../../state";
import { stringQueries } from "../../../string-overrides/api/queries";
import { nameHash } from "../../shared/utils/binHash";
import { chunkPath, decideFileLink, decideObjectLink } from "../utils/linkDecision";

/** One group of rows checked together: a node's rows, or the tab's roots. */
export interface RowGroup {
  readonly key: string;
  readonly rows: readonly BinRow[];
  /** Object hashes a typed read draws as chips, checked beside the rows' own links. */
  readonly links?: readonly string[];
}

/** What a page's checks answered for the links and hashes in it. */
export interface LinkTargets {
  /** The slot the index is in, as the latest check reports it. Absent before one answers. */
  readonly index: ObjectIndexStatus | null;
  /** By object hash, `0x` and eight hex digits: what declares it, in resolution order. */
  readonly declared: ReadonlyMap<string, DeclaredObject>;
  /**
   * By resolved chunk path: the copy the build uses in the document's sandbox, where a
   * layer's copy comes before the install's. A path nothing holds is absent.
   */
  readonly located: ReadonlyMap<string, AssetRef>;
  /** By string as the file holds it: the in-game line of the string-table key it is. */
  readonly strings: ReadonlyMap<string, string>;
  /** A check is on its way for some page. */
  readonly pending: boolean;
}

export const NO_LINK_TARGETS: LinkTargets = {
  index: null,
  declared: new Map(),
  located: new Map(),
  strings: new Map(),
  pending: false,
};

/** The checks the tree ran, read by every chip in it. */
export const LinkTargetsContext = createContext<LinkTargets>(NO_LINK_TARGETS);

/** The checks of the enclosing tree. A chip outside one reads nothing as resolved. */
export function useLinkTargets(): LinkTargets {
  return use(LinkTargetsContext);
}

/** The name of the object an entry hash addresses. Outside a tree, the hash itself. */
export const ObjectNameContext = createContext<(entry: string) => string>((entry) => entry);

/** The tree's way to open a link whose target the index has not answered for. */
export interface LinkOpen {
  /** Build the index, and open `hash` with `intent` on the answer. */
  readonly wantOpen: (hash: string, intent: OpenIntent) => void;
  /** The hashes a click is waiting on. */
  readonly wanting: ReadonlySet<string>;
}

export const NO_LINK_OPEN: LinkOpen = { wantOpen: () => {}, wanting: new Set() };

/** The enclosing tree's warm-and-open, shared by a chip and the row menu. */
export const LinkOpenContext = createContext<LinkOpen>(NO_LINK_OPEN);

export function useLinkOpen(): LinkOpen {
  return use(LinkOpenContext);
}

/**
 * What opens the object `hash` names from the enclosing tree, or null where nothing does.
 *
 * A declared object opens its tab. While the index is absent or building, the open warms
 * it and lands on the answer. A hash nothing declares, a check still on its way and a null
 * `hash` open nothing.
 */
export function useObjectOpen(hash: string | null): ((intent: OpenIntent) => void) | null {
  const targets = useLinkTargets();
  const { wantOpen } = useLinkOpen();
  const open = useOpenDocumentAs();
  if (hash === null) return null;
  const decision = decideObjectLink(hash, targets);
  if (decision.kind === "chip") return (intent) => open(decision.document, intent);
  if (decision.kind === "warm") return (intent) => wantOpen(hash, intent);
  return null;
}

/**
 * What opens the chunk `path` names from the enclosing tree, or null where nothing holds it:
 * the layer's copy, else the install's. Compared lowercased, as the tables spell a chunk path.
 */
export function useChunkOpen(path: string): ((intent: OpenIntent) => void) | null {
  const targets = useLinkTargets();
  const chunk = path.toLowerCase();
  const title = useLayerTitle();
  const open = useOpenDocumentAs();
  const decision = decideFileLink(chunk, targets, title);
  if (decision.kind !== "chip") return null;
  return (intent) => open(decision.document, intent);
}

/**
 * The warm-and-open a surface of link chips provides to the chips and menus under it.
 *
 * A link clicked while the index is absent: the build runs, and the click lands on the
 * answer. A target the answer lacks is forgotten.
 */
export function useWarmLinkOpen(targets: LinkTargets): LinkOpen {
  const warm = useWarmObjectIndex();
  const open = useOpenDocumentAs();
  const [wanting, setWanting] = useState<ReadonlyMap<string, OpenIntent>>(() => new Map());

  const warmMutate = warm.mutate;
  const linkOpen = useMemo<LinkOpen>(
    () => ({
      wantOpen: (hash, intent) => {
        setWanting((current) => new Map(current).set(hash, intent));
        warmMutate();
      },
      wanting: new Set(wanting.keys()),
    }),
    [wanting, warmMutate],
  );

  useEffect(() => {
    if (targets.index?.status !== "ready" && targets.index?.status !== "failed") return;
    const settled = [...wanting].filter(([hash, intent]) => {
      const decision = decideObjectLink(hash, targets);
      if (decision.kind === "chip") open(decision.document, intent);
      return decision.kind !== "pending" && decision.kind !== "warm";
    });
    if (settled.length === 0) return;
    setWanting((current) => {
      const next = new Map(current);
      for (const [hash] of settled) next.delete(hash);
      return next;
    });
  }, [targets, open, wanting]);

  return linkOpen;
}

/**
 * A string shaped like a string-table key: one word of letters, digits, dots and at
 * least one underscore, which a plain word such as `Idle` never has.
 */
const STRING_KEY = /^(?=.*_)[\w.]+$/;

/** The strings a group's `string` values hold that could be string-table keys, sorted, each once. */
export function linkStringKeys(rows: readonly BinRow[]): string[] {
  const keys = new Set<string>();
  for (const { value } of rows) {
    if (value.type !== "string" || chunkPath(value.value) !== null) continue;
    if (STRING_KEY.test(value.value)) keys.add(value.value);
  }
  return [...keys].sort();
}

export const linkKeys = {
  declared: (
    sandbox: SandboxRef,
    document: BinDocumentId,
    key: string,
    hashes: readonly string[],
  ) => [...gameKeys.objectSearches, "links", sandboxKey(sandbox), document, key, hashes] as const,
  located: (sandbox: SandboxRef, key: string, paths: readonly string[]) =>
    [...gameKeys.dirs, "files", sandboxKey(sandbox), key, paths] as const,
};

/**
 * The object hashes a group's rows name, sorted, each once.
 *
 * A `link` and a `hash` carry theirs, and a patch target row its entry. A `string` is
 * hashed as an object path, so a string that names one resolves in the same call rather
 * than in one of its own.
 */
export function linkHashes(rows: readonly BinRow[]): string[] {
  const hashes = new Set<string>();
  for (const { node, entry, value } of rows) {
    if (node === "target") hashes.add(entry);
    if (value.type === "objectLink" || value.type === "hash") hashes.add(value.hash);
    if (value.type === "string") hashes.add(nameHash(value.value));
  }
  return [...hashes].sort();
}

/** The chunk paths a group's `file` values and its path-shaped strings name, sorted, each once. */
export function linkPaths(rows: readonly BinRow[]): string[] {
  const paths = new Set<string>();
  for (const { value } of rows) {
    if (value.type === "wadChunkLink" && value.path !== null) paths.add(value.path);
    if (value.type === "string") {
      const path = chunkPath(value.value);
      if (path !== null) paths.add(path);
    }
  }
  return [...paths].sort();
}

type DeclaredQuery = UseQueryOptions<
  DeclaredObjects,
  AppError,
  DeclaredObjects,
  ReturnType<typeof linkKeys.declared>
>;

type LocatedQuery = UseQueryOptions<
  Partial<Record<string, AssetRef>>,
  AppError,
  Partial<Record<string, AssetRef>>,
  ReturnType<typeof linkKeys.located>
>;

/** What the declared checks answered across every group. */
interface DeclaredAnswer {
  readonly index: ObjectIndexStatus | null;
  readonly objects: Readonly<Record<string, DeclaredObject>>;
  readonly pending: boolean;
}

/** What the located checks answered across every group. */
interface LocatedAnswer {
  readonly entries: Readonly<Partial<Record<string, AssetRef>>>;
  readonly pending: boolean;
}

/* Both answers are plain records rather than maps, and both combines sit at module
   scope. Structural sharing then holds one identity across a render that changed
   nothing, which is what the memo reading them depends on. */

function combineDeclared(
  results: readonly UseQueryResult<DeclaredObjects, AppError>[],
): DeclaredAnswer {
  const objects: Record<string, DeclaredObject> = {};
  let index: ObjectIndexStatus | null = null;
  let pending = false;
  for (const result of results) {
    if (result.isPending) pending = true;
    if (!result.data) continue;
    index = result.data.index;
    Object.assign(objects, result.data.objects);
  }
  return { index, objects, pending };
}

function combineLocated(
  results: readonly UseQueryResult<Partial<Record<string, AssetRef>>, AppError>[],
): LocatedAnswer {
  const entries: Partial<Record<string, AssetRef>> = {};
  let pending = false;
  for (const result of results) {
    if (result.isPending) pending = true;
    if (!result.data) continue;
    Object.assign(entries, result.data);
  }
  return { entries, pending };
}

function combineStrings(
  results: readonly UseQueryResult<Record<string, string>, AppError>[],
): Readonly<Record<string, string>> {
  const lines: Record<string, string> = {};
  for (const result of results) Object.assign(lines, result.data);
  return lines;
}

/**
 * Check every group's link, hash and `file` targets in the document's sandbox, one call
 * per group and per kind. The backend returns a layer's copy before the install's
 * (ADR-0056).
 *
 * "Links" in docs/ux/BIN_EDITOR.md. The declared checks sit under the object searches,
 * and a warm or a drop settling asks them again. A check the build has not answered
 * asks again each second.
 */
export function useCheckLinkTargets(
  document: BinDocumentId,
  groups: readonly RowGroup[],
): LinkTargets {
  const sandbox = useSandbox();

  const targets = useMemo(
    () =>
      groups.map((group) => ({
        key: group.key,
        hashes: [...new Set([...linkHashes(group.rows), ...(group.links ?? [])])].sort(),
        paths: linkPaths(group.rows),
        keys: linkStringKeys(group.rows),
      })),
    [groups],
  );

  const declaredQueries: DeclaredQuery[] = targets
    .filter((group) => group.hashes.length > 0)
    .map((group) => ({
      queryKey: linkKeys.declared(sandbox, document, group.key, group.hashes),
      queryFn: async () =>
        unwrapForQuery(await api.objects.declared(sandbox, group.hashes, document)),
      staleTime: Infinity,
      retry: false,
      refetchInterval: (query) => pollWhileBuilding(query.state.data?.index.status),
    }));
  const declaredAnswer = useQueries({ queries: declaredQueries, combine: combineDeclared });

  const locatedQueries: LocatedQuery[] = targets
    .filter((group) => group.paths.length > 0)
    .map((group) => ({
      queryKey: linkKeys.located(sandbox, group.key, group.paths),
      queryFn: async () => unwrapForQuery(await api.bin.locateFilesNear(sandbox, group.paths)),
      staleTime: Infinity,
      retry: false,
    }));
  const locatedAnswer = useQueries({ queries: locatedQueries, combine: combineLocated });

  /* Not part of `pending`: the first lookup builds the string index, and a `file` link
     waiting on it would hold off its missing mark for no reason. */
  const lines = useQueries({
    queries: targets
      .filter((group) => group.keys.length > 0)
      .map((group) => stringQueries.values(group.keys)),
    combine: combineStrings,
  });

  return useMemo(() => {
    const located = new Map<string, AssetRef>();
    for (const [path, asset] of Object.entries(locatedAnswer.entries)) {
      if (asset !== undefined) located.set(path, asset);
    }

    return {
      index: declaredAnswer.index,
      declared: new Map(Object.entries(declaredAnswer.objects)),
      located,
      strings: new Map(Object.entries(lines)),
      pending: declaredAnswer.pending || locatedAnswer.pending,
    };
  }, [declaredAnswer, locatedAnswer, lines]);
}

/**
 * The display title of a layer, which a `file` chip for a layer's copy shows. Outside a
 * project, the layer's name.
 */
export function useLayerTitle(): (layer: string) => string {
  const project = useOptionalProjectContext();
  return useCallback(
    (layer: string) => (project === null ? layer : layerTitle(project, layer)),
    [project],
  );
}

/** What a layer directory holding an archive's chunks is named. */
const WAD_DIR_SUFFIX = ".wad.client";

/** The tree's asset, which a string key and a path suggestion read. Null outside a tree. */
export const LinkAssetContext = createContext<AssetRef | null>(null);

/**
 * The chunk path a layer's file holds, or null for a file outside an archive directory.
 *
 * A layer entry is addressed from the layer root, so its first segment is the archive
 * directory. What a `file` value addresses is everything after that.
 */
export function entryChunkPath(relativePath: string): string | null {
  const cut = relativePath.indexOf("/");
  if (cut < 0) return null;
  if (!relativePath.slice(0, cut).toLowerCase().endsWith(WAD_DIR_SUFFIX)) return null;
  return relativePath.slice(cut + 1);
}
