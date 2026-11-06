/**
 * The e2e build's fake desktop signs with `test/webauthn/authenticator.ts`. Its output must pass
 * the same independent verifier as the Rust authenticator's, or the hostile-page specs would test
 * a fake that real relying parties would not accept.
 */
import { describe, expect, it } from "vitest";
import { clientDataJSON, register, sign } from "../../test/webauthn/authenticator.ts";
import { parseAttestationObject, parseAuthData, verifyAssertion, verifyRegistration } from "../../test/webauthn/verifier.ts";

const challenge = Uint8Array.from({ length: 32 }, (_, i) => i);
const origin = "https://bank.test:4843";

describe("the test authenticator", () => {
  it("makes a registration and an assertion the independent verifier accepts", async () => {
    const made = await register({ rpId: "bank.test", origin, challenge, userHandle: new Uint8Array(8), userName: "ada", userVerified: true });
    expect(made.authenticatorData.length).toBe(148);
    expect(made.attestationObject.length).toBe(178);
    const verified = verifyRegistration({
      attestationObject: made.attestationObject,
      clientDataJSON: made.clientDataJSON,
      expectedOrigin: origin,
      expectedRpId: "bank.test",
      expectedChallenge: challenge,
    });
    expect(verified.flags).toBe(0x5d);
    expect(verified.signCount).toBe(0);
    expect(Buffer.from(verified.credentialId)).toEqual(Buffer.from(made.credential.credentialId));

    const signed = await sign(made.credential, { origin, challenge, userVerified: false });
    const asserted = verifyAssertion({
      authenticatorData: signed.authenticatorData,
      clientDataJSON: signed.clientDataJSON,
      signature: signed.signature,
      jwk: verified.jwk,
      expectedOrigin: origin,
      expectedRpId: "bank.test",
      expectedChallenge: challenge,
    });
    expect(asserted.flags).toBe(0x19);
  });

  it("has the layout of CRYPTO_SPEC section 16 without user verification", async () => {
    const made = await register({ rpId: "example.com", origin: "https://example.com", challenge, userHandle: new Uint8Array(1), userName: "x", userVerified: false });
    const data = parseAuthData(parseAttestationObject(made.attestationObject).authData);
    expect(data.flags).toBe(0x59);
    expect(data.attested?.aaguid).toEqual(new Uint8Array(16));
    expect(made.credential.cose.length).toBe(77);
    expect(made.credential.spki.length).toBe(91);
  });

  it("builds client data like the spec, member order and all", () => {
    const json = new TextDecoder().decode(clientDataJSON("webauthn.get", challenge, "https://example.com"));
    expect(json).toBe(
      '{"type":"webauthn.get","challenge":"AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8","origin":"https://example.com","crossOrigin":false}',
    );
  });

  it("makes DER signatures that survive any r and s (a leading zero byte, a high bit)", async () => {
    const made = await register({ rpId: "bank.test", origin, challenge, userHandle: new Uint8Array(8), userName: "ada", userVerified: true });
    const { jwk } = verifyRegistration({
      attestationObject: made.attestationObject,
      clientDataJSON: made.clientDataJSON,
      expectedOrigin: origin,
      expectedRpId: "bank.test",
      expectedChallenge: challenge,
    });
    for (let i = 0; i < 40; i++) {
      const signed = await sign(made.credential, { origin, challenge, userVerified: true });
      verifyAssertion({ ...signed, jwk, expectedOrigin: origin, expectedRpId: "bank.test", expectedChallenge: challenge });
    }
  });
});
