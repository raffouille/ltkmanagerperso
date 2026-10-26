import type { AssetRef, BinValue } from "@/lib/tauri";

import {
  type ContentDocument,
  type ContentDocumentOf,
  objectDocument,
  previewDocument,
} from "../../../documents/utils/contentDocument";
import { assetContext } from "../../../preview/utils/assetRef";
import { nameHash } from "../../shared/utils/binHash";
import type { LinkTargets } from "../hooks/useLinkTargets";

/** How a link value draws, and what its chip opens. "Links" in docs/ux/BIN_EDITOR.md. */
export type LinkDecision =
  | {
      /** A chip that opens `document`. `side` is the word a `file` chip carries. */
      readonly kind: "chip";
      readonly document: ContentDocument;
      readonly side?: string;
    }
  | {
      /** A chip whose click builds the index. The target opens on the answer. */
      readonly kind: "warm";
    }
  | {
      /** The check has not answered. */
      readonly kind: "pending";
    }
  | {
      /** No chip: dim hex for a `link`, text for a `hash` and a `file`. */
      readonly kind: "text";
    };

/** A path that resolved and that nothing on this machine holds. */
export interface MissingChunk {
  readonly kind: "missing";
}

/** A `file` link's decision, whose chip opens a preview. */
export type FileLinkDecision =
  | Exclude<LinkDecision, { kind: "chip" }>
  | MissingChunk
  | {
      readonly kind: "chip";
      readonly document: ContentDocumentOf<"preview">;
      readonly side?: string;
    };

/** The display title of a layer, which a chip for a layer's copy shows. */
export type LayerTitle = (layer: string) => string;

/** The title of the layer `asset` is a file of, or undefined for an asset of the install. */
export function layerCopyTitle(asset: AssetRef, title: LayerTitle): string | undefined {
  return asset.kind === "layer" ? title(asset.layer) : undefined;
}

const TEXT = { kind: "text" } as const satisfies LinkDecision;
const MISSING = { kind: "missing" } as const satisfies MissingChunk;
const WARM = { kind: "warm" } as const satisfies LinkDecision;
const PENDING = { kind: "pending" } as const satisfies LinkDecision;

/**
 * What an `ObjectLink` draws as.
 *
 * A declared target opens its first declaration, which the backend orders per
 * ADR-0028. While the index is absent or building, a target outside the file is a chip
 * whose click warms the index. A target the ready index does not hold is text.
 */
export function decideObjectLink(hash: string, targets: LinkTargets): LinkDecision {
  const declared = targets.declared.get(hash);
  if (declared) {
    const [first] = declared.declarations;
    if (!first) return TEXT;
    return {
      kind: "chip",
      document: objectDocument(first.asset, hash, declared.path, first.file, first.class),
    };
  }
  const status = targets.index?.status;
  if (status === "ready" || status === "failed") return TEXT;
  if (status === "building" || status === "absent") return WARM;
  return targets.pending ? PENDING : TEXT;
}

/** What a `Hash` draws as: a chip where the index declares an object under it, else text. */
export function decideHash(hash: string, targets: LinkTargets): LinkDecision {
  const declared = targets.declared.get(hash);
  const [first] = declared?.declarations ?? [];
  if (!declared || !first) return TEXT;
  return {
    kind: "chip",
    document: objectDocument(first.asset, hash, declared.path, first.file, first.class),
  };
}

/**
 * What a `WadChunkLink` draws as.
 *
 * The sandbox returns the copy the build uses, a layer's before the install's, and the chip
 * shows where that copy is. A path nothing resolves is text. A path nothing holds is
 * missing, which is a chunk the file names and nothing on this machine holds.
 */
export function decideFileLink(
  path: string | null,
  targets: LinkTargets,
  title: LayerTitle,
): FileLinkDecision {
  if (path === null) return TEXT;
  const located = targets.located.get(path);
  if (located) {
    const side = located.kind === "layer" ? title(located.layer) : assetContext(located);
    return { kind: "chip", document: previewDocument(located, path), side };
  }
  return targets.pending ? PENDING : MISSING;
}

/** The roots the game's own chunks sit under, whose paths may hold a space. */
const GAME_ROOTS = ["assets/", "data/"];

/**
 * The chunk path a `string` names, or null where it names none.
 *
 * A folder and a file name with an extension. A mod's own chunks sit under roots of its
 * choosing, so any root counts, and one outside `ASSETS/` and `DATA/` holds no whitespace
 * so that prose with a slash stays text. Lowercased as the tables spell it, which is the
 * one spelling the resolver, the layer and the preview all answer under.
 */
export function chunkPath(text: string): string | null {
  const path = text.toLowerCase();
  const slash = path.lastIndexOf("/");
  if (slash < 1 || path.startsWith("/")) return null;

  const gameRoot = GAME_ROOTS.some((root) => path.startsWith(root));
  if (!gameRoot && /\s/.test(path)) return null;

  const name = path.slice(slash + 1);
  const dot = name.lastIndexOf(".");
  if (dot < 1 || dot === name.length - 1) return null;

  return path;
}

/**
 * What a `string` draws as, per "A string that names a thing" in docs/ux/BIN_EDITOR.md.
 *
 * A path the resolver holds takes the chunk, and any other string takes the object its
 * hash declares. One that answers on both sides takes the chunk.
 */
export function decideStringLink(
  text: string,
  targets: LinkTargets,
  title: LayerTitle,
): LinkDecision | MissingChunk {
  const path = chunkPath(text);
  if (path !== null) {
    const chunk = decideFileLink(path, targets, title);
    if (chunk.kind === "chip") return chunk;
    /* A path-shaped string the index also declares is that object, not a lost chunk. */
    const declared = decideHash(nameHash(text), targets);
    return declared.kind === "chip" ? declared : chunk;
  }
  return decideHash(nameHash(text), targets);
}

/** The decision for any row value, or null for a value that is no link. */
export function decideLink(
  value: BinValue,
  targets: LinkTargets,
  title: LayerTitle,
): LinkDecision | MissingChunk | null {
  switch (value.type) {
    case "objectLink":
      return decideObjectLink(value.hash, targets);
    case "hash":
      return decideHash(value.hash, targets);
    case "wadChunkLink":
      return decideFileLink(value.path, targets, title);
    case "string":
      return decideStringLink(value.value, targets, title);
    default:
      return null;
  }
}
