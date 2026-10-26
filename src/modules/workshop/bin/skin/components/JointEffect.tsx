import { useFrame } from "@react-three/fiber";
import { useMemo } from "react";

import type { JointEffect as JointEffectModel } from "@/lib/tauri";
import type { Pose, SceneClock } from "@/modules/viewport";

import type { SystemModel } from "../../vfx/engine/model/model";
import { VfxSystem } from "../../vfx/rendering/components/VfxSystem";
import { useParticleSystem } from "../../vfx/rendering/hooks/useParticleSystem";
import { drawnEmitters } from "../../vfx/rendering/utils/definitions";
import { followClock, following } from "../utils/follow";
import { jointRig } from "../utils/skinScene";

/** The seed every idle effect runs on, so two readers of one skin see the same run. */
const IDLE_SEED = 1337;

export interface JointEffectProps {
  readonly effect: JointEffectModel;
  readonly system: SystemModel;
  readonly pose: Pose;
  readonly clock: SceneClock;
  /** `skinScale`, which carries the joint the effect rides. */
  readonly scale: number;
}

/**
 * One effect the skin wears, riding its joint.
 *
 * The driver follows the scene's clock rather than the frame, so the effect and the pose
 * it rides are sampled at one time, and a seek or a pause of the clock holds both.
 */
export function JointEffect({ effect, system, pose, clock, scale }: JointEffectProps) {
  const drawn = useMemo(() => drawnEmitters(system, true), [system]);
  const rig = useMemo(() => jointRig(pose, effect, scale), [pose, effect, scale]);
  const { textures, meshes, driver } = useParticleSystem(system, drawn, IDLE_SEED, rig);
  const followed = useMemo(following, []);

  useFrame(() => followClock(driver, clock, followed));

  return <VfxSystem drawn={drawn} driver={driver} textures={textures} meshes={meshes} />;
}
