/**
 * GLSL shared by the two passes of the hero art: the full-screen triangle, hashing, value
 * noise and the OKLab conversions (Björn Ottosson's published matrices).
 */
export const VERT = /* glsl */ `#version 300 es
const vec2 VERTS[3] = vec2[3](vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
out vec2 vUv;
void main() {
  vec2 p = VERTS[gl_VertexID];
  vUv = p * 0.5 + 0.5;
  gl_Position = vec4(p, 0.0, 1.0);
}
`;

export const LIB = /* glsl */ `
float hash12(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 33.33);
  return fract((p3.x + p3.y) * p3.z);
}

float vnoise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  float a = hash12(i);
  float b = hash12(i + vec2(1.0, 0.0));
  float c = hash12(i + vec2(0.0, 1.0));
  float d = hash12(i + vec2(1.0, 1.0));
  return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

float fbm(vec2 p) {
  float s = 0.0;
  float a = 0.5;
  for (int i = 0; i < 4; i++) {
    s += a * vnoise(p);
    p = mat2(1.6, 1.2, -1.2, 1.6) * p + 3.1;
    a *= 0.5;
  }
  return s;
}

vec3 labToSrgb(vec3 c) {
  float l_ = c.x + 0.3963377774 * c.y + 0.2158037573 * c.z;
  float m_ = c.x - 0.1055613458 * c.y - 0.0638541728 * c.z;
  float s_ = c.x - 0.0894841775 * c.y - 1.2914855480 * c.z;
  float l = l_ * l_ * l_;
  float m = m_ * m_ * m_;
  float s = s_ * s_ * s_;
  vec3 lin = vec3(
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s
  );
  lin = clamp(lin, 0.0, 1.0);
  return mix(lin * 12.92, 1.055 * pow(lin, vec3(1.0 / 2.4)) - 0.055, step(0.0031308, lin));
}

vec3 srgbToLab(vec3 c) {
  vec3 lin = mix(c / 12.92, pow((c + 0.055) / 1.055, vec3(2.4)), step(0.04045, c));
  vec3 lms = vec3(
    0.4122214708 * lin.r + 0.5363325363 * lin.g + 0.0514459929 * lin.b,
    0.2119034982 * lin.r + 0.6806995451 * lin.g + 0.1073969566 * lin.b,
    0.0883024619 * lin.r + 0.2817188376 * lin.g + 0.6299787005 * lin.b
  );
  lms = pow(max(lms, 0.0), vec3(1.0 / 3.0));
  return vec3(
    0.2104542553 * lms.x + 0.7936177850 * lms.y - 0.0040720468 * lms.z,
    1.9779984951 * lms.x - 2.4285922050 * lms.y + 0.4505937099 * lms.z,
    0.0259040371 * lms.x + 0.7827717662 * lms.y - 0.8086757660 * lms.z
  );
}
`;

/**
 * Pass 1: the decrypted image, a grain-free mesh gradient rendered at a fraction of the
 * canvas size (it is low-frequency; bilinear sampling keeps it smooth). Six colour points
 * drift on slow orbits through a domain-warped field and blend in OKLab with a sharp
 * inverse-distance falloff, so each colour owns a region and the seams stay saturated.
 */
export const SCENE_FRAG = /* glsl */ `#version 300 es
precision highp float;
uniform vec2 uAspect;
uniform float uTime;
uniform vec3 uPal[6];
uniform vec3 uFlow;
in vec2 vUv;
out vec4 outColor;
${LIB}
vec3 loud(int i) {
  float shift = uFlow.z;
  int k = int(floor(shift));
  vec3 a = uPal[1 + (i + k) % 5];
  vec3 b = uPal[1 + (i + k + 1) % 5];
  return mix(a, b, fract(shift));
}

void main() {
  vec2 p = (vUv - 0.5) * uAspect;
  float t = uTime;
  vec2 q = vec2(fbm(p * 1.1 + vec2(0.0, t * 0.05)), fbm(p * 1.1 + vec2(5.2, 1.3) - vec2(t * 0.04, 0.0)));
  vec2 w = p + uFlow.x * (q - 0.5) * 1.5;
  float folds = fbm(w * 1.8 + q * 2.0 + vec2(t * 0.03, -t * 0.02));
  float ridge = 1.0 - abs(2.0 * fbm(w * 2.6 - q * 1.5 + vec2(-t * 0.02, t * 0.025)) - 1.0);
  vec3 lab = vec3(0.0);
  float sum = 0.0;
  float chroma = 0.0;
  for (int i = 0; i < 6; i++) {
    float fi = float(i);
    vec2 c = vec2(sin(t * (0.11 + 0.031 * fi) + fi * 2.1), cos(t * (0.093 + 0.027 * fi) + fi * 1.7));
    c *= vec2(0.58 * uAspect.x, 0.52);
    float d = length(w - c);
    float wgt = 1.0 / pow(d * d + 0.01, 2.6);
    vec3 col = i == 0 ? uPal[0] : loud(i - 1);
    wgt *= i == 0 ? 1.35 : 1.0;
    lab += col * wgt;
    chroma += length(col.yz) * wgt;
    sum += wgt;
  }
  lab /= sum;
  // Blend hue, keep chroma: where two colours meet, the seam keeps their average chroma
  // instead of averaging toward grey.
  float c = length(lab.yz);
  lab.yz = c > 1e-4 ? lab.yz / c * mix(c, chroma / sum, 0.85) : lab.yz;
  // Silky folds: broad light and shade, then thin bright ridges like light on fabric.
  lab.x += (folds - 0.5) * 0.22 + pow(ridge, 8.0) * 0.22;
  lab.yz *= uFlow.y * (0.9 + 0.25 * ridge);
  outColor = vec4(labToSrgb(lab), 1.0);
}
`;
