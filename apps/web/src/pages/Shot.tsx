/**
 * A screenshot of the desktop app (mock data only, captured at 2x in dark theme) in a quiet
 * frame: a hairline, a soft shadow and a slight dark gradient at the foot. No fake window
 * chrome. Each locale gets its own capture.
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

export const SHOTS = {
  vault: { name: "vault", widths: [1280, 2560], height: 800, sizes: "(min-width: 1000px) 62vw, 100vw" },
  generator: { name: "generator", widths: [800, 1180], height: 706, sizes: "(min-width: 1000px) 30vw, 100vw" },
  sync: { name: "sync", widths: [1000, 1820], height: 758, sizes: "(min-width: 900px) 54vw, 100vw" },
} as const satisfies Record<string, ShotSpec>;

export function Shot({ spec, locale, alt, className }: { spec: ShotSpec; locale: Locale; alt: string; className?: string }) {
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
          loading="lazy"
          decoding="async"
        />
      </picture>
    </figure>
  );
}
