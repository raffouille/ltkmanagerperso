import Mark from "@/assets/icons/RuneforgeIcon.svg?react";

interface RuneforgeIconProps {
  className?: string;
}

/**
 * The RuneForge flame in its diamond, from the site's own `runeforge-logo-centered.svg`.
 *
 * The crimson is the site's identity, so it keeps its color in both themes rather than taking
 * `currentColor`, as `HexshadeIcon` does.
 */
export function RuneforgeIcon({ className }: RuneforgeIconProps) {
  return <Mark className={className} />;
}
