import { describe, expect, it } from "vitest";

import type { MaterialPreview, VfxValue } from "@/lib/tauri";

import { nameHash } from "../../../../shared/utils/binHash";
import {
  bool,
  list,
  number,
  struct,
  valueCurve,
  vector,
} from "../../../engine/drivers/__tests__/driverFixture";
import { field } from "../../../engine/parsing/readValue";
import { shimmerParticles } from "../../../engine/shimmer/shimmerRun";
import { shimmerMeshesOf } from "../shimmerMeshes";

function asset(path: string): VfxValue {
  return { type: "asset", path, asset: { kind: "file", path } };
}

/** One cube-grid shimmer emitter as the Hall of Legends writes it. */
function cube(name: string, mesh: VfxValue): VfxValue {
  return struct("VfxShimmerEmitterDefinitionData", {
    emitterName: { type: "string", value: name },
    disabled: bool(true),
    VfxComponents: struct("VfxComponents", {
      PhysicsComponent: struct("VfxModularPhysicsComponent", {
        Modifiers: list(
          struct("0x710b2bc2", {
            InitialScale: struct("VfxVector3DynamicProperty", {
              Vector3: struct("VfxVector3ConstantDriver", { Vector3: vector(10, 10, 10) }),
            }),
            InitialRotation: struct("VfxVector3DynamicProperty", {
              Vector3: struct("VfxVector3ConstantDriver", { Vector3: vector(180, 0, 0) }),
            }),
          }),
        ),
      }),
      RenderComponent: struct("VfxMaterialRenderComponent", {
        Color: struct("0x12345678", {
          InitialColor: struct("VfxVector4DynamicProperty", {
            Vector4: struct("0x7cc5a312", {
              colors: valueCurve("ValueColor", vector(0.5, 0.25, 1, 1)),
            }),
          }),
        }),
      }),
      GeometryComponent: struct("VfxGeometryComponent", {
        Primitive: struct("VfxShimmerPrimitiveMesh", {
          texture: asset("assets/cube.tex"),
          mesh,
          count: number(1),
        }),
      }),
    }),
  });
}

function system(...emitters: VfxValue[]): VfxValue {
  return struct("VfxSystemDefinitionData", { shimmerEmitterDefinitionData: list(...emitters) });
}

describe("shimmerMeshesOf", () => {
  it("reads each emitter's mesh, texture and components", () => {
    const [cubeMesh] = shimmerMeshesOf(system(cube("Cube", asset("assets/cube.gmesh"))));

    expect(cubeMesh).toMatchObject({
      index: 0,
      name: "Cube",
      disabled: true,
      mesh: { path: "assets/cube.gmesh" },
      texture: { kind: "file", path: "assets/cube.tex" },
    });
    if (cubeMesh === undefined) throw new Error("the cube has no mesh");
    expect(shimmerParticles(cubeMesh.components, 0, 1)[0]).toMatchObject({
      scale: [10, 10, 10],
      rotation: [180, 0, 0],
      position: [0, 0, 0],
      color: [0.5, 0.25, 1, 1],
    });
  });

  it("draws the complex list's component emitters over their shimmer copies, with their material", () => {
    const drawn = cube("Cube", asset("assets/cube.gmesh"));
    const render = field(field(drawn, nameHash("VfxComponents")), nameHash("RenderComponent"));
    if (render?.type !== "struct" || drawn.type !== "struct") {
      throw new Error("the cube has no render component");
    }
    drawn.fields = drawn.fields.filter((each) => each.hash !== nameHash("disabled"));
    render.fields.push({
      hash: "0x1f14dbe7",
      name: null,
      value: struct("0xd2807c60", { Material: struct("StaticMaterialDef") }),
    });
    const root = struct("VfxSystemDefinitionData", {
      complexEmitterDefinitionData: list(drawn),
      shimmerEmitterDefinitionData: list(cube("Cube", asset("assets/cube.gmesh"))),
    });
    if (root.type === "struct") root.object = { entry: "0x0000abcd", name: null };

    const meshes = shimmerMeshesOf(root);
    const segment = (name: string) => nameHash(name).slice(2);

    expect(meshes.map((each) => each.list)).toEqual(["complex"]);
    expect(meshes[0]?.material).toEqual({
      entry: "0x0000abcd",
      path: [
        `${segment("complexEmitterDefinitionData")}[0]`,
        segment("VfxComponents"),
        segment("RenderComponent"),
        "1f14dbe7",
        segment("Material"),
      ].join("."),
    });
  });

  it("reads a render component's linked material with the file that declares it", () => {
    const emitter = cube("Cube", asset("assets/cube.gmesh"));
    const render = field(field(emitter, nameHash("VfxComponents")), nameHash("RenderComponent"));
    if (render?.type !== "struct") throw new Error("the cube has no render component");
    render.fields.push({
      hash: nameHash("Material"),
      name: "Material",
      value: { type: "link", hash: "0x1234abcd", name: null },
    });
    const preview = { hash: "0x1234abcd", missing: false } as MaterialPreview;

    const [linked] = shimmerMeshesOf(system(emitter), [preview]);
    const [unread] = shimmerMeshesOf(system(emitter));

    expect(linked?.linked).toEqual({ hash: "0x1234abcd" });
    expect(unread?.linked).toBeNull();
  });

  it("leaves out an emitter whose geometry names no mesh", () => {
    const meshes = shimmerMeshesOf(
      system(cube("Quad", asset("assets/quad.tex")), cube("Cube", asset("assets/cube.tmesh"))),
    );

    expect(meshes.map((each) => each.name)).toEqual(["Cube"]);
    expect(meshes[0]?.index).toBe(1);
  });

  it("leaves out a disabled complex emitter, as the game does", () => {
    const root = struct("VfxSystemDefinitionData", {
      complexEmitterDefinitionData: list(cube("Off", asset("assets/cube.gmesh"))),
    });

    expect(shimmerMeshesOf(root)).toEqual([]);
  });
});
