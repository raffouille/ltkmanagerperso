import type { BinDocumentId, BinRow } from "@/lib/tauri";

import type { LayoutPages } from "../../classes/components/ClassCells";
import { useBinRead } from "../../documents/hooks/useBinRead";
import { useHeldRows } from "../../tree/state/rowRegistry";
import { childCount, rowKey } from "../../tree/utils/binRows";

const NO_ROWS: readonly BinRow[] = [];

/**
 * Returns the elements of the list row `list` of `document`, and the fields of each, read when
 * the caller mounts.
 *
 * The elements are held, so a right-click on one aims the view's menu as on any row of the view.
 */
export function useListElements(
  document: BinDocumentId,
  list: BinRow | undefined,
): { elements: readonly BinRow[]; pages: LayoutPages } {
  const key = list === undefined ? null : rowKey(list);
  const requests = list === undefined || key === null ? [] : [{ key, rows: childCount(list) }];
  const elements = useBinRead(document, requests).get(key ?? "")?.rows ?? NO_ROWS;
  const pages = useBinRead(
    document,
    elements
      .filter((row) => childCount(row) > 0)
      .map((row) => ({ key: rowKey(row), rows: childCount(row) })),
  );
  useHeldRows(elements);

  return { elements, pages };
}
