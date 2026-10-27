/** Generator contract: mirrors `src-tauri/src/contract/generator.rs`. */

export const strengths = ["veryWeak", "weak", "fair", "strong", "veryStrong"] as const;
export type Strength = (typeof strengths)[number];

export type GenerateRequest =
  | {
      kind: "password";
      length: number;
      lowercase: boolean;
      uppercase: boolean;
      digits: boolean;
      symbols: boolean;
      excludeAmbiguous: boolean;
    }
  | { kind: "passphrase"; words: number; separator: string; capitalize: boolean; appendDigit: boolean }
  | { kind: "forSite"; host: string };

export interface Generated {
  /** The password, for display. Copy it with `{ kind: "generated" }`. */
  value: string;
  entropyBits: number;
  strength: Strength;
}
