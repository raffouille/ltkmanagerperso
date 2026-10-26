import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { useEffect } from "react";

import type { AppError, AssetRef, BinDocumentId, SkinModel } from "@/lib/tauri";

import { useBinDocument } from "../../documents/hooks/useBinDocument";
import { skinQueries } from "../api/skinQueries";

/** Where an animation graph is read from: the skin's document, and the graph's own hash. */
export interface GraphSource {
  /** The skin's document, whose read reaches a graph that a linked bin declares. */
  readonly document: BinDocumentId;
  /** The `AnimationGraphData` object, `0x` and eight hex digits, and null where the skin names none. */
  readonly graph: string | null;
}

/** What a skin's graph read needs: the skin, and the source its graph is read through. */
export interface SkinGraphSource {
  readonly skin: UseQueryResult<SkinModel, AppError>;
  readonly source: GraphSource;
}

/** Returns the skin at `entry` of `document`, and the source its graph is read through. */
export function useSkinGraphSource(document: BinDocumentId, entry: string): SkinGraphSource {
  const skin = useQuery(skinQueries.skin(document, entry));
  const graph = skin.data?.animationGraph ?? null;
  return { skin, source: { document, graph } };
}

export interface DocumentOpenerProps {
  readonly asset: AssetRef;
  /** The object the open is narrowed to, `0x` and eight hex digits, and null for the whole file. */
  readonly entry?: string | null;
  /** The handle once the file is open, and null while it opens and once this unmounts. */
  readonly onOpen: (document: BinDocumentId | null) => void;
}

/** A file held open for as long as this is mounted, its handle reported to the owner. */
export function DocumentOpener({ asset, entry = null, onOpen }: DocumentOpenerProps) {
  const { state } = useBinDocument(asset, entry);
  const opened = state.status === "open" ? state.handle.document : null;
  useEffect(() => {
    onOpen(opened);
    return () => onOpen(null);
  }, [opened, onOpen]);
  return null;
}
