import type {
  HashRef,
  JointTreeGroup,
  PoseModifier,
  ScaledCurve,
  SkinModel,
  Socket,
} from "@/lib/tauri";
import { createPose, type JointModel, type SkeletonModel } from "@/modules/viewport";

import { nameHash } from "../../../shared/utils/binHash";
import { MESH_PROPERTIES } from "../dynamicsFields";
import type { WireChain } from "../dynamicsModel";

function joint(name: string, parent: number, y = -10): JointModel {
  return {
    name,
    hash: 0,
    parent,
    translation: [0, y, 0],
    rotation: [0, 0, 0, 1],
    scale: [1, 1, 1],
    inverseBind: new Float32Array(16),
  };
}

export const SKELETON: SkeletonModel = {
  joints: [
    joint("Root", -1, 100),
    joint("Hair_Root", 0),
    joint("Hair_1", 1),
    joint("Hair_Clip", 2),
    joint("Hair_2", 2),
    joint("L_Pauldron", 0),
  ],
  influences: Uint32Array.of(0, 1, 2, 3, 4, 5),
};
export const POSE = createPose(SKELETON, null);

export const MESH = MESH_PROPERTIES.slice(2);
export const MODIFIERS = `${MESH}.${nameHash("rigPoseModifierData").slice(2)}`;
export const SOCKETS = `${MESH}.${nameHash("SocketDefinitions").slice(2)}`;
export const GROUPS = nameHash("JointTreeGroups").slice(2);
export const TREES = nameHash("JointTrees").slice(2);

/** A hash the tables name. */
export function ref(name: string): HashRef {
  return { name, hash: nameHash(name) };
}

/** The hash of `name` where no table names it, so its name is its hex. */
export function unnamed(name: string): HashRef {
  return { name: nameHash(name), hash: nameHash(name) };
}

export function constant(value: number): ScaledCurve {
  return { value, useCurve: false, curve: null };
}

export function group(at: number, trees: { root: string; excluded?: string[] }[]): JointTreeGroup {
  const path = `${MODIFIERS}[0].${GROUPS}[${at}]`;
  return {
    path,
    trees: trees.map((tree, index) => ({
      path: `${path}.${TREES}[${index}]`,
      root: ref(tree.root),
      excluded: (tree.excluded ?? []).map(ref),
    })),
    properties: {
      useRodPhysics: false,
      damping: {
        value: 0.8,
        useCurve: true,
        curve: { times: [0, 1], values: [1, 0], modes: [0] },
      },
      attraction: constant(0.5),
      radius: constant(5),
      envelope: constant(1),
      limitAngle: constant(180),
      stretch: constant(0),
      rodBend: constant(1),
      rodTwist: constant(1),
      rodStretch: constant(1),
      rodShear: constant(1),
    },
    sharedCurveLength: false,
    lateralLinks: false,
    lateralLinkMaterial: 2,
    restLengthFromPose: true,
    tipsWithoutRadius: false,
  };
}

export function chain(groups: JointTreeGroup[]): WireChain {
  return {
    kind: "dynamicsChain",
    path: `${MODIFIERS}[0]`,
    defaultOn: true,
    globalEnvelope: 1,
    gravityScale: 1,
    gravityOverride: null,
    colliderFile: null,
    groups,
  };
}

export function spring(at: number, on: HashRef): PoseModifier {
  return {
    kind: "spring",
    path: `${MODIFIERS}[${at}]`,
    name: ref("pauldron"),
    joint: on,
    mass: 0.1,
    stiffness: 2.5,
    damping: 1,
    doTranslation: true,
    doRotation: false,
    maxDistance: 0,
    maxAngle: 0,
    invert: false,
    defaultOn: true,
  };
}

export function orientation(at: number, over: Partial<PoseModifier> = {}): PoseModifier {
  return {
    kind: "jointOrientation",
    path: `${MODIFIERS}[${at}]`,
    joints: [ref("l_pauldron"), ref("gone")],
    source: "0x19da44b2",
    orientationType: 1,
    planeConstraint: 0,
    tiltAxis: 2,
    aimAxis: 1,
    aimNegated: true,
    flipped: true,
    maxAngle: 12,
    defaultOn: false,
    ...over,
  } as PoseModifier;
}

export function socket(at: number, name: string, parent: HashRef): Socket {
  return {
    kind: "singleJoint",
    path: `${SOCKETS}[${at}]`,
    name,
    parent,
    position: [0, 12, 0],
    rotation: [0, 0, 0],
    freezePosition: [false, false, false],
    freezeRotation: [false, false, false],
  };
}

export function skin(over: Partial<SkinModel> = {}): SkinModel {
  return {
    mesh: null,
    skeleton: null,
    texture: null,
    emissiveTexture: null,
    material: null,
    overrides: [],
    hidden: [],
    scale: 1,
    selfIllumination: 0,
    animationGraph: null,
    idleEffects: [],
    persistentConditions: [],
    effectSystems: [],
    poseModifiers: [],
    sockets: [],
    ...over,
  };
}

/** A hair chain with one excluded joint, a spring on a pauldron and a socket on the hair's tip. */
export const HAIR = skin({
  poseModifiers: [
    chain([group(0, [{ root: "Hair_Root", excluded: ["Hair_Clip"] }])]),
    spring(1, ref("L_Pauldron")),
  ],
  sockets: [socket(0, "HairTip", ref("Hair_2"))],
});
