import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

import { m } from "@/i18n";
import type { BinRow } from "@/lib/tauri";

import type { LayoutPages, ViewContext } from "../../classes/components/ClassCells";
import { ChildAddPicker } from "../../vfx/children/ChildAddPicker";
import { childQueries } from "../../vfx/children/useChildren";
import { useEffectAdd } from "../hooks/useEffectAdd";
import { type EffectListPath, resolverKey } from "../utils/effectEdits";
import { EffectRows } from "./EffectRows";
import type { EffectScope, ResourceTarget } from "./ResolverScope";

export interface EffectListProps {
  view: ViewContext;
  scope: EffectScope;
  /** Where the list sits, which an add appends to. */
  path: EffectListPath;
  /** The list's elements. */
  effects: readonly BinRow[];
  /** The fields of each element, by the element's row key. */
  pages: LayoutPages;
  /** The number of effects the list holds. */
  count: number;
}

/**
 * A list of effects of the skin: one row per effect, and the picker that adds one where the
 * effects take edits.
 *
 * The idle effects of the VFX section and the effects of a persistent condition are both one,
 * per "The VFX section" and "The Conditions section" in docs/ux/SKIN_EDITOR.md.
 */
export function EffectList({ view, scope, path, effects, pages, count }: EffectListProps) {
  const { resources, edit, resolver, joints } = scope;

  return (
    <>
      <EffectRows
        effects={effects}
        pages={pages}
        resources={resources}
        edit={edit}
        joints={joints}
      />
      {resolver !== null && (
        <AddEffect
          view={view}
          path={path}
          count={count}
          resolver={resolver}
          resources={resources}
          bone={joints[0] ?? ""}
        />
      )}
    </>
  );
}

interface AddEffectProps {
  view: ViewContext;
  path: EffectListPath;
  count: number;
  /** The resolver object, which this file declares. */
  resolver: string;
  resources: ReadonlyMap<string, ResourceTarget>;
  /** The joint a new effect sits on. */
  bone: string;
}

/**
 * The combobox that adds an effect on the skeleton's first joint, listing the systems that the
 * file declares or that its resolvers map. The resolver takes a key for a system that has none.
 */
function AddEffect({ view, path, count, resolver, resources, bone }: AddEffectProps) {
  const systems = useQuery(childQueries.targets(view.document)).data;
  const add = useEffectAdd();
  const actions = useMemo(() => {
    const system = (entry: string) => systems?.find((each) => each.entry === entry);
    return {
      add:
        add === null
          ? null
          : async (entry: string) => {
              const picked = system(entry);
              if (picked === undefined) return false;

              add({
                skin: view.entry,
                path,
                count,
                resolver,
                key: resolverKey(resources, picked),
                system: entry,
                bone,
              });
              return true;
            },
      create: null,
    };
  }, [add, systems, view.entry, path, count, resolver, resources, bone]);

  return (
    <div data-ui="EffectList:add" className="flex min-h-6 items-center px-1.5 pt-1">
      <ChildAddPicker
        actions={actions}
        systems={systems}
        holder=""
        label={m.workshop_bin_skin_effect_add_action()}
      />
    </div>
  );
}
