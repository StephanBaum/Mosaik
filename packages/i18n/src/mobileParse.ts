/**
 * The ICU subset the mobile export accepts: plain text, `{name}`, and one
 * `{n, plural, one {…} other {…}}` per message with `#` for the number. Everything else
 * (`select`, `selectordinal`, `number`, `date`, nesting, `=N` and `offset:` selectors, apostrophe
 * quoting, a second plural) throws an error that starts with the message key, so a catalog author
 * sees which string to change.
 */

export interface Piece {
  /** Literal text, or `undefined` for an argument. */
  readonly text?: string;
  /** Argument name, or `undefined` for text. A plural's `#` is an argument too. */
  readonly arg?: string;
}

export interface Variant {
  /** A CLDR plural category; `other` for a message without a plural. */
  readonly selector: string;
  readonly pieces: readonly Piece[];
}

export interface ParsedMessage {
  /** Argument names in order of first appearance. */
  readonly args: readonly string[];
  /** The plural argument (numeric on both platforms); every other argument is a string. */
  readonly plural: string | null;
  /** One variant per plural category, with the text around the plural merged into each. */
  readonly variants: readonly Variant[];
}

const categories = ["zero", "one", "two", "few", "many", "other"];

interface PluralSegment {
  readonly name: string;
  readonly branches: readonly Variant[];
}
type Segment = Piece | { readonly plural: PluralSegment };

class Cursor {
  i = 0;
  readonly args: string[] = [];
  readonly key: string;
  readonly s: string;

  // No parameter properties: Node strips types when it runs the export script.
  constructor(key: string, s: string) {
    this.key = key;
    this.s = s;
  }

  fail(why: string): never {
    throw new Error(`${this.key}: ${why}`);
  }

  seen(name: string): void {
    if (!this.args.includes(name)) this.args.push(name);
  }

  skipSpace(): void {
    while (/\s/.test(this.s[this.i] ?? "x")) this.i++;
  }

  word(): string {
    const m = /^[A-Za-z_][A-Za-z0-9_]*/.exec(this.s.slice(this.i));
    if (m === null) this.fail(`expected a name at "${this.s.slice(this.i, this.i + 12)}"`);
    this.i += m[0].length;
    return m[0];
  }

  expect(ch: string): void {
    this.skipSpace();
    if (this.s[this.i] !== ch) this.fail(`expected "${ch}" at position ${String(this.i)}`);
    this.i++;
  }
}

function pushText(out: Segment[], text: string): void {
  const last = out[out.length - 1];
  if (last !== undefined && "text" in last) {
    out[out.length - 1] = { text: last.text + text };
  } else {
    out.push({ text });
  }
}

function parseSegments(c: Cursor, pluralArg: string | null): Segment[] {
  const out: Segment[] = [];
  while (c.i < c.s.length) {
    const ch = c.s[c.i] ?? "";
    if (ch === "}") {
      if (pluralArg === null) c.fail('unbalanced "}"');
      return out;
    }
    if (ch === "{") {
      c.i++;
      out.push(parseArgument(c, pluralArg));
      continue;
    }
    if (ch === "'" && (c.s[c.i + 1] === "{" || c.s[c.i + 1] === "}")) {
      c.fail("apostrophe quoting of braces is not supported");
    }
    if (ch === "#" && pluralArg !== null) {
      out.push({ arg: pluralArg });
    } else {
      pushText(out, ch);
    }
    c.i++;
  }
  if (pluralArg !== null) c.fail("unclosed plural branch");
  return out;
}

function parseArgument(c: Cursor, pluralArg: string | null): Segment {
  c.skipSpace();
  const name = c.word();
  c.skipSpace();
  if (c.s[c.i] === "}") {
    c.i++;
    c.seen(name);
    return { arg: name };
  }
  c.expect(",");
  c.skipSpace();
  const type = c.word();
  if (type !== "plural") c.fail(`"${type}" is not supported (only {name} and plural)`);
  if (pluralArg !== null) c.fail("plural inside a plural branch (nesting) is not supported");
  c.expect(",");
  c.seen(name);
  return { plural: parsePlural(c, name) };
}

function parsePlural(c: Cursor, name: string): PluralSegment {
  const branches: Variant[] = [];
  for (;;) {
    c.skipSpace();
    if (c.s[c.i] === "}") break;
    const m = /^[^\s{}]+/.exec(c.s.slice(c.i));
    if (m === null) c.fail("expected a plural category");
    const selector = m[0];
    if (!categories.includes(selector)) {
      c.fail(`plural selector "${selector}" is not supported (use ${categories.join(", ")})`);
    }
    if (branches.some((b) => b.selector === selector)) c.fail(`duplicate plural "${selector}"`);
    c.i += selector.length;
    c.expect("{");
    const inner = parseSegments(c, name);
    c.expect("}");
    branches.push({ selector, pieces: inner as Piece[] });
  }
  c.i++;
  if (!branches.some((b) => b.selector === "other")) c.fail('the plural has no "other" branch');
  return { name, branches };
}

function checkWhitespace(c: Cursor, pieces: readonly Piece[]): void {
  // Android collapses runs of whitespace and trims the ends of unquoted resource text.
  const first = pieces[0]?.text ?? "";
  const last = pieces[pieces.length - 1]?.text ?? "";
  if (/^\s/.test(first) || /\s$/.test(last)) c.fail("leading or trailing whitespace");
  if (pieces.some((p) => /\s{2,}|[\t\r]/.test(p.text ?? ""))) c.fail("repeated or tab whitespace");
}

function mergePieces(parts: Piece[][]): Piece[] {
  const merged: Segment[] = [];
  for (const piece of parts.flat()) {
    if (piece.text !== undefined) pushText(merged, piece.text);
    else merged.push(piece);
  }
  return merged as Piece[];
}

/** Parses `message` of catalog entry `key`; throws `Error("<key>: <reason>")` outside the subset. */
export function parseMobileMessage(key: string, message: string): ParsedMessage {
  const c = new Cursor(key, message);
  const segments = parseSegments(c, null);
  const pluralSegments = segments.filter((s): s is { plural: PluralSegment } => "plural" in s);
  if (pluralSegments.length > 1) c.fail("more than one plural in a message");
  const plural = pluralSegments[0]?.plural ?? null;
  const index = segments.findIndex((s) => "plural" in s);
  const before = (index < 0 ? segments : segments.slice(0, index)) as Piece[];
  const after = (index < 0 ? [] : segments.slice(index + 1)) as Piece[];
  const variants: Variant[] =
    plural === null
      ? [{ selector: "other", pieces: mergePieces([before]) }]
      : plural.branches.map((b) => ({
          selector: b.selector,
          pieces: mergePieces([before, [...b.pieces], after]),
        }));
  for (const v of variants) checkWhitespace(c, v.pieces);
  return { args: c.args, plural: plural?.name ?? null, variants };
}
