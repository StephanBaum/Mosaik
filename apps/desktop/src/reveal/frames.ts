/** One frame of each run as a pure function of the run time (study L823-828 and L813-818). */
import { lensBounds, lensClip, type Lens } from "./lens.ts";
import type { Scene } from "./scene.ts";
import { bevelWidth, lensAt } from "./timing.ts";

/** Pixels the lens must clear the farthest corner by before the end state is applied. */
const MARGIN = 2;

export function unlockFrame(u: number, scene: Scene): { lens: Lens; clip: string | null; bandOn: boolean } {
  const lens = lensAt("unlock", u, scene.maxD);
  const { inner } = lensBounds(lens);
  const covered = inner >= scene.maxD + MARGIN;
  const bandOn = !(covered && inner - bevelWidth(lens.R) >= scene.maxD + MARGIN);
  return { lens, clip: covered ? null : lensClip(lens, scene.o), bandOn };
}

export function lockFrame(u: number, scene: Scene): { lens: Lens; clip: string | null; gone: boolean } {
  const lens = lensAt("lock", u, scene.maxD);
  const gone = lens.R <= 0;
  return { lens, clip: gone ? null : lensClip(lens, scene.o), gone };
}
