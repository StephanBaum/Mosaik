/** Mock generator: uniform choices from `crypto.getRandomValues`, small wordlist. */
import type { GenerateRequest, Strength } from "../contract/index.ts";
import { fail, randomIndex, type MockHandlers } from "./state.ts";

const LOWER = "abcdefghijklmnopqrstuvwxyz";
const UPPER = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS = "0123456789";
const SYMBOLS = "!#$%&*+-=?@^_";
const WORDS = ["orbit", "tulip", "canyon", "velvet", "meadow", "lantern", "quiet", "harbor"];
/** Bits per word of the real EFF large wordlist, so the UI shows realistic figures. */
const EFF_BITS_PER_WORD = 12.9;

interface Draw {
  value: string;
  entropyBits: number;
}

function pickWord(capitalize: boolean): string {
  const word = WORDS[randomIndex(WORDS.length)] ?? "";
  return capitalize ? word.charAt(0).toUpperCase() + word.slice(1) : word;
}

function generatePassphrase(request: Extract<GenerateRequest, { kind: "passphrase" }>): Draw {
  if (request.words < 3 || request.words > 20) {
    fail("invalidInput");
  }
  const words = Array.from({ length: request.words }, () => pickWord(request.capitalize));
  const digit = request.appendDigit ? DIGITS.charAt(randomIndex(DIGITS.length)) : "";
  return {
    value: words.join(request.separator) + digit,
    entropyBits: request.words * EFF_BITS_PER_WORD,
  };
}

function alphabetOf(request: Extract<GenerateRequest, { kind: "password" }>): string {
  return [
    request.lowercase ? LOWER : "",
    request.uppercase ? UPPER : "",
    request.digits ? DIGITS : "",
    request.symbols ? SYMBOLS : "",
  ].join("");
}

function generateCharacters(length: number, alphabet: string): Draw {
  if (alphabet === "" || length < 4 || length > 128) {
    fail("invalidInput");
  }
  const value = Array.from({ length }, () => alphabet.charAt(randomIndex(alphabet.length))).join("");
  return { value, entropyBits: length * Math.log2(alphabet.length) };
}

function generate(request: GenerateRequest): Draw {
  switch (request.kind) {
    case "passphrase":
      return generatePassphrase(request);
    case "password":
      return generateCharacters(request.length, alphabetOf(request));
    case "forSite":
      return generateCharacters(20, LOWER + UPPER + DIGITS + SYMBOLS);
  }
}

/** The buckets of `mosaic_generator::Strength`. */
export function strengthOf(bits: number): Strength {
  if (bits >= 100) {
    return "veryStrong";
  }
  if (bits >= 75) {
    return "strong";
  }
  if (bits >= 60) {
    return "fair";
  }
  return bits >= 36 ? "weak" : "veryWeak";
}

export const generatorHandlers: MockHandlers = {
  generate_password: (state, request) => {
    const { value, entropyBits } = generate(request);
    state.lastGenerated = value;
    return { value, entropyBits, strength: strengthOf(entropyBits) };
  },
};
