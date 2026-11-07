/**
 * Pixel sorting over the hero painting, in one fragment pass (WebGL2).
 *
 * Technique: threshold-gated directional runs, sorted for real. Each canvas row is cut into
 * cells along the wind (left to right); the cell grid drifts with time and is offset per row,
 * so runs never line up into a grid. A cell joins a run when the luminance at its centre
 * exceeds the threshold; the strength field (sky, flowing bands, right edge, calm behind the
 * copy, the cursor's clean circle, the scroll gain) raises that threshold out of reach where
 * sorting should fade. Around the fragment's cell the shader samples N cells, keeps the
 * contiguous run of qualifying cells, and sorts the run by luminance with a fixed sorting
 * network (Batcher's odd-even merge sort: straight-line compare-and-swap, cheap to compile).
 * Each fragment then shows the run's colour at its position in that sorted sequence,
 * interpolated between neighbouring ranks: dark upwind, bright downwind, a hard edge where the
 * run ends, as a real pixel sort looks.
 *
 * The output is premultiplied: sorted colour where a run is, transparent elsewhere, so the
 * crisp `<img>` shows through everywhere the painting is clean. All fragments of a run see the
 * same cells, so a run is one consistent gradient.
 */

/** Cells sampled around a fragment (a power of two for the merge network). */
export const CELLS = 16;
/** Index of the fragment's own cell among them. */
const CENTRE = CELLS / 2;

/**
 * Comparators of Batcher's odd-even merge sort for `n` (a power of two) elements: applying
 * them in order sorts any input.
 */
export function oddEvenMergeNetwork(n: number): [number, number][] {
  const pairs: [number, number][] = [];
  for (let p = 1; p < n; p *= 2) {
    for (let k = p; k >= 1; k = Math.floor(k / 2)) {
      for (let j = k % p; j <= n - 1 - k; j += 2 * k) {
        pairs.push(...mergeStep(n, p, k, j));
      }
    }
  }
  return pairs;
}

/** The comparators of one block of one merge round: pairs `k` apart inside the same 2p run. */
function mergeStep(n: number, p: number, k: number, j: number): [number, number][] {
  const pairs: [number, number][] = [];
  for (let i = 0; i <= Math.min(k - 1, n - j - k - 1); i++) {
    if (Math.floor((i + j) / (p * 2)) === Math.floor((i + j + k) / (p * 2))) {
      pairs.push([i + j, i + j + k]);
    }
  }
  return pairs;
}

const NETWORK = oddEvenMergeNetwork(CELLS)
  .map(([a, b]) => `CX(${String(a)}, ${String(b)})`)
  .join(" ");

export const VERT = `#version 300 es
void main(){float x=gl_VertexID==1?3.0:-1.0;float y=gl_VertexID==2?3.0:-1.0;gl_Position=vec4(x,y,0.0,1.0);}`;

export const FRAG = `#version 300 es
precision highp float;
uniform sampler2D uImage;
uniform vec2 uView;
uniform vec4 uCover;
uniform float uTime;
uniform vec4 uSort;
uniform vec4 uBand;
uniform vec4 uText;
uniform vec2 uCalm;
uniform vec4 uCursor;
uniform float uGain;
out vec4 fragColor;

const int K = ${String(CENTRE)};
const int N = ${String(CELLS)};
// Compare and swap two cells by luminance (constant indices: plain registers, no branches).
#define CX(a, b) { bool s = ls[a] > ls[b]; float l = ls[a]; vec3 c = cs[a]; ls[a] = s ? ls[b] : l; ls[b] = s ? l : ls[b]; cs[a] = s ? cs[b] : c; cs[b] = s ? c : cs[b]; }

float hash11(float n){ return fract(sin(n * 91.3458) * 47453.5453); }
vec2 imageUv(vec2 px){ return px / uView * uCover.xy + uCover.zw; }
vec3 tap(vec2 px){ return texture(uImage, imageUv(px)).rgb; }
float luma(vec3 c){ return dot(c, vec3(0.2126, 0.7152, 0.0722)); }

float field(vec2 px){
  vec2 uv = px / uView;
  float iv = imageUv(px).y;
  float sky = uBand.z * (1.0 - smoothstep(0.30, 0.47, iv));
  float wave = 0.035 * sin(uv.x * 4.7 + uTime * 0.13) + 0.018 * sin(uv.x * 10.3 - uTime * 0.21);
  float d1 = (uv.y - uBand.x - wave) / max(uBand.y, 1e-3);
  float d2 = (uv.y - uBand.x - 0.36 - 1.6 * wave) / max(uBand.y * 0.42, 1e-3);
  float band = max(exp(-d1 * d1), 0.7 * exp(-d2 * d2));
  float edge = uBand.w * smoothstep(0.58, 1.0, uv.x);
  float m = max(max(sky, band), edge);
  // Flowing bands with clean paint between them, so the painting stays readable.
  float flow = 0.5 + 0.3 * sin(uv.y * 17.0 + 2.0 * sin(uv.x * 2.3 + uTime * 0.08))
    + 0.2 * sin(uv.y * 41.0 - uv.x * 3.0 + uTime * 0.05);
  m *= mix(0.12, 1.0, smoothstep(0.38, 0.72, flow));
  vec2 q = max(max(uText.xy - uv, uv - uText.zw), 0.0);
  m *= 1.0 - uCalm.x * (1.0 - smoothstep(0.0, uCalm.y, length(q)));
  m = clamp(m * uGain, 0.0, 1.0);
  float r = length(px - uCursor.xy);
  m *= 1.0 - uCursor.w * (1.0 - smoothstep(uCursor.z * 0.2, uCursor.z, r));
  return m;
}

// Luminance a cell must exceed: out of reach where the field is 0, the threshold from 0.5 up,
// and lower still toward 1 (scrolling out lets darker paint join the runs).
float gate(vec2 px, float thr){
  float f = field(px);
  return mix(1.05, thr, smoothstep(0.0, 0.5, f)) - 0.12 * smoothstep(0.5, 1.0, f);
}

void main(){
  vec2 px = vec2(gl_FragCoord.x, uView.y - gl_FragCoord.y);
  float here = field(px);
  if (here < 0.02) { fragColor = vec4(0.0); return; }
  float row = floor(px.y);
  float cell = max(uSort.y / float(N), 2.0);
  float shift = uTime * uSort.z + hash11(row) * cell;
  float thr = uSort.x + (hash11(row + 17.0) - 0.5) * 0.07;
  float c0 = floor((px.x - shift) / cell);
  vec2 p0 = vec2((c0 + 0.5) * cell + shift, px.y);
  if (luma(tap(p0)) <= gate(p0, thr)) { fragColor = vec4(0.0); return; }
  vec3 cs[N]; float ls[N]; float ok[N];
  for (int i = 0; i < N; i++) {
    vec2 p = vec2((c0 + float(i - K) + 0.5) * cell + shift, px.y);
    cs[i] = tap(p); ls[i] = luma(cs[i]);
    ok[i] = ls[i] > gate(p, thr) ? 1.0 : 0.0;
  }
  // Keep only the contiguous run through the fragment's own cell.
  ok[K] = 1.0;
  for (int i = K - 1; i >= 0; i--) { ok[i] *= ok[i + 1]; }
  for (int i = K + 1; i < N; i++) { ok[i] *= ok[i - 1]; }
  float n = 0.0; float left = 0.0;
  for (int i = 0; i < N; i++) {
    n += ok[i];
    left += i < K ? ok[i] : 0.0;
    // Cells outside the run sort to the end.
    ls[i] = ok[i] > 0.5 ? ls[i] : 10.0;
  }
  ${NETWORK}
  float start = (c0 - left) * cell + shift;
  float t = clamp((px.x - start) / (n * cell), 0.0, 1.0);
  float qk = t * (n - 1.0);
  float k0 = floor(qk);
  float k1 = min(k0 + 1.0, n - 1.0);
  vec3 lo = vec3(0.0); vec3 hi = vec3(0.0);
  for (int i = 0; i < N; i++) {
    lo += cs[i] * (1.0 - step(0.5, abs(float(i) - k0)));
    hi += cs[i] * (1.0 - step(0.5, abs(float(i) - k1)));
  }
  vec3 sorted = mix(lo, hi, qk - k0);
  float alpha = smoothstep(0.02, 0.3, here) * smoothstep(1.0, 3.0, n) * uSort.w;
  fragColor = vec4(sorted * alpha, alpha);
}`;

export const UNIFORMS = [
  "uImage",
  "uView",
  "uCover",
  "uTime",
  "uSort",
  "uBand",
  "uText",
  "uCalm",
  "uCursor",
  "uGain",
] as const;

export type SortUniform = (typeof UNIFORMS)[number];
