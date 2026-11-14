import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { borderWidth, colors, radius, size, space } from "./index.ts";
import { androidDimensXml, argb, kotlinTokens, parseColor, shadowHint, swiftColor, swiftTokens } from "./mobileExport.ts";

describe("mobile token export", () => {
  it("hex and rgba become ARGB", () => {
    expect(argb("#09090B")).toBe("0xFF09090B");
    expect(argb("#ffffff")).toBe("0xFFFFFFFF");
    // alpha = round(a * 255): 0.75 -> 191 (BF), 0.40 -> 102 (66), 0.04 -> 10 (0A)
    expect(argb("rgba(230, 230, 235, 0.75)")).toBe("0xBFE6E6EB");
    expect(argb("rgba(9, 9, 11, 0.40)")).toBe("0x6609090B");
    expect(argb("rgba(9, 9, 11, 0.04)")).toBe("0x0A09090B");
    expect(parseColor("rgba(0, 0, 0, 0.60)")).toEqual({ a: 153, r: 0, g: 0, b: 0 });
    expect(swiftColor("#FFFFFF")).toBe("Color(.sRGB, red: 1, green: 1, blue: 1, opacity: 1)");
    expect(swiftColor("rgba(0, 0, 0, 0.60)")).toBe(
      "Color(.sRGB, red: 0, green: 0, blue: 0, opacity: 0.6)",
    );
    expect(() => argb("red")).toThrow(/Expected/);
    expect(shadowHint("0 8px 24px rgba(9, 9, 11, 0.08), 0 1px 2px rgba(9, 9, 11, 0.04)")).toEqual({
      y: 8,
      blur: 24,
      opacity: 0.08,
    });
  });

  it("every semantic color has light and dark", () => {
    expect(Object.keys(colors.dark)).toEqual(Object.keys(colors.light));
    const kotlin = kotlinTokens();
    const swift = swiftTokens();
    const light = kotlin.slice(kotlin.indexOf("val MosaicLightColors"), kotlin.indexOf("val MosaicDarkColors"));
    const dark = kotlin.slice(kotlin.indexOf("val MosaicDarkColors"), kotlin.indexOf("object MosaicSpace"));
    const swiftLight = swift.slice(swift.indexOf("static let light"), swift.indexOf("static let dark"));
    const swiftDark = swift.slice(swift.indexOf("static let dark"), swift.indexOf("public static let space"));
    for (const name of [...Object.keys(colors.light), "scrim", "primaryActionRim"]) {
      for (const block of [light, dark]) expect(block, name).toContain(`${name} = Color(0x`);
      for (const block of [swiftLight, swiftDark]) expect(block, name).toContain(`${name}: Color(.sRGB`);
    }
    expect(light).toContain(`bg = Color(${argb(colors.light.bg)})`);
    expect(dark).toContain(`bg = Color(${argb(colors.dark.bg)})`);
  });

  it("spacing radius and size are exported unchanged", () => {
    const kotlin = kotlinTokens();
    const swift = swiftTokens();
    space.forEach((value, i) => {
      expect(kotlin).toContain(`val s${String(i)}: Dp = ${String(value)}.dp`);
      expect(swift).toContain(`public static let s${String(i)}: CGFloat = ${String(value)}\n`);
    });
    expect(swift).toContain(`public static let space: [CGFloat] = [${space.join(", ")}]`);
    for (const group of [radius, size, borderWidth]) {
      for (const [name, value] of Object.entries(group)) {
        expect(kotlin, name).toContain(`val ${name}: Dp = ${String(value)}.dp`);
        expect(swift, name).toContain(`public static let ${name}: CGFloat = ${String(value)}\n`);
      }
    }
    expect(kotlin).toContain("const val microMs: Int = 120");
    expect(kotlin).toContain("CubicBezierEasing(0.2f, 0f, 0f, 1f)");
    expect(swift).toContain("public static let panelMs = 200");
  });

  it("dimens.xml is generated from the tokens and the committed file is in sync", () => {
    const xml = androidDimensXml();
    expect(xml).toContain('<dimen name="mosaic_size_input_xl">56dp</dimen>');
    expect(xml).toContain('<dimen name="mosaic_space_s5">16dp</dimen>');
    expect(xml).toContain('<dimen name="mosaic_space_s3">8dp</dimen>');
    expect(xml).toContain('<dimen name="mosaic_font_size_md">16sp</dimen>');
    expect(xml).toContain('<dimen name="mosaic_font_size_sm">14sp</dimen>');
    expect(xml).not.toContain("\r");
    const committed = readFileSync(
      new URL("../../../apps/mobile/android/app/src/main/res/values/dimens.xml", import.meta.url),
      "utf8",
    ).replaceAll("\r\n", "\n");
    expect(committed).toBe(xml);
  });

  it("output is deterministic", () => {
    expect(kotlinTokens()).toBe(kotlinTokens());
    expect(swiftTokens()).toBe(swiftTokens());
    expect(kotlinTokens()).not.toMatch(/\r|\d{4}-\d{2}-\d{2}/);
  });
});
