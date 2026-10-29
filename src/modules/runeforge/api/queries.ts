import { type QueryClient, queryOptions } from "@tanstack/react-query";

import {
  api,
  type AppError,
  type OwnedModPost,
  type PackFormat,
  type PublishingContext,
  type PublishReview,
  type Release,
  type RuneforgeStatus,
} from "@/lib/tauri";
import { queryFn, queryFnWithArgs, unwrapForQuery } from "@/utils/query";

import { runeforgeKeys } from "./keys";

/** What the app reads from RuneForge, and the connection it reads it through. */
export const runeforgeQueries = {
  /** The stored account. Sends no request to RuneForge. */
  status: () =>
    queryOptions<RuneforgeStatus, AppError>({
      queryKey: runeforgeKeys.status(),
      queryFn: queryFn(api.runeforge.status),
    }),

  ownedMods: () =>
    queryOptions<OwnedModPost[], AppError>({
      queryKey: runeforgeKeys.ownedMods(),
      queryFn: queryFn(api.runeforge.ownedMods),
    }),

  /** The game versions and categories. They change with a patch, so a read holds for an hour. */
  context: () =>
    queryOptions<PublishingContext, AppError>({
      queryKey: runeforgeKeys.context(),
      queryFn: queryFn(api.runeforge.context),
      staleTime: 60 * 60 * 1000,
    }),

  /**
   * Packs and uploads a project, once per `attempt`.
   *
   * A query rather than a mutation, because the dialog starts it on mount and a mutation started
   * in an effect loses its result to the double mount of StrictMode. It never refetches on its own,
   * and a closed dialog drops it.
   */
  prepare: (client: QueryClient, projectPath: string, format: PackFormat, attempt: number) =>
    queryOptions<PublishReview, AppError>({
      queryKey: runeforgeKeys.prepare(projectPath, format, attempt),
      queryFn: async () => {
        try {
          return unwrapForQuery(await api.runeforge.prepare(projectPath, format));
        } finally {
          void client.invalidateQueries({ queryKey: runeforgeKeys.status() });
        }
      },
      staleTime: Infinity,
      gcTime: 0,
      retry: false,
      refetchOnMount: false,
      refetchOnWindowFocus: false,
      refetchOnReconnect: false,
    }),

  release: (modId: string, releaseId: string) =>
    queryOptions<Release, AppError>({
      queryKey: runeforgeKeys.release(modId, releaseId),
      queryFn: queryFnWithArgs(api.runeforge.release, modId, releaseId),
    }),
} as const;
