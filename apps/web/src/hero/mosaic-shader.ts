/**
 * The living mosaic (design concept §3 M1): one full-canvas fragment pass. Per pixel it finds
 * the tile cell and its local coordinates, masks a rounded square, then runs the app's
 * `foil()` and `wavelet()`.
 *
 * The foil library (`hash12` … `wavelet`) is cut out of the copied desktop shader at load
 * time, so it is the desktop source byte for byte (src/fx/parity.test.ts checks both). The
 * library reads `uTime`, `uSeed`, `uHover` and `uMouse` as globals; here they are plain
 * per-pixel globals that `main()` sets per tile, so every tile gets its own seed, light and
 * sheen phase without touching a character of the library.
 */
import { FRAG } from "../fx/shader.ts";

const LIB_START = "float hash12(";
const LIB_END = "void main(){";

/** The foil library of a desktop-style fragment shader, verbatim. */
export function foilLibrary(frag: string = FRAG): string {
  const start = frag.indexOf(LIB_START);
  const end = frag.indexOf(LIB_END);
  if (start < 0 || end <= start) {
    throw new Error("mosaic: the foil library markers are missing from the holo shader");
  }
  return frag.slice(start, end);
}

const HEADER = `#version 300 es
precision highp float;
uniform vec2 uView;
uniform float uInset, uTilt, uGlow;
uniform float uPitch, uTileK, uRadius, uScrPer, uStripeK, uDPR, uDark, uClock;
uniform vec2 uGrid;
uniform vec3 uCursor;
uniform vec4 uLight;
uniform vec4 uHue;
uniform vec4 uRipple; uniform vec4 uRipP; uniform vec2 uRipQ;
uniform vec3 uIntro;
uniform vec4 uGlyph;
uniform sampler2D uGlyphs;
out vec4 fragColor;
float uTime; float uSeed; float uHover; vec2 uMouse;
`;

/** Rounded square, ease-enter (cubic-bezier(0.16, 1, 0.3, 1)) and the etched glyph. */
const HELPERS = `
float bez(float t,float a,float b){ float u=1.0-t; return 3.0*u*u*t*a+3.0*u*t*t*b+t*t*t; }
float easeEnter(float x){
  float t=x;
  for(int i=0;i<6;i++){
    float u=1.0-t;
    float dx=3.0*u*u*0.16+6.0*u*t*(0.3-0.16)+3.0*t*t*(1.0-0.3);
    t=clamp(t-(bez(t,0.16,0.3)-x)/max(dx,1e-4),0.0,1.0);
  }
  return bez(t,1.0,1.0);
}
float sdTile(vec2 p,float r){ vec2 q=abs(p)-(0.5-r); return length(max(q,0.0))+min(max(q.x,q.y),0.0)-r; }
vec3 etch(vec3 col, vec2 fp, vec2 cell, vec2 tilt, float H){
  float pick=hash12(cell*2.3+vec2(11.0,5.0));
  if(pick>=uGlyph.x){ return col; }
  vec2 gu=vec2(fp.x,-fp.y)/uGlyph.y+0.5;
  if(any(lessThan(gu,vec2(0.0)))||any(greaterThan(gu,vec2(1.0)))){ return col; }
  float idx=floor(hash12(cell+vec2(19.0,2.0))*5.0);
  vec2 uv=vec2((idx+gu.x)/5.0,gu.y);
  float lod=log2(max(uGlyph.w/(uGlyph.y*H),1.0));
  vec2 off=normalize(vec2(tilt.x,-tilt.y)+vec2(1e-3,-0.6))*(1.3/(uGlyph.y*H))*vec2(0.2,1.0);
  float a0=textureLod(uGlyphs,uv,lod).a;
  float a1=textureLod(uGlyphs,uv+off,lod).a;
  col*=1.0-uGlyph.z*a0;
  col+=vec3(0.45*uGlyph.z)*clamp(a0-a1,0.0,1.0);
  return col;
}
`;

const MAIN = `
void main(){
  vec2 g=(vec2(gl_FragCoord.x,uView.y-gl_FragCoord.y)+uInset)/uPitch;
  vec2 cell=floor(g);
  if(cell.x<0.0||cell.y<0.0||cell.x>=uGrid.x||cell.y>=uGrid.y){ fragColor=vec4(0.0); return; }
  float H=uPitch*uTileK; float PX=1.0/H;
  vec2 lp=(g-cell-0.5)/uTileK; vec2 p=vec2(lp.x,-lp.y);
  float k=easeEnter(clamp((uIntro.x-min((cell.x+cell.y)*uIntro.y,0.5))/uIntro.z,0.0,1.0));
  if(k<=0.0){ fragColor=vec4(0.0); return; }
  float sc=0.96+0.04*k;
  vec3 wr=vec3(0.0);
  if(uRipple.w>0.0){ wr=uRipple.w*wavelet(vec2(g.x,-g.y),uRipple.xy,uRipple.z*uRipP.x,uRipP.y,uRipP.z); }
  float d=sdTile(p/sc,uRadius)*sc-uRipP.w*wr.x/uTileK;
  float a=clamp(0.5-d/PX,0.0,1.0)*k;
  if(a<=0.0){ fragColor=vec4(0.0); return; }
  float e=max(-d,0.0);
  float h=hash12(cell*1.7+vec2(3.1,7.9));
  uSeed=uHue.x+uHue.y*dot(cell,vec2(0.37,0.23))+uHue.z*h;
  vec2 c=vec2(cell.x+0.5,-(cell.y+0.5));
  uTime=uClock-uHue.w*dot(c/uTileK,normalize(vec2(1.0,0.55)))-uSeed*0.875;
  vec2 idle=vec2(sin(uTime*0.7+uSeed)*0.275,0.3+0.12*sin(uTime*0.98+uSeed*2.0));
  vec2 cur=vec2(uCursor.x,-uCursor.y);
  float dist=length(cur-c);
  float near=uCursor.z*exp(-dist*dist/(2.0*uLight.x*uLight.x));
  vec2 toward=0.6*(cur-c)/uTileK;
  toward*=min(1.0,uTilt/max(length(toward),1e-4));
  uMouse=mix(idle,toward,near);
  uHover=near*uLight.y;
  vec2 B=vec2(0.5);
  vec2 tilt=clamp(uMouse/vec2(B.x*0.75+0.2,0.6),-1.2,1.2);
  vec2 fp=p-wr.yz*uRipQ.x/uTileK;
  vec2 nb=-wr.yz*0.04;
  float vib=max(uLight.z,clamp(abs(wr.x)*uRipQ.y*6.0,0.0,1.0));
  vec3 col=foil(fp,B,PX,tilt,e,0.55*wr.x,nb,vib);
  float l=dot(col,vec3(0.2126,0.7152,0.0722));
  col=mix(vec3(l),col,uLight.w)*mix(0.9,1.0,uDark);
  col*=1.0+uGlow*near;
  col=etch(col,fp,cell,tilt,H);
  col*=mix(1.0,0.94,uDark);
  col+=(hash12(gl_FragCoord.xy+fract(uClock))-0.5)/255.0;
  col=clamp(col,0.0,1.0);
  fragColor=vec4(col*a,a);
}`;

export const MOSAIC_FRAG = HEADER + foilLibrary() + HELPERS + MAIN;

export const MOSAIC_UNIFORMS = [
  "uView",
  "uInset",
  "uTilt",
  "uGlow",
  "uPitch",
  "uTileK",
  "uRadius",
  "uScrPer",
  "uStripeK",
  "uDPR",
  "uDark",
  "uClock",
  "uGrid",
  "uCursor",
  "uLight",
  "uHue",
  "uRipple",
  "uRipP",
  "uRipQ",
  "uIntro",
  "uGlyph",
  "uGlyphs",
] as const;

export type MosaicUniform = (typeof MOSAIC_UNIFORMS)[number];
