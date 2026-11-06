// @vitest-environment node
/**
 * The known-answer vectors of CRYPTO_SPEC section 16 (V1 to V11), as the Rust
 * `mosaic-passkey` crate pins them in `crates/passkey/fixtures/vectors.json`,
 * checked here with an independent implementation: `test/webauthn/verifier.ts`
 * has its own CBOR, COSE and authenticator-data parsers and only `node:crypto`
 * for hashing and ECDSA.
 */
import { createHash, createPrivateKey, createPublicKey } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  coseToJwk,
  parseAttestationObject,
  parseAuthData,
  verifyAssertion,
  verifyEs256,
  verifyRegistration,
} from "../../test/webauthn/verifier.ts";

interface Fixture {
  key: { d: string; x: string; y: string; cose: string; spki: string };
  rfc6979: { sample: string; test: string };
  rpId: string;
  rpIdHash: string;
  origin: string;
  challenge: string;
  credentialId: string;
  create: { flags: string; authData: string; clientDataJson: string; attestationObject: string };
  get: { flags: string; signCount: number; authData: string; clientDataJson: string; signature: string };
  getImportedCount: {
    flags: string;
    storedCount: number;
    signCount: number;
    authData: string;
    clientDataJson: string;
    signature: string;
  };
  ccd: { inputUtf8: string; outputUtf8: string };
}

const fixture = JSON.parse(
  readFileSync(join(import.meta.dirname, "../../../../crates/passkey/fixtures/vectors.json"), "utf8"),
) as Fixture;

const bytes = (hex: string): Uint8Array => new Uint8Array(Buffer.from(hex, "hex"));
const hex = (b: Uint8Array): string => Buffer.from(b).toString("hex");
const b64url = (b: Uint8Array): string => Buffer.from(b).toString("base64url");
const sha256Hex = (text: string): string => createHash("sha256").update(text, "utf8").digest("hex");

// Values fixed by standards, not by the Rust code: RFC 6979 A.2.5 (P-256, SHA-256).
const RFC_D = "c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f6721";
const RFC_X = "60fed4ba255a9d31c961eb74c6356d68c049b8923b61fa6ce669622e60f29fb6";
const RFC_Y = "7903fe1008b8bc99a41ae9e95628bc64f2f1b20c2d7e9f5177a3c294d4462299";
const RFC_SAMPLE =
  "3046022100efd48b2aacb6a8fd1140dd9cd45e81d69d2c877b56aaf991c34d0ea84eaf3716022100f7cb1c942d657c41d436c7a1b6e29f65f3e900dbb9aff4064dc4ab2f843acda8";
const RFC_TEST =
  "3045022100f1abb023518351cd71d881567b1ea663ed3efcf6c5132b354f28d3b0b7d383670220019f4113742a2b14bd25926b49c649155f267e60d3814b4c0cc84250e46f0083";
const SPKI_PREFIX = "3059301306072a8648ce3d020106082a8648ce3d030107034200";

/**
 * CCDToString (WebAuthn L3, 5.8.1.1), written from the specification text and
 * independent of the Rust code. Per code point: U+0020, U+0021, U+0023 to
 * U+005B and U+005D to U+10FFFF are copied (so DEL and U+2028 stay raw),
 * U+0022 becomes backslash-quote, U+005C becomes two backslashes, and every
 * other code point (U+0000 to U+001F) becomes backslash-u and four lowercase
 * hex digits.
 */
function ccdToString(input: string): string {
  let out = '"';
  for (const ch of input) {
    const cp = ch.codePointAt(0) ?? 0;
    if (cp === 0x20 || cp === 0x21 || (cp >= 0x23 && cp <= 0x5b) || (cp >= 0x5d && cp <= 0x10ffff)) {
      out += ch;
    } else if (cp === 0x22) {
      out += '\\"';
    } else if (cp === 0x5c) {
      out += "\\\\";
    } else {
      out += `\\u${cp.toString(16).padStart(4, "0")}`;
    }
  }
  return `${out}"`;
}

const challenge = bytes(fixture.challenge);
const expected = {
  expectedOrigin: fixture.origin,
  expectedRpId: fixture.rpId,
  expectedChallenge: challenge,
};

describe("known-answer vectors", () => {
  it("the fixture is for the documented origin and credential id", () => {
    expect(fixture.origin).toBe("https://example.com");
    expect(fixture.credentialId).toBe("000102030405060708090a0b0c0d0e0f");
  });

  it("V1: the key is the RFC 6979 key and its COSE public key has the canonical layout", () => {
    expect(fixture.key.d).toBe(RFC_D);
    expect(fixture.key.x).toBe(RFC_X);
    expect(fixture.key.y).toBe(RFC_Y);
    expect(fixture.key.cose).toBe(`a5010203262001215820${RFC_X}225820${RFC_Y}`);
    expect(bytes(fixture.key.cose)).toHaveLength(77);
    const jwk = coseToJwk(bytes(fixture.key.cose));
    expect(jwk).toEqual({ kty: "EC", crv: "P-256", x: b64url(bytes(RFC_X)), y: b64url(bytes(RFC_Y)) });
  });

  it("V1: node derives the same public key from the private scalar", () => {
    const priv = createPrivateKey({
      key: { kty: "EC", crv: "P-256", d: b64url(bytes(RFC_D)), x: b64url(bytes(RFC_X)), y: b64url(bytes(RFC_Y)) },
      format: "jwk",
    });
    const pub = createPublicKey(priv).export({ format: "jwk" });
    expect(pub.x).toBe(b64url(bytes(RFC_X)));
    expect(pub.y).toBe(b64url(bytes(RFC_Y)));
  });

  it("V2: the SPKI is the fixed prefix, 04, X, Y and node reads it", () => {
    expect(fixture.key.spki).toBe(`${SPKI_PREFIX}04${RFC_X}${RFC_Y}`);
    expect(bytes(fixture.key.spki)).toHaveLength(91);
    const imported = createPublicKey({ key: Buffer.from(fixture.key.spki, "hex"), format: "der", type: "spki" });
    const jwk = imported.export({ format: "jwk" });
    expect(jwk.x).toBe(b64url(bytes(RFC_X)));
    expect(jwk.y).toBe(b64url(bytes(RFC_Y)));
  });

  it("V3, V4: the RFC 6979 signatures are the standard's and verify", () => {
    expect(fixture.rfc6979.sample).toBe(RFC_SAMPLE);
    expect(fixture.rfc6979.test).toBe(RFC_TEST);
    const jwk = coseToJwk(bytes(fixture.key.cose));
    expect(verifyEs256(jwk, new TextEncoder().encode("sample"), bytes(RFC_SAMPLE))).toBe(true);
    expect(verifyEs256(jwk, new TextEncoder().encode("test"), bytes(RFC_TEST))).toBe(true);
    expect(verifyEs256(jwk, new TextEncoder().encode("test"), bytes(RFC_SAMPLE))).toBe(false);
  });

  it("V5: the rpIdHash is SHA-256 of the rp id", () => {
    expect(fixture.rpId).toBe("example.com");
    expect(fixture.rpIdHash).toBe(sha256Hex("example.com"));
    expect(fixture.rpIdHash).toBe("a379a6f6eeafb9a55e378c118034e2751e682fab9f2d30ab13d2125586ce1947");
  });

  it("V6, V7: clientDataJSON is byte for byte the limited verification format", () => {
    expect(challenge).toHaveLength(32);
    expect(b64url(challenge)).toBe("AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8");
    const make = (type: string): string =>
      `{"type":"${type}","challenge":"${b64url(challenge)}","origin":"${fixture.origin}","crossOrigin":false}`;
    expect(Buffer.from(fixture.create.clientDataJson, "hex").toString("utf8")).toBe(make("webauthn.create"));
    expect(Buffer.from(fixture.get.clientDataJson, "hex").toString("utf8")).toBe(make("webauthn.get"));
    expect(Buffer.from(fixture.getImportedCount.clientDataJson, "hex").toString("utf8")).toBe(make("webauthn.get"));
  });

  it("V8: CCDToString of a hostile string matches an independent implementation", () => {
    const input = Buffer.from(fixture.ccd.inputUtf8, "hex").toString("utf8");
    // The fixture string really contains every hard case.
    for (const needle of ['"', "\\", "\u0007", "\u007f", " ", "ü", "\u{1f600}"]) {
      expect(input).toContain(needle);
    }
    const expectedOutput = ccdToString(input);
    expect(Buffer.from(fixture.ccd.outputUtf8, "hex").toString("utf8")).toBe(expectedOutput);
    expect(fixture.ccd.outputUtf8).toBe(Buffer.from(expectedOutput, "utf8").toString("hex"));
    // Valid JSON that reads back as the original.
    expect(JSON.parse(expectedOutput)).toBe(input);
    // Controls are escaped in lowercase hex; DEL is copied raw (U+007F is in 0x5D..0x10FFFF).
    expect(expectedOutput).toContain("\\u0007");
    expect(expectedOutput).toContain("\u007f");
  });

  it("V9: the registration parses, verifies and has the exact layout", () => {
    const cid = bytes(fixture.credentialId);
    expect(cid).toHaveLength(16);
    expect(fixture.create.flags).toBe("5d");
    const authData = bytes(fixture.create.authData);
    expect(authData).toHaveLength(148);
    expect(fixture.create.authData).toBe(
      `${fixture.rpIdHash}5d00000000${"00".repeat(16)}0010${fixture.credentialId}${fixture.key.cose}`,
    );
    const attestation = bytes(fixture.create.attestationObject);
    expect(attestation).toHaveLength(178);
    expect(fixture.create.attestationObject).toBe(
      `a363666d74646e6f6e656761747453746d74a06861757468446174615894${fixture.create.authData}`,
    );
    const parsed = parseAttestationObject(attestation);
    expect(parsed.fmt).toBe("none");
    expect(parsed.attStmt.size).toBe(0);
    expect(hex(parsed.authData)).toBe(fixture.create.authData);
    const auth = parseAuthData(authData);
    expect(auth).toMatchObject({ up: true, uv: true, be: true, bs: true, at: true, ed: false, signCount: 0 });
    expect(hex(auth.attested?.aaguid ?? new Uint8Array())).toBe("00".repeat(16));
    expect(hex(auth.attested?.credentialId ?? new Uint8Array())).toBe(fixture.credentialId);

    const result = verifyRegistration({
      attestationObject: attestation,
      clientDataJSON: bytes(fixture.create.clientDataJson),
      ...expected,
    });
    expect(result.jwk).toEqual(coseToJwk(bytes(fixture.key.cose)));
    expect(hex(result.credentialId)).toBe(fixture.credentialId);
    expect(result.flags).toBe(0x5d);
  });

  it("V10: the assertion with UV verifies and has the exact layout", () => {
    expect(fixture.get.flags).toBe("1d");
    expect(fixture.get.signCount).toBe(0);
    expect(fixture.get.authData).toBe(`${fixture.rpIdHash}1d00000000`);
    expect(bytes(fixture.get.authData)).toHaveLength(37);
    const result = verifyAssertion({
      authenticatorData: bytes(fixture.get.authData),
      clientDataJSON: bytes(fixture.get.clientDataJson),
      signature: bytes(fixture.get.signature),
      jwk: coseToJwk(bytes(fixture.key.cose)),
      ...expected,
    });
    expect(result).toEqual({ flags: 0x1d, signCount: 0 });
  });

  it("V11: an imported counter 41 signs with 42 and no UV", () => {
    const g = fixture.getImportedCount;
    expect(g.storedCount).toBe(41);
    expect(g.signCount).toBe(42);
    expect(g.flags).toBe("19");
    expect(g.authData).toBe(`${fixture.rpIdHash}190000002a`);
    const result = verifyAssertion({
      authenticatorData: bytes(g.authData),
      clientDataJSON: bytes(g.clientDataJson),
      signature: bytes(g.signature),
      jwk: coseToJwk(bytes(fixture.key.cose)),
      ...expected,
    });
    expect(result).toEqual({ flags: 0x19, signCount: 42 });
  });
});

describe("the verifier rejects what it should", () => {
  const jwk = coseToJwk(bytes(fixture.key.cose));
  const assertion = {
    authenticatorData: bytes(fixture.get.authData),
    clientDataJSON: bytes(fixture.get.clientDataJson),
    signature: bytes(fixture.get.signature),
    jwk,
    ...expected,
  };
  const registration = {
    attestationObject: bytes(fixture.create.attestationObject),
    clientDataJSON: bytes(fixture.create.clientDataJson),
    ...expected,
  };

  it("a wrong origin, rp id or challenge", () => {
    expect(() => verifyAssertion({ ...assertion, expectedOrigin: "https://evil.test" })).toThrow(/origin/);
    expect(() => verifyAssertion({ ...assertion, expectedRpId: "evil.test" })).toThrow(/rpIdHash/);
    expect(() => verifyAssertion({ ...assertion, expectedChallenge: new Uint8Array(32) })).toThrow(/challenge/);
    expect(() => verifyRegistration({ ...registration, expectedRpId: "evil.test" })).toThrow(/rpIdHash/);
  });

  it("a flipped bit in the signature, the client data or the authenticator data", () => {
    const flip = (b: Uint8Array, at: number): Uint8Array => {
      const copy = new Uint8Array(b);
      copy[at] = (copy[at] ?? 0) ^ 1;
      return copy;
    };
    expect(() => verifyAssertion({ ...assertion, signature: flip(assertion.signature, 10) })).toThrow();
    expect(() =>
      verifyAssertion({ ...assertion, authenticatorData: flip(assertion.authenticatorData, 36) }),
    ).toThrow(/signature/);
    // A flipped client data byte must be caught by the checks or the signature.
    expect(() => verifyAssertion({ ...assertion, clientDataJSON: flip(assertion.clientDataJSON, 1) })).toThrow();
  });

  it("malformed CBOR and layouts", () => {
    const good = registration.attestationObject;
    expect(() => parseAttestationObject(good.subarray(0, good.length - 1))).toThrow();
    expect(() => parseAttestationObject(new Uint8Array([...good, 0]))).toThrow(/trailing/);
    // attStmt before fmt: well-formed CBOR, but not the required member order.
    const swapped = bytes(`a36761747453746d74a063666d74646e6f6e656861757468446174615825${fixture.get.authData}`);
    expect(() => parseAttestationObject(swapped)).toThrow(/order/);
    const authData = bytes(fixture.create.authData);
    expect(() => parseAuthData(authData.subarray(0, 36))).toThrow();
    expect(() => parseAuthData(new Uint8Array([...authData, 0]))).toThrow(/trailing/);
    const withEd = new Uint8Array(bytes(fixture.get.authData));
    withEd[32] = 0x99;
    expect(() => parseAuthData(withEd)).toThrow(/extension/);
    // A get with AT set and a create without it are both wrong for their ceremony.
    const noAt = new Uint8Array(authData);
    noAt[32] = 0x19;
    expect(() => parseAuthData(noAt)).toThrow(/trailing/);
  });
});
