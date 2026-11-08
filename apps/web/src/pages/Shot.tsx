/**
 * A screenshot of the desktop app (mock data only, captured at 2x in dark theme). It floats as
 * a glass card over image stages (P1) or is cropped to one detail in bento tiles (P3). No fake
 * window chrome. Each locale gets its own capture.
 */
import type { Locale } from "@mosaic/i18n";

export interface ShotSpec {
  /** File stem in public/img: `app-<name>-<locale>-<width>.<ext>`. */
  name: string;
  widths: readonly [number, number];
  /** Intrinsic size of the smaller file (sets the aspect ratio). */
  height: number;
  sizes: string;
}

/** Every shot is 1280 x 800 CSS px, captured at 2x. */
const SIZES = "(min-width: 1200px) 1120px, 100vw";

export const SHOTS = {
  vault: { name: "vault", widths: [1280, 2560], height: 800, sizes: SIZES },
  item: { name: "item", widths: [1280, 2560], height: 800, sizes: SIZES },
  passkeys: { name: "passkeys", widths: [1280, 2560], height: 800, sizes: SIZES },
  sync: { name: "sync", widths: [1280, 2560], height: 800, sizes: SIZES },
  insights: { name: "insights", widths: [1280, 2560], height: 800, sizes: SIZES },
  import: { name: "import", widths: [1280, 2560], height: 800, sizes: SIZES },
  unlock: { name: "unlock", widths: [1280, 2560], height: 800, sizes: SIZES },
  warning: { name: "warning", widths: [1280, 2560], height: 800, sizes: SIZES },
} as const satisfies Record<string, ShotSpec>;

export function Shot({
  spec,
  locale,
  alt,
  className,
  eager = false,
}: {
  spec: ShotSpec;
  locale: Locale;
  alt: string;
  className?: string;
  /** Above the fold: load with the page instead of lazily. */
  eager?: boolean;
}) {
  const stem = `/img/app-${spec.name}-${locale}`;
  const set = (ext: string): string => spec.widths.map((w) => `${stem}-${String(w)}.${ext} ${String(w)}w`).join(", ");
  return (
    <figure className={["shot", className].filter(Boolean).join(" ")}>
      <picture>
        <source type="image/avif" srcSet={set("avif")} sizes={spec.sizes} />
        <source type="image/webp" srcSet={set("webp")} sizes={spec.sizes} />
        <img
          src={`${stem}-${String(spec.widths[0])}.webp`}
          alt={alt}
          width={spec.widths[0]}
          height={spec.height}
          loading={eager ? "eager" : "lazy"}
          decoding="async"
        />
      </picture>
    </figure>
  );
}
