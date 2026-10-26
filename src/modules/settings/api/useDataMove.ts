import { useMutation } from "@tanstack/react-query";

import { settingsMutations } from "./mutations";

/** Plan a move of the data directory to a folder, or to the app data directory for `null`. */
export function usePlanDataMove() {
  return useMutation(settingsMutations.planDataMove());
}

/** Move the data directory to a folder, or to the app data directory for `null`, and restart. */
export function useMoveData() {
  return useMutation(settingsMutations.moveData());
}
