import { queryOptions, skipToken } from "@tanstack/react-query";

import { api, type AppError, type AssetRef, type BinDocumentId } from "@/lib/tauri";
import { queryFnWithArgs } from "@/utils/query";

/**
 * Where an object that an open document reaches is declared, for a tab or an edit of it.
 *
 * A read of the object's value goes through the document itself. Only a tab that opens the
 * object, or an edit that writes it, asks for the bin that declares it.
 */
export const declaringQueries = {
  /**
   * The bin that declares `entry`, found the way every read of `document` finds it, and null
   * where no bin within reach declares it.
   */
  object: (document: BinDocumentId, entry: string | null) =>
    queryOptions<AssetRef | null, AppError>({
      queryKey: ["declaring", document, entry],
      queryFn: entry === null ? skipToken : queryFnWithArgs(api.bin.locateObject, document, entry),
      staleTime: Infinity,
      retry: false,
    }),
};
