// stage — the scratchpad: a live UI harness the agent drives in the very
// browser the user is chatting from. The chat's tool loop already runs in
// this page, so its stage / eval / snapshot tools call straight into this
// control — no server round-trip, no second browser:
//
//   mount(name, spec)    mount lib:ctl[:{props}] as a named frame (a fresh
//                        read of the control, so the agent's latest facet
//                        edits show); mounting an existing name replaces it
//   unmount(name)        remove a frame (unmountAll clears the pad)
//   evaluate(js, ms)     run page JS with the mounts in scope as `pad`
//   outline(name)        the mount's rendered structure as compact text
//   capture(name)        the mount rasterized to a PNG (+ a small thumbnail)
//   errors()             page errors since the last call — uncaught errors,
//                        unhandled rejections, console.error / console.warn
//
// Nothing here knows about the chat: any page can mount agent:stage and
// drive it through this api. onchange(fn) reports the mount list.

var me = this;
var ME = document.getElementById(me.UUID);

var readyP = new Promise(function (res) { me.ready = res; }).then(function () {
  const body = ME.querySelector(".nb-stage-body");
  const empty = ME.querySelector(".nb-stage-empty");
  const mounts = new Map();          // name -> {name, spec, lib, ctl, el, api, error}
  const listeners = new Set();
  const MOUNT_TIMEOUT = 10000;

  const list = () => [...mounts.values()].map((m) => ({
    name: m.name, spec: m.spec, ready: !!m.api, error: m.error }));
  function changed() {
    empty.hidden = mounts.size > 0;
    for (const fn of listeners) { try { fn(list()); } catch (e) { /* a listener's problem */ } }
  }
  const pause = (ms) => new Promise((res) => setTimeout(res, ms));

  // ── the page's error stream. Hooks are page-wide by nature; each stage
  // instance keeps its own ring, and reading it advances that instance's
  // cursor, so every agent call sees exactly what happened since its last.
  const ring = [];
  let seq = 0, seen = 0;
  const fmt = (x) => {
    if (x && x.stack) return String(x.stack);
    if (typeof x === "string") return x;
    try { return JSON.stringify(x); } catch (e) { return String(x); }
  };
  function logErr(kind, msg) {
    ring.push({ n: ++seq, line: kind + ": " + String(msg).slice(0, 1200) });
    if (ring.length > 100) ring.shift();
  }
  window.addEventListener("error", (e) => logErr("error", e.error ? fmt(e.error) : e.message));
  window.addEventListener("unhandledrejection", (e) => logErr("unhandled rejection", fmt(e.reason)));
  for (const lvl of ["error", "warn"]) {
    const orig = console[lvl];
    console[lvl] = function () {
      try { logErr("console." + lvl, Array.from(arguments).map(fmt).join(" ")); } catch (e) { /* never break logging */ }
      return orig.apply(this, arguments);
    };
  }
  function errors() {
    const out = ring.filter((e) => e.n > seen).map((e) => e.line);
    seen = seq;
    return out;
  }

  // ── specs: lib:ctl or lib:ctl:{json-props}. Only the first two colons
  // split — props JSON is free to contain colons of its own.
  function parseSpec(spec) {
    const s = String(spec || "").trim();
    const i = s.indexOf(":");
    const j = i < 0 ? -1 : s.indexOf(":", i + 1);
    const lib = i < 0 ? "" : s.slice(0, i);
    const ctl = i < 0 ? "" : (j < 0 ? s.slice(i + 1) : s.slice(i + 1, j));
    if (!lib || !ctl) throw new Error("spec must be lib:ctl or lib:ctl:{json-props}, e.g. scratch:clock:{\"tz\":\"UTC\"}");
    let props = {};
    if (j >= 0 && s.slice(j + 1).trim()) {
      try { props = JSON.parse(s.slice(j + 1)); }
      catch (e) { throw new Error("props after lib:ctl: are not valid JSON — " + e.message); }
    }
    return { lib, ctl, props };
  }

  function readControls(lib) {
    return new Promise((res) => json("../app/read", "lib=" + encodeURIComponent(lib) + "&id=controls", (r) => {
      const raw = (r && r.status === "ok" && r.data && r.data.list) || [];
      res(raw.map((c) => { try { return typeof c === "string" ? JSON.parse(c) : c; } catch (e) { return {}; } }));
    }));
  }

  // The platform mounter caches control ids (CTLCACHE, 5s), records
  // (allmetas) and command stubs (alljsapis) per page. A remount after the
  // agent edits a facet must not be served the old code, so drop the
  // settled entries — in-flight ones (arrays of waiting callbacks) stay.
  function forgetCaches(lib) {
    try { if (window.CTLCACHE && CTLCACHE[lib]) CTLCACHE[lib].when = 0; } catch (e) { /* no cache */ }
    for (const cache of [window.allmetas, window.alljsapis]) {
      if (!cache) continue;
      for (const l of Object.keys(cache)) {
        for (const id of Object.keys(cache[l])) {
          if (!Array.isArray(cache[l][id])) delete cache[l][id];
        }
      }
    }
  }

  function frameFor(name, spec) {
    const frame = document.createElement("section");
    frame.className = "nb-stage-frame";
    frame.dataset.name = name;
    const bar = document.createElement("header");
    bar.className = "nb-stage-bar";
    const b = document.createElement("b");
    b.textContent = name;
    const sp = document.createElement("span");
    sp.className = "nb-stage-spec";
    sp.textContent = spec;
    const st = document.createElement("span");
    st.className = "nb-stage-status";
    const re = document.createElement("button");
    re.type = "button";
    re.textContent = "↻";
    re.title = "remount — reloads the control's current code";
    re.addEventListener("click", () => { mount(name, spec).catch((e) => logErr("stage", e.message)); });
    const x = document.createElement("button");
    x.type = "button";
    x.textContent = "✕";
    x.title = "remove from the scratchpad";
    x.addEventListener("click", () => unmount(name));
    bar.append(b, sp, st, re, x);
    const el = document.createElement("div");
    el.className = "nb-stage-instance";
    frame.append(bar, el);
    return { frame, el, status: st };
  }

  async function mount(name, spec) {
    name = String(name || "").trim();
    if (!/^[A-Za-z_$][\w$]*$/.test(name)) throw new Error("name must be a plain identifier, e.g. clock or form2");
    const { lib, ctl, props } = parseSpec(spec);
    const ctls = await readControls(lib);
    if (!ctls.some((c) => c.name === ctl)) {
      throw new Error("unknown control " + lib + ":" + ctl + " — create it first (dev-code-create_control)");
    }
    forgetCaches(lib);
    const { frame, el, status } = frameFor(name, spec);
    const old = mounts.get(name);
    if (old) old.frame.replaceWith(frame); else body.appendChild(frame);
    const m = { name, spec, lib, ctl, frame, el, api: null, error: null };
    mounts.set(name, m);
    status.textContent = "mounting…";
    changed();
    const api = await new Promise((res) => {
      const timer = setTimeout(() => res(null), MOUNT_TIMEOUT);
      try {
        installControl(el, lib, ctl, (a) => { clearTimeout(timer); res(a); }, props);
      } catch (e) {
        clearTimeout(timer);
        logErr("error", fmt(e));
        res(null);
      }
    });
    if (mounts.get(name) !== m) return { name, spec, ready: false, error: "replaced by a newer mount" };
    m.api = api;
    m.error = api ? null : "never became ready within " + (MOUNT_TIMEOUT / 1000) + "s — its script most likely threw (see errors)";
    status.textContent = api ? "" : "not ready";
    // ready() bodies run right after the mount callback; give their first
    // render a moment before anyone looks.
    await pause(150);
    changed();
    return { name, spec, ready: !!api, error: m.error };
  }

  function unmount(name) {
    const m = mounts.get(name);
    if (!m) return false;
    m.frame.remove();
    mounts.delete(name);
    changed();
    return true;
  }
  function unmountAll() {
    for (const m of mounts.values()) m.frame.remove();
    mounts.clear();
    changed();
  }

  function need(name) {
    const m = mounts.get(name);
    if (!m) throw new Error("nothing is mounted as " + name + (mounts.size ? " — mounted: " + [...mounts.keys()].join(", ") : " — the scratchpad is empty"));
    return m;
  }

  // ── evaluate: page JS with `pad` in scope (pad.<name>.api / .el / .spec).
  // A lone expression returns its value; a statement body needs `return`.
  function scope() {
    const pad = {};
    for (const m of mounts.values()) pad[m.name] = { el: m.el, api: m.api, spec: m.spec };
    // a name that isn't mounted says so, instead of "cannot read … of undefined"
    return new Proxy(pad, { get(t, k) {
      if (typeof k === "string" && !(k in t) && k !== "then" && k !== "toJSON") need(k);
      return t[k];
    } });
  }
  function serialize(v) {
    if (v === undefined) return "undefined";
    const seenObjs = new WeakSet();
    const out = JSON.stringify(v, function (k, x) {
      if (typeof x === "function") return "[function " + (x.name || "anonymous") + "]";
      if (typeof x === "bigint") return x.toString() + "n";
      if (typeof Element !== "undefined" && x instanceof Element) {
        const t = (x.textContent || "").replace(/\s+/g, " ").trim().slice(0, 80);
        return "<" + x.tagName.toLowerCase() + (x.className && typeof x.className === "string" ? "." + x.className.trim().split(/\s+/).join(".") : "") + ">" + (t ? " \"" + t + "\"" : "");
      }
      if (x instanceof Map) return { "[Map]": [...x.entries()] };
      if (x instanceof Set) return { "[Set]": [...x.values()] };
      if (x && typeof x === "object") {
        if (seenObjs.has(x)) return "[circular]";
        seenObjs.add(x);
      }
      return x;
    });
    return out === undefined ? String(v) : out;
  }
  async function evaluate(js, timeoutMs) {
    const src = String(js || "");
    let fn;
    try {
      // a trailing semicolon would make a lone expression a statement
      const expr = src.replace(/[;\s]+$/, "");
      fn = new Function("pad", "\"use strict\";\nreturn (async () => (\n" + expr + "\n))();");
    } catch (e1) {
      try {
        fn = new Function("pad", "\"use strict\";\nreturn (async () => {\n" + src + "\n})();");
      } catch (e2) {
        return { ok: false, error: "SyntaxError: " + e2.message, errors: errors() };
      }
    }
    const ms = timeoutMs > 0 ? timeoutMs : 10000;
    const t0 = performance.now();
    let ok = true, value, error = null;
    try {
      value = await Promise.race([
        fn(scope()),
        pause(ms).then(() => { throw new Error("timed out after " + ms + "ms"); }),
      ]);
    } catch (e) {
      // the message, not the stack: its frames are the harness's, not the agent's code
      ok = false;
      error = e && e.name && e.message !== undefined ? e.name + ": " + e.message : fmt(e);
    }
    await pause(50);   // let errors the call set off asynchronously land
    let text;
    try { text = ok ? serialize(value) : undefined; } catch (e) { text = "[unserializable: " + e.message + "]"; }
    return { ok, value: text, error, ms: Math.round(performance.now() - t0), errors: errors() };
  }

  // ── outline: what actually rendered, as compact text a model can read.
  const SKIP = new Set(["SCRIPT", "STYLE", "TEMPLATE", "LINK", "META"]);
  const ATTRS = ["type", "name", "role", "aria-label", "placeholder", "title", "href", "src", "alt"];
  function outline(name, maxLines) {
    const m = need(name);
    const limit = maxLines > 0 ? maxLines : 160;
    const root = m.el.getBoundingClientRect();
    const lines = [];
    let extra = 0;
    function desc(el) {
      const cs = getComputedStyle(el);
      const r = el.getBoundingClientRect();
      let s = el.tagName.toLowerCase();
      if (el.id && !/^[0-9a-f-]{20,}$/i.test(el.id)) s += "#" + el.id;
      if (typeof el.className === "string" && el.className.trim()) s += "." + el.className.trim().split(/\s+/).slice(0, 3).join(".");
      for (const a of ATTRS) {
        const v = el.getAttribute(a);
        if (v != null && v !== "") s += " " + a + "=\"" + v.slice(0, 60) + "\"";
      }
      if ("value" in el && /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName) && el.type !== "file") s += " value=\"" + String(el.value).slice(0, 80) + "\"";
      if (el.checked) s += " checked";
      if (el.disabled) s += " disabled";
      const hidden = cs.display === "none" || cs.visibility === "hidden" || (r.width === 0 && r.height === 0);
      s += hidden ? " (hidden)" : " " + Math.round(r.width) + "×" + Math.round(r.height) + " @" + Math.round(r.left - root.left) + "," + Math.round(r.top - root.top);
      if (el.scrollHeight > el.clientHeight + 1 && /(auto|scroll|hidden)/.test(cs.overflowY)) s += " overflow-y:" + el.scrollHeight + ">" + el.clientHeight;
      let own = "";
      for (const n of el.childNodes) if (n.nodeType === 3) own += n.textContent;
      own = own.replace(/\s+/g, " ").trim();
      if (own) s += " \"" + own.slice(0, 100) + (own.length > 100 ? "…" : "") + "\"";
      return { s, hidden };
    }
    function walk(el, depth) {
      if (SKIP.has(el.tagName)) return;
      if (lines.length >= limit) { extra++; return; }
      const { s, hidden } = desc(el);
      lines.push("  ".repeat(depth) + s);
      if (hidden || el.tagName === "svg" || el.tagName === "SVG") return;
      for (const c of el.children) walk(c, depth + 1);
    }
    const rootStyle = getComputedStyle(m.el);
    lines.push("[" + m.name + " · " + m.spec + " · " + Math.round(root.width) + "×" + Math.round(root.height) +
      " · font " + rootStyle.fontSize + " " + rootStyle.fontFamily.split(",")[0] + " · color " + rootStyle.color + "]");
    for (const c of m.el.children) walk(c, 0);
    if (lines.length === 1) lines.push("(empty — the control rendered no elements)");
    if (extra) lines.push("… " + extra + " more elements not shown");
    return lines.join("\n");
  }

  // ── capture: rasterize a mount without any library. The subtree is
  // cloned with every computed style inlined (so page stylesheets, scoped
  // styles and theme variables all come along), canvases and same-origin
  // images become data URLs, and the clone renders through an SVG
  // foreignObject onto a canvas. Lost by design: pseudo-elements, scroll
  // positions, iframes, shadow DOM, and web fonts (system fallback).
  const INHERITED = new Set(["color", "cursor", "direction", "font-family", "font-size", "font-style",
    "font-variant", "font-weight", "font-stretch", "font-feature-settings", "font-kerning", "letter-spacing",
    "line-height", "list-style-image", "list-style-position", "list-style-type", "quotes", "tab-size",
    "text-align", "text-indent", "text-shadow", "text-transform", "visibility", "white-space",
    "word-break", "word-spacing", "overflow-wrap", "writing-mode", "border-collapse", "border-spacing",
    "caption-side", "empty-cells", "hyphens", "caret-color", "accent-color", "color-scheme",
    "-webkit-text-fill-color", "-webkit-text-stroke-color", "-webkit-text-stroke-width",
    "fill", "fill-opacity", "fill-rule", "stroke", "stroke-width", "stroke-opacity", "stroke-linecap",
    "stroke-linejoin", "stroke-dasharray", "paint-order", "text-rendering", "-webkit-font-smoothing"]);
  let sandbox = null;
  const baselines = {};
  function baseline(tag) {
    if (baselines[tag]) return baselines[tag];
    if (!sandbox) {
      sandbox = document.createElement("iframe");
      sandbox.setAttribute("aria-hidden", "true");
      sandbox.style.cssText = "position:fixed;left:-10000px;top:0;width:400px;height:300px;visibility:hidden;border:0";
      document.body.appendChild(sandbox);
    }
    const doc = sandbox.contentDocument;
    const el = doc.createElement(tag);
    doc.body.appendChild(el);
    const cs = sandbox.contentWindow.getComputedStyle(el);
    const snap = {};
    for (let i = 0; i < cs.length; i++) snap[cs[i]] = cs.getPropertyValue(cs[i]);
    el.remove();
    return (baselines[tag] = snap);
  }
  function inlineStyle(src, dst, parentCs) {
    const cs = getComputedStyle(src);
    const base = baseline(src.tagName.toLowerCase());
    let css = "";
    for (let i = 0; i < cs.length; i++) {
      const p = cs[i];
      if (p.startsWith("--")) continue;
      const v = cs.getPropertyValue(p);
      // inherited properties flow from the clone's parent; the rest are
      // compared with the tag's browser default
      if (INHERITED.has(p) ? (parentCs && parentCs.getPropertyValue(p) === v) : base[p] === v) continue;
      css += p + ":" + v + ";";
    }
    dst.setAttribute("style", css);
    return cs;
  }
  function cloneStyled(node, parentCs) {
    if (node.nodeType === 3) return document.createTextNode(node.textContent);
    if (node.nodeType !== 1 || SKIP.has(node.tagName)) return null;
    const tag = node.tagName;
    if (tag === "IFRAME") {
      const ph = document.createElement("div");
      inlineStyle(node, ph, parentCs);
      ph.textContent = "[iframe]";
      return ph;
    }
    if (tag === "CANVAS") {
      const img = document.createElement("img");
      try { img.setAttribute("src", node.toDataURL()); } catch (e) { /* tainted: leave it blank */ }
      inlineStyle(node, img, parentCs);
      return img;
    }
    const out = node.cloneNode(false);
    out.removeAttribute("id");
    if (tag === "IMG" && node.complete && node.naturalWidth) {
      try {
        const c = document.createElement("canvas");
        c.width = node.naturalWidth;
        c.height = node.naturalHeight;
        c.getContext("2d").drawImage(node, 0, 0);
        out.setAttribute("src", c.toDataURL());
      } catch (e) { out.removeAttribute("src"); }
      out.removeAttribute("srcset");
    }
    if (tag === "INPUT") {
      if (node.type === "checkbox" || node.type === "radio") { if (node.checked) out.setAttribute("checked", ""); else out.removeAttribute("checked"); }
      else if (node.type !== "file") out.setAttribute("value", node.value);
    }
    const cs = inlineStyle(node, out, parentCs);
    if (tag === "TEXTAREA") { out.textContent = node.value; return out; }
    if (tag === "SELECT") {
      for (const c of node.children) { const k = cloneStyled(c, cs); if (k) out.appendChild(k); }
      [...out.querySelectorAll("option")].forEach((o, i) => { if (node.options[i] && node.options[i].selected) o.setAttribute("selected", ""); });
      return out;
    }
    for (const c of node.childNodes) { const k = cloneStyled(c, cs); if (k) out.appendChild(k); }
    return out;
  }
  function backdrop(el) {
    for (let n = el; n && n.nodeType === 1; n = n.parentElement) {
      const bg = getComputedStyle(n).backgroundColor;
      if (bg && bg !== "transparent" && !/rgba\(.*,\s*0\)$/.test(bg)) return bg;
    }
    return "#ffffff";
  }
  async function capture(name, maxWidth) {
    const m = need(name);
    const src = m.el;
    const r = src.getBoundingClientRect();
    const w = Math.ceil(Math.max(src.scrollWidth, r.width));
    const h = Math.ceil(Math.max(src.scrollHeight, r.height));
    if (!w || !h) throw new Error(name + " has no size to capture (hidden, or it rendered nothing)");
    const clone = cloneStyled(src, null);
    clone.style.margin = "0";
    clone.style.width = w + "px";
    clone.style.height = h + "px";
    clone.style.overflow = "visible";
    const xml = new XMLSerializer().serializeToString(clone);
    const svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"" + w + "\" height=\"" + h + "\">" +
      "<foreignObject x=\"0\" y=\"0\" width=\"100%\" height=\"100%\">" + xml + "</foreignObject></svg>";
    const img = await new Promise((res, rej) => {
      const i = new Image();
      i.onload = () => res(i);
      i.onerror = () => rej(new Error("the browser could not render " + name + " to an image"));
      i.src = "data:image/svg+xml;charset=utf-8," + encodeURIComponent(svg);
    });
    const scale = Math.min(window.devicePixelRatio || 1, 2, (maxWidth > 0 ? maxWidth : 1600) / w);
    const canvas = document.createElement("canvas");
    canvas.width = Math.max(1, Math.round(w * scale));
    canvas.height = Math.max(1, Math.round(h * scale));
    const ctx = canvas.getContext("2d");
    ctx.fillStyle = backdrop(src);
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
    const blob = await new Promise((res, rej) => {
      try { canvas.toBlob((b) => b ? res(b) : rej(new Error("PNG encoding failed")), "image/png"); }
      catch (e) { rej(new Error("the browser refused to export the image (" + e.message + ")")); }
    });
    const tw = Math.min(480, canvas.width);
    const thumbC = document.createElement("canvas");
    thumbC.width = tw;
    thumbC.height = Math.max(1, Math.round(canvas.height * tw / canvas.width));
    thumbC.getContext("2d").drawImage(canvas, 0, 0, thumbC.width, thumbC.height);
    return { blob, width: w, height: h, thumb: thumbC.toDataURL("image/jpeg", 0.8) };
  }

  changed();

  me.mount = mount;
  me.unmount = unmount;
  me.unmountAll = unmountAll;
  me.list = list;
  me.has = (name) => mounts.has(name);
  me.evaluate = evaluate;
  me.outline = outline;
  me.capture = capture;
  me.errors = errors;
  me.onchange = (fn) => { listeners.add(fn); return () => listeners.delete(fn); };
});
