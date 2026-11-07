/**
 * Pass 2: decryption you can see. Outside the key the image is ciphertext: a quadtree pixel
 * mosaic snapped to the reduced palette, with pixel-sort streaks and channel-split /
 * difference glitches at block edges. Blocks split finer toward the key (a lens-like edge)
 * and dissolve into the clear gradient inside it, ringed by a thin difference line. Film
 * grain over everything. All lengths are device px, y runs top-down.
 */
import { LIB } from "./glsl.ts";

export const COMPOSITE_UNIFORMS = [
  "uScene",
  "uView",
  "uDpr",
  "uPal",
  "uBlock",
  "uKey",
  "uAmb",
  "uMask",
  "uSort",
  "uSortX",
  "uGlitch",
  "uGrain",
  "uBand",
] as const;

export type CompositeUniform = (typeof COMPOSITE_UNIFORMS)[number];

export const COMPOSITE_FRAG = /* glsl */ `#version 300 es
precision highp float;
uniform sampler2D uScene;
uniform vec2 uView;
uniform float uDpr;
uniform vec3 uPal[6];
uniform vec4 uBlock;   // size, split share, palette snap, dither
uniform vec4 uKey;     // cursor key: x, y, radius, strength
uniform vec4 uAmb;     // breathing key
uniform vec4 uMask;    // softness, lens, ring width, ring amount
uniform vec4 uSort;    // length, threshold, density, strength
uniform vec2 uSortX;   // downward pull, sweep position (< 0: none)
uniform vec2 uGlitch;  // amount, tick
uniform vec3 uGrain;   // amount, tick, bevel
uniform float uBand;   // still frame: 1 = the encrypted band instead of keys
out vec4 outColor;
${LIB}
vec3 scene(vec2 p) {
  return texture(uScene, vec2(p.x / uView.x, 1.0 - p.y / uView.y)).rgb;
}

float keyMask(vec2 p, vec4 k) {
  if (k.z < 1.0) {
    return 0.0;
  }
  float d = length(p - k.xy);
  return k.w * (1.0 - smoothstep(k.z * (1.0 - max(uMask.x, 0.01)), k.z, d));
}

float mask(vec2 p) {
  if (uBand > 0.5) {
    float y = p.y / uView.y;
    return 1.0 - step(0.56, y) * (1.0 - step(0.8, y));
  }
  return max(keyMask(p, uKey), keyMask(p, uAmb));
}

vec3 snap(vec3 rgb, vec2 id) {
  vec3 lab = srgbToLab(rgb);
  vec3 j = lab + (vec3(hash12(id * 1.7), hash12(id * 2.3 + 5.0), hash12(id * 3.1 + 9.0)) - 0.5) * uBlock.w * vec3(1.0, 1.6, 1.6);
  float best = 1e9;
  vec3 pick = uPal[0];
  for (int i = 0; i < 6; i++) {
    vec3 d = j - uPal[i];
    d.x *= 1.3;
    float e = dot(d, d);
    if (e < best) {
      best = e;
      pick = uPal[i];
    }
  }
  return labToSrgb(mix(lab, pick, uBlock.z));
}

// The block under p: a base cell, split up to three times where the key is near or at random.
vec3 cellAt(vec2 p, out vec2 id) {
  float s = uBlock.x;
  id = floor(p / s);
  for (int i = 0; i < 3; i++) {
    float m = mask((id + 0.5) * s);
    bool split = m > 0.04 + 0.3 * float(i) || (i == 0 && hash12(id + s * 0.137) < uBlock.y);
    if (!split || s < 6.0 * uDpr) {
      break;
    }
    s *= 0.5;
    id = floor(p / s);
  }
  return vec3((id + 0.5) * s, s);
}

vec3 baseColor(vec2 p) {
  float s = uBlock.x;
  vec2 id = floor(p / s);
  return snap(scene((id + 0.5) * s), id + s);
}

vec3 blockColor(vec3 cell, vec2 id, vec2 p) {
  vec3 col = snap(scene(cell.xy), id + cell.z);
  vec2 f = p - (cell.xy - 0.5 * cell.z);
  if (f.x < uDpr || f.y < uDpr) {
    col += uGrain.z;
  } else if (f.x > cell.z - uDpr || f.y > cell.z - uDpr) {
    col -= uGrain.z;
  }
  return col;
}

// Datamosh: a glitched block shows a slice of the mosaic shifted sideways, with the red and
// blue channels pulled apart along its edges; now and then the slice difference-blends.
vec3 glitch(vec3 col, vec3 cell, vec2 id, vec2 p) {
  if (hash12(id * 0.731 + uGlitch.y * 1.37 + cell.z) > uGlitch.x * 0.16) {
    return col;
  }
  float dx = (hash12(id + uGlitch.y) - 0.5) * cell.z * 2.4;
  vec2 f = (p - (cell.xy - 0.5 * cell.z)) / cell.z;
  float edge = min(min(f.x, 1.0 - f.x), min(f.y, 1.0 - f.y));
  if (edge < 0.2) {
    col = vec3(baseColor(p + vec2(dx, 0.0)).r, col.g, baseColor(p - vec2(dx, 0.0)).b);
  }
  float band = hash12(id + 9.0 + uGlitch.y);
  if (abs(f.y - band) < 0.14) {
    vec3 shifted = baseColor(p + vec2(dx * 1.6, 0.0));
    col = hash12(id + 4.0) < 0.35 ? abs(shifted - col.gbr) : shifted;
  }
  return col;
}

// Pixel sort: column groups carry runs that start on a bright block and ramp to the colour
// one run-length below, like sorted pixels. The intro sweep turns every group on behind it.
vec3 pixelSort(vec3 col, vec2 p, float enc) {
  float column = floor(p.x / (2.0 * uDpr));
  float group = floor(column / 6.0);
  float boost = 0.0;
  if (uSortX.y >= 0.0) {
    float dy = p.y / uView.y - uSortX.y;
    boost = dy < 0.0 ? smoothstep(-0.45, 0.0, dy) : 0.0;
  }
  if (hash12(vec2(group, 3.1)) > uSort.z + boost) {
    return col;
  }
  float len = uSort.x * (0.45 + 0.55 * hash12(vec2(group, 7.7))) * (0.85 + 0.3 * hash12(vec2(column, 1.3)));
  float phase = hash12(vec2(group, 5.3)) * len + uSortX.x;
  float u = (p.y - phase) / len;
  float k = floor(u);
  float t = fract(u);
  if (hash12(vec2(group, k)) > 0.6 + boost) {
    return col;
  }
  vec2 top = vec2(p.x, k * len + phase);
  vec3 a = baseColor(top);
  if (dot(a, vec3(0.2126, 0.7152, 0.0722)) < uSort.y - boost) {
    return col;
  }
  vec3 run = mix(a, baseColor(top + vec2(0.0, len)), smoothstep(0.0, 1.0, t));
  float alpha = clamp(uSort.w * (1.0 - smoothstep(0.82, 1.0, t)) + boost * 0.5, 0.0, 1.0) * enc;
  return mix(col, run, alpha);
}

vec3 clearColor(vec2 p) {
  vec4 k = keyMask(p, uKey) >= keyMask(p, uAmb) ? uKey : uAmb;
  vec2 d = p - k.xy;
  float e = clamp(length(d) / max(k.z, 1.0), 0.0, 1.0);
  float lens = uMask.y * e * e * e * step(uBand, 0.5);
  vec2 q = p - d * lens;
  vec2 ca = d * lens * 0.08;
  return vec3(scene(q + ca).r, scene(q).g, scene(q - ca).b);
}

float ring(vec2 p, vec4 k) {
  float fade = smoothstep(0.0, 40.0 * uDpr, k.z);
  return k.w * fade * (1.0 - smoothstep(0.0, max(uMask.z, 0.01), abs(length(p - k.xy) - k.z)));
}

void main() {
  vec2 p = vec2(gl_FragCoord.x, uView.y - gl_FragCoord.y);
  float m = mask(p);
  vec3 col = vec3(0.0);
  if (m < 0.999) {
    vec2 id;
    vec3 cell = cellAt(p, id);
    col = blockColor(cell, id, p);
    col = glitch(col, cell, id, p);
    col = pixelSort(col, p, 1.0 - m);
  }
  if (m > 0.001) {
    col = mix(col, clearColor(p), m);
  }
  if (uBand < 0.5) {
    float r = max(ring(p, uKey), ring(p, uAmb) * 0.6) * uMask.w;
    col = mix(col, abs(vec3(0.96) - col), r);
  }
  vec2 gp = floor(p / max(uDpr * 0.75, 1.0));
  float g = hash12(gp + fract(uGrain.y * 0.618) * vec2(173.0, 311.0)) + hash12(gp * 1.31 + uGrain.y * vec2(17.0, 29.0)) - 1.0;
  float lum = dot(col, vec3(0.2126, 0.7152, 0.0722));
  col += g * uGrain.x * mix(0.6, 1.0, m) * (0.55 + 0.45 * (1.0 - abs(lum * 2.0 - 1.0)));
  outColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
`;
