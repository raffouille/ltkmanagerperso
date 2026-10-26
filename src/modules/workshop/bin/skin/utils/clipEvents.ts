import type { EffectSystem, GraphClip, HashRef } from "@/lib/tauri";

import { nameHash } from "../../shared/utils/binHash";

/** The rate a clip's frames are read at where it sets no tick and its file has no rate. */
const DEFAULT_FPS = 30;

/** One atomic clip of a playlist, placed on the pass the whole playlist makes. */
export interface TimedStep {
  readonly clip: GraphClip;
  /** Seconds into the pass the step starts. */
  readonly start: number;
  /** Seconds one frame of the step's clip lasts, which places its events. */
  readonly frame: number;
}

/**
 * A particle event of the pass, as the viewport spawns it: which system, on which
 * joint, and when.
 */
export interface ParticleCue {
  /** One key per spawn of the pass, which a draw keeps its driver under. */
  readonly key: string;
  /** The `VfxSystemDefinitionData` object, `0x` and eight hex digits. */
  readonly system: string;
  /** Seconds into the pass the system starts. */
  readonly at: number;
  /** Seconds into the pass the system is stopped, from the end frame, and null to play out. */
  readonly until: number | null;
  /** `mBoneName`, and null for the skeleton's own origin. */
  readonly bone: HashRef | null;
  /** `mTargetBoneName`, and null to aim at nothing. */
  readonly target: HashRef | null;
}

/** A joint snap event of the pass: which joint stands on which, and over what span. */
export interface SnapCue {
  readonly joint: HashRef;
  readonly snapTo: HashRef;
  readonly offset: readonly [number, number, number];
  /** Seconds into the pass the snap starts. */
  readonly at: number;
  /** Seconds into the pass the snap ends, and null to hold to the pass's end. */
  readonly until: number | null;
}

/** Every joint snap event of the pass naming both joints, in pass order. */
export function snapCues(steps: readonly TimedStep[]): SnapCue[] {
  const cues: SnapCue[] = [];
  for (const step of steps) {
    for (const event of step.clip.events) {
      const { kind } = event;
      if (kind.kind !== "jointSnap" || kind.joint === null || kind.snapTo === null) continue;
      const at = secondsOf(step, event.startFrame);
      const until = event.endFrame === null ? null : secondsOf(step, event.endFrame);
      cues.push({
        joint: kind.joint,
        snapTo: kind.snapTo,
        offset: [kind.offset[0] ?? 0, kind.offset[1] ?? 0, kind.offset[2] ?? 0],
        at,
        until: until !== null && until > at ? until : null,
      });
    }
  }
  return cues.sort((a, b) => a.at - b.at);
}

/** The submeshes hidden from one moment of the pass on. */
export interface VisibilityEntry {
  /** Seconds into the pass. */
  readonly at: number;
  readonly hidden: readonly string[];
}

/** Seconds one frame of `clip` lasts: its own tick, else its file's rate, else the default. */
export function frameSeconds(clip: GraphClip, fps: number | null): number {
  const tick = clip.tickDuration;
  if (tick !== null && tick > 0) return tick;
  return 1 / (fps !== null && fps > 0 ? fps : DEFAULT_FPS);
}

/**
 * How long one frame lasts while `clip` plays a file of `fps`, and null for no clip.
 *
 * What a pose of that step is sampled on, so the pose and the events read the same clock.
 */
export function clipFrameSeconds(
  clip: GraphClip | null | undefined,
  fps: number | null,
): number | null {
  return clip == null ? null : frameSeconds(clip, fps);
}

/**
 * `playlist` placed on its pass: each step starts where the ones before it end.
 *
 * `owner` is the clip the playlist was made from. Where it plays other clips and fires
 * events of its own, it joins the steps over the whole pass, a frame of it lasting the
 * pass over the frames its clips hold, which is how the game places a sequencer's and a
 * parametric clip's own events.
 */
export function timedSteps(
  playlist: readonly GraphClip[],
  durations: readonly number[],
  rates: readonly (number | null)[],
  owner: GraphClip | null = null,
): TimedStep[] {
  let start = 0;
  let frames = 0;
  const steps = playlist.map((clip, at) => {
    const step = { clip, start, frame: frameSeconds(clip, rates[at] ?? null) };
    start += durations[at] ?? 0;
    frames += (durations[at] ?? 0) / step.frame;
    return step;
  });
  if (owner === null || !owner.ownEvents || playlist.includes(owner) || frames <= 0) return steps;
  return [...steps, { clip: owner, start: 0, frame: start / frames }];
}

/** Seconds into the pass frame `frame` of `step` falls on. */
function secondsOf(step: TimedStep, frame: number | null): number {
  return step.start + (frame ?? 0) * step.frame;
}

/**
 * Every particle event of the pass whose key `systems` maps, one cue per pair.
 *
 * A kill event spawns nothing. A pair naming no bone rides the skeleton's origin, as the
 * engine spawns one.
 */
export function particleCues(
  steps: readonly TimedStep[],
  systems: readonly EffectSystem[],
): ParticleCue[] {
  const cues: ParticleCue[] = [];
  steps.forEach((step, index) => {
    for (const event of step.clip.events) {
      const { kind } = event;
      if (kind.kind !== "particle" || kind.isKill) continue;
      const mapped = systems.find((each) => each.key === kind.effectKey);
      if (mapped === undefined) continue;
      const at = secondsOf(step, event.startFrame);
      const until = event.endFrame === null ? null : secondsOf(step, event.endFrame);
      const spawns = kind.spawns.length === 0 ? [NO_SPAWN] : kind.spawns;
      spawns.forEach((spawn, pair) => {
        cues.push({
          key: `${index}:${event.hash}:${pair}`,
          system: mapped.system,
          at,
          until: until !== null && until > at ? until : null,
          bone: spawn.bone,
          target: spawn.targetBone,
        });
      });
    }
  });
  return cues;
}

const NO_SPAWN = { bone: null, targetBone: null } as const;

/**
 * The submeshes hidden over one pass, as a timeline from `base` at zero, one entry per
 * moment an event changes them.
 *
 * An event hides and shows from its start frame, and one with an end frame puts back
 * what it changed there. Each name the event carries is matched to `submeshes` by hash,
 * which is how the `.skn`'s spelling is reached, and kept as it is where none matches.
 * The pass starts over from `base`, as the engine puts the skin back when a clip ends.
 */
export function visibilityTimeline(
  base: readonly string[],
  submeshes: readonly string[],
  steps: readonly TimedStep[],
): VisibilityEntry[] {
  const spelled = new Map(submeshes.map((name) => [nameHash(name), name]));
  const nameOf = (ref: HashRef) => spelled.get(ref.hash) ?? ref.name;
  const changes: { at: number; show: string[]; hide: string[] }[] = [];
  for (const step of steps) {
    for (const event of step.clip.events) {
      if (event.kind.kind !== "submeshVisibility") continue;
      const show = event.kind.show.map(nameOf);
      const hide = event.kind.hide.map(nameOf);
      changes.push({ at: secondsOf(step, event.startFrame), show, hide });
      if (event.endFrame !== null) {
        changes.push({ at: secondsOf(step, event.endFrame), show: hide, hide: show });
      }
    }
  }
  changes.sort((a, b) => a.at - b.at);

  const timeline: VisibilityEntry[] = [{ at: 0, hidden: base }];
  let hidden = base;
  for (const change of changes) {
    hidden = applyChange(hidden, change);
    const last = timeline[timeline.length - 1];
    if (last.at === change.at) timeline[timeline.length - 1] = { at: change.at, hidden };
    else timeline.push({ at: change.at, hidden });
  }
  return timeline;
}

function applyChange(
  hidden: readonly string[],
  change: { show: readonly string[]; hide: readonly string[] },
): readonly string[] {
  const shown = new Set(change.show.map((name) => name.toLowerCase()));
  const next = hidden.filter((name) => !shown.has(name.toLowerCase()));
  for (const name of change.hide) {
    if (!next.some((held) => held.toLowerCase() === name.toLowerCase())) next.push(name);
  }
  return next;
}

/** The submeshes hidden `time` seconds into the pass: the last entry reached. */
export function hiddenAt(timeline: readonly VisibilityEntry[], time: number): readonly string[] {
  let held = timeline[0]?.hidden ?? [];
  for (const entry of timeline) {
    if (entry.at > time) break;
    held = entry.hidden;
  }
  return held;
}
