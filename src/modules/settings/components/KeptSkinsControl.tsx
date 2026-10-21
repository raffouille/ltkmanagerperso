import { TrashIcon } from "@phosphor-icons/react";
import { useMemo, useRef, useState } from "react";

import {
  badgeFill,
  Combobox,
  Fill,
  IconButton,
  Inline,
  Stack,
  useComboboxFilter,
} from "@/components";
import { m } from "@/i18n";
import type { Champion, ChampionSkin } from "@/lib/tauri";
import { ChampionPortrait, useChampionRoster } from "@/modules/champions";
import { twMerge } from "@/utils";

import { useChampionSkins, useLoadedSettings, useUpdateSettings } from "../api";
import { type KeptSkins, keptSkinsOf, withKeptSkins } from "../builtinMods";

/**
 * One line per champion in the `keptSkins` setting, each with a search over the champion's
 * skins, and a search that adds a champion.
 *
 * A kept id the install no longer lists stays as a chip, so a patch that renumbers nothing loses
 * no choice.
 */
export function KeptSkinsControl() {
  const settings = useLoadedSettings();
  const update = useUpdateSettings();
  const kept: KeptSkins = settings.builtinMods.keptSkins ?? {};

  if (!settings.leaguePath) {
    return (
      <span className="text-sm text-surface-400">{m.settings_builtins_kept_skins_no_path()}</span>
    );
  }

  const save = (champion: string, ids: readonly number[] | null) =>
    update({
      builtinMods: { ...settings.builtinMods, keptSkins: withKeptSkins(kept, champion, ids) },
    });

  const champions = Object.keys(kept);

  return (
    <Stack gap={2} data-ui="KeptSkinsControl">
      {champions.map((champion) => (
        <KeptChampionLine
          key={champion}
          champion={champion}
          value={keptSkinsOf(kept, champion)}
          onChange={(ids) => save(champion, ids)}
          onRemove={() => save(champion, null)}
        />
      ))}
      <ChampionAdder kept={champions} onAdd={(champion) => save(champion, [])} />
    </Stack>
  );
}

interface KeptChampionLineProps {
  champion: string;
  value: number[];
  onChange: (ids: number[]) => void;
  onRemove: () => void;
}

/** A champion's portrait and name, the search over its skins, and a button that drops it. */
function KeptChampionLine({ champion, value, onChange, onRemove }: KeptChampionLineProps) {
  const roster = useChampionRoster();
  const label = roster.labelOf(champion);

  return (
    <Inline gap={3} align="center">
      <span className="flex w-36 shrink-0 items-center gap-2 text-sm text-surface-200 select-none">
        <ChampionPortrait champion={roster.find(champion)} className="size-5" />
        <span className="min-w-0 truncate">{label}</span>
      </span>
      <Fill>
        <KeptSkinsPicker champion={champion} label={label} value={value} onChange={onChange} />
      </Fill>
      <IconButton
        icon={<TrashIcon weight="bold" />}
        size="sm"
        aria-label={m.settings_builtins_kept_skins_remove_action({ champion: label })}
        onClick={onRemove}
      />
    </Inline>
  );
}

/** One skin row of the picker. */
interface SkinOption {
  readonly id: number;
  readonly label: string;
}

interface KeptSkinsPickerProps {
  champion: string;
  /** The champion's display name, for the field's label. */
  label: string;
  value: number[];
  onChange: (ids: number[]) => void;
}

/** The kept skins of one champion as chips, in a field that searches the champion's skins. */
function KeptSkinsPicker({ champion, label, value, onChange }: KeptSkinsPickerProps) {
  const settings = useLoadedSettings();
  const { data: skins, isPending } = useChampionSkins(settings.leaguePath, champion);
  const filter = useComboboxFilter();
  const field = useRef<HTMLDivElement>(null);
  const [query, setQuery] = useState("");

  const options = useMemo(() => skinOptions(skins ?? [], value), [skins, value]);
  const selected = useMemo(
    () => options.filter((option) => value.includes(option.id)),
    [options, value],
  );

  return (
    <Combobox.Root<SkinOption, true>
      multiple
      items={options}
      value={selected}
      onValueChange={(next) => {
        onChange(next.map((option) => option.id));
        setQuery("");
      }}
      inputValue={query}
      onInputValueChange={setQuery}
      disabled={isPending}
      filter={(option, search) => filter.contains(option, search, (item) => item.label)}
      itemToStringLabel={(option) => option.label}
      itemToStringValue={(option) => String(option.id)}
      isItemEqualToValue={(option, chosen) => option.id === chosen.id}
    >
      <Combobox.Chips
        ref={field}
        data-ui="KeptSkinsControl:picker"
        className={twMerge(
          "min-h-10 w-full items-center rounded-md border border-surface-500 bg-surface-700 px-1.5 py-1.5 transition-colors",
          /* DS-HOVER */
          "focus-within:border-accent-500 focus-within:ring-1 focus-within:ring-accent-500 hover:border-accent-hover",
        )}
      >
        <Combobox.Value>
          {(chips: SkinOption[]) =>
            chips.map((option) => (
              <Combobox.Chip
                key={option.id}
                aria-label={option.label}
                className={twMerge(
                  "gap-1 rounded-sm py-0.5 pr-1 pl-1.5 text-meta select-none",
                  badgeFill.champion,
                )}
              >
                {option.label}
                {/* DS-VEIL */}
                <Combobox.ChipRemove
                  aria-label={m.common_chip_remove_action({ label: option.label })}
                  className="rounded-full text-current hover:bg-surface-veil hover:text-current"
                />
              </Combobox.Chip>
            ))
          }
        </Combobox.Value>
        <Combobox.Input
          aria-label={m.settings_builtins_kept_skins_skins_label({ champion: label })}
          placeholder={
            value.length === 0 ? m.settings_builtins_kept_skins_placeholder() : undefined
          }
          className="h-6 min-w-24 flex-1 rounded-none border-0 bg-transparent px-1.5 hover:border-transparent focus:border-transparent focus:ring-0"
        />
      </Combobox.Chips>
      <Combobox.Content
        anchor={field}
        positionerClassName="w-(--anchor-width)"
        className="max-h-72 select-none"
      >
        <Combobox.Empty>{m.settings_builtins_kept_skins_skin_empty()}</Combobox.Empty>
        <Combobox.List>
          {(option: SkinOption) => (
            <Combobox.Item key={option.id} value={option}>
              <span className="min-w-0 truncate">{option.label}</span>
            </Combobox.Item>
          )}
        </Combobox.List>
      </Combobox.Content>
    </Combobox.Root>
  );
}

/** A row for each of `skins`, then one for each id of `kept` the install does not list. */
function skinOptions(skins: readonly ChampionSkin[], kept: readonly number[]): SkinOption[] {
  const options = skins.map((skin) => ({ id: skin.id, label: skinLabel(skin.id, skin.name) }));
  const listed = new Set(skins.map((skin) => skin.id));
  for (const id of kept) {
    if (!listed.has(id)) {
      options.push({ id, label: skinLabel(id, null) });
    }
  }

  return options;
}

function skinLabel(id: number, name: string | null): string {
  return name ?? m.settings_builtins_kept_skins_unnamed_label({ id: String(id) });
}

interface ChampionAdderProps {
  /** The champions the setting already holds, which the search leaves out. */
  kept: readonly string[];
  onAdd: (champion: string) => void;
}

/** A search over the install's champions that adds the one picked. */
function ChampionAdder({ kept, onAdd }: ChampionAdderProps) {
  const roster = useChampionRoster();
  const filter = useComboboxFilter();
  const [query, setQuery] = useState("");

  const champions = useMemo(() => {
    const held = new Set(kept.map((champion) => champion.toLowerCase()));
    return roster.champions.filter((champion) => !held.has(champion.id.toLowerCase()));
  }, [roster, kept]);

  return (
    <Combobox.Root<Champion>
      items={champions}
      value={null}
      onValueChange={(champion) => {
        if (champion) {
          onAdd(champion.id);
        }
        setQuery("");
      }}
      inputValue={query}
      onInputValueChange={setQuery}
      itemToStringLabel={championLabel}
      itemToStringValue={(champion) => champion.id}
      isItemEqualToValue={(champion, chosen) => champion.id === chosen.id}
      filter={(champion, search) =>
        filter.contains(
          champion,
          search,
          (item) => `${championLabel(item)} ${item.id} ${item.metadataName}`,
        )
      }
    >
      <div className="relative w-64" data-ui="KeptSkinsControl:add">
        <Combobox.Input
          aria-label={m.settings_builtins_kept_skins_add_placeholder()}
          placeholder={m.settings_builtins_kept_skins_add_placeholder()}
          className="pr-8"
        />
        <Combobox.Trigger className="absolute top-0 right-0 flex h-full items-center pr-3">
          <Combobox.Icon />
        </Combobox.Trigger>
      </div>

      <Combobox.Content
        positionerClassName="min-w-(--anchor-width)"
        className="max-h-72 select-none"
      >
        <Combobox.Empty>{m.settings_builtins_kept_skins_champion_empty()}</Combobox.Empty>
        <Combobox.List>
          {(champion: Champion) => (
            <Combobox.Item key={champion.id} value={champion}>
              <ChampionPortrait champion={champion} className="size-5" />
              <span className="min-w-0 truncate">{championLabel(champion)}</span>
            </Combobox.Item>
          )}
        </Combobox.List>
      </Combobox.Content>
    </Combobox.Root>
  );
}

function championLabel(champion: Champion): string {
  return champion.name ?? champion.metadataName;
}
