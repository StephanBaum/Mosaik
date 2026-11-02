/** Mock generator: uniform choices from `crypto.getRandomValues`, a 64-word sample list; entropy is computed from the list actually used. */
import type { GenerateRequest, Strength } from "../contract/index.ts";
import { fail, randomIndex, type MockHandlers } from "./state.ts";

const LOWER = "abcdefghijklmnopqrstuvwxyz";
const UPPER = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS = "0123456789";
const SYMBOLS = "!#$%&*+-=?@^_";
export const WORDS = [
  "orbit",
  "tulip",
  "canyon",
  "velvet",
  "meadow",
  "lantern",
  "quiet",
  "harbor",
  "amber",
  "birch",
  "cobalt",
  "dune",
  "ember",
  "fjord",
  "glacier",
  "hazel",
  "island",
  "jasper",
  "kettle",
  "lagoon",
  "maple",
  "nectar",
  "opal",
  "pebble",
  "quartz",
  "raven",
  "saffron",
  "thistle",
  "umber",
  "valley",
  "willow",
  "yarrow",
  "zephyr",
  "anchor",
  "beacon",
  "cedar",
  "delta",
  "falcon",
  "garnet",
  "heron",
  "indigo",
  "juniper",
  "kelp",
  "lilac",
  "marble",
  "nova",
  "olive",
  "prairie",
  "quill",
  "ripple",
  "sierra",
  "timber",
  "violet",
  "walnut",
  "cinder",
  "tundra",
  "basil",
  "comet",
  "drift",
  "fable",
  "grove",
  "hollow",
  "ivory",
  "meteor",
] as const;

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
    entropyBits:
      request.words * Math.log2(WORDS.length) +
      (request.appendDigit ? Math.log2(DIGITS.length) : 0),
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
