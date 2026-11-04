/**
 * The glass band shaders, copied verbatim from docs/design/holo/reveal-studies.html
 * (section 4, L586-654). The band paints only the bevel and the rim line, alpha 0
 * elsewhere, and samples only the vault snapshot, never the unlock screen. Cosmetic only.
 * Do not change the maths here: the look is the owner's (ADR-0032).
 */
import { LENS_GLSL } from "./lens.ts";

export const BAND_VERT = `#version 300 es
out vec2 vUv;
void main(){ vec2 p=vec2(gl_VertexID==1?3.0:-1.0, gl_VertexID==2?3.0:-1.0); vUv=p*0.5+0.5; gl_Position=vec4(p,0.0,1.0); }`;

export const BAND_FRAG = `#version 300 es
precision highp float;
in vec2 vUv; out vec4 oCol;
uniform vec2 uSize, uO;
uniform float uT, uR, uWob, uShape, uH, uPres, uDark, uRefr, uBevelK, uBevelMax, uSpread, uFilm, uMag;
uniform sampler2D uVault;
vec3 thinFilm(float t){ return 0.5+0.5*cos(6.2831853*(t+vec3(0.0,0.33,0.67))); }
vec3 foilPal(float t){ vec3 c=thinFilm(t); return pow(c,vec3(1.45))*1.08+0.015; }
vec3 V(vec2 p){ return texture(uVault,p/uSize).rgb; }
${LENS_GLSL}
void main(){
  vec2 p=vec2(vUv.x,1.0-vUv.y)*uSize;
  float w=clamp(uR*uBevelK,10.0,uBevelMax);
  float d0=lensD(p);
  if(uPres<=0.0||d0>4.0||d0<-w-4.0){ oCol=vec4(0.0); return; }
  vec2 g=vec2(lensD(p+vec2(0.5,0.0))-lensD(p-vec2(0.5,0.0)), lensD(p+vec2(0.0,0.5))-lensD(p-vec2(0.0,0.5)));
  float gl=max(length(g),1e-4); vec2 gd=g/gl; float d=d0/gl;
  float x=clamp(-d/w,0.0,1.0);
  float prof=sqrt(max(1.0-(1.0-x)*(1.0-x),0.0));
  float slope=(1.0-x)/max(prof,0.10);
  float inside=step(d,0.0);
  float kd=1.0-smoothstep(0.62,0.97,x);                 // refraction is gone before the inner edge
  float h=uH*prof*inside;
  vec3 n=normalize(vec3(gd*(uH/w)*slope*inside*kd,1.0));
  const vec3 S0=vec3(0.95,0.04,0.12), S1=vec3(0.80,0.45,0.0), S2=vec3(0.22,0.92,0.05), S3=vec3(0.0,0.70,0.62), S4=vec3(0.05,0.25,1.0), S5=vec3(0.50,0.0,0.85);
  vec3 SP[6]=vec3[6](S0,S1,S2,S3,S4,S5);
  vec3 acc=vec3(0.0), wsum=vec3(0.0);
  float mag=1.0+uMag*0.001*h*kd;
  for(int i=0;i<6;i++){
    float eta=1.0/(1.40+uSpread*float(i));
    vec3 tr=refract(vec3(0.0,0.0,-1.0),n,eta);
    vec2 disp=tr.xy/max(-tr.z,0.22)*(h+10.0)*uRefr;
    vec2 sp=uO+(p-uO)/mag+disp;
    acc+=V(sp)*SP[i]; wsum+=SP[i];          // only the vault: the unlock screen simply ends at the rim
  }
  vec3 ins=acc/wsum;
  float lum=dot(ins,vec3(0.2126,0.7152,0.0722));
  ins=mix(vec3(lum),ins,1.0+0.25*kd);                    // undo the pale spectral average (not at the seam)
  float cosT=n.z;
  float F=0.04+0.96*pow(1.0-cosT,5.0);
  float band=pow(1.0-x,1.5)*inside;
  float ft=0.2+1.7*(1.0-cosT)+0.0018*dot(p,vec2(1.0,0.7))+0.18*sin(dot(p,vec2(0.0042,0.0031))*6.2831853+uT*0.5)+uT*0.3;
  vec3 film=foilPal(ft);
  float irw=clamp((F*1.6-0.07)+band*0.16,0.0,0.75)*uFilm*step(0.5,uH);
  ins=mix(ins,film*mix(0.95,1.05,uDark),irw);
  vec3 L1=normalize(vec3(-0.55,-0.7,0.6)), L2=normalize(vec3(0.65,0.55,0.5));
  float s1=pow(max(dot(n,normalize(L1+vec3(0,0,1))),0.0),120.0);
  float s2=pow(max(dot(n,normalize(L2+vec3(0,0,1))),0.0),60.0);
  vec3 spec=vec3(1.0)*s1*1.8+film*s2*0.9;
  float hk=clamp(uH/20.0,0.0,1.0);
  float el=exp(-pow((-d-1.8)/1.3,2.0))*hk;                // crisp foil line just inside the glass edge
  spec+=film*el*mix(0.9,1.3,uDark);
  ins*=1.0-0.28*pow(1.0-x,4.0)*inside*(1.0-uDark)*hk;     // glass edge reads darker on white
  float e1=exp(-pow(d/1.2,2.0));
  float face=dot(gd,L1.xy)/length(L1.xy);
  ins=mix(ins,ins*0.35,e1*clamp(-face,0.0,1.0)*0.85*hk);
  spec+=film*e1*clamp(face*1.4+0.3,0.0,1.0)*1.3*hk;
  vec3 col=clamp(ins+spec,0.0,1.0);
  float aIn=(1.0-smoothstep(-0.7,0.7,d))*(1.0-smoothstep(0.82,1.0,x));
  // thin rim line just outside the edge
  float lA=exp(-pow((d-0.6)/0.8,2.0))*step(0.0,d)*0.55*hk;
  vec3 lC=mix(film*0.55,film*1.1,uDark);
  vec4 o=vec4(col*aIn,aIn);
  o=vec4(lC*lA,lA)+o*(1.0-lA);
  oCol=o*uPres;
}`;

/** Every uniform of the band program (the 15 scalars and vectors of `BandUniforms`, plus the texture). */
export const BAND_UNIFORMS = [
  "uSize",
  "uO",
  "uT",
  "uR",
  "uWob",
  "uShape",
  "uH",
  "uPres",
  "uDark",
  "uRefr",
  "uBevelK",
  "uBevelMax",
  "uSpread",
  "uFilm",
  "uMag",
  "uVault",
] as const;

export type BandUniform = (typeof BAND_UNIFORMS)[number];
