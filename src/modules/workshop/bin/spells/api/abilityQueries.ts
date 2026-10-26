import { queryOptions } from "@tanstack/react-query";

import { api, type BinDocumentId, type EffectSystem } from "@/lib/tauri";
import { queryFnWithArgs } from "@/utils/query";

export const abilityQueries = {
  /** The system `effect` maps to, read through the skin's document, which reaches linked bins. */
  effect: (document: BinDocumentId, effect: EffectSystem) =>
    queryOptions({
      queryKey: ["spell", "ability-effect", document, effect.system],
      queryFn: queryFnWithArgs(api.bin.readVfxSystem, document, effect.system),
      staleTime: Infinity,
      retry: false,
    }),
};
