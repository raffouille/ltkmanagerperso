import { elementsOf, type WidgetProps } from "../../classes/components/ClassCells";
import { EFFECT } from "../../classes/utils/classLayouts";
import { fieldHash } from "../../tree/utils/binRows";
import { IDLE_EFFECTS, IDLE_LIST, listLength } from "../utils/effectEdits";
import { EffectList } from "./EffectList";
import { ResolverScope } from "./ResolverScope";

/**
 * The idle effects, each joined to the system its key names through the resolver.
 *
 * "The skin view" in docs/research/bin-editor-higher-order-views.md, and "The VFX section" in
 * docs/ux/SKIN_EDITOR.md.
 */
export function EffectTable({ section, pages, view }: WidgetProps) {
  const effects = elementsOf(
    section.rows.filter((row) => fieldHash(row.path) !== EFFECT.resolver),
    pages,
  );
  const list = section.rows.find((row) => fieldHash(row.path) === IDLE_EFFECTS);

  return (
    <ResolverScope rows={section.rows} view={view}>
      {(scope) => (
        <EffectList
          view={view}
          scope={scope}
          path={IDLE_LIST}
          effects={effects}
          pages={pages}
          count={listLength(list)}
        />
      )}
    </ResolverScope>
  );
}
