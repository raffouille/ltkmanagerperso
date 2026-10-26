import { CubeIcon, CylinderIcon, type Icon, SphereIcon, SquareIcon } from "@phosphor-icons/react";
import { use, useState } from "react";
import { Sphere, Vector3 } from "three";

import { m } from "@/i18n";
import type { MaterialProgram } from "@/lib/tauri";
import {
  MaterialSubject,
  PREVIEW_BOUNDS,
  PREVIEW_SHAPES,
  type PreviewShape,
} from "@/modules/viewport";
import { twMerge } from "@/utils";

import {
  useEmbeddedMaterialPasses,
  useMaterialPasses,
} from "../../rendering/hooks/useParticlePrograms";
import type { MaterialRef } from "../utils/materialNodes";
import { GraphActionsContext, NO_DOCUMENT } from "./graphActions";
import { NODE_BOX, Turntable } from "./NodePreviews";
import { PreviewView } from "./PreviewView";
import { StripButton } from "./StripButton";

const SHAPE_LABEL: Record<PreviewShape, () => string> = {
  sphere: m.workshop_bin_material_shape_sphere_label,
  cube: m.workshop_bin_material_shape_cube_label,
  plane: m.workshop_bin_material_shape_plane_label,
  cylinder: m.workshop_bin_material_shape_cylinder_label,
};

const SHAPE_ICON: Record<PreviewShape, Icon> = {
  sphere: SphereIcon,
  cube: CubeIcon,
  plane: SquareIcon,
  cylinder: CylinderIcon,
};

/* The shapes stand on the ground, so the turntable turns them about the middle of their box. */
const MIDDLE = new Vector3(...PREVIEW_BOUNDS.min)
  .add(new Vector3(...PREVIEW_BOUNDS.max))
  .multiplyScalar(0.5);
const SHIFT: [number, number, number] = [-MIDDLE.x, -MIDDLE.y, -MIDDLE.z];

/** The reach the camera frames, a little past the widest shape's half width so a cube's corners fit. */
const FRAMED = new Sphere(
  new Vector3(),
  ((PREVIEW_BOUNDS.max[0] - PREVIEW_BOUNDS.min[0]) / 2) * 1.3,
);

/**
 * A material node's material on a preview shape, a sphere until the reader picks another,
 * drawn with its translated passes as the material shell's preview draws it. The Component
 * row of decision 2.8 in docs/plans/shimmer-driver-graph.md.
 */
export function MaterialShape({ material }: { material: MaterialRef | undefined }) {
  const [shape, setShape] = useState<PreviewShape>("sphere");
  const actions = use(GraphActionsContext);
  const document = actions?.document ?? null;
  const linked = material?.type === "linked" ? material.entry : null;
  const linkedRead = useMaterialPasses(document, linked);
  const at =
    material?.type === "embedded" && document !== null
      ? { entry: material.entry ?? actions?.entry ?? "", path: material.path }
      : null;
  const embeddedRead = useEmbeddedMaterialPasses(document ?? NO_DOCUMENT, at);
  const { program, passes, failed } = material?.type === "embedded" ? embeddedRead : linkedRead;

  return (
    <div data-ui="MaterialShape" className={twMerge(NODE_BOX, "relative")}>
      {program != null && (
        <PreviewView className="absolute inset-0">
          <Turntable sphere={FRAMED}>
            <group position={SHIFT}>
              <MaterialSubject
                programs={passes}
                skinned={program.kind === "skinnedMesh"}
                shape={shape}
                turntable={false}
              />
            </group>
          </Turntable>
        </PreviewView>
      )}
      <Note named={material !== undefined} program={program} failed={failed} />
      <div
        role="group"
        aria-label={m.workshop_bin_material_shape_label()}
        /* DS-GLASS, DS-RADIUS, DS-VEIL */
        className="absolute bottom-1 left-1 flex items-center gap-1 rounded-sm border border-surface-veil bg-scrim p-0.5 backdrop-blur-sm"
      >
        {PREVIEW_SHAPES.map((each) => {
          const Glyph = SHAPE_ICON[each];
          return (
            <StripButton
              key={each}
              label={SHAPE_LABEL[each]()}
              pressed={each === shape}
              onClick={() => setShape(each)}
            >
              <Glyph weight="bold" className="size-3" />
            </StripButton>
          );
        })}
      </div>
    </div>
  );
}

/** Why the box draws no shape: the read runs, or it found no material to draw. */
function Note({
  named,
  program,
  failed,
}: {
  /** The node names a material to read. */
  named: boolean;
  program: MaterialProgram | null | undefined;
  failed: boolean;
}) {
  const reading = named && program === undefined && !failed;
  const missing = !named || failed || program === null;
  if (!reading && !missing) return null;

  return (
    <span className="absolute inset-0 flex items-center justify-center p-4 text-center text-meta text-surface-400">
      {reading && m.workshop_bin_material_preview_loading_label()}
      {missing && m.workshop_bin_material_preview_failed_empty()}
    </span>
  );
}
