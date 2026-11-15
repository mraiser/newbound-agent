// memorypane — the control editor's memory tab, grafted onto the
// workbench by the registry (dev.plugins: target dev.workbench, selector
// .wb-tab-plugins, a hidden slot). It registers a `memory` facet tab through the workbench's
// facetTabs seam; the pane it is handed lives in the workbench's own
// tab panel. Claims are read as the union of the shipped facet and the
// instance-local overlay (agent.memorypane.claims) and written only
// through agent.archivist.remember / promote — never by patching the
// facet's bytes.
var me = this;
var ME = document.getElementById(me.UUID);

me.ready = function () {
  // the workbench's own element: the nearest ancestor carrying an api
  var wb = ME.parentElement;
  while (wb && !wb.api) wb = wb.parentElement;
  if (!wb) return;
  var invokeP = function (l2, c2, m2, a2) {
    return new Promise(function (res2) { invokeCommand(l2, c2, m2, a2, res2); });
  };
  var CSS = [
    ".mp{padding:12px 14px;font-size:13px;line-height:1.45}",
    ".mp-head{display:flex;gap:10px;align-items:center;flex-wrap:wrap;margin-bottom:10px}",
    ".mp-sum{opacity:.75;flex:1;min-width:200px}",
    ".mp button{font:inherit;padding:3px 10px;border:1px solid currentColor;border-radius:4px;background:transparent;color:inherit;cursor:pointer;opacity:.85}",
    ".mp button:hover{opacity:1}.mp button:disabled{opacity:.4;cursor:default}",
    ".mp-row{border-top:1px solid rgba(128,128,128,.3);padding:9px 0}",
    ".mp-row.sup{opacity:.5}",
    ".mp-claim{font-weight:600}",
    ".mp-detail{margin-top:4px;opacity:.8;white-space:pre-wrap}",
    ".mp-detail.clip{display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;cursor:pointer}",
    ".mp-meta{margin-top:5px;display:flex;gap:6px;flex-wrap:wrap;font-size:11px}",
    ".mp-tag{padding:1px 6px;border-radius:8px;background:rgba(128,128,128,.18)}",
    ".mp-tag.stale{background:rgba(220,120,40,.3)}",
    ".mp-tag.local{background:rgba(120,140,230,.3)}",
    ".mp-tag.conf{background:rgba(90,170,110,.28)}",
    ".mp-form{display:grid;gap:6px;margin:0 0 12px;padding:10px;border:1px solid rgba(128,128,128,.35);border-radius:6px}",
    ".mp-form input,.mp-form textarea,.mp-form select{font:inherit;padding:4px 6px;background:transparent;color:inherit;border:1px solid rgba(128,128,128,.5);border-radius:4px}",
    ".mp-form .mp-line{display:flex;gap:8px;align-items:center;flex-wrap:wrap}",
    ".mp-form .mp-line input{flex:1;min-width:160px}",
    ".mp-note{font-size:12px;opacity:.8}",
    ".mp-empty{opacity:.7;padding:8px 0}",
  ].join("\n");

  function go(tabs) {
    if (!tabs || typeof tabs.add !== "function") return;
    var lib = tabs.lib, ctl = tabs.ctl;
    var root = null;        // built on first selection
    var data = null;
    var handle = tabs.add({
      facet: "memory",
      label: "memory",
      hue: "var(--f-mem, #b48ad6)",
      title: "what the agent has learned about this control — the shipped memory facet plus this instance's overlay",
      mount: function (el) { root = el; build(); return load(); },
    });
    if (!handle) return;

    function tag(text, cls, title) {
      var s = document.createElement("span");
      s.className = "mp-tag" + (cls ? " " + cls : "");
      s.textContent = text;
      if (title) s.title = title;
      return s;
    }

    function chip() {
      if (!data) return;
      handle.setAbsent(data.count === 0);
      handle.setCount(data.count === 0 ? "none" : String(data.count));
    }

    async function load() {
      var r = await invokeP("agent", "memorypane", "claims", { lib: lib, ctl: ctl });
      var d = r && r.status === "ok" ? r.data : null;
      if (!d || d.status !== "ok") {
        if (root) root.querySelector(".mp-sum").textContent =
          "could not read this control's memory: " + ((d && d.msg) || (r && r.msg) || "no answer");
        return;
      }
      data = d;
      chip();
      if (root) render();
    }

    function build() {
      root.innerHTML =
        '<style></style><div class="mp">' +
        '<div class="mp-head"><span class="mp-sum">reading…</span>' +
        '<button class="mp-add" aria-expanded="false">+ remember</button>' +
        '<button class="mp-promote" hidden>promote ' + lib + '</button>' +
        '<button class="mp-reload" title="read again">↻</button></div>' +
        '<form class="mp-form" hidden>' +
        '<textarea class="mp-f-claim" rows="2" placeholder="the claim — one atomic sentence" required></textarea>' +
        '<textarea class="mp-f-detail" rows="2" placeholder="detail (optional)"></textarea>' +
        '<div class="mp-line"><input class="mp-f-tags" placeholder="tags, comma-separated">' +
        '<select class="mp-f-conf"><option>medium</option><option>high</option><option>low</option></select>' +
        '<button type="submit">remember</button><span class="mp-note"></span></div></form>' +
        '<div class="mp-rows"></div></div>';
      root.querySelector("style").textContent = CSS;
      var form = root.querySelector(".mp-form");
      var addBtn = root.querySelector(".mp-add");
      var note = root.querySelector(".mp-note");
      addBtn.addEventListener("click", function () {
        form.hidden = !form.hidden;
        addBtn.setAttribute("aria-expanded", String(!form.hidden));
        if (!form.hidden) form.querySelector(".mp-f-claim").focus();
      });
      root.querySelector(".mp-reload").addEventListener("click", load);
      form.addEventListener("submit", async function (ev) {
        ev.preventDefault();
        var claim = form.querySelector(".mp-f-claim").value.trim();
        if (!claim) return;
        var entry = { claim: claim, confidence: form.querySelector(".mp-f-conf").value };
        var detail = form.querySelector(".mp-f-detail").value.trim();
        var tags = form.querySelector(".mp-f-tags").value.trim();
        if (detail) entry.detail = detail;
        if (tags) entry.tags = tags;
        note.textContent = "remembering…";
        var r = await invokeP("agent", "archivist", "remember",
          { lib: lib, domain: ctl, entry: entry, author: "" });
        var d = r && r.data && typeof r.data === "object" ? r.data : r;
        if (!r || r.status !== "ok" || (d && d.status === "err")) {
          note.textContent = "not remembered: " + ((d && d.msg) || (r && r.msg) || "no answer");
          return;
        }
        note.textContent = d && d.routed ? "remembered (" + d.routed + ")" : "remembered";
        form.querySelector(".mp-f-claim").value = "";
        form.querySelector(".mp-f-detail").value = "";
        await load();
      });
      root.querySelector(".mp-promote").addEventListener("click", async function (ev) {
        var btn = ev.currentTarget;
        if (!confirm("Promote folds this instance's overlay for EVERY control of " + lib +
            " into the shipped memory facets (tracked files change). Continue?")) return;
        btn.disabled = true;
        var r = await invokeP("agent", "archivist", "promote", { lib: lib });
        btn.disabled = false;
        var d = r && r.data && typeof r.data === "object" ? r.data : r;
        root.querySelector(".mp-sum").textContent = (!r || r.status !== "ok")
          ? "promote failed: " + ((r && r.msg) || "no answer")
          : "promoted — " + JSON.stringify(d);
        setTimeout(load, 1500);
      });
    }

    function render() {
      var local = data.claims.filter(function (c) { return c.origin !== "shipped"; }).length;
      root.querySelector(".mp-sum").textContent = data.count === 0
        ? lib + "." + ctl + " has no memory yet"
        : data.count + " claim" + (data.count === 1 ? "" : "s") + " · " + data.shipped + " shipped · " +
          local + " from this instance's overlay" + (data.stale ? " · " + data.stale + " stale" : "");
      root.querySelector(".mp-promote").hidden = local === 0;
      var rows = root.querySelector(".mp-rows");
      rows.replaceChildren();
      if (!data.claims.length) {
        var none = document.createElement("p");
        none.className = "mp-empty";
        none.textContent = "Nothing has been learned about this control. Claims arrive from the archivist, or from + remember.";
        rows.appendChild(none);
        return;
      }
      data.claims.slice().sort(function (a, b) { return (b.time || 0) - (a.time || 0); }).forEach(function (c) {
        var row = document.createElement("div");
        row.className = "mp-row" + (c.superseded ? " sup" : "");
        var claim = document.createElement("div");
        claim.className = "mp-claim";
        claim.textContent = c.claim;
        row.appendChild(claim);
        if (c.detail) {
          var det = document.createElement("div");
          det.className = "mp-detail clip";
          det.textContent = c.detail;
          det.title = "click to expand";
          det.addEventListener("click", function () { det.classList.toggle("clip"); });
          row.appendChild(det);
        }
        var meta = document.createElement("div");
        meta.className = "mp-meta";
        if (c.stale) meta.appendChild(tag("stale", "stale", "the source this claim points at has changed since it was learned"));
        if (c.superseded) meta.appendChild(tag("superseded"));
        if (c.origin === "overlay") meta.appendChild(tag("overlay", "local", "only in this instance's overlay — not shipped until promoted"));
        if (c.origin === "both") meta.appendChild(tag("overlay edit", "local", "an overlay line supersedes the shipped entry"));
        if (c.confidence) meta.appendChild(tag(c.confidence, "conf", "confidence"));
        if (c.age_days >= 0) meta.appendChild(tag(c.age_days === 0 ? "today" : c.age_days + "d", "", new Date(c.time).toString()));
        if (c.author) meta.appendChild(tag(c.author, "", "author"));
        String(c.tags || "").split(",").forEach(function (t) {
          t = t.trim();
          if (t) meta.appendChild(tag(t));
        });
        row.appendChild(meta);
        rows.appendChild(row);
      });
    }

    load();   // the chip's count, before the tab is ever opened
  }

  if (wb.api && wb.api.facetTabs) go(wb.api.facetTabs);
  else wb.addEventListener("nb-facet-tabs", function (ev) { go(ev.detail); }, { once: true });
};
