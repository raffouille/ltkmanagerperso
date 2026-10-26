import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { settingsMutations } from "./mutations";
import { settingsQueries } from "./queries";

/** The disk usage of each area of the manager's data. */
export function useDataUsage() {
  return useQuery(settingsQueries.dataUsage());
}

/** Clear one area of the manager's data, and measure the areas again. */
export function useClearDataArea() {
  return useMutation(settingsMutations.clearDataArea(useQueryClient()));
}
