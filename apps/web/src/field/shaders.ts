/**
 * The field's shaders (prototype field.js). One full-screen triangle, two texture reads. The
 * page texture is a quiet tonal mosaic of towers around the key lens; the same program paints
 * the transition veil (`uOverlay`): a diagonal wipe that re-scrambles the screen. Cosmetic only.
 * Identifiers stay off the GLSL ES 3.00 reserved list (glsl.test.ts).
 */
import { fieldColors } from "@mosaic/design-tokens";

/** One channel of a token colour as a GLSL float literal. */
const channel = (v: number): string => (v / 255).toFixed(3);
const cover = fieldColors.cover.map(channel).join(",");

export const FIELD_VERT = `#version 300 es
void main(){float x=gl_VertexID==1?3.0:-1.0;float y=gl_VertexID==2?3.0:-1.0;gl_Position=vec4(x,y,0.0,1.0);}`;

export const FIELD_FRAG = `#version 300 es
precision highp float;
uniform vec2 uRes;
uniform float uTime, uLensR, uScroll, uVh, uWipe, uOverlay, uCell, uPage, uHeroY, uDpr;
uniform vec2 uLens;
uniform vec3 uClick;
uniform sampler2D uAtlas;
out vec4 o;
float h21(vec2 p){vec3 p3=fract(vec3(p.xyx)*0.1031);p3+=dot(p3,p3.yzx+33.33);return fract((p3.x+p3.y)*p3.z);}
float vn(vec2 p){vec2 i=floor(p),f=fract(p);vec2 u=f*f*(3.0-2.0*f);return mix(mix(h21(i),h21(i+vec2(1,0)),u.x),mix(h21(i+vec2(0,1)),h21(i+vec2(1,1)),u.x),u.y);}
float fbm(vec2 p){float s=0.0,a=0.5;for(int i=0;i<3;i++){s+=a*vn(p);p=p*2.03+vec2(1.7,9.2);a*=0.5;}return s/0.875;}
vec3 film(float t){return 0.5+0.5*cos(6.2831853*(t+vec3(0.0,0.33,0.67)));}
/* Tower height 0..1 of a cell (mirrored in JS by towers.ts for the data layer). */
float towerH(vec2 c,vec2 cs,float t){
  vec2 cc=(c+0.5)*cs;
  float d=length(cc-uLens);
  float r=(0.72*exp(-pow(d/(uLensR*0.5),2.0))+0.28*exp(-d/(uLensR*1.1)))*(1.0-smoothstep(uLensR*2.2,uLensR*2.6,d));
  float wv=0.0;
  if(uClick.z>=0.0&&uClick.z<1.6){
    float cd=length(cc-uClick.xy);
    wv=exp(-pow((cd-uClick.z*uLensR*1.5)/(cs.x*3.5),2.0))*(1.0-uClick.z/1.6)*(0.45+0.55*h21(c+2.2));
  }
  if(r<0.002&&wv<0.002)return 0.0;
  float n=vn(c*0.17+vec2(t*0.09,-t*0.06));
  float n2=vn(c*0.045+vec2(-t*0.03,t*0.025)+3.0);
  float x=(0.55*n+0.45*h21(c+13.7))*(0.55+0.65*n2);
  x=x*x*7.0;
  return clamp(clamp((floor(x)+smoothstep(0.7,1.0,fract(x)))/7.0,0.0,1.0)*r+0.6*wv,0.0,1.0);
}
float glyph(vec2 f,float idx){
  vec2 g=(f-0.5)*vec2(1.18,1.08)+0.5;
  if(g.x<0.0||g.x>1.0||g.y<0.0||g.y>1.0)return 0.0;
  return texture(uAtlas,vec2((idx+g.x)/16.0,1.0-g.y)).a;
}
void main(){
  vec2 px=gl_FragCoord.xy;
  vec2 cs=uOverlay>0.5?vec2(uCell*0.72,uCell):vec2(uCell);
  vec2 cell=floor(px/cs);
  vec2 f=fract(px/cs);
  float t=uTime;
  float rnd=h21(cell);
  vec2 uv=px/uRes;
  vec3 pm=vec3(0.0);float a=0.0;
  if(uOverlay<0.5){
    float docY=(uRes.y-px.y)+uScroll;
    float hero=1.0-smoothstep(uVh*0.8,uVh*1.6,docY);
    float base=0.8*mix(0.45,uPage,hero);
    vec2 hc=vec2(uRes.x*0.5,uRes.y-(uHeroY-uScroll));
    float center=1.0-smoothstep(0.1,0.85,length((px-hc)/uRes.y*vec2(0.72,1.0)));
    vec2 cam=uLens-vec2(0.0,uLensR*0.9);
    float w=0.42*cs.x;
    bool act=length(px-uLens)<uLensR*2.6+3.0*cs.x||(uClick.z>=0.0&&uClick.z<1.6&&abs(length(px-uClick.xy)-uClick.z*uLensR*1.5)<cs.x*12.0);
    vec2 sg=vec2(px.x>=cam.x?1.0:-1.0,px.y>=cam.y?1.0:-1.0);
    float bz=-1.0,bh=0.0,bt=0.0,face=0.0;vec2 bc=cell,bq=vec2(0.0);
    for(int j=0;j<3;j++){for(int i=0;i<3;i++){
      if(!act&&(i>0||j>0))continue;
      vec2 c=cell-vec2(float(i)*sg.x,float(j)*sg.y);
      float h=act?towerH(c,cs,t):0.0;
      vec2 cc=(c+0.5)*cs;
      vec2 dir=cc-cam;float dd=max(length(dir),1.0);
      vec2 o=dir/dd*min(dd/uLensR*h*2.4,1.95)*cs.x;
      vec2 r=px-cc;
      vec2 lo,hi;
      if(abs(o.x)<1e-3){lo.x=abs(r.x)<=w?0.0:2.0;hi.x=abs(r.x)<=w?1.0:-1.0;}
      else{float a1=(r.x-w)/o.x,a2=(r.x+w)/o.x;lo.x=min(a1,a2);hi.x=max(a1,a2);}
      if(abs(o.y)<1e-3){lo.y=abs(r.y)<=w?0.0:2.0;hi.y=abs(r.y)<=w?1.0:-1.0;}
      else{float a1=(r.y-w)/o.y,a2=(r.y+w)/o.y;lo.y=min(a1,a2);hi.y=max(a1,a2);}
      float t0=max(max(lo.x,lo.y),0.0),t1=min(min(hi.x,hi.y),1.0);
      if(t0<=t1&&t1*h>bz){
        bz=t1*h;bh=h;bt=t1;bc=c;bq=r-o;
        face=t1>0.9999?1.0:(hi.x<hi.y?2.0:3.0);
      }
    }}
    float flow=fbm(bc*vec2(0.06,0.08)+vec2(t*0.02,-t*0.013));
    float flow2=fbm(bc*0.21+vec2(-t*0.03,t*0.02)+5.0);
    float tone=0.012+0.05*flow*flow+0.018*flow2;
    float brnd=h21(bc);
    float rate=mix(0.04,0.18,h21(bc+7.1));
    float age=fract(t*rate+brnd*7.0);
    tone+=0.028*(1.0-smoothstep(0.0,0.4,age))*step(0.9,h21(bc+3.3));
    float lensW=1.0-smoothstep(uLensR*0.15,uLensR,length(px-uLens));
    float edgeD=w-max(abs(bq.x),abs(bq.y));
    float rim=1.0-smoothstep(0.0,1.3*uDpr,edgeD);
    float cover=face>0.5?1.0:0.0;
    if(face>0.5&&face<1.5){
      cover=smoothstep(-0.6,0.6,edgeD);
      tone=tone*(1.0+2.0*bh)+0.07*bh+0.01*lensW+rim*bh*0.11;
    }else if(face>1.5){
      tone=(tone*0.7+0.04*bh)*(0.35+0.65*bt)*(face<2.5?0.9:0.55);
    }
    float headerMask=1.0-smoothstep(uRes.y-150.0*uDpr,uRes.y-60.0*uDpr,px.y);
    tone*=cover*base*headerMask*(1.0-0.8*center*hero);
    vec3 tint=mix(vec3(0.7,0.68,0.95),vec3(0.9,0.89,1.0),flow);
    tint=mix(tint,vec3(0.6,0.55,1.0),clamp(bh*(face<1.5?0.55+0.45*rim:0.35),0.0,1.0));
    pm=tint*tone;a=tone;
  }
  float s=(uv.x+(1.0-uv.y))*0.5;
  float cov=smoothstep(s-0.07,s+0.07,uWipe)*(1.0-smoothstep(s-0.07,s+0.07,uWipe-1.0));
  if(uWipe>0.0&&cov>0.001){
    float front=exp(-pow(uWipe-s,2.0)*420.0)+exp(-pow(uWipe-1.0-s,2.0)*420.0);
    float cg=glyph(f,floor(h21(cell+floor(t*22.0+rnd*9.0))*16.0));
    vec3 holo=film(rnd*0.7+t*0.08+s*1.6);
    holo=mix(vec3(dot(holo,vec3(0.3333))),holo,0.55);
    float flow2=fbm(cell*0.08+t*0.3);
    vec3 covPm=vec3(${cover})+mix(vec3(0.3,0.27,0.58),holo,0.2+0.8*front)*cg*(0.16+0.3*flow2*flow2+1.1*front);
    pm=mix(pm,covPm,cov);a=mix(a,1.0,cov);
  }
  pm+=(h21(px+fract(t))-0.5)/255.0;
  o=vec4(clamp(pm,0.0,1.0),clamp(a,0.0,1.0));
}`;

export const FIELD_UNIFORMS = [
  "uRes",
  "uTime",
  "uLensR",
  "uScroll",
  "uVh",
  "uWipe",
  "uOverlay",
  "uCell",
  "uPage",
  "uHeroY",
  "uDpr",
  "uLens",
  "uClick",
  "uAtlas",
] as const;
