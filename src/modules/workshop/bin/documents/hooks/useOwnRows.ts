import { useMemo } from "react";

import type { BinDocumentId, BinRow } from "@/lib/tauri";

import { objectKey, PAGE_SIZE } from "../../tree/utils/binRows";
import { useBinReadState } from "./useBinRead";

/** The rows of one object as the open document declares it. */
export interface OwnRows {
  /** The object's properties, and none while the read runs or where the document lacks it. */
  readonly rows: readonly BinRow[];
  /** The document declares no object under the hash, so a linked bin declares it, if any. */
  readonly elsewhere: boolean;
}

const NO_ROWS: readonly BinRow[] = [];

/**
 * Returns the properties of the object `entry` of `document`, and whether the document declares
 * no object under it.
 *
 * A view that edits an object reads its rows. Where another bin declares the object, the row read
 * answers `BIN_NODE_NOT_FOUND`, and the view draws the object's typed read instead (ADR-0068).
 */
export function useOwnRows(document: BinDocumentId, entry: string | null): OwnRows {
  const root = entry === null ? null : objectKey(entry);
  const requests = useMemo(() => (root === null ? [] : [{ key: root, rows: PAGE_SIZE }]), [root]);
  const read = useBinReadState(document, requests);

  return {
    rows: root === null ? NO_ROWS : (read.pages.get(root)?.rows ?? NO_ROWS),
    elsewhere: read.error?.code === "BIN_NODE_NOT_FOUND",
  };
}
