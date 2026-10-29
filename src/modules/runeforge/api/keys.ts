export const runeforgeKeys = {
  all: ["runeforge"] as const,
  status: () => [...runeforgeKeys.all, "status"] as const,
  ownedMods: () => [...runeforgeKeys.all, "ownedMods"] as const,
  context: () => [...runeforgeKeys.all, "context"] as const,
  prepare: (projectPath: string, format: string, attempt: number) =>
    [...runeforgeKeys.all, "prepare", projectPath, format, attempt] as const,
  release: (modId: string, releaseId: string) =>
    [...runeforgeKeys.all, "release", modId, releaseId] as const,
};
