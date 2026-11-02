/** Splits a generated password into runs of letters and runs of digits and symbols. */
export interface Run {
  text: string;
  /** Digits and symbols: shown in a lighter colour so the letters read first. */
  minor: boolean;
}

export function splitRuns(value: string): Run[] {
  const runs: Run[] = [];
  for (const char of Array.from(value)) {
    const minor = !/\p{L}/u.test(char);
    const last = runs.at(-1);
    if (last !== undefined && last.minor === minor) {
      last.text += char;
    } else {
      runs.push({ text: char, minor });
    }
  }
  return runs;
}
