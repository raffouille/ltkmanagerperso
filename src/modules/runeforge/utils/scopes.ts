import type { Scope } from "@/lib/tauri";

/** The scopes that publishing a new post from the app uses. */
export const PUBLISHING_SCOPES: readonly Scope[] = [
  "mods:read",
  "mods:write",
  "releases:write",
  "publish",
];

/** Returns the scopes of [`PUBLISHING_SCOPES`] that `scopes` lacks. */
export function missingScopes(scopes: readonly Scope[]): Scope[] {
  return PUBLISHING_SCOPES.filter((scope) => !scopes.includes(scope));
}
