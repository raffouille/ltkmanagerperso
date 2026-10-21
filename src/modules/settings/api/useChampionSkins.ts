import { useQuery } from "@tanstack/react-query";

import { leagueInstallQueries } from "./queries";

/** Every skin past the base of `champion` in the install at `leaguePath`, in id order. */
export function useChampionSkins(leaguePath: string | null | undefined, champion: string) {
  return useQuery(leagueInstallQueries.championSkins(leaguePath, champion));
}
