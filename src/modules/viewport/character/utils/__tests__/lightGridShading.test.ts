import {
  type Material,
  ShaderLib,
  Texture,
  type WebGLProgramParametersWithUniforms,
  type WebGLRenderer,
} from "three";
import { describe, expect, it } from "vitest";

import { blackTexel } from "../../../hexshade/neutralTextures";
import {
  applyEmissiveMask,
  gridLitMaterial,
  gridPbrMaterial,
  lightGridUniforms,
} from "../lightGridShading";

/** Three's own shaders of `kind` after `material`'s patch, as three would compile them. */
function compiled(
  material: Material,
  kind: "standard" | "lambert" = "standard",
): WebGLProgramParametersWithUniforms {
  const shader = {
    uniforms: {},
    vertexShader: ShaderLib[kind].vertexShader,
    fragmentShader: ShaderLib[kind].fragmentShader,
  } as unknown as WebGLProgramParametersWithUniforms;
  material.onBeforeCompile(shader, {} as WebGLRenderer);
  return shader;
}

describe("gridPbrMaterial", () => {
  it("replaces each chunk it patches, so none of them is left to three's own reading", () => {
    const shader = compiled(gridPbrMaterial(lightGridUniforms()));

    expect(shader.fragmentShader).not.toContain("#include <roughnessmap_fragment>");
    expect(shader.fragmentShader).not.toContain("#include <metalnessmap_fragment>");
    expect(shader.fragmentShader).not.toContain("#include <normal_fragment_maps>");
    expect(shader.vertexShader).toContain("vLightGrid =");
  });

  it("reads roughness from red, metalness from green and occlusion from blue", () => {
    const { fragmentShader } = compiled(gridPbrMaterial(lightGridUniforms()));

    expect(fragmentShader).toContain("roughnessFactor *= rmaTexel.r;");
    expect(fragmentShader).toContain("metalnessFactor *= rmaTexel.g;");
    expect(fragmentShader).toContain("rmaOcclusion = rmaTexel.b;");
  });

  it("moves the ambient light into image-based light before three resolves it", () => {
    const { fragmentShader } = compiled(gridPbrMaterial(lightGridUniforms()));

    const ambient = fragmentShader.indexOf("iblIrradiance += irradiance;");
    expect(ambient).toBeGreaterThan(fragmentShader.indexOf("#include <lights_fragment_maps>"));
    expect(ambient).toBeLessThan(fragmentShader.indexOf("#include <lights_fragment_end>"));
    expect(
      fragmentShader.indexOf("reflectedLight.indirectSpecular *= rmaOcclusion;"),
    ).toBeGreaterThan(fragmentShader.indexOf("#include <aomap_fragment>"));
  });

  it("shares the light grid uniforms it was made with", () => {
    const uniforms = lightGridUniforms();
    const shader = compiled(gridPbrMaterial(uniforms));

    expect(shader.uniforms.lightGridCube).toBe(uniforms.cube);
    expect(shader.uniforms.lightGridOn).toBe(uniforms.on);
  });
});

describe("applyEmissiveMask", () => {
  it("binds the mask a patched material samples, and a black texel for none", () => {
    const material = gridPbrMaterial(lightGridUniforms());
    const shader = compiled(material);
    const mask = new Texture();

    applyEmissiveMask(material, mask);
    expect(shader.uniforms.lightGridEmissive.value).toBe(mask);

    applyEmissiveMask(material, null);
    expect(shader.uniforms.lightGridEmissive.value).toBe(blackTexel());
  });

  it("keeps a mask per material, apart from the uniforms the materials share", () => {
    const uniforms = lightGridUniforms();
    const lit = gridLitMaterial(uniforms);
    const pbr = gridPbrMaterial(uniforms);
    const mask = new Texture();

    applyEmissiveMask(pbr, mask);

    expect(compiled(pbr).uniforms.lightGridEmissive.value).toBe(mask);
    expect(compiled(lit, "lambert").uniforms.lightGridEmissive.value).toBe(blackTexel());
  });
});

describe("emissive mask", () => {
  it("blends the physically based colour toward the albedo by the mask's green", () => {
    const { fragmentShader } = compiled(gridPbrMaterial(lightGridUniforms()));

    const blend = fragmentShader.indexOf(
      "outgoingLight = mix( outgoingLight, diffuseColor.rgb, emissiveMask );",
    );
    expect(fragmentShader).toContain("emissiveMask = texture2D( lightGridEmissive, vMapUv ).g;");
    expect(blend).toBeGreaterThan(fragmentShader.indexOf("#include <aomap_fragment>"));
    expect(blend).toBeLessThan(fragmentShader.indexOf("#include <opaque_fragment>"));
  });

  it("replaces the grid light of the Lambert material by the mask, as LIT_UBER does", () => {
    const { fragmentShader } = compiled(gridLitMaterial(lightGridUniforms()), "lambert");

    expect(fragmentShader).toContain("vLightGrid * ( 1.0 - emissiveMask ) * lightGridScale.x");
    expect(fragmentShader).toContain("+ emissiveMask");
  });
});
