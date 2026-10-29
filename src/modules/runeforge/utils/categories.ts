import { m } from "@/i18n";

/**
 * Returns the label of the RuneForge category `category`, as the site names it.
 *
 * The site sends a category as its id. An id this build does not know shows as itself.
 */
export function categoryLabel(category: string): string {
  switch (category) {
    case "map_skin":
      return m.runeforge_category_map_skin_label();
    case "champion_skin":
      return m.runeforge_category_champion_skin_label();
    case "announcer":
      return m.runeforge_category_announcer_label();
    case "font":
      return m.runeforge_category_font_label();
    case "ui":
      return m.runeforge_category_ui_label();
    case "voiceover":
      return m.runeforge_category_voiceover_label();
    case "loading_screen":
      return m.runeforge_category_loading_screen_label();
    case "vfx":
      return m.runeforge_category_vfx_label();
    case "sfx":
      return m.runeforge_category_sfx_label();
    case "miscellaneous":
      return m.runeforge_category_miscellaneous_label();
    default:
      return category;
  }
}
