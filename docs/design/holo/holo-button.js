// holo-button.js
// Holographic foil buttons, "angled rainbow stripes" style (Mosaic button studies, v5 base, texture 10).
// Plain ES module, no dependencies. One shared WebGL2 context and one fixed canvas paint every
// mounted button; labels and icons stay ordinary DOM on top of the foil.

const DEFAULT_SEED = ((7 * 0.6180339) % 1) * 10 + 1; // the seed the study page used for this row
const BASE_PERIOD_DEV = 6;      // texture base period in device px at DPR >= 2 (0.75x of it at DPR 1)
const STRIPE_MULT = 0.33;       // stripe frequency relative to the base period: on-screen period = base / 0.33

const VERT = `#version 300 es
void main(){float x=gl_VertexID==1?3.0:-1.0;float y=gl_VertexID==2?3.0:-1.0;gl_Position=vec4(x,y,0.0,1.0);}`;

// Fragment shader: the study page's v5 code path with texture 10, constants folded in.
// uMask=1 paints a pill (buttons); uMask=0 paints the bare surface (stills).
// uUnit.x>0 overrides the unit size (device px per button height) and uUnit.y the half-aspect, for stills.
const FRAG = `#version 300 es
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

/** Shader sources, exported for testing and for apps that want their own renderer. */
export const HOLO_SHADER = { vertex: VERT, fragment: FRAG };

const UNIFORMS = ["uRect", "uMouse", "uTime", "uHover", "uPress", "uFocus", "uDark", "uSeed", "uDPR", "uScrPer", "uStripeK", "uMask", "uUnit"];

function createProgram(gl) {
  const compile = (type, src) => {
    const sh = gl.createShader(type);
    gl.shaderSource(sh, src);
    gl.compileShader(sh);
    if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
      const log = gl.getShaderInfoLog(sh);
      gl.deleteShader(sh);
      throw new Error("holo-button: shader compile failed: " + log);
    }
    return sh;
  };
  const p = gl.createProgram();
  gl.attachShader(p, compile(gl.VERTEX_SHADER, VERT));
  gl.attachShader(p, compile(gl.FRAGMENT_SHADER, FRAG));
  gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error("holo-button: link failed: " + gl.getProgramInfoLog(p));
  const loc = {};
  for (const u of UNIFORMS) loc[u] = gl.getUniformLocation(p, u);
  return { p, loc };
}

const STYLE_ID = "holo-button-style";
const CSS = `
button[data-holo]{position:relative;z-index:var(--holo-z,2);background:transparent;border:0;border-radius:999px;
  color:#09090B;text-shadow:0 1px 0 rgba(255,255,255,.55);cursor:pointer;-webkit-tap-highlight-color:transparent;
  transition:transform 140ms cubic-bezier(0.2,0,0,1)}
button[data-holo].holo-pressed{transform:scale(.975)}
button[data-holo] svg{filter:drop-shadow(0 1px 0 rgba(255,255,255,.55))}
button[data-holo].holo-fallback{background:#18181B;color:#FAFAFA;text-shadow:none}
button[data-holo].holo-fallback svg{filter:none}
button[data-holo="dark"].holo-fallback{background:#FAFAFA;color:#09090B}
`;

function clipAncestor(el) {
  for (let n = el.parentElement; n && n !== document.body && n !== document.documentElement; n = n.parentElement) {
    const cs = getComputedStyle(n);
    if (cs.overflowX !== "visible" || cs.overflowY !== "visible") return n;
  }
  return null;
}

/**
 * Upgrade every `button[data-holo]` inside `root` to a holo foil button.
 * data-holo="light" (default) or "dark" picks the card surface the button sits on.
 * data-holo-ambient makes a button animate its sheen continuously (use for one hero at most).
 * Returns { unmount(), refresh() }.
 */
export function mountHoloButtons(root = document, options = {}) {
  const opt = Object.assign({ zIndex: 1, dprCap: 2, injectStyles: true }, options);
  if (opt.injectStyles && !document.getElementById(STYLE_ID)) {
    const st = document.createElement("style");
    st.id = STYLE_ID;
    st.textContent = CSS;
    document.head.appendChild(st);
  }
  const buttons = Array.from(root.querySelectorAll("button[data-holo]"));
  const mq = window.matchMedia ? window.matchMedia("(prefers-reduced-motion: reduce)") : null;
  let reduced = !!(mq && mq.matches);
  let DPR = Math.min(window.devicePixelRatio || 1, opt.dprCap);
  let slowMode = false;

  const cv = document.createElement("canvas");
  cv.setAttribute("aria-hidden", "true");
  cv.style.cssText = `position:fixed;left:0;top:0;width:100%;height:100%;pointer-events:none;z-index:${opt.zIndex};display:block`;
  let gl = null, prog = null;
  try {
    gl = cv.getContext("webgl2", { alpha: true, premultipliedAlpha: true, antialias: false, depth: false, stencil: false, preserveDrawingBuffer: true });
    if (gl) prog = createProgram(gl);
  } catch (e) { prog = null; }
  let glOK = !!(gl && prog);

  const states = buttons.map((btn) => ({
    btn, clip: null, dark: btn.getAttribute("data-holo") === "dark" ? 1 : 0,
    ambient: btn.hasAttribute("data-holo-ambient"),
    seed: btn.hasAttribute("data-holo-seed") ? +btn.getAttribute("data-holo-seed") : DEFAULT_SEED,
    bx: 1.5, time: 2.0, hover: 0, hoverT: 0, press: 0, pressT: 0, focus: 0, focusT: 0,
    mx: 0, my: 0.3, mxT: 0, myT: 0.3, inside: false, active: false, leaveAt: -1e9, dirty: true, off: []
  }));

  function fallback() {
    cv.remove();
    for (const s of states) s.btn.classList.add("holo-fallback");
  }
  if (!glOK) { fallback(); return { unmount() { for (const s of states) s.btn.classList.remove("holo-fallback"); }, refresh() {} }; }

  document.body.appendChild(cv);
  gl.bindVertexArray(gl.createVertexArray());
  gl.disable(gl.BLEND); gl.disable(gl.DEPTH_TEST);
  gl.useProgram(prog.p);
  const L = prog.loc;
  gl.uniform1f(L.uMask, 1);
  gl.uniform1f(L.uStripeK, 1);
  gl.uniform2f(L.uUnit, 0, 0);

  let needFull = true, raf = 0, last = performance.now(), slowCount = 0, guardFrom = performance.now() + 2500;
  const kick = () => { if (glOK && !raf && !document.hidden) { last = performance.now(); raf = requestAnimationFrame(frame); } };
  const full = () => { needFull = true; kick(); };

  function resize() {
    const w = Math.round(document.documentElement.clientWidth * DPR), h = Math.round(window.innerHeight * DPR);
    if (cv.width !== w || cv.height !== h) { cv.width = w; cv.height = h; }
    guardFrom = performance.now() + 2500; slowCount = 0;
    for (const s of states) s.clip = clipAncestor(s.btn);
    full();
  }

  const approach = (cur, target, rate, dt) => reduced ? target : cur + (target - cur) * (1 - Math.exp(-rate * dt));
  const idle = (s) => { s.mxT = Math.sin(s.time * 0.7 + s.seed) * s.bx * 0.55; s.myT = 0.3 + 0.12 * Math.sin(s.time * 0.98 + s.seed * 2.0); };
  function update(s, dt, now) {
    const ow = s.btn.offsetWidth, oh = s.btn.offsetHeight;
    if (oh) s.bx = 0.5 * ow / oh;   // aspect before the idle pointer position is derived from it
    const ambient = s.ambient && !slowMode && !reduced;
    if (!s.inside) { if (s.focusT > 0.5) { s.mxT = Math.sin(s.time * 0.6) * s.bx * 0.4; s.myT = 0.25; } else idle(s); }
    s.hover = approach(s.hover, Math.max(s.hoverT, s.focusT * 0.6), 9, dt);
    s.press = approach(s.press, s.pressT, 22, dt);
    s.focus = approach(s.focus, s.focusT, 12, dt);
    const rate = s.inside ? 24 : 7;
    s.mx = approach(s.mx, s.mxT, rate, dt);
    s.my = approach(s.my, s.myT, rate, dt);
    s.active = !reduced && (ambient || s.inside || s.focusT > 0 || s.pressT > 0 || now - s.leaveAt < 900 ||
      Math.abs(s.hover - Math.max(s.hoverT, s.focusT * 0.6)) > 0.002 || Math.abs(s.press - s.pressT) > 0.002);
    if (s.active) s.time += dt; else { s.mx = s.mxT; s.my = s.myT; }
  }

  let CH = 0, VW = 0, VH = 0;
  function draw(s, clear) {
    const r = s.btn.getBoundingClientRect();
    if (!r.width || r.bottom < -4 || r.top > VH + 4 || r.right < -4 || r.left > VW + 4) return;
    const c = s.clip ? s.clip.getBoundingClientRect() : { left: 0, top: 0, right: VW, bottom: VH };
    s.bx = 0.5 * r.width / r.height;
    const grow = Math.max(2, r.width * 0.02);
    const x0 = Math.max(r.left - grow, c.left, 0), x1 = Math.min(r.right + grow, c.right, VW);
    const y0 = Math.max(r.top - grow, c.top, 0), y1 = Math.min(r.bottom + grow, c.bottom, VH);
    if (x1 <= x0 || y1 <= y0) return;
    const sx = Math.floor(x0 * DPR), sy = Math.floor(CH - y1 * DPR), sw = Math.ceil((x1 - x0) * DPR) + 1, sh = Math.ceil((y1 - y0) * DPR) + 1;
    gl.scissor(sx, sy, sw, sh);
    gl.viewport(sx, sy, sw, sh);
    if (clear) gl.clear(gl.COLOR_BUFFER_BIT);
    gl.uniform4f(L.uRect, r.left * DPR, CH - r.bottom * DPR, r.width * DPR, r.height * DPR);
    gl.uniform2f(L.uMouse, s.mx, s.my);
    gl.uniform1f(L.uTime, s.time);
    gl.uniform1f(L.uHover, s.hover);
    gl.uniform1f(L.uPress, s.press);
    gl.uniform1f(L.uFocus, s.focus);
    gl.uniform1f(L.uDark, s.dark);
    gl.uniform1f(L.uSeed, s.seed);
    gl.uniform1f(L.uDPR, DPR);
    gl.uniform1f(L.uScrPer, BASE_PERIOD_DEV * (DPR >= 2 ? 1 : 0.75));
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  function frame(now) {
    raf = 0;
    if (!glOK) return;
    const dtMs = now - last; last = now;
    const dt = Math.min(Math.max(dtMs / 1000, 0), 0.05);
    CH = cv.height; VW = document.documentElement.clientWidth; VH = window.innerHeight;
    let anyActive = false, anyDirty = false;
    for (const s of states) { update(s, dt, now); if (s.active) anyActive = true; if (s.dirty) anyDirty = true; }
    if (needFull || anyActive || anyDirty) {
      gl.clearColor(0, 0, 0, 0);
      if (needFull) {
        gl.disable(gl.SCISSOR_TEST); gl.clear(gl.COLOR_BUFFER_BIT); gl.enable(gl.SCISSOR_TEST);
        for (const s of states) { draw(s, false); s.dirty = false; }
        needFull = false;
      } else {
        gl.enable(gl.SCISSOR_TEST);
        for (const s of states) { if (s.active || s.dirty) draw(s, true); s.dirty = false; }
      }
      // slow-frame downgrade: three frames over 24 ms while animating -> 1x resolution, no ambient sheen
      if (!slowMode && anyActive && now > guardFrom && dtMs > 24 && dtMs < 500) { if (++slowCount >= 3) { slowMode = true; DPR = 1; resize(); } }
      else if (dtMs <= 24) slowCount = 0;
    }
    if (!document.hidden && (anyActive || needFull)) raf = requestAnimationFrame(frame);
  }

  // events
  const on = (s, el, type, fn, o) => { el.addEventListener(type, fn, o); s.off.push(() => el.removeEventListener(type, fn, o)); };
  const local = (s, e) => { const r = s.btn.getBoundingClientRect(); s.mxT = (e.clientX - (r.left + r.width / 2)) / r.height; s.myT = -(e.clientY - (r.top + r.height / 2)) / r.height; };
  const poke = (s) => { s.dirty = true; kick(); };
  const pressStart = (s) => { s.pressT = 1; s.btn.classList.add("holo-pressed"); poke(s); };
  const pressEnd = (s) => { s.pressT = 0; s.btn.classList.remove("holo-pressed"); poke(s); };
  for (const s of states) {
    const b = s.btn;
    on(s, b, "pointerenter", (e) => { s.inside = true; s.hoverT = 1; local(s, e); poke(s); });
    on(s, b, "pointermove", (e) => { local(s, e); poke(s); });
    on(s, b, "pointerleave", () => { s.inside = false; s.hoverT = 0; s.leaveAt = performance.now(); pressEnd(s); });
    on(s, b, "pointerdown", (e) => { if (e.button !== 0) return; local(s, e); pressStart(s); });
    on(s, b, "pointerup", () => pressEnd(s));
    on(s, b, "pointercancel", () => { s.hoverT = 0; pressEnd(s); });
    on(s, b, "keydown", (e) => { if ((e.key === " " || e.key === "Enter") && !e.repeat) { s.mxT = 0; s.myT = 0.15; pressStart(s); } });
    on(s, b, "keyup", (e) => { if (e.key === " " || e.key === "Enter") pressEnd(s); });
    on(s, b, "focus", () => { let fv = true; try { fv = b.matches(":focus-visible"); } catch (err) { fv = true; } s.focusT = fv ? 1 : 0; poke(s); });
    on(s, b, "blur", () => { s.focusT = 0; s.leaveAt = performance.now(); pressEnd(s); });
  }
  const onScroll = () => full();
  const onVis = () => { if (document.hidden) { if (raf) cancelAnimationFrame(raf); raf = 0; } else full(); };
  const onRM = () => { reduced = !!(mq && mq.matches); full(); };
  const onLost = (e) => { e.preventDefault(); glOK = false; fallback(); };
  window.addEventListener("resize", resize);
  window.addEventListener("scroll", onScroll, { passive: true, capture: true });
  document.addEventListener("visibilitychange", onVis);
  if (mq) { if (mq.addEventListener) mq.addEventListener("change", onRM); else if (mq.addListener) mq.addListener(onRM); }
  cv.addEventListener("webglcontextlost", onLost);
  const ro = window.ResizeObserver ? new ResizeObserver(full) : null;
  if (ro) ro.observe(document.body);
  if (document.fonts && document.fonts.ready) document.fonts.ready.then(full);
  resize();

  return {
    refresh: full,
    unmount() {
      if (raf) cancelAnimationFrame(raf);
      raf = 0; glOK = false;
      for (const s of states) { s.off.forEach((f) => f()); s.btn.classList.remove("holo-pressed", "holo-fallback"); }
      window.removeEventListener("resize", resize);
      window.removeEventListener("scroll", onScroll, { capture: true });
      document.removeEventListener("visibilitychange", onVis);
      if (mq) { if (mq.removeEventListener) mq.removeEventListener("change", onRM); else if (mq.removeListener) mq.removeListener(onRM); }
      if (ro) ro.disconnect();
      const ext = gl.getExtension("WEBGL_lose_context");
      if (ext) ext.loseContext();
      cv.remove();
    }
  };
}

/**
 * Render the bare foil surface (no pill, no rim, no label) into a 2D canvas, at rest.
 * width/height in device px. The stripe period in device px matches a DPR-2 page.
 * opts.dark: as on a black card. opts.tileable: seamless horizontally (stripe frequency adjusted < 0.5%).
 * opts.unitPx / opts.halfAspect: size and half-aspect of the reference button the low-frequency sheen is laid
 * out for (defaults: the 88 px hero pill at DPR 2).
 */
export function renderHoloSurface(width, height, opts = {}) {
  const o = Object.assign({ dark: false, tileable: false, unitPx: 176, halfAspect: 1.36, time: 2.0, seed: DEFAULT_SEED, dpr: 2 }, opts);
  const overlap = o.tileable ? Math.min(128, Math.floor(width / 2)) : 0;
  const W = width + overlap, Hh = height;
  const c = document.createElement("canvas"); c.width = W; c.height = Hh;
  const gl = c.getContext("webgl2", { alpha: true, premultipliedAlpha: true, antialias: false, preserveDrawingBuffer: true });
  if (!gl) throw new Error("holo-button: WebGL2 unavailable");
  const { p, loc } = createProgram(gl);
  gl.useProgram(p);
  gl.bindVertexArray(gl.createVertexArray());
  gl.viewport(0, 0, W, Hh);
  const scr = BASE_PERIOD_DEV * (o.dpr >= 2 ? 1 : 0.75);
  let stripeK = 1;
  if (o.tileable) {
    // make `width` an integer number of stripe periods along x
    const dgx = 1 / Math.sqrt(1 + 0.49);
    const xPeriod = (2 * Math.PI) / ((STRIPE_MULT * 2 * Math.PI / scr) * dgx);
    const n = Math.max(1, Math.round(width / xPeriod));
    stripeK = (n * xPeriod) / width;
  }
  const mx = Math.sin(o.time * 0.7 + o.seed) * o.halfAspect * 0.55;
  const my = 0.3 + 0.12 * Math.sin(o.time * 0.98 + o.seed * 2.0);
  gl.uniform4f(loc.uRect, 0, 0, W, Hh);
  gl.uniform2f(loc.uMouse, mx, my);
  gl.uniform1f(loc.uTime, o.time); gl.uniform1f(loc.uHover, 0); gl.uniform1f(loc.uPress, 0); gl.uniform1f(loc.uFocus, 0);
  gl.uniform1f(loc.uDark, o.dark ? 1 : 0); gl.uniform1f(loc.uSeed, o.seed); gl.uniform1f(loc.uDPR, o.dpr);
  gl.uniform1f(loc.uScrPer, scr); gl.uniform1f(loc.uStripeK, stripeK); gl.uniform1f(loc.uMask, 0);
  gl.uniform2f(loc.uUnit, o.unitPx, o.halfAspect);
  gl.drawArrays(gl.TRIANGLES, 0, 3);
  const px = new Uint8Array(W * Hh * 4);
  gl.readPixels(0, 0, W, Hh, gl.RGBA, gl.UNSIGNED_BYTE, px);
  const out = document.createElement("canvas"); out.width = width; out.height = height;
  const ctx = out.getContext("2d");
  const img = ctx.createImageData(width, height);
  for (let y = 0; y < height; y++) {
    const sy = height - 1 - y;
    for (let x = 0; x < width; x++) {
      const di = (y * width + x) * 4;
      if (x < overlap) {
        // wrap: blend the strip just past the right edge into the left edge
        const f = x / overlap, a = (sy * W + x + width) * 4, b = (sy * W + x) * 4;
        for (let k = 0; k < 3; k++) img.data[di + k] = Math.round(px[a + k] * (1 - f) + px[b + k] * f);
      } else {
        const si = (sy * W + x) * 4;
        img.data[di] = px[si]; img.data[di + 1] = px[si + 1]; img.data[di + 2] = px[si + 2];
      }
      img.data[di + 3] = 255;
    }
  }
  ctx.putImageData(img, 0, 0);
  const lose = gl.getExtension("WEBGL_lose_context");
  if (lose) lose.loseContext();
  return out;
}
