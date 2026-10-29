import { describe, expect, it } from "vitest";

import type { MeshGeometry } from "@/modules/viewport";

import type { MeshModel } from "../../../engine/model/model";
import { flightInto, mirrorInto, multiplyInto, standingInto } from "../../../engine/utils/basis";
import { geometryOf } from "../useVfxMeshes";

/** A cone as `Ahri_Base_Sharp_Mesh.scb` is modelled: its tip at the origin, opening along `+X`. */
const CONE: MeshGeometry = {
  positions: new Float32Array([0, 0, 0, 151, 100, 0, 151, -100, 0]),
  normals: null,
  uvs: null,
  skinIndices: null,
  skinWeights: null,
  colors: null,
  tangents: null,
  indices: new Uint32Array([0, 1, 2]),
  ranges: [],
};

const MODEL: MeshModel = {
  skeleton: null,
  animation: null,
  animationVariants: [],
  asset: { kind: "gameChunk", wad: "Ahri.wad.client", pathHash: "0" },
  path: null,
  submeshes: [],
  submeshesAlways: [],
  alignPitch: false,
  alignYaw: false,
  skinned: false,
};

/** The viewport's `x` of vertex `at` of `geometry`, under the row-major `turn`. */
function drawnX(turn: Float32Array, geometry: ReturnType<typeof geometryOf>, at: number): number {
  const position = geometry.getAttribute("position");
  return turn[0] * position.getX(at) + turn[1] * position.getY(at) + turn[2] * position.getZ(at);
}

describe("geometryOf", () => {
  it("hands the shaders the file's colour per vertex, and white for a file that holds none", () => {
    const plain = geometryOf(CONE, MODEL).getAttribute("vertexColor");
    const faded = geometryOf(
      {
        ...CONE,
        colors: new Uint8Array([255, 255, 255, 0, 255, 255, 255, 191, 255, 255, 255, 255]),
      },
      MODEL,
    ).getAttribute("vertexColor");

    expect([plain.getX(0), plain.getY(0), plain.getZ(0), plain.getW(0)]).toEqual([1, 1, 1, 1]);
    expect(faded.getW(0)).toBe(0);
    expect(faded.getW(1)).toBeCloseTo(191 / 255, 6);
    expect(faded.getW(2)).toBe(1);
  });

  it("opens cone_add of Ahri_Base_R_mis_02 behind the missile it rides", () => {
    const geometry = geometryOf(CONE, MODEL);

    /* The turn `Meshes` hands the instance: `birthRotation0` on the flight frame of a
       missile flying the engine's +X, mirrored whole into the viewport. */
    const turn = standingInto(new Float32Array([0, 180, -90]), 0, 0, new Float32Array(9));
    multiplyInto(flightInto([1, 0, 0], new Float32Array(9)), turn, turn);
    const drawn = new Float32Array(9);
    mirrorInto(turn, 0, drawn, 0);

    /* The flight runs to the viewport's -X, so behind the missile is +X. */
    expect(drawnX(drawn, geometry, 1)).toBeGreaterThan(0);
    expect(drawnX(drawn, geometry, 2)).toBeGreaterThan(0);
  });

  it("keeps each face facing the side it faced in the engine's space, across the mirror", () => {
    const geometry = geometryOf(CONE, MODEL);
    const position = geometry.getAttribute("position");
    const [a, b, c] = Array.from(geometry.getIndex()?.array ?? []);
    const edge = (to: number) => [
      position.getX(to) - position.getX(a),
      position.getY(to) - position.getY(a),
    ];
    const [ux, uy] = edge(b);
    const [vx, vy] = edge(c);

    /* The file's face turns toward -Z, which the mirror on X leaves where it is. */
    expect(ux * vy - uy * vx).toBeLessThan(0);
  });

  it("carries a file's own normals across the mirror", () => {
    const geometry = geometryOf(
      { ...CONE, normals: new Float32Array([1, 0, 0, 0, 1, 0, 0, 0, 1]) },
      MODEL,
    );

    expect(Array.from(geometry.getAttribute("normal").array)).toEqual([
      -1, 0, 0, -0, 1, 0, -0, 0, 1,
    ]);
  });
});
