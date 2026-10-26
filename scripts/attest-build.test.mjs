import { generateKeyPairSync, verify } from "node:crypto";

import { describe, expect, it } from "vitest";

import { attest, normalizePem, releaseVersion } from "./attest-build.mjs";

const COMMIT = "a".repeat(40);
const PURPOSE = Buffer.from("ltk-manager-attestation\0");

describe("attest", () => {
  it("signs the release version and the commit", () => {
    const { privateKey, publicKey } = generateKeyPairSync("ed25519");

    const value = attest(
      "1.29.0-rc.1",
      COMMIT,
      privateKey.export({ type: "pkcs8", format: "pem" }),
    );

    const [statement, signature] = value.split(".");
    const bytes = Buffer.from(statement, "base64url");
    expect(JSON.parse(bytes.toString())).toEqual({ manager: "1.29.0", commit: COMMIT });
    expect(
      verify(null, Buffer.concat([PURPOSE, bytes]), publicKey, Buffer.from(signature, "base64url")),
    ).toBe(true);
  });

  it("fails if the commit is not 40 hex", () => {
    const { privateKey } = generateKeyPairSync("ed25519");

    expect(() =>
      attest("1.29.0", "abc", privateKey.export({ type: "pkcs8", format: "pem" })),
    ).toThrow("Not a commit hash");
  });
});

describe("normalizePem", () => {
  it("restores a PEM that lost its line breaks or carries the public key line", () => {
    const { privateKey, publicKey } = generateKeyPairSync("ed25519");
    const pem = privateKey.export({ type: "pkcs8", format: "pem" });

    for (const text of [
      pem.replace(/\n/g, ""),
      pem.replace(/\n/g, "\r\n"),
      `${pem}Public key: ${publicKey.export({ format: "jwk" }).x}\n`,
      pem.replace(/-----(BEGIN|END) PRIVATE KEY-----/g, "").trim(),
    ]) {
      const value = attest("1.30.2", COMMIT, text);
      const [statement, signature] = value.split(".");
      expect(
        verify(
          null,
          Buffer.concat([PURPOSE, Buffer.from(statement, "base64url")]),
          publicKey,
          Buffer.from(signature, "base64url"),
        ),
      ).toBe(true);
    }
    expect(normalizePem(pem)).toBe(pem);
  });

  it("fails on text that is not a key", () => {
    expect(() => normalizePem("not a key!")).toThrow("Not a PKCS#8 PEM private key");
  });
});

describe("releaseVersion", () => {
  it("strips a pre-release and build suffix", () => {
    expect(releaseVersion("1.29.0")).toBe("1.29.0");
    expect(releaseVersion("1.29.0-rc.1")).toBe("1.29.0");
    expect(releaseVersion("1.29.0+build.5")).toBe("1.29.0");
  });

  it("fails on a version that is not semver", () => {
    expect(() => releaseVersion("v1.29")).toThrow("Not a semver version");
  });
});
