import type { ReleaseNote } from "@/lib/tauri";

/** The `major.minor.patch` numbers of `version`, or `null` for a version that is not one. */
function versionParts(version: string): [number, number, number] | null {
  const match = /^(\d+)\.(\d+)\.(\d+)/.exec(version);
  if (!match) return null;

  return [Number(match[1]), Number(match[2]), Number(match[3])];
}

/** Whether `version` is a later `major.minor.patch` than `installed`. */
function isNewer(version: string, installed: string): boolean {
  const candidate = versionParts(version);
  const current = versionParts(installed);
  if (!candidate || !current) return false;

  for (let index = 0; index < 3; index++) {
    if (candidate[index] !== current[index]) return candidate[index] > current[index];
  }
  return false;
}

/**
 * The newest release of `releases` that is not a pre-release and is later than `installed`, or
 * `null` when the installed build is the newest.
 */
export function newerOfficialRelease(
  releases: readonly ReleaseNote[],
  installed: string | undefined,
): ReleaseNote | null {
  if (!installed) return null;

  let newest: ReleaseNote | null = null;
  for (const release of releases) {
    if (release.prerelease || !isNewer(release.version, installed)) continue;
    if (!newest || isNewer(release.version, newest.version)) newest = release;
  }
  return newest;
}
