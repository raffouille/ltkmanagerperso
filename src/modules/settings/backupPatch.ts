import type { Settings } from "@/lib/tauri";

/**
 * Returns the keys whose values differ between `current` and `next`, with the values `current` holds.
 *
 * Saving the result over `next` puts back what a restore changed and nothing else.
 */
export function revertPatch(current: Settings, next: Settings): Partial<Settings> {
  const patch: Record<string, unknown> = {};
  for (const key of Object.keys(next) as (keyof Settings)[]) {
    if (JSON.stringify(current[key]) !== JSON.stringify(next[key])) patch[key] = current[key];
  }

  return patch as Partial<Settings>;
}
