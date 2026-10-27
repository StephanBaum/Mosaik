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

function generate(request: GenerateRequest): { value: string; entropyBits: number } {
  if (request.kind === "passphrase") {
    if (request.words < 3 || request.words > 20) {
      fail("invalidInput");
    }
    const words = Array.from({ length: request.words }, () => {
      const word = WORDS[randomIndex(WORDS.length)] ?? "";
      return request.capitalize ? word.charAt(0).toUpperCase() + word.slice(1) : word;
    });
    const digit = request.appendDigit ? DIGITS.charAt(randomIndex(DIGITS.length)) : "";
    return {
      value: words.join(request.separator) + digit,
      entropyBits: request.words * EFF_BITS_PER_WORD,
    };
  }
  const length = request.kind === "password" ? request.length : 20;
  const alphabet =
    request.kind === "password"
      ? (request.lowercase ? LOWER : "") +
        (request.uppercase ? UPPER : "") +
        (request.digits ? DIGITS : "") +
        (request.symbols ? SYMBOLS : "")
      : LOWER + UPPER + DIGITS + SYMBOLS;
  if (alphabet === "" || length < 4 || length > 128) {
    fail("invalidInput");
  }
  const value = Array.from({ length }, () => alphabet.charAt(randomIndex(alphabet.length))).join("");
  return { value, entropyBits: length * Math.log2(alphabet.length) };
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
