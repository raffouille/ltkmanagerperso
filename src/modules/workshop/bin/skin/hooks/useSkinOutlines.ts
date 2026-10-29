import { useQuery } from "@tanstack/react-query";
import { useCallback, useMemo } from "react";
import { NoColorSpace } from "three";

import type { BinDocumentId, MaterialProgram, SkinModel } from "@/lib/tauri";
import { programTextureAssets, type SubmeshOutline, useAssetTextures } from "@/modules/viewport";

import { skinQueries } from "../api/skinQueries";
import { outlineMaskAssets, outlineMaterialHashes, submeshOutline } from "../utils/skinScene";

/** Masks and program textures load without colour decoding. */
const RAW_TEXTURES = { colorSpace: NoColorSpace } as const;

/** The outline materials read while the shaders are off, which is none. */
const NO_HASHES: readonly string[] = [];

const NO_PROGRAMS: readonly (MaterialProgram | null)[] = [];

/**
 * The outline each submesh of `skin` draws, read from `document`, and null for none.
 *
 * While `shaders` is on, an outline draws with its material's translated pass, and an outline
 * whose pass did not translate draws the stock outline material, as every outline does while
 * `shaders` is off.
 */
export function useSkinOutlines(
  document: BinDocumentId,
  skin: SkinModel,
  shaders: boolean,
): (submesh: string) => SubmeshOutline | null {
  const hashes = useMemo(
    () => (shaders ? outlineMaterialHashes(skin) : NO_HASHES),
    [shaders, skin],
  );
  const programs = useQuery(skinQueries.programs(document, hashes)).data ?? NO_PROGRAMS;
  const programAssets = useMemo(() => programTextureAssets(programs), [programs]);
  const textures = useAssetTextures(programAssets, RAW_TEXTURES);
  const maskAssets = useMemo(() => outlineMaskAssets(skin), [skin]);
  const masks = useAssetTextures(maskAssets, RAW_TEXTURES);

  return useCallback(
    (submesh: string) => submeshOutline(skin, programs, textures, masks, submesh),
    [skin, programs, textures, masks],
  );
}
