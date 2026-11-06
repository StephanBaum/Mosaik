import type { JsonWebKey } from "node:crypto";

export const HTTPS_PORT: number;
export const HTTP_PORT: number;

/** The independent WebAuthn verifier behind `/passkey/verify` (`test/webauthn/verifier.ts`). */
export interface PasskeyVerifier {
  verifyRegistration(input: {
    attestationObject: Uint8Array;
    clientDataJSON: Uint8Array;
    expectedOrigin: string;
    expectedRpId: string;
    expectedChallenge: Uint8Array;
  }): { jwk: JsonWebKey; credentialId: Uint8Array; flags: number; signCount: number };
  verifyAssertion(input: {
    authenticatorData: Uint8Array;
    clientDataJSON: Uint8Array;
    signature: Uint8Array;
    jwk: JsonWebKey;
    expectedOrigin: string;
    expectedRpId: string;
    expectedChallenge: Uint8Array;
  }): { flags: number; signCount: number };
}

export function startServer(options?: {
  passkeyVerifier?: PasskeyVerifier;
}): Promise<{ httpsPort: number; httpPort: number; close(): Promise<void> }>;
