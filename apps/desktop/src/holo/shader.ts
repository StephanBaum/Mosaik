/**
 * The holo-foil shaders (ADR-0014), copied verbatim from docs/design/holo/holo-button.js
 * (Mosaic button studies, v5 base, texture 10). Cosmetic only.
 */

/** Full-screen triangle. */
export const VERT = `#version 300 es
void main(){float x=gl_VertexID==1?3.0:-1.0;float y=gl_VertexID==2?3.0:-1.0;gl_Position=vec4(x,y,0.0,1.0);}`;

/**
 * Fragment shader. `uMask = 1` paints a pill (buttons); `uMask = 0` paints the bare
 * surface (stills). `uUnit.x > 0` overrides the unit size (device px per button height)
 * and `uUnit.y` the half-aspect, for stills.
 */
export const FRAG = `#version 300 es
precision highp float;
uniform vec4 uRect;
uniform vec2 uMouse;
uniform float uTime, uHover, uPress, uFocus, uDark, uSeed, uDPR, uScrPer, uStripeK, uMask;
uniform vec2 uUnit;
out vec4 fragColor;
float hash12(vec2 p){ vec3 p3=fract(vec3(p.xyx)*0.1031); p3+=dot(p3,p3.yzx+33.33); return fract((p3.x+p3.y)*p3.z); }
float vnoise(vec2 p){ vec2 i=floor(p); vec2 f=fract(p); vec2 u=f*f*(3.0-2.0*f); return mix(mix(hash12(i),hash12(i+vec2(1.0,0.0)),u.x),mix(hash12(i+vec2(0.0,1.0)),hash12(i+vec2(1.0,1.0)),u.x),u.y); }
float fbm3(vec2 p){ float s=0.0; float a=0.5; for(int i=0;i<3;i++){ s+=a*vnoise(p); p=p*2.03+vec2(1.7,9.2); a*=0.5; } return s/0.875; }
vec3 thinFilm(float t){ return 0.5+0.5*cos(6.2831853*(t+vec3(0.0,0.33,0.67))); }
float boxK(float w){ float h=0.5*w; return h<1e-3?1.0:max(sin(h)/h,0.0); }
void stripe(float ph, vec2 gp, float PXv, float wvv, out float r, out vec2 tt, out float am){
  float w=length(gp)*PXv; am=1.0-smoothstep(1.3,2.5,w);
  float sn=sin(ph); float cs=cos(ph); float kp=0.8;
  float bk=boxK(w); sn*=bk; cs*=bk; kp=mix(0.8,1.6,smoothstep(1.4,2.2,w));
  r=mix(0.5,smoothstep(-kp,kp,sn-wvv),am);
  tt=-0.55*cs*(gp/max(length(gp),1e-4))*am*0.75;
}
void main(){
  float H=uRect.w; float PX=1.0/H;
  vec2 B=vec2(0.5*uRect.z/H,0.5);
  if(uUnit.x>0.0){ H=uUnit.x; PX=1.0/H; B=vec2(uUnit.y,0.5); }
  vec2 p=(gl_FragCoord.xy-(uRect.xy+uRect.zw*0.5))/H;
  float a=1.0; float e=10.0;
  if(uMask>0.5){
    vec2 q0=abs(p)-B+0.5; float d=length(max(q0,0.0))+min(max(q0.x,q0.y),0.0)-0.5;
    a=clamp(0.5-d/PX,0.0,1.0);
    if(a<=0.0){ fragColor=vec4(0.0); return; }
    e=max(-d,0.0);
  }
  vec2 tilt=clamp(uMouse/vec2(B.x*0.75+0.2,0.6),-1.2,1.2);
  float period=uScrPer*PX;
  float K=6.2831853/period;
  float warp=fbm3(p*2.0+vec2(3.7,1.3));
  float wv=0.40*(vnoise(p*4.0+11.0)-0.5);
  float rid; vec2 t; float amp;
  vec2 dg=normalize(vec2(1.0,0.7)); float Ks=K*0.33*uStripeK;
  stripe(Ks*dot(p,dg)+0.6*(warp-0.5),Ks*dg,PX,wv,rid,t,amp);
  vec3 n=normalize(vec3(t*0.5*1.6,1.0));
  vec2 g=normalize(vec2(p.x-clamp(p.x,-(B.x-0.5),B.x-0.5),p.y)+1e-6);
  float kk=clamp(1.0-e/0.06,0.0,1.0);
  n=normalize(n+vec3(g*kk*kk*1.2,0.0));
  vec3 V=normalize(vec3(-tilt*0.6,1.0));
  float cosT=clamp(dot(n,V),0.0,1.0);
  float thick=0.45+0.30*warp+0.10*rid*3.5+0.18*dot(p,vec2(0.6,0.9))+0.45*tilt.x+0.25*tilt.y+uTime*0.05+uSeed*0.1;
  vec3 irid=thinFilm(2.2*thick*cosT);
  irid=mix(vec3(dot(irid,vec3(0.3333))),irid,0.72);
  vec3 Lk=normalize(vec3(-0.35,0.6,1.0));
  float base=0.72+0.22*dot(n,Lk);
  vec2 dir=normalize(vec2(1.0,0.55));
  float band=exp(-pow(dot(p,dir)-tilt.x*B.x*0.8,2.0)*2.2);
  float amb=exp(-pow(dot(p,dir)-mix(-B.x-1.0,B.x+1.0,fract(uTime*0.08+uSeed*0.07)),2.0)*3.0);
  float sheen=(0.20+0.38*band+0.32*amb)*(0.8+0.3*uHover)*(0.75+0.35*(1.0-rid)*amp)*1.0;
  vec3 col=vec3(base)*vec3(0.985,0.99,1.0);
  col=mix(col,col*(0.58+0.85*irid),clamp(sheen,0.0,1.0));
  vec3 L=normalize(vec3(uMouse,0.8)-vec3(p,0.0));
  float sp=pow(max(dot(n,normalize(L+V)),0.0),70.0);
  col+=vec3(1.0)*sp*(0.08+0.22*uHover);
  if(uMask>0.5){
    float ep=e/PX; float rw=max(1.0,uDPR);
    float rimM=1.0-smoothstep(rw-0.5,rw+0.5,ep);
    float hl=(smoothstep(rw-0.5,rw+0.5,ep)-smoothstep(2.0*rw-0.5,2.0*rw+0.5,ep))*smoothstep(0.05,0.45,p.y);
    col=mix(col,vec3(1.0),hl*0.35);
    col=mix(col,mix(vec3(0.831,0.831,0.847),vec3(0.90,0.90,0.92),uDark),rimM*mix(1.0,0.75,uDark));
  }
  col*=mix(1.0,0.94,uDark)*(1.0-0.06*uPress)*(1.0+0.03*uFocus);
  col+=(hash12(gl_FragCoord.xy+fract(uTime))-0.5)/255.0;
  col=clamp(col,0.0,1.0);
  fragColor=vec4(col*a,a);
}`;

export const HOLO_SHADER = { vertex: VERT, fragment: FRAG } as const;

export const UNIFORMS = [
  "uRect",
  "uMouse",
  "uTime",
  "uHover",
  "uPress",
  "uFocus",
  "uDark",
  "uSeed",
  "uDPR",
  "uScrPer",
  "uStripeK",
  "uMask",
  "uUnit",
] as const;

export type Uniform = (typeof UNIFORMS)[number];

/** The seed the study page used for this row. */
export const DEFAULT_SEED = ((7 * 0.6180339) % 1) * 10 + 1;
/** Texture base period in device px at DPR >= 2 (0.75x of it at DPR 1). */
export const BASE_PERIOD_DEV = 6;
/** Stripe frequency relative to the base period: on-screen period = base / 0.33. */
export const STRIPE_MULT = 0.33;
