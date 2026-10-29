// @vitest-environment happy-dom

import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { AssetRef, BinRow, BinValue, FieldSchema } from "@/lib/tauri";
import type { MeshGeometry } from "@/modules/viewport";
import { createTestQueryClient } from "@/test/utils";

import { FieldLabelsContext } from "../../../../classes/state/fieldLabels";
import { nameHash } from "../../../../shared/utils/binHash";
import { LeafEditContext } from "../../../../tree/hooks/useLeafEdit";
import { RowDocumentContext } from "../../../../tree/state/rowFold";
import { rowKey } from "../../../../tree/utils/binRows";
import type { EmissionSurfaceModel } from "../../../engine/model/model";
import {
  GENERATOR_CLASSES,
  MESH_SURFACE,
  SURFACE,
  SURFACE_FIELD,
} from "../../utils/emissionSource";
import type { DefaultField } from "../../utils/emitterGroups";
import { emitterLabel } from "../../utils/emitterLabels";
import { SurfaceProperty } from "../SurfaceProperty";

const DEFINITION = nameHash("VfxEmissionSurfaceData");
const MESH_ASSET: AssetRef = {
  kind: "gameChunk",
  wad: "Ahri.wad.client",
  pathHash: "0".repeat(16),
};

function schemaField(name: string, kind: NonNullable<FieldSchema["declared"]>["kind"]) {
  return {
    hash: nameHash(name),
    name,
    declared: { kind, key: null, value: null },
    classHash: null,
    defaultValue: kind === "string" ? '""' : null,
    owner: null,
    revisions: [],
  } satisfies FieldSchema;
}

const SCHEMAS: Record<string, { name: string; fields: FieldSchema[] }> = {
  [MESH_SURFACE.hash]: {
    name: MESH_SURFACE.name,
    fields: [
      schemaField("meshName", "string"),
      schemaField("skeletonName", "string"),
      schemaField("Submeshes", "list"),
      schemaField("AnimationName", "string"),
    ],
  },
};

const reads = new Map<string, BinRow[]>();
const fixture = vi.hoisted(() => ({
  surface: null as EmissionSurfaceModel | null,
  host: undefined as
    | { mesh: { path: string } | null; skeleton: { path: string } | null }
    | undefined,
}));

vi.mock("../../../../classes/hooks/useClassSchema", () => ({
  useClassSchema: (classHash: string | null) => ({
    data: classHash === null ? undefined : SCHEMAS[classHash],
  }),
}));
vi.mock("../../../../documents/hooks/useBinRead", () => ({
  useBinRead: (_document: number, requests: readonly { key: string }[]) =>
    new Map(requests.map(({ key }) => [key, { rows: reads.get(key) ?? [] }])),
}));
vi.mock("../../../../classes/components/ClassCells", async (original) => ({
  ...(await original<typeof import("../../../../classes/components/ClassCells")>()),
  AlsoCheck: ({ children }: { children: ReactNode }) => children,
}));
vi.mock("../../../../documents/components/DeclaredLayer", () => ({
  DeclaredRowState: () => null,
  DeclaredRowMark: () => null,
  DeclaredDiagnosticsMark: () => null,
}));
vi.mock("../../../../../state", async (original) => ({
  ...(await original<typeof import("../../../../../state")>()),
  useOpenDocumentAs: () => () => {},
}));
vi.mock("../../state/emitterModel", () => ({
  useEmitterModel: () => ({ emissionSurface: fixture.surface }),
}));
vi.mock("../../../preview/components/VfxHost", () => ({
  useHostSkin: () => ({ model: fixture.host, name: "Ahri" }),
}));

afterEach(cleanup);
beforeEach(() => {
  reads.clear();
  fixture.surface = null;
  fixture.host = undefined;
});

const holder: BinRow = {
  entry: "0x00000001",
  path: "00000002[0]",
  label: "emitters[0]",
  name: "[0]",
  node: "element",
  unnamed: false,
  kind: "embed",
  declared: null,
  value: { type: "struct", classHash: "0x09cde442", class: "VfxEmitterDefinitionData", len: 0 },
};

const field: DefaultField = {
  hash: SURFACE_FIELD,
  name: "emissionSurfaceDefinition",
  declared: { kind: "pointer", key: null, value: null },
  classHash: DEFINITION,
  defaultValue: "null",
};

/** The row of field `name` under `parent`, or of the list item when `name` is an index. */
function under(parent: BinRow, name: string, value: BinValue): BinRow {
  const item = name.startsWith("[");
  return {
    entry: parent.entry,
    path: item ? `${parent.path}${name}` : `${parent.path}.${nameHash(name).slice(2)}`,
    label: `${parent.label}.${name}`,
    name,
    node: item ? "element" : "property",
    unnamed: false,
    kind: null,
    declared: null,
    value,
  };
}

/** A definition with a skinned mesh surface whose `Submeshes` lists `Tail`. */
function skinned() {
  const definition = under(holder, "emissionSurfaceDefinition", {
    type: "struct",
    classHash: DEFINITION,
    class: "VfxEmissionSurfaceData",
    len: 1,
  });
  const surface = under(definition, "EmissionSurface", {
    type: "struct",
    classHash: MESH_SURFACE.hash,
    class: MESH_SURFACE.name,
    len: 2,
  });
  const submeshes = under(surface, "Submeshes", { type: "container", len: 1, itemKind: "hash" });
  const animation = under(surface, "AnimationName", { type: "string", value: "idle.anm" });

  reads.set(rowKey(definition), [surface]);
  reads.set(rowKey(surface), [submeshes, animation]);
  reads.set(rowKey(submeshes), [
    under(submeshes, "[0]", { type: "hash", hash: nameHash("Tail"), name: null }),
  ]);

  return { definition, surface };
}

function mount(authored?: BinRow, mesh?: MeshGeometry) {
  const commit = vi.fn();
  const editProperty = vi.fn().mockResolvedValue(true);
  const client = createTestQueryClient();
  if (mesh !== undefined) client.setQueryData(["viewport", "mesh", MESH_ASSET], mesh);

  render(
    <QueryClientProvider client={client}>
      <LeafEditContext value={{ commit, editProperty, refused: new Map() }}>
        <RowDocumentContext value={1}>
          <FieldLabelsContext value={emitterLabel}>
            <SurfaceProperty
              field={field}
              holder={holder}
              authored={authored}
              width="w-32"
              owner="0x09cde442"
            />
          </FieldLabelsContext>
        </RowDocumentContext>
      </LeafEditContext>
    </QueryClientProvider>,
  );

  return { editProperty };
}

function meshOf(...names: string[]): MeshGeometry {
  return {
    positions: new Float32Array(),
    normals: null,
    uvs: null,
    skinIndices: null,
    skinWeights: null,
    colors: null,
    tangents: null,
    indices: new Uint32Array(),
    ranges: names.map((name) => ({ name, startIndex: 0, indexCount: 0 })),
  };
}

const SURFACE_MODEL: EmissionSurfaceModel = {
  kind: "mesh",
  mesh: { path: "ahri.skn", asset: MESH_ASSET },
  skeleton: null,
  submeshes: [],
  joints: [],
  scale: 1,
  maxJointWeights: 4,
  useNormal: true,
};

describe("the surface's two parts", () => {
  it("shows both parts as Not set on an emitter with no surface, and writes the definition on a pick", async () => {
    const { editProperty } = mount();
    const picker = screen.getByRole("combobox", { name: "Surface" });
    expect(picker).toHaveTextContent("Not set");
    expect(screen.getByRole("combobox", { name: "Spawn Generator" })).toHaveTextContent("Not set");

    await userEvent.click(picker);
    await userEvent.click(await screen.findByRole("option", { name: /^Skinned Mesh/ }));

    expect(editProperty).toHaveBeenCalledWith(holder, SURFACE_FIELD, [
      { type: "ensurePointer", path: "", class: "VfxEmissionSurfaceData" },
      { type: "ensureProperty", path: "", field: SURFACE.surface },
      { type: "replacePointer", path: SURFACE.surface.slice(2), class: MESH_SURFACE.hash },
    ]);
  });

  it("sets the generator's class with an edit that does not change the surface", async () => {
    const { definition } = skinned();
    const { editProperty } = mount(definition);

    await userEvent.click(screen.getByRole("combobox", { name: "Spawn Generator" }));
    await userEvent.click(await screen.findByRole("option", { name: /^Navigation Grid/ }));

    expect(editProperty).toHaveBeenCalledWith(holder, SURFACE_FIELD, [
      { type: "ensurePointer", path: "", class: "VfxEmissionSurfaceData" },
      { type: "ensureProperty", path: "", field: SURFACE.generator },
      {
        type: "replacePointer",
        path: SURFACE.generator.slice(2),
        class: GENERATOR_CLASSES[0]?.hash,
      },
    ]);
  });

  it("clears the surface pointer when Not set is picked", async () => {
    const { definition } = skinned();
    const { editProperty } = mount(definition);
    const picker = screen.getByRole("combobox", { name: "Surface" });
    expect(picker).toHaveTextContent("Skinned Mesh");

    await userEvent.click(picker);
    await userEvent.click(await screen.findByRole("option", { name: /^Not set/ }));

    expect(editProperty.mock.calls[0]?.[2].at(-1)).toEqual({
      type: "replacePointer",
      path: SURFACE.surface.slice(2),
      class: null,
    });
  });
});

describe("a skinned mesh surface's fields", () => {
  it("shows a submesh hash as the loaded mesh's name for it, and removes it in one edit", async () => {
    const { definition, surface } = skinned();
    fixture.surface = SURFACE_MODEL;
    const { editProperty } = mount(definition, meshOf("Body", "tail"));

    expect(screen.getByText("tail")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Remove tail" }));

    expect(editProperty).toHaveBeenCalledWith(surface, SURFACE.submeshes, [
      { type: "removeItem", path: "[0]" },
    ]);
  });

  it("appends a submesh picked from the loaded mesh", async () => {
    const { definition, surface } = skinned();
    fixture.surface = SURFACE_MODEL;
    const { editProperty } = mount(definition, meshOf("Body", "tail"));

    await userEvent.click(screen.getByRole("combobox", { name: "Add submesh" }));
    await userEvent.click(await screen.findByRole("option", { name: "Body" }));

    expect(editProperty).toHaveBeenCalledWith(surface, SURFACE.submeshes, [
      { type: "insertItem", path: "", item: { index: null, key: null, class: null } },
      { type: "setLeaf", path: "[1]", value: { type: "hash", text: "Body" } },
    ]);
  });

  it("accepts a typed name when no mesh has loaded, and shows the listed hash", async () => {
    const { definition, surface } = skinned();
    const { editProperty } = mount(definition);

    expect(screen.getByText(nameHash("Tail"))).toBeInTheDocument();
    await userEvent.type(screen.getByRole("textbox", { name: "Add submesh" }), "Orb{Enter}");

    expect(editProperty).toHaveBeenCalledWith(surface, SURFACE.submeshes, [
      { type: "insertItem", path: "", item: { index: null, key: null, class: null } },
      { type: "setLeaf", path: "[1]", value: { type: "hash", text: "Orb" } },
    ]);
  });

  it("states that AnimationName is not read when the file has it", () => {
    const { definition } = skinned();
    mount(definition);

    expect(screen.getByText(/A complex emitter does not read this field/)).toBeInTheDocument();
  });

  it("writes the host's mesh and skeleton paths in one edit, and hides the button without a host", async () => {
    const { definition } = skinned();
    mount(definition);
    expect(screen.queryByRole("button", { name: "Use host" })).not.toBeInTheDocument();
    cleanup();

    fixture.host = { mesh: { path: "ahri.skn" }, skeleton: { path: "ahri.skl" } };
    const { editProperty } = mount(definition);
    await userEvent.click(screen.getByRole("button", { name: "Use host" }));

    expect(editProperty).toHaveBeenCalledWith(definition, SURFACE.surface, [
      { type: "ensureProperty", path: "", field: SURFACE.mesh },
      {
        type: "setLeaf",
        path: SURFACE.mesh.slice(2),
        value: { type: "string", value: "ahri.skn" },
      },
      { type: "ensureProperty", path: "", field: SURFACE.skeleton },
      {
        type: "setLeaf",
        path: SURFACE.skeleton.slice(2),
        value: { type: "string", value: "ahri.skl" },
      },
    ]);
  });
});
