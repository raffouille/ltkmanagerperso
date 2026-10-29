import { Matrix4 } from "three";
import { describe, expect, it } from "vitest";

import { createPose, type MeshGeometry } from "@/modules/viewport";

import { nameHash } from "../../../../shared/utils/binHash";
import type { EmissionSurfaceModel } from "../../../engine/model/model";
import { Rng } from "../../../engine/utils/Rng";
import {
  boundJoints,
  type BoundUnit,
  meshSurface,
  skeletonSurface,
  staticMeshSurface,
} from "../emissionSurface";
import { CLIP, MESH, SKELETON } from "./skinnedFixture";

const MODEL: EmissionSurfaceModel = {
  kind: "mesh",
  mesh: null,
  skeleton: null,
  submeshes: [],
  joints: [],
  scale: 1,
  maxJointWeights: 4,
  useNormal: true,
};

function birth() {
  return { position: new Float32Array(3), normal: new Float32Array(3) };
}

/** A generator answering `draws` in turn, which a sampler reads in the order it documents. */
function scripted(...draws: number[]): Rng {
  let at = 0;
  return {
    unitFloat: () => {
      const draw = draws[at];
      at += 1;
      if (draw === undefined) throw new Error("the sampler drew more often than scripted");
      return draw;
    },
  } as unknown as Rng;
}

/** A static mesh of `triangles`, each three corners, with nothing but positions. */
function staticMesh(...triangles: number[][]): MeshGeometry {
  const positions = Float32Array.from(triangles.flat());
  return {
    positions,
    normals: null,
    uvs: null,
    skinIndices: null,
    skinWeights: null,
    colors: null,
    tangents: null,
    indices: Uint32Array.from({ length: positions.length / 3 }, (_, index) => index),
    ranges: [],
  };
}

/** A bound unit with the fixture's skeleton, playing the fixture's clip. */
function playing(offset = 0): BoundUnit {
  return { pose: createPose(SKELETON, CLIP), offset };
}

describe("boundJoints", () => {
  const world = new Float32Array(16);

  it("uses the rest pose when the effect is bound to no unit", () => {
    const joints = boundJoints(SKELETON, null);

    expect(joints.worldInto(1, 0.5, world)[12]).toBeCloseTo(2);
  });

  it("samples the unit's pose at the run's time plus the unit's offset", () => {
    expect(boundJoints(SKELETON, playing()).worldInto(1, 0.5, world)[12]).toBeCloseTo(4);
    expect(boundJoints(SKELETON, playing(0.25)).worldInto(1, 0.25, world)[12]).toBeCloseTo(4);
  });

  it("matches joints by name, and uses the rest pose for a joint the unit does not have", () => {
    const [root, tip] = SKELETON.joints;
    const renamed = { ...SKELETON, joints: [root, { ...tip, name: "other" }] };
    const reordered = createPose({ ...SKELETON, joints: [{ ...root, name: "TIP" }] }, null);

    expect(boundJoints(renamed, playing()).worldInto(1, 0.5, world)[12]).toBeCloseTo(2);
    expect(
      boundJoints(SKELETON, { pose: reordered, offset: 0 }).worldInto(1, 0, world)[12],
    ).toBeCloseTo(0);
  });
});

describe("meshSurface", () => {
  it("samples the mesh in the unit's pose, and returns the same birth for the same time and seed", () => {
    const surface = meshSurface(MODEL, MESH, SKELETON, boundJoints(SKELETON, playing()));
    const first = birth();
    const later = birth();
    const replay = birth();

    expect(surface.sample(0, new Rng(42), first)).toBe(true);
    surface.sample(0.5, new Rng(42), later);
    surface.sample(0, new Rng(42), replay);

    expect(first.position[0]).toBeCloseTo(2);
    expect(later.position[0]).toBeCloseTo(4);
    expect(first.normal).toEqual(Float32Array.of(1, 0, 0));
    expect(replay).toEqual(first);
    expect(first.position[1] + first.position[2]).toBeLessThanOrEqual(1);
  });

  it("uses the rest pose at every time when there is no unit", () => {
    const surface = meshSurface(MODEL, MESH, SKELETON, boundJoints(SKELETON, null));
    const out = birth();

    surface.sample(0.5, new Rng(42), out);

    expect(out.position[0]).toBeCloseTo(2);
  });

  it("blends the vertex normals when the mesh has them, in place of the face normal", () => {
    const smooth = { ...MESH, normals: Float32Array.of(0, 1, 0, 0, 1, 0, 0, 1, 0) };
    const out = birth();

    meshSurface(MODEL, smooth, SKELETON, boundJoints(SKELETON, null)).sample(0, new Rng(1), out);

    expect(Array.from(out.normal)).toEqual([0, 1, 0]);
  });

  it("picks every triangle with the same probability, whatever its area", () => {
    const sliver = [0, 0, 0, 0, 0.001, 0, 0, 0, 0.001];
    const wide = [9, 0, 0, 9, 100, 0, 9, 0, 100];
    const mesh: MeshGeometry = {
      ...staticMesh(sliver, wide),
      skinIndices: new Uint8Array(24),
      skinWeights: Float32Array.from({ length: 24 }, (_, at) => (at % 4 === 0 ? 1 : 0)),
      colors: null,
      tangents: null,
    };
    const surface = meshSurface(MODEL, mesh, SKELETON, boundJoints(SKELETON, null));
    const out = birth();

    surface.sample(0, scripted(0.49, 0, 0), out);
    expect(out.position[0]).toBeCloseTo(0);

    surface.sample(0, scripted(0.5, 0, 0), out);
    expect(out.position[0]).toBeCloseTo(9);
  });
});

describe("skeletonSurface", () => {
  const MASKED = { ...MODEL, kind: "skeleton", joints: [nameHash("tip")] } as const;

  it("samples a selected bone and refuses a mask that selects no bones", () => {
    const joints = boundJoints(SKELETON, playing());
    const out = birth();

    expect(skeletonSurface(MASKED, SKELETON, joints).sample(0.5, new Rng(42), out)).toBe(true);
    expect(out.position[0]).toBeGreaterThanOrEqual(0);
    expect(out.position[0]).toBeLessThanOrEqual(4);
    expect(out.position[1]).toBe(0);
    expect(
      skeletonSurface({ ...MODEL, joints: [nameHash("missing")] }, SKELETON, joints).sample(
        0,
        new Rng(1),
        out,
      ),
    ).toBe(false);
  });

  it("picks a bone by its rest-pose length, and places the point in the unit's pose", () => {
    const [root, tip] = SKELETON.joints;
    const short = {
      ...tip,
      name: "short",
      hash: 3,
      translation: [0, 1, 0] as const,
      inverseBind: Float32Array.from(new Matrix4().makeTranslation(0, -1, 0).elements),
    };
    const skeleton = { ...SKELETON, joints: [root, tip, short] };
    const surface = skeletonSurface(
      { ...MODEL, kind: "skeleton" },
      skeleton,
      boundJoints(skeleton, playing()),
    );
    const out = birth();

    /* The rest lengths are 2 and 1. The clip makes the first bone 4 long. */
    surface.sample(0.5, scripted(0.66, 1, 0), out);
    expect(Array.from(out.position)).toEqual([4, 0, 0]);

    surface.sample(0.5, scripted(0.67, 1, 0), out);
    expect(Array.from(out.position)).toEqual([0, 1, 0]);
  });

  it("returns a unit normal perpendicular to the bone, at an angle set by the third draw", () => {
    const surface = skeletonSurface(MASKED, SKELETON, boundJoints(SKELETON, null));
    const first = birth();
    const turned = birth();

    surface.sample(0, scripted(0, 0.5, 0), first);
    surface.sample(0, scripted(0, 0.5, 0.25), turned);

    for (const out of [first, turned]) {
      expect(out.normal[0]).toBeCloseTo(0);
      expect(Math.hypot(...out.normal)).toBeCloseTo(1);
    }
    expect(first.normal[1] * turned.normal[1] + first.normal[2] * turned.normal[2]).toBeCloseTo(0);
  });
});

describe("staticMeshSurface", () => {
  /** One unit of area, in the plane `z = 0`, wound so its normal is `+Z`. */
  const SMALL = [1, 0, 0, 3, 0, 0, 1, 1, 0];

  /** Three units of area, standing far off the first. */
  const LARGE = [10, 0, 0, 16, 0, 0, 10, 1, 0];

  /** The corner a sampler lands on where both of its point draws are zero. */
  function cornerPicked(mesh: MeshGeometry, pick: number): number[] {
    const out = birth();
    expect(staticMeshSurface(mesh).sample(0, scripted(pick, 0, 0), out)).toBe(true);
    return Array.from(out.position);
  }

  it("picks a triangle by its share of the area rather than by the count", () => {
    const mesh = staticMesh(SMALL, LARGE);

    expect(cornerPicked(mesh, 0)).toEqual([1, 0, 0]);
    expect(cornerPicked(mesh, 0.24)).toEqual([1, 0, 0]);
    /* A quarter of the area is the small triangle's, so the rest of the draw is the large one's. */
    expect(cornerPicked(mesh, 0.25)).toEqual([10, 0, 0]);
    expect(cornerPicked(mesh, 0.4)).toEqual([10, 0, 0]);
    expect(cornerPicked(mesh, 0.999)).toEqual([10, 0, 0]);
  });

  it("skips a triangle of no area, however many of them the mesh holds", () => {
    const flat = [5, 5, 5, 6, 6, 6, 7, 7, 7];
    const mesh = staticMesh(flat, flat, SMALL);

    expect(cornerPicked(mesh, 0)).toEqual([1, 0, 0]);
    expect(cornerPicked(mesh, 0.5)).toEqual([1, 0, 0]);
  });

  it("blends the three corners by its two draws, which lands inside the triangle", () => {
    const surface = staticMeshSurface(staticMesh(SMALL));
    const at = (u: number, v: number) => {
      const out = birth();
      surface.sample(0, scripted(0, u, v), out);
      return Array.from(out.position);
    };

    expect(at(0, 0)).toEqual([1, 0, 0]);
    expect(at(0, 1)).toEqual([3, 0, 0]);
    expect(at(1, 0)).toEqual([1, 1, 0]);
    /* The first draw is the third corner's weight, and the second splits the rest. */
    expect(at(0.5, 0.5)).toEqual([0.25 * 1 + 0.25 * 3 + 0.5 * 1, 0.5, 0]);
    expect(at(0.25, 0.75)).toEqual([0.1875 * 1 + 0.5625 * 3 + 0.25 * 1, 0.25, 0]);
  });

  it("returns the point unscaled and a normal of length one", () => {
    const out = birth();
    const surface = staticMeshSurface(staticMesh(SMALL));

    expect(surface.sample(0, scripted(0, 0.5, 0.5), out)).toBe(true);

    expect(Array.from(out.position)).toEqual([1.5, 0.5, 0]);
    expect(Array.from(out.normal)).toEqual([0, 0, 1]);
  });

  it("answers the triangle's geometric normal, by its winding, and reads no vertex normal", () => {
    const out = birth();
    const reversed = [1, 0, 0, 1, 1, 0, 3, 0, 0];
    const lying = { ...staticMesh(SMALL), normals: Float32Array.of(1, 0, 0, 1, 0, 0, 1, 0, 0) };
    const tilted = [0, 0, 0, 0, 2, 0, 0, 0, 2];

    staticMeshSurface(lying).sample(0, scripted(0, 0.3, 0.3), out);
    expect(Array.from(out.normal)).toEqual([0, 0, 1]);

    staticMeshSurface(staticMesh(reversed)).sample(0, scripted(0, 0.3, 0.3), out);
    expect(Array.from(out.normal)).toEqual([0, 0, -1]);

    staticMeshSurface(staticMesh(tilted)).sample(0, scripted(0, 0.3, 0.3), out);
    expect(Array.from(out.normal)).toEqual([1, 0, 0]);
  });

  it("samples nothing off a mesh with no area, and draws no number for it", () => {
    const out = birth();
    const flat = [5, 5, 5, 6, 6, 6, 7, 7, 7];

    expect(staticMeshSurface(staticMesh(flat)).sample(0, scripted(), out)).toBe(false);
    expect(staticMeshSurface(staticMesh()).sample(0, scripted(), out)).toBe(false);
    expect(Array.from(out.position)).toEqual([0, 0, 0]);
    expect(Array.from(out.normal)).toEqual([0, 0, 0]);
  });

  it("reads its three draws in a fixed order, so a seed replays the same birth", () => {
    const surface = staticMeshSurface(staticMesh(SMALL, LARGE));
    const first = birth();
    const replay = birth();

    surface.sample(0, new Rng(7), first);
    surface.sample(3, new Rng(7), replay);

    expect(replay).toEqual(first);
  });
});
