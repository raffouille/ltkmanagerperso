import { type ReactNode, useCallback, useState } from "react";

import { Button } from "@/components";
import { m } from "@/i18n";
import type { AssetRef, BinDocumentId } from "@/lib/tauri";

import { useOptionalProjectContext } from "../../../projects/state/ProjectContext";
import { useWorkshopEditorStore } from "../../../shell/state/workshopEditor";
import { Notice } from "../../shared/preview/Notice";
import { useSkinGraphSource } from "../../skin/hooks/useGraphSource";
import { abilityRecipeSchema, type AbilityRecipe } from "../utils/abilityRecipe";
import { RecipeFields } from "./RecipeFields";
import { SpellRecipe, type SpellSelection } from "./SpellRecipe";

export interface AbilitySource {
  readonly document: BinDocumentId;
  readonly asset: AssetRef;
  readonly entry: string;
}

interface Props {
  readonly character: string;
  readonly source: AbilitySource;
  readonly onPreview: (recipe: AbilityRecipe | null) => void;
  readonly children: (select: (spell: SpellSelection) => void) => ReactNode;
}

/** Saved visual recipes above the character's individual spell previews. */
export function AbilityLibrary({ character, source, onPreview, children }: Props) {
  const project = useOptionalProjectContext();
  const recipes = useWorkshopEditorStore((state) =>
    project === null ? undefined : state.byProject[project.path]?.abilities,
  );
  const save = useWorkshopEditorStore((state) => state.saveAbility);
  const remove = useWorkshopEditorStore((state) => state.removeAbility);
  const [draft, setDraft] = useState<AbilityRecipe | null>(null);
  const [saved, setSaved] = useState(false);
  const [selected, setSelected] = useState<SpellSelection | null>(null);
  const opened = useCallback(
    (recipe: AbilityRecipe) => {
      setSelected(null);
      setDraft(recipe);
      setSaved(false);
      onPreview(abilityRecipeSchema.safeParse(recipe).success ? recipe : null);
    },
    [onPreview],
  );
  if (selected !== null)
    return (
      <div className="flex min-h-0 flex-1 flex-col">
        <div className="p-2">
          <Button size="sm" variant="ghost" onClick={() => setSelected(null)}>
            {m.workshop_missile_back_action()}
          </Button>
        </div>
        <SpellRecipe selected={selected} source={source} character={character} onReady={opened} />
      </div>
    );
  if (draft !== null)
    return (
      <div className="flex min-h-0 flex-1 flex-col">
        <div className="flex items-center gap-2 border-b border-surface-700/50 p-2">
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              setDraft(null);
              onPreview(null);
            }}
          >
            {m.workshop_missile_back_action()}
          </Button>
          <span className="truncate text-sm font-medium text-surface-100">{draft.name}</span>
        </div>
        <AbilityEditor
          key={draft.id}
          source={source}
          recipe={draft}
          onChange={(next) => {
            setDraft(next);
            setSaved(false);
          }}
        />
        <div className="flex flex-wrap items-center gap-2 border-t border-surface-700/50 p-3">
          <Button
            size="sm"
            disabled={!abilityRecipeSchema.safeParse(draft).success}
            onClick={() => onPreview(abilityRecipeSchema.parse(draft))}
          >
            {m.workshop_ability_preview_action()}
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={project === null || !abilityRecipeSchema.safeParse(draft).success}
            onClick={() => {
              if (project !== null) {
                save(project.path, abilityRecipeSchema.parse(draft));
                setSaved(true);
              }
            }}
          >
            {m.workshop_ability_save_action()}
          </Button>
          {saved && (
            <span role="status" className="text-meta text-surface-400">
              {m.workshop_ability_saved_label()}
            </span>
          )}
          {recipes?.some((item) => item.id === draft.id) && (
            <Button
              size="sm"
              variant="ghost"
              onClick={() => {
                if (project !== null) remove(project.path, draft.id);
                setDraft(null);
                onPreview(null);
              }}
            >
              {m.workshop_ability_remove_action()}
            </Button>
          )}
        </div>
      </div>
    );
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <section className="flex shrink-0 flex-col gap-2 border-b border-surface-700/50 p-2">
        <div className="flex items-center justify-between gap-2">
          <h3 className="text-meta font-medium text-surface-200">{m.workshop_ability_title()}</h3>
          <Button
            size="sm"
            variant="outline"
            onClick={() => {
              setSaved(false);
              setDraft({
                version: 1,
                id: crypto.randomUUID(),
                character,
                name: m.workshop_ability_new_label(),
                clip: "",
                bone: "",
                release: 0.4,
                castEffect: null,
                projectileEffect: null,
                flightDuration: 0.5,
                impactEffect: null,
                impactDuration: 0.5,
                target: [500, 0, 0],
              });
            }}
          >
            {m.workshop_ability_create_action()}
          </Button>
        </div>
        {recipes
          ?.filter((recipe) => recipe.character.toLowerCase() === character.toLowerCase())
          .map((recipe) => (
            <Button
              key={recipe.id}
              variant="ghost"
              size="sm"
              className="justify-start"
              onClick={() => {
                setDraft(recipe);
                setSaved(true);
                onPreview(recipe);
              }}
            >
              {recipe.name}
            </Button>
          ))}
      </section>
      {children(setSelected)}
    </div>
  );
}

function AbilityEditor({
  source,
  recipe,
  onChange,
}: {
  source: AbilitySource;
  recipe: AbilityRecipe;
  onChange: (recipe: AbilityRecipe) => void;
}) {
  const read = useSkinGraphSource(source.document, source.entry);
  return (
    <div className="min-h-0 flex-1 overflow-auto p-3 scrollbar-md">
      {read.skin.isError && <Notice text={m.workshop_bin_preview_failed_empty()} />}
      {read.skin.isPending && <Notice text={m.workshop_spells_loading_label()} />}
      {read.skin.data !== undefined && (
        <RecipeFields
          document={source.document}
          skin={read.skin.data}
          graph={read.source}
          recipe={recipe}
          onChange={onChange}
        />
      )}
    </div>
  );
}
