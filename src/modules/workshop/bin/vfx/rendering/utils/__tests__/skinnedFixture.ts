import { Matrix4 } from "three";

import type { ClipModel, MeshGeometry, SkeletonModel } from "@/modules/viewport";

export const SKELETON: SkeletonModel = {
  joints: [
    {
      name: "root",
      hash: 1,
      parent: -1,
      translation: [0, 0, 0],
      rotation: [0, 0, 0, 1],
      scale: [1, 1, 1],
      inverseBind: Float32Array.from(new Matrix4().elements),
    },
    {
      name: "tip",
      hash: 2,
      parent: 0,
      translation: [2, 0, 0],
      rotation: [0, 0, 0, 1],
      scale: [1, 1, 1],
      inverseBind: Float32Array.from(new Matrix4().makeTranslation(-2, 0, 0).elements),
    },
  ],
  influences: Uint32Array.of(1, 0),
};

export const CLIP: ClipModel = {
  fps: 1,
  frames: 2,
  joints: Uint32Array.of(2),
  poses: Float32Array.of(2, 0, 0, 0, 0, 0, 1, 1, 1, 1, 6, 0, 0, 0, 0, 0, 1, 1, 1, 1),
};

export const MESH: MeshGeometry = {
  positions: Float32Array.of(2, 0, 0, 2, 1, 0, 2, 0, 1),
  normals: null,
  uvs: null,
  skinIndices: new Uint8Array(12),
  skinWeights: Float32Array.of(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0),
  colors: null,
  tangents: null,
  indices: Uint32Array.of(0, 1, 2),
  ranges: [],
};
