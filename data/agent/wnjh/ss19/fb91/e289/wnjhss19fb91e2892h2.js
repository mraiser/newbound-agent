// chat — the Agent app's UI (docs/agent-app.md). A standalone venue for
// the same machinery the notebook's ask row drives: agentloop's chatTurn
// over chat_llm, the full tool model (auto-run reads, typed confirm for
// mutating calls, meta-tools), the store-resident curriculum core, and
// the memory fence. Plus the surfaces the IDE doesn't have: the MEMORY
// INSPECTOR (domains, entries, stale checks, the owner's audit — bless /
// edit / forget — and remember-by-form), the PROMPTS editors (core /
// addendum / archivist — all journaled facet patches), and the ARCHIVIST
// strip (queue depth + sweep now).

var me = this;
var ME = document.getElementById(me.UUID);

var readyP = new Promise(function (res) { me.ready = res; }).then(async () => {
  // zero-globals doctrine: the libraries ride this control's own child
  // mounts — descendants, so ready strictly before this runs (el.api IS
  // each child's constructor object). The viewctx mount carries the
  // page's .nb-viewctx registry class.
  const viewctx = ME.querySelector(".nb-viewctx").api;
  const agent = ME.querySelector('[data-control="agent:agentloop"]').api;
  // the scratchpad (agent.stage) — optional: a page without it still chats
  const stageCtl = ME.querySelector('[data-control="agent:stage"]')?.api ?? null;
  // The owner addendum is instance state — a DATA RECORD in the instance-local
  // `runtime` library (id "agentchataddendum", data.addendum = plain text), NOT
  // a control (runtime holds instance data/settings, no controls). The library
  // may not exist on a fresh install and the addendum is optional, so a missing
  // library/record simply yields "" — skip it, never error. Fetched lazily and
  // cached like agentloop's corePrompt; a failed read clears the cache so the
  // next ask retries. Written via app.app.write (the prompts tab creates the
  // record on first save).
  let addendumPromise = null;
  const ownerAddendum = () => {
    if (!addendumPromise) {
      addendumPromise = (async () => {
        try {
          const r = await invokeP("app", "app", "read", { lib: "runtime", id: "agentchataddendum", nn_sessionid: null });
          if (r?.status === "ok" && r.data && typeof r.data.addendum === "string") {
            return r.data.addendum.trim();
          }
          return ""; // missing library/record/field — no addendum configured
        } catch (e) {
          addendumPromise = null; // transient failure — retry next ask
          return "";
        }
      })();
    }
    return addendumPromise;
  };
  const jsonP = (c2, v2) => new Promise((res2) => json(c2, v2, res2));
  const invokeP = (l2, c2, m2, a2) => new Promise((res2) => invokeCommand(l2, c2, m2, a2, res2));
  const invoke = async (l2, c2, m2, a2) => {
    const t0 = performance.now();
    const envelope = await invokeP(l2, c2, m2, a2);
    return { envelope, ms: Math.round(performance.now() - t0) };
  };
  const code = (m2, a2) => invokeP("dev", "code", m2, a2);
  const readRec = async (l2, id2) => {
    const r2 = await jsonP("../app/read", "lib=" + encodeURIComponent(l2) + "&id=" + encodeURIComponent(id2));
    return r2.status === "ok" ? r2.data : new Error(r2.msg || "read failed");
  };
  const controlsOf = async (l2) => {
    const d2 = await readRec(l2, "controls");
    return d2 instanceof Error ? d2 : (d2.list ?? []);
  };
  const readFacet = (l2, c2, f2) => code("read_control_facet", { lib: l2, ctl: c2, facet: f2 });
  const patchFacet = (l2, c2, f2, { oldSnippet, newSnippet, base = "", label = "" }) =>
    code("patch_control_facet", { lib: l2, ctl: c2, facet: f2, old_snippet: oldSnippet,
      new_snippet: newSnippet, base, label, author: "" });
  const listPatches = (l2, c2, n2) => code("list_control_patches", { lib: l2, ctl: c2, limit: n2 ?? 0 });
  const userP = async () => {
    const r2 = await jsonP("../security/current_user", null);
    return r2.status === "ok" ? (r2.data ?? null) : null;
  };

const CHAT_SHELL = `Right now you are speaking through the Newbound Agent app — a standalone chat. There is no IDE around you; every tool call you make renders as a visible cell in this thread, and mutating commands stop for the user's typed confirmation.

THIS APP'S OWN UI
- This chat interface IS a Newbound control: lib "agent", control "chat" (html/css/js facets); the page boot is "agent.agent". When the user asks you to change this app's UI, go straight there: dev-code-read_control_facet on the right agent.chat facet, then ONE minimal exact-match dev-code-patch_control_facet (base = the hash you just read). No searching needed; a reload shows the change.

WHAT YOU SEE
- The context may include a memory:index fence listing your long-term memory domains ("N stale?" = hash-stamped sources changed since learning — re-read those referents before trusting the marked entries).

MEMORY IN THIS CHAT
- Memory FORMATION is automatic: the archivist reviews completed sessions in the background. Do not file memories on your own initiative. When the user explicitly asks you to remember something, call dev-code-remember — it runs without confirmation, because the request itself is the authorization.

YOUR BROWSER: THE SCRATCHPAD
- You run IN the user's browser — this chat page — so you DO have a browser. Never say you could not test UI end-to-end for lack of one. The scratchpad beside the thread is where you prototype, show, and verify controls live while the user watches (and can click along).
- stage({name, spec}) mounts a control as pad.<name>; spec is lib:ctl or lib:ctl:{json-props}. Staging an existing name remounts it with the control's CURRENT code — do that after every facet edit. stage({name, remove:true}) takes it down. The reply is the mount's rendered structure plus any page errors.
- eval({js}) runs page JS with pad.<name>.api / .el / .spec in scope: click, type, read state, call the api. A lone expression returns its value; multiple statements need an explicit return. async/await and dynamic import() are fine; no import/export statements; never a literal <\/script>. Every reply lists the page errors since your last look.
- snapshot({name}) shows you the mount: its rendered structure as text and, by default, a PNG you can see (the user sees the thumbnail). Use it to check layout and looks, not just state.
- The loop for UI work: author with the dev-code commands (dev-code-create_control — the scratch lib for prototypes — and dev-code-patch_control_facet) → stage → eval to exercise it → snapshot to look → fix → stage again. Report what you verified and how.
- Mounts live in this page for this session: they persist across turns, not across a reload or a session switch (the thread then offers re-mount buttons). The first pad call asks the user to type "pad" once; after that the pad tools run freely for the session.

HOW TO ANSWER
- Be concise and concrete. If you are unsure, say so plainly rather than inventing platform behavior.`;

const STORE_KEY = "agent.chat.v1";

// The scratchpad tools. They run in this page (execTool → agent.stage), so
// they are not catalog commands and carry no lib-ctl-cmd names.
const PAD_TOOL_DEFS = [
  { type: "function", function: { name: "stage",
    description: "Mount a live Newbound control in the scratchpad beside this chat, as pad.<name> — prototype, demonstrate, or test UI in the user's own browser. Staging an existing name remounts it with the control's current code (do it after every facet edit). Returns the rendered structure plus any page errors. remove:true takes the mount down.",
    parameters: { type: "object",
      properties: {
        name: { type: "string", description: "identifier for the mount, e.g. clock — eval sees it as pad.clock" },
        spec: { type: "string", description: "lib:ctl or lib:ctl:{json-props}, e.g. scratch:clock:{\"tz\":\"UTC\"}" },
        remove: { type: "boolean", description: "true to remove the named mount instead of mounting" } },
      required: ["name"] } } },
  { type: "function", function: { name: "eval",
    description: "Run JavaScript in the chat page against the scratchpad's mounts — click, type, read state, call apis. pad.<name>.api is the control's api object, pad.<name>.el its element, pad.<name>.spec its spec. A lone expression returns its value; multiple statements need an explicit return. async/await OK; no import/export statements. Returns the JSON value plus the page errors since your last look.",
    parameters: { type: "object",
      properties: {
        js: { type: "string", description: "page JS, e.g. pad.clock.el.querySelector('button').click()" },
        timeout_ms: { type: "number", description: "give up after this long (default 10000)" } },
      required: ["js"] } } },
  { type: "function", function: { name: "snapshot",
    description: "Look at a scratchpad mount: its rendered structure as text (tags, classes, sizes, positions, visible text, form values) and, unless image is false, a PNG of it that you can see. Use it to verify layout and appearance after a change.",
    parameters: { type: "object",
      properties: {
        name: { type: "string", description: "the mount to capture" },
        image: { type: "boolean", description: "false for the text outline only (default true)" } },
      required: ["name"] } } },
];

async function init(host) {
  // ── tabs ─────────────────────────────────────────────────────────────
  for (const tab of host.querySelectorAll(".ag-tab")) {
    tab.addEventListener("click", () => {
      for (const t of host.querySelectorAll(".ag-tab")) t.classList.toggle("on", t === tab);
      for (const p of host.querySelectorAll(".ag-pane")) p.hidden = p.dataset.pane !== tab.dataset.tab;
      if (tab.dataset.tab === "memory") loadDomains();
      if (tab.dataset.tab === "prompts") loadPrompts();
      if (tab.dataset.tab === "mind") loadMind();
    });
  }

  // ── the archivist strip ──────────────────────────────────────────────
  const archQ = host.querySelector(".ag-arch-q");
  async function refreshArchivist() {
    const r = await invoke("agent", "archivist", "queue_status", {});
    if (r instanceof Error || r.envelope.status !== "ok") { archQ.textContent = ""; return; }
    const n = r.envelope.queued ?? 0;
    archQ.textContent = `archivist: ${n} turn${n === 1 ? "" : "s"} queued`;
  }
  host.querySelector(".ag-sweep").addEventListener("click", async (ev) => {
    const btn = ev.currentTarget;
    btn.disabled = true;
    btn.textContent = "sweeping…";
    const r = await invoke("agent", "archivist", "consolidate", {});
    btn.disabled = false;
    btn.textContent = "sweep now";
    const env = r instanceof Error ? { status: "err", msg: r.message } : r.envelope;
    note(host.querySelector(".ag-arch-note"), env.status === "ok"
      ? `swept ${env.swept ?? 0} · filed ${env.filed ?? 0} · skipped ${env.skipped ?? 0}`
      : `sweep failed: ${env.msg ?? "?"}`, env.status !== "ok");
    await refreshArchivist();
    if (!host.querySelector('[data-pane="memory"]').hidden) loadDomains();
  });
  refreshArchivist();

  // ── chat: transcript + persistence ───────────────────────────────────
  const thread = host.querySelector(".ag-thread");
  let messages = [];       // the raw conversation (openai shapes), persisted
  let transcript = [];     // rendered cells {kind, title?, text, error?}
  let padGranted = false;  // the user typed "pad" for this session

  // ── the scratchpad pane: opens when the agent stages, or by hand ──────
  const padEl = host.querySelector(".ag-pad");
  const padToggle = host.querySelector(".ag-padtoggle");
  function showPad(open) {
    padEl.hidden = !open;
    padToggle.classList.toggle("on", open);
  }
  padToggle.addEventListener("click", () => showPad(padEl.hidden));
  host.querySelector(".ag-pad-close").addEventListener("click", () => showPad(false));
  // the pad belongs to the conversation that staged it: a session switch
  // clears it (the thread's notes offer re-mounts) and asks for the grant again
  function resetPad() {
    padGranted = false;
    stageCtl?.unmountAll?.();
  }

  // ── sessions: many conversations, each a store record tied to the user ──
  // Records live in the runtime lib (admin-only by default) and are written
  // through the platform's gated app.write; the sidebar list + delete ride
  // agent.chat.session_index (the per-user index), and openSession rides the
  // owner-checking agent.chat.session_open. Ids are minted server-side with
  // leading entropy, so the store's filename sharding is preserved.
  const userName = (u) => (u && (u.username || u.id || u.displayname)) || "anonymous";
  function sessTitle(msgs) {
    const first = (msgs ?? []).find((m) =>
      m.role === "user" && !String(m.content ?? "").startsWith("[CONTEXT]"));
    const t = String(first?.content ?? "").replace(/\s+/g, " ").trim();
    return t ? t.slice(0, 48) : "untitled session";
  }
  let sessId = null;        // minted server-side on the first save
  let customTitle = null;   // the user's rename, if any — always wins over sessTitle
  const persist = async () => {
    if (!messages.length) return;   // empty sessions are never saved
    const me2 = (await userP()) ?? {};
    // Any-typed params are NOT optional over HTTP: a null id reaches app.write
    // as the literal string "null" (cast_params coerces it), collapsing every
    // session onto one record. Mint a real server-side id up front instead.
    if (!sessId) {
      const idr = await jsonP("../app/unique_session_id", null);
      const newId = idr && idr.status === "ok" ? (idr.data ?? idr.msg) : null;
      if (newId) sessId = String(newId);
    }
    const res = await fetch("../app/write", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        lib: "runtime", id: sessId, readers: [], writers: [],
        data: {
          username: userName(me2), title: customTitle || sessTitle(messages),
          ...(customTitle ? { custom_title: customTitle } : {}),
          time: Date.now(),
          messages, transcript } }),
    });
    const w = await res.json();
    if (w && w.status === "ok" && w.id) {
      sessId = w.id;
      // keep the sidebar (index) in step with the store (truth)
      await invokeP("agent", "chat", "session_touch", { id: sessId });
    }
    await renderSessions();
  };
  async function openSession(id) {
    const r = await invokeP("agent", "chat", "session_open", { id });
    const rec = (r.status === "ok" && r.data && r.data.data) ? r.data.data : null;
    sessId = id;
    resetPad();
    messages = (rec && Array.isArray(rec.messages)) ? rec.messages : [];
    transcript = (rec && Array.isArray(rec.transcript)) ? rec.transcript : [];
    customTitle = (rec && typeof rec.custom_title === "string" && rec.custom_title.trim()) ? rec.custom_title : null;
    thread.replaceChildren();
    for (const entry of transcript) thread.appendChild(cellEl(entry));
    thread.scrollTop = thread.scrollHeight;
    await renderSessions();
  }
  function newSession() {
    sessId = null;
    customTitle = null;
    resetPad();
    messages = [];
    transcript = [];
    thread.replaceChildren();
    renderSessions();
  }
  const sessListEl = host.querySelector(".ag-sess-list");
  // ── session drag-and-drop reorder ── the dragged id is shared between the
  // per-row handlers (wired in renderSessions) and these container handlers,
  // which are attached ONCE so re-renders never stack listeners.
  let dragSid = null;
  const clearDropMarks = () => {
    for (const el of sessListEl.querySelectorAll(".ag-drop-above")) el.classList.remove("ag-drop-above");
  };
  const sessOrderIds = () => Array.from(sessListEl.querySelectorAll(".ag-sess")).map((el) => el.dataset.sid);
  async function applySessOrder(ids) {
    await invokeP("agent", "chat", "session_index", { op: "reorder", id: "", arg: JSON.stringify(ids) });
    await renderSessions();
  }
  // dropping in the empty space below the rows moves the session to the END
  sessListEl.addEventListener("dragover", (ev) => {
    if (!dragSid) return;
    ev.preventDefault();
    ev.dataTransfer.dropEffect = "move";
  });
  sessListEl.addEventListener("drop", async (ev) => {
    if (!dragSid) return;
    ev.preventDefault();
    if (ev.target.closest && ev.target.closest(".ag-sess")) return; // row drops are handled on the row
    const ids = sessOrderIds().filter((i) => i !== dragSid);
    ids.push(dragSid);
    clearDropMarks();
    await applySessOrder(ids);
  });
  async function renderSessions() {
    const r = await invokeP("agent", "chat", "session_index", { op: "list", id: "", arg: "" });
    const ix = (r.status === "ok" && r.data && r.data.list) ? r.data.list : [];
    sessListEl.replaceChildren();
    for (const s of ix) {
      const row = document.createElement("div");
      row.className = "ag-sess" + (s.id === sessId ? " on" : "");
      row.dataset.sid = s.id;
      row.draggable = true;
      row.addEventListener("dragstart", (ev) => {
        if (row.querySelector(".ag-sess-edit")) { ev.preventDefault(); return; } // renaming — don't drag
        dragSid = s.id;
        row.classList.add("dragging");
        ev.dataTransfer.setData("text/plain", s.id);
        ev.dataTransfer.effectAllowed = "move";
      });
      row.addEventListener("dragover", (ev) => {
        if (!dragSid) return;
        ev.preventDefault();
        ev.dataTransfer.dropEffect = "move";
        clearDropMarks();
        if (s.id !== dragSid) row.classList.add("ag-drop-above"); // insert-above indicator
      });
      row.addEventListener("dragleave", () => row.classList.remove("ag-drop-above"));
      row.addEventListener("drop", async (ev) => {
        ev.preventDefault();
        ev.stopPropagation();
        clearDropMarks();
        if (!dragSid || dragSid === s.id) return; // drop on itself = no-op
        const ids = sessOrderIds().filter((i) => i !== dragSid);
        ids.splice(ids.indexOf(s.id), 0, dragSid); // insert ABOVE the target row
        await applySessOrder(ids);
      });
      row.addEventListener("dragend", () => {
        dragSid = null;
        row.classList.remove("dragging");
        clearDropMarks();
      });
      const open = document.createElement("button");
      open.className = "ag-sess-open";
      open.textContent = s.title;
      open.title = new Date(s.time).toLocaleString();
      open.addEventListener("click", () => { if (s.id !== sessId) openSession(s.id); });
      const del = document.createElement("button");
      del.className = "ag-sess-del";
      del.textContent = "✕";
      del.title = "forget this session (removes it from the list)";
      del.addEventListener("click", async () => {
        if (del.textContent !== "sure?") {
          del.textContent = "sure?";
          setTimeout(() => { del.textContent = "✕"; }, 2500);
          return;
        }
        await invokeP("agent", "chat", "session_index", { op: "delete", id: s.id, arg: "" });
        if (s.id === sessId) newSession(); else await renderSessions();
      });
      const ren = document.createElement("button");
      ren.className = "ag-sess-ren";
      ren.textContent = "✎";
      ren.title = "rename this session";
      ren.addEventListener("click", () => {
        const inp = document.createElement("input");
        inp.className = "ag-sess-edit";
        inp.value = s.title;
        inp.maxLength = 120;
        let done = false;
        const finish = async (commit) => {
          if (done) return;
          done = true;
          if (commit) {
            const t = inp.value.replace(/\s+/g, " ").trim();
            if (t && t !== s.title) {
              // open session: stamp the title locally BEFORE the server call —
              // a persist() racing in from a reply in progress must already
              // carry it, or a reload loses the rename. Roll back on failure;
              // on success persist() rewrites the record with custom_title.
              const mine = s.id === sessId;
              const prev = customTitle;
              if (mine) customTitle = t;
              const rr = await invokeP("agent", "chat", "session_index", { op: "rename", id: s.id, arg: t });
              if (rr && rr.status === "ok") { if (mine) persist(); }
              else if (mine) customTitle = prev;
            }
          }
          await renderSessions();
        };
        inp.addEventListener("keydown", (e) => {
          if (e.key === "Enter") finish(true);
          else if (e.key === "Escape") finish(false);
        });
        inp.addEventListener("blur", () => finish(false));
        row.replaceChildren(inp);
        inp.focus();
        inp.select();
      });
      row.append(open, ren, del);
      sessListEl.appendChild(row);
    }
  }

  // one clipboard write for every copy affordance: navigator.clipboard only
  // exists on https/localhost, so on any throw fall back to the hidden
  // textarea — covers plain http from another machine on the LAN.
  async function copyText(txt) {
    try { await navigator.clipboard.writeText(txt); return true; } catch (e) {}
    let ok = false;
    const ta = document.createElement("textarea");
    ta.value = txt;
    ta.style.position = "fixed";
    ta.style.opacity = "0";
    document.body.appendChild(ta);
    ta.select();
    try { ok = document.execCommand("copy"); } catch (e) {}
    ta.remove();
    return ok;
  }

  function cellEl(entry) {
    const div = document.createElement("div");
    div.className = "ag-cell ag-" + entry.kind + (entry.error ? " err" : "");
    if (entry.title) {
      const h = document.createElement("div");
      h.className = "ag-cell-title";
      // the call is rendered separately from the result so it can be
      // visually clamped (not hard-cut): full args in the DOM, a more/less
      // toggle on genuine overflow. Legacy cells have no .args — their
      // title already holds the clamped call.
      h.textContent = entry.args != null
        ? (entry.title + " " + entry.args)
        : entry.title;
      div.appendChild(h);
      if (entry.args != null) clampVisually(div, h);
    }
    const body = document.createElement("div");
    body.className = "ag-cell-body";
    renderRich(body, entry.text ?? "", entry.kind === "agent");
    div.appendChild(body);
     // quiet meta row: timestamp (only when the entry has one — never invented)
    // + copy on user and agent messages. Tool/stage/snap/busy cells get nothing.
    const wantCopy = entry.kind === "agent" || entry.kind === "user";
    const wantTime = (entry.kind === "user" || entry.kind === "agent") && entry.time;
    if (wantCopy || wantTime) {
      const meta = document.createElement("div");
      meta.className = "ag-cell-meta";
      if (wantTime) {
        const d = new Date(entry.time);
        const ts = document.createElement("span");
        ts.className = "ag-cell-time";
        const t = d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
        ts.textContent = d.toDateString() === new Date().toDateString()
          ? t
          : d.toLocaleDateString([], { year: "numeric", month: "short", day: "numeric" }) + " " + t;
        ts.title = d.toLocaleString();
        meta.appendChild(ts);
      }
      if (wantCopy) {
        const cp = document.createElement("button");
        cp.type = "button";
        cp.className = "ag-cell-copy";
        cp.textContent = "copy";
        cp.title = entry.kind === "user" ? "copy this message"
          : "copy this reply's Markdown";
        cp.onclick = async () => {
          // user cells: entry.typed is what was actually typed, without the
          // [ATTACHED FILES] block; older entries fall back to the full text.
          const ok = await copyText(entry.typed ?? entry.text ?? "");
          cp.textContent = ok ? "✓ copied" : "copy failed";
          setTimeout(() => { cp.textContent = "copy"; }, 1500);
        };
        meta.appendChild(cp);
      }
      div.appendChild(meta);
    }
    // per-code-block copy: each fenced block in a user/agent cell gets its
    // own hover button (top-right). The button lives in a wrapper, never
    // inside the pre, so pre.textContent stays exactly the code. Tool cells
    // are excluded — their clipped-expansion path re-renders its own body.
    if (entry.kind === "user" || entry.kind === "agent") {
      body.querySelectorAll("pre").forEach((pre) => {
        const wrap = document.createElement("div");
        wrap.className = "ag-code-wrap";
        pre.replaceWith(wrap);
        wrap.appendChild(pre);
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = "ag-code-copy";
        btn.textContent = "copy";
        btn.title = "copy this code block";
        btn.onclick = async () => {
          const ok = await copyText(pre.textContent);
          btn.textContent = ok ? "✓" : "✗";
          setTimeout(() => { btn.textContent = "copy"; }, 1500);
        };
        wrap.appendChild(btn);
      });
    }
    if (entry.kind === "stage") {
      // a mount note. Replay never re-runs a control's code on its own: once
      // the live instance is gone (reload, session switch) the note offers it.
      if (entry.name && entry.spec && !entry.removed && stageCtl?.has && !stageCtl.has(entry.name)) {
        const re = document.createElement("button");
        re.type = "button";
        re.className = "ag-stage-remount";
        re.textContent = "↻ re-mount " + entry.name;
        re.title = "mount " + entry.spec + " in the scratchpad again (runs its code)";
        re.addEventListener("click", async () => {
          re.disabled = true;
          try {
            showPad(true);
            await stageCtl.mount(entry.name, entry.spec);
            re.remove();
          } catch (e) {
            re.disabled = false;
            re.textContent = "re-mount failed: " + (e.message || e);
          }
        });
        body.appendChild(re);
      }
      return div;
    }
    if (entry.kind === "snap") {
      if (entry.thumb) {
        const img = document.createElement("img");
        img.className = "ag-snap-img";
        img.src = entry.thumb;
        img.alt = "snapshot of pad." + (entry.name ?? "");
        body.appendChild(img);
      }
      return div;
    }
    if (entry.kind === "tool") {
      // clipped = the flag, OR a legacy cell whose truncation is baked into
      // its text. The visible fragment is an exact prefix of what chat_llm
      // captured (LLM_CAPTURE=on), so the store can return the rest.
      const isCut = () => entry.clipped || /…\[\d+ chars clipped\]\s*$/.test(entry.text ?? "");
      if (isCut()) {
        const more = document.createElement("button");
        more.type = "button";
        more.className = "ag-expand";
        more.textContent = "▸ expand full output (from capture)";
        more.onclick = async () => {
          more.disabled = true;
          more.textContent = "expanding…";
          // recompute the fragment from CURRENT text — it changes each expand
          const clip = entry.text.replace(/\n?…\[\d+ chars clipped\]\s*$/, "");
          let full = entry.full;
          if (!full) {
            const r = await invoke("agent", "msg", "expand_clipped", { fragment: clip });
            const env = r.envelope ?? {};
            const pay = env.data ?? env;
            full = (env.status === "ok" && pay && pay.full) ? pay.full : null;
            if (!full) {
              more.disabled = false;
              more.textContent = "no captured full text (predates capture?)";
              return;
            }
          }
          entry.full = null;                 // consumed — don't reuse a stale cache
          entry.text = full;
          entry.clipped = false;
          body.textContent = "";
          renderRich(body, full);
          persist();
          if (/…\[\d+ chars clipped\]\s*$/.test(full)) {
            // the capture itself was cut by the model-facing 4000-char clamp,
            // so this is all the store has. Show it and stop — don't loop.
            more.textContent = "fully expanded (model saw only the first 4000 chars)";
            clampVisually(div, body);
          } else {
            more.remove();
            clampVisually(div, body);        // a plain visual clamp now
          }
        };
        div.appendChild(more);
      } else {
        // full text present — clamp VISUALLY like the dev session; no data lost
        clampVisually(div, body);
      }
    }
    return div;
  }

  // The dev session's clampOutput, adapted: defer until connected so
  // scrollHeight is real, add the toggle only on genuine overflow, and
  // insert it immediately after `el` so several clamps in one cell
  // (call-title + body) keep their toggles in the right place.
  function clampVisually(div, el) {
    el.classList.add("clamped");
    requestAnimationFrame(() => {
      if (!el.isConnected) return;
      if (el.scrollHeight <= el.clientHeight + 1) {
        el.classList.remove("clamped");   // it fits — no clamp, no toggle
        return;
      }
      if (el.nextElementSibling && el.nextElementSibling.classList.contains("ag-more")) return;
      const toggle = document.createElement("button");
      toggle.type = "button";
      toggle.className = "ag-more";
      const label = () => {
        const cl = el.classList.contains("clamped");
        toggle.textContent = cl ? "more ▾" : "less ▴";
        toggle.setAttribute("aria-expanded", String(!cl));
      };
      toggle.addEventListener("click", () => {
        el.classList.toggle("clamped");
        label();
        if (el.classList.contains("clamped")) div.scrollIntoView({ block: "nearest" });
      });
      label();
      el.after(toggle);
    });
  }

  /** think-folds + fenced code; agent prose (md=true) gets light markdown,
   *  everything else stays text. */
  function renderRich(el, raw, md) {
    let text = String(raw);
    const think = text.match(/<think>([\s\S]*?)<\/think>/);
    if (think) {
      const det = document.createElement("details");
      det.className = "ag-think";
      const sum = document.createElement("summary");
      sum.textContent = "thinking…";
      det.append(sum, document.createTextNode(think[1].trim()));
      el.appendChild(det);
      text = text.replace(think[0], "").trim();
    }
    const parts = text.split(/```([\s\S]*?)```/);
    parts.forEach((part, i) => {
      if (!part.trim()) return;
      if (i % 2 === 1) {
        const pre = document.createElement("pre");
        pre.textContent = part.replace(/^[a-z:.\-\d]*\n/i, "");
        el.appendChild(pre);
      } else if (md) {
        const html = mdBlocks(part);
        if (!html) return;
        const d = document.createElement("div");
        d.className = "ag-md";
        d.innerHTML = html;
        el.appendChild(d);
      } else {
        const p = document.createElement("div");
        p.textContent = part.trim();
        el.appendChild(p);
      }
    });
  }

  // ── light markdown (agent prose only) ────────────────────────────────
  // ESCAPE FIRST, then a small safe subset: headings, lists (nested by
  // indent; ordered lists keep their start so a fence mid-list doesn't
  // renumber), quotes, rules, GFM tables, paragraphs; inline code, bold,
  // italic, strike, links (http(s) / relative / #anchor only). Raw HTML
  // from the model never reaches the DOM.
  const mdEsc = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;")
    .replace(/>/g, "&gt;").replace(/"/g, "&quot;");
  const mdEmph = (t) => t
    .replace(/\*\*(\S(?:[^*\n]*?\S)?)\*\*/g, "<strong>$1</strong>")
    .replace(/(^|\W)__(\S(?:[^_\n]*?\S)?)__(?!\w)/g, "$1<strong>$2</strong>")
    .replace(/~~(\S(?:[^~\n]*?\S)?)~~/g, "<del>$1</del>")
    .replace(/(^|[^*\w])\*(\S(?:[^*\n]*?\S)?)\*(?![*\w])/g, "$1<em>$2</em>")
    .replace(/(^|\W)_(\S(?:[^_\n]*?\S)?)_(?!\w)/g, "$1<em>$2</em>");
  function mdInline(s) {
    const stash = []; // code spans + links are held out of the emphasis pass
    const hold = (h) => "\u0001" + (stash.push(h) - 1) + "\u0001";
    let t = mdEsc(s).replace(/`([^`\n]+)`/g, (_, c) => hold("<code>" + c + "</code>"));
    t = t.replace(/\[([^\]\n]+)\]\(([^)\s]+)\)/g, (m, txt, url) =>
      /^(https?:\/\/|\/|\.\.?\/|#)/i.test(url)
        ? hold(`<a href="${url}" target="_blank" rel="noopener">${mdEmph(txt)}</a>`) : m);
    t = mdEmph(t);
    for (let n = 0; n < 3 && t.includes("\u0001"); n++)
      t = t.replace(/\u0001(\d+)\u0001/g, (_, k) => stash[+k]);
    return t;
  }
  function mdBlocks(text) {
    const lines = text.replace(/\r\n?/g, "\n").split("\n");
    const isSep = (l) => l.includes("|") && /^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$/.test(l);
    const cells = (l) => l.trim().replace(/^\||\|$/g, "").split("|").map((c) => c.trim());
    let html = "", para = [], quote = [], gap = false;
    const lists = []; // open lists, innermost last: {tag, indent}
    const flushPara = () => { if (para.length) html += "<p>" + para.map(mdInline).join("<br>") + "</p>"; para = []; };
    const flushQuote = () => { if (quote.length) html += "<blockquote>" + quote.map(mdInline).join("<br>") + "</blockquote>"; quote = []; };
    const closeLists = () => { while (lists.length) html += "</li></" + lists.pop().tag + ">"; };
    const flush = () => { flushPara(); flushQuote(); closeLists(); };
    for (let i = 0; i < lines.length; i++) {
      const ln = lines[i];
      let m;
      if (!ln.trim()) { flushPara(); flushQuote(); gap = true; continue; }
      if (/^\s{0,3}([-*_])(\s*\1){2,}\s*$/.test(ln)) { flush(); html += "<hr>"; gap = false; continue; }
      if ((m = ln.match(/^\s{0,3}(#{1,6})\s+(.*?)(?:\s+#+)?\s*$/))) {
        flush(); const n = m[1].length; html += `<h${n}>${mdInline(m[2])}</h${n}>`; gap = false; continue;
      }
      if (ln.includes("|") && isSep(lines[i + 1] ?? "")) {
        flush();
        const al = cells(lines[i + 1]).map((c) => c.endsWith(":") ? (c.startsWith(":") ? "center" : "right") : "");
        const row = (l, tg) => "<tr>" + cells(l).map((c, k) =>
          `<${tg}${al[k] ? ` style="text-align:${al[k]}"` : ""}>${mdInline(c)}</${tg}>`).join("") + "</tr>";
        let t = "<div class=\"ag-md-table\"><table><thead>" + row(ln, "th") + "</thead><tbody>";
        for (i += 2; i < lines.length && lines[i].includes("|") && lines[i].trim(); i++) t += row(lines[i], "td");
        i--; html += t + "</tbody></table></div>"; gap = false; continue;
      }
      if ((m = ln.match(/^\s{0,3}>\s?(.*)$/))) { flushPara(); closeLists(); quote.push(m[1]); gap = false; continue; }
      if ((m = ln.match(/^(\s*)([-*+]|(\d{1,9})[.)])\s+(.*)$/))) {
        flushPara(); flushQuote();
        const indent = m[1].replace(/\t/g, "    ").length, tag = m[3] ? "ol" : "ul";
        const start = m[3] && +m[3] !== 1 ? ` start="${+m[3]}"` : "";
        while (lists.length && lists[lists.length - 1].indent > indent + 1) html += "</li></" + lists.pop().tag + ">";
        const top = lists[lists.length - 1];
        if (!top || indent >= top.indent + 2) { html += `<${tag}${start}><li>`; lists.push({ tag, indent }); }
        else if (top.tag !== tag) { html += `</li></${lists.pop().tag}><${tag}${start}><li>`; lists.push({ tag, indent }); }
        else html += "</li><li>";
        html += mdInline(m[4]); gap = false; continue;
      }
      if (lists.length && /^\s/.test(ln)) { html += (gap ? "<br><br>" : "<br>") + mdInline(ln.trim()); gap = false; continue; }
      closeLists(); flushQuote();
      para.push(ln.trim()); gap = false;
    }
    flush();
    return html;
  }

  function pushCell(entry) {
    if (entry.time == null) entry.time = Date.now();
    transcript.push(entry);
    thread.appendChild(cellEl(entry));
    thread.scrollTop = thread.scrollHeight;
    persist();
    return entry;
  }
  { const r = await invokeP("agent", "chat", "session_index", { op: "list", id: "", arg: "" });
    const ix = (r.status === "ok" && r.data && r.data.list) ? r.data.list : [];
    ix.length ? await openSession(ix[0].id) : await renderSessions(); }

  // new session — the current one stays in the sidebar; nothing is lost.
  host.querySelector(".ag-newsession").addEventListener("click", newSession);
  host.querySelector(".ag-sess-new").addEventListener("click", newSession);

  // ── sessions ↔ chat divider: drag to resize, dblclick resets, width persists ──
  const sessAside = host.querySelector(".ag-sessions");
  const sdiv = host.querySelector(".ag-sdiv");
  const SESSW_KEY = "agent.chat.sessw";
  // blocked storage throws on any touch — a throw during init would kill the
  // rest of the chat setup, so every localStorage access goes through these.
  const lsGet = (k) => { try { return localStorage.getItem(k); } catch (e) { return null; } };
  const lsSet = (k, v) => { try { localStorage.setItem(k, v); } catch (e) {} };
  const lsDel = (k) => { try { localStorage.removeItem(k); } catch (e) {} };
  // restore through the same limits the drag enforces (min 140px, max half
  // the window) — a width saved on a big monitor must fit a laptop.
  const clampSessW = () => {
    const savedW = parseInt(lsGet(SESSW_KEY), 10);
    if (savedW) sessAside.style.width =
      Math.min(Math.max(140, savedW), Math.floor(window.innerWidth / 2)) + "px";
  };
  clampSessW();
  sdiv.addEventListener("pointerdown", (ev) => {
    ev.preventDefault();
    sdiv.setPointerCapture(ev.pointerId);
    sdiv.classList.add("drag");
    const startX = ev.clientX;
    const startW = sessAside.getBoundingClientRect().width;
    const move = (e) => {
      const w = Math.min(Math.max(140, startW + e.clientX - startX),
                         Math.floor(window.innerWidth / 2));
      sessAside.style.width = w + "px";
    };
    const up = () => {
      sdiv.classList.remove("drag");
      sdiv.removeEventListener("pointermove", move);
      sdiv.removeEventListener("pointerup", up);
      sdiv.removeEventListener("pointercancel", up);
      lsSet(SESSW_KEY,
        String(Math.round(sessAside.getBoundingClientRect().width)));
    };
    sdiv.addEventListener("pointermove", move);
    sdiv.addEventListener("pointerup", up);
    sdiv.addEventListener("pointercancel", up); // a cancelled drag must not stick
  });
  sdiv.addEventListener("dblclick", () => {
    lsDel(SESSW_KEY);
    sessAside.style.removeProperty("width");
  });

  // ── thread ↔ input divider: drag up to grow the message box, dblclick resets ──
  const askInput = host.querySelector(".ag-input");
  const adiv = host.querySelector(".ag-adiv");
  const INPUTH_KEY = "agent.chat.inputh";
  // same limits as the drag: input never under 52px, thread keeps >= 120px.
  const clampInputH = () => {
    const savedH = parseInt(lsGet(INPUTH_KEY), 10);
    if (!savedH) return;
    const room = askInput.getBoundingClientRect().height
      + thread.getBoundingClientRect().height - 120;
    if (room <= 0) return; // a hidden pane measures 0 — don't clamp on a resize there
    askInput.style.height = Math.max(52, Math.min(savedH, room)) + "px";
  };
  clampInputH();
  adiv.addEventListener("pointerdown", (ev) => {
    ev.preventDefault();
    adiv.setPointerCapture(ev.pointerId);
    adiv.classList.add("drag");
    const startY = ev.clientY;
    const startH = askInput.getBoundingClientRect().height;
    const maxH = startH + thread.getBoundingClientRect().height - 120;
    const move = (e) => {
      const h = Math.min(Math.max(52, startH + startY - e.clientY), maxH);
      askInput.style.height = h + "px";
    };
    const up = () => {
      adiv.classList.remove("drag");
      adiv.removeEventListener("pointermove", move);
      adiv.removeEventListener("pointerup", up);
      adiv.removeEventListener("pointercancel", up);
      lsSet(INPUTH_KEY,
        String(Math.round(askInput.getBoundingClientRect().height)));
    };
    adiv.addEventListener("pointermove", move);
    adiv.addEventListener("pointerup", up);
    adiv.addEventListener("pointercancel", up); // a cancelled drag must not stick
  });
  adiv.addEventListener("dblclick", () => {
    lsDel(INPUTH_KEY);
    askInput.style.removeProperty("height");
  });
  // a shrinking window re-applies both saved sizes through their limits, so
  // a tall box or wide sidebar from a big monitor never squeezes the thread.
  const onWinResize = () => { clampSessW(); clampInputH(); };
  window.addEventListener("resize", onWinResize);
  // ── the tool model (the notebook's ceremony, this venue's chrome) ────
  const confirmEl = host.querySelector(".ag-confirm");
  const askConfirmed = new Set();
  function askConfirm(cmdName, lite, msg) {
    return new Promise((done) => {
      confirmEl.hidden = false;
      confirmEl.querySelector(".ag-confirm-msg").textContent = lite
        ? `run ${cmdName} again?`
        : (msg || `the agent wants to run ${cmdName} — type the command name to allow it:`);
      const input = confirmEl.querySelector(".ag-confirm-input");
      const okBtn = confirmEl.querySelector(".ag-confirm-ok");
      input.hidden = lite;
      input.value = "";
      const finish = (v) => { confirmEl.hidden = true; done(v); };
      okBtn.hidden = !lite;
      okBtn.onclick = () => finish(true);
      input.onkeydown = (ev) => {
        if (ev.key === "Enter") finish(input.value.trim() === cmdName);
        if (ev.key === "Escape") finish(false);
      };
      confirmEl.querySelector(".ag-confirm-cancel").onclick = () => finish(false);
      if (!lite) input.focus();
    });
  }

  let mcpTools = null;
  async function ensureCatalog() {
    if (mcpTools !== null) return mcpTools;
    const r = await agent.listTools();
    mcpTools = r.status === "ok" ? (r.tools ?? []) : [];
    return mcpTools;
  }

  const TOOL_HARD_CUT = 20000; // safety only — visual clamping is CSS's job
  const toolCell = (title, args, out) => {
    const raw = out.output ?? "";
    const text = raw.length > TOOL_HARD_CUT
      ? agent.clamp(raw, TOOL_HARD_CUT) : raw;
    const argsFull = JSON.stringify(args ?? {});
    return pushCell({
      kind: "tool", error: !!out.error,
      title,
      args: argsFull,
      text,
      // full is set only when the string itself was cut (so the capture
      // store can recover it); visual overflow needs no recovery.
      full: (raw.length > TOOL_HARD_CUT) ? raw : null,
      clipped: raw.length > TOOL_HARD_CUT,
    });
  };

  // ── the scratchpad tools: in-page calls into agent.stage. One typed
  // grant ("pad") covers stage / eval / snapshot for the rest of the session.
  const PAD_TITLE = { stage: "◈ stage", eval: "◈ eval", snapshot: "◈ snapshot" };
  const errBlock = (errs) => (errs && errs.length)
    ? "\n\nPAGE ERRORS since your last look:\n" + errs.slice(-12).join("\n") : "";
  async function padTool(tool, args) {
    const title = PAD_TITLE[tool];
    if (!stageCtl?.mount) {
      toolCell(title, args, { error: true, output: "the scratchpad is not loaded on this page" });
      return "ERROR: the scratchpad (agent.stage) is not loaded on this page — ask the user to reload the chat.";
    }
    if (!padGranted && agent.gateFor("pad", null) !== "auto") {
      const ok = await askConfirm("pad", false,
        "the agent wants to use the scratchpad — mount controls and run JS in this page. Type pad to allow it for this session:");
      if (!ok) {
        toolCell(title, args, { error: true, output: "denied by the user" });
        return "DENIED: the user declined scratchpad access. Continue without it.";
      }
    }
    padGranted = true;
    try {
      if (tool === "stage") return await padStage(args);
      if (tool === "eval") return await padEval(args);
      return await padSnapshot(args);
    } catch (e) {
      const msg = (e && e.message) || String(e);
      const errs = stageCtl.errors();
      toolCell(title, args, { error: true, output: msg + errBlock(errs) });
      return "ERROR: " + msg + errBlock(errs);
    }
  }
  async function padStage(args) {
    const name = String(args.name ?? "").trim();
    if (args.remove) {
      const gone = stageCtl.unmount(name);
      pushCell({ kind: "stage", title: PAD_TITLE.stage, args: JSON.stringify(args), name, removed: true,
        text: gone ? `removed pad.${name}` : `nothing was mounted as ${name}` });
      return gone ? `Removed pad.${name}.` : `Nothing is mounted as ${name}.`;
    }
    const spec = String(args.spec ?? "").trim();
    showPad(true);
    const r = await stageCtl.mount(name, spec);
    let outline = "";
    try { outline = stageCtl.outline(name); } catch (e) { outline = "(no outline: " + e.message + ")"; }
    const errs = stageCtl.errors();
    const status = r.ready ? "mounted" : "mounted but NOT ready — " + r.error;
    pushCell({ kind: "stage", title: PAD_TITLE.stage, args: JSON.stringify(args), name, spec,
      error: !r.ready, text: `${status}: pad.${name}` + errBlock(errs) });
    return `${status}: pad.${name} (${spec}). Rendered structure:\n${outline}` + errBlock(errs);
  }
  async function padEval(args) {
    const js = String(args.js ?? "");
    const r = await stageCtl.evaluate(js, Number(args.timeout_ms) || 10000);
    const shown = r.ok ? r.value : "threw: " + r.error;
    toolCell(PAD_TITLE.eval, args, { error: !r.ok, output: shown + errBlock(r.errors) });
    const hint = (r.ok && r.value === "undefined" && /[;\n]/.test(js.trim()) && !/\breturn\b/.test(js))
      ? "\n(no value: a multi-statement script needs an explicit return)" : "";
    return (r.ok ? agent.clamp(r.value, 4000) : "ERROR: " + agent.clamp(r.error, 2000)) + hint + errBlock(r.errors);
  }
  async function padSnapshot(args) {
    const name = String(args.name ?? "").trim();
    const outline = stageCtl.outline(name);
    let images = [], thumb = null, note = "";
    if (args.image !== false) {
      try {
        const shot = await stageCtl.capture(name);
        const file = new File([shot.blob], `snapshot-${name}-${Date.now()}.png`, { type: "image/png" });
        const up = await uploadOne(file);
        images = [up.path];
        thumb = shot.thumb;
        note = `image ${shot.width}×${shot.height} attached`;
      } catch (e) {
        note = "image capture failed (" + ((e && e.message) || e) + ") — the outline is still exact";
      }
    }
    const errs = stageCtl.errors();
    pushCell({ kind: "snap", title: PAD_TITLE.snapshot, args: JSON.stringify(args), name, thumb,
      text: `pad.${name}` + (note ? ` — ${note}` : "") + errBlock(errs) });
    const content = `Snapshot of pad.${name}` + (note ? ` (${note})` : "") + `:\n${outline}` + errBlock(errs);
    // {content, images}: agentloop's chatTurn delivers the pixels on its own
    return images.length ? { content, images } : content;
  }

  async function execTool(call) {
    let args = {};
    try { args = call.arguments ? JSON.parse(call.arguments) : {}; } catch { /* raw below */ }

    if (Object.hasOwn(PAD_TITLE, call.name)) return padTool(call.name, args);
    if (call.name === "find_tools") {
      const catalog = await ensureCatalog();
      const hits = agent.searchCatalog(catalog, args.query ?? "");
      toolCell("✳ find_tools", args, { output: hits.length
        ? hits.map((h) => `${h.name} [${h.gate}] — ${h.summary}`).join("\n") : "no matches" });
      return hits.length ? JSON.stringify(hits)
        : "No commands matched. Try different keywords, or list_commands on a likely control.";
    }
    if (call.name === "describe_tool") {
      const catalog = await ensureCatalog();
      const entry = catalog.find((t) => t.name === args.name);
      toolCell("✳ describe_tool", args, {
        output: entry ? `${entry.name} [${agent.gateFor(entry.name, entry)}]` : "unknown tool",
        error: !entry });
      return entry ? JSON.stringify({ ...entry, gate: agent.gateFor(entry.name, entry) })
        : `Unknown tool "${args.name}" — use find_tools to search the catalog.`;
    }
    let name = call.name;
    if (call.name === "call_command") {
      name = args.name;
      args = (args.args && typeof args.args === "object") ? args.args : {};
      const catalog = await ensureCatalog();
      const entry = catalog.find((t) => t.name === name);
      if (!entry) {
        toolCell("✳ call_command", { name }, { error: true, output: "unknown tool" });
        return `Unknown tool "${name}" — use find_tools to search the catalog.`;
      }
      const complaints = agent.schemaComplaints(entry.inputSchema, args);
      if (complaints) {
        toolCell("✳ call_command", { name, args }, { error: true,
          output: `arguments rejected: ${complaints}` });
        return `ARGUMENTS REJECTED for ${name}: ${complaints}. Fix and call again.`;
      }
    }
    const target = agent.parseToolName(name);
    if (!target) return `ERROR: tool name "${name}" is not lib-ctl-cmd shaped`;
    const callText = `${target.lib}.${target.ctl}.${target.cmd}`;
    const catalog = await ensureCatalog();
    const entry = catalog.find((t) => t.name === name);
    if (agent.gateFor(name, entry) !== "auto") {
      const confirmed = await askConfirm(target.cmd, askConfirmed.has(target.cmd));
      if (!confirmed) {
        toolCell(callText, args, { error: true, output: "denied by the user" });
        return "DENIED: the user declined this action. Continue without it.";
      }
      askConfirmed.add(target.cmd);
    }
    const result = await invoke(target.lib, target.ctl, target.cmd, args);
    if (result instanceof Error) {
      toolCell(callText, args, { error: true, output: result.message });
      return `ERROR: ${result.message}`;
    }
    const env = result.envelope;
    const payload = env.status === "ok" ? (env.data ?? env.msg ?? env) : env.msg;
    const text = typeof payload === "string" ? payload : JSON.stringify(payload, null, 1);
    toolCell(callText, args, { error: env.status !== "ok", output: text });
    return env.status === "ok" ? agent.clamp(text, 4000) : `ERROR: ${env.msg}`;
  }

  // ── ask ──────────────────────────────────────────────────────────────
  const form = host.querySelector(".ag-ask");
  const input = host.querySelector(".ag-input");
  const sendBtn = host.querySelector(".ag-send");
  form.addEventListener("submit", async (ev) => {
    ev.preventDefault();
    let message = input.value.trim();
    const typed = message;  // before the [ATTACHED FILES] block — copy uses this
    if ((!message && !attachments.length) || sendBtn.disabled) return;
    input.value = "";
    sendBtn.disabled = true;
    let images = [];
    if (attachments.length) {
      const batch = attachments.splice(0);
      renderAttachments();
      try {
        const up = await uploadBatch(batch);
        message = message ? message + "\n\n" + up.block : up.block;
        images = up.images;
      } catch (e) {
        attachments.unshift(...batch);
        renderAttachments();
        pushCell({ kind: "agent", error: true,
          text: `attachment upload failed: ${e.message || e}` });
        sendBtn.disabled = false;
        return;
      }
    }
    pushCell({ kind: "user", text: message, typed });
    const busy = pushCell({ kind: "busy", text: "thinking…" });
    const busyEl = thread.lastElementChild;
    try {
      const core = await agent.corePrompt();
      const catalog = await ensureCatalog();
      const attached = agent.DEFAULT_TOOLS.filter((n) => catalog.some((t) => t.name === n));
      const tools = [...agent.META_TOOL_DEFS, ...(stageCtl ? PAD_TOOL_DEFS : []),
        ...agent.toolDefs(catalog, attached)];
      const addendum = (await ownerAddendum()).trim();
      if (messages.length === 0 || messages[0].role !== "system") {
        messages.unshift({ role: "system", content: "" });
      }
      messages[0].content = core + "\n\n" + CHAT_SHELL + agent.TOOLS_PROMPT +
        (addendum ? "\n\nOWNER ADDENDUM\n" + addendum : "");
      // A snapshot's pixels serve the turn that took it. Re-sending every old
      // image on every call would grow each request without bound; the text
      // stays, and the agent takes a fresh snapshot when it needs to look.
      for (const m of messages) {
        if (m.role === "user" && m.images && String(m.content ?? "").startsWith("[SNAPSHOT IMAGES")) {
          delete m.images;
          m.content += " (image no longer attached)";
        }
      }
      const ctx = agent.contextBlock(viewctx.snapshot());
      const userMsg = { role: "user", content: message };
      if (images.length) userMsg.images = images;
      if (ctx) {
        messages.push({ role: "user", content: "[CONTEXT]\n\n" + ctx }, userMsg);
      } else {
        messages.push(userMsg);
      }
      const text = await agent.chatTurn({
        messages, tools, execTool, venue: "chat", entity: userName((await userP()) ?? {}),
        onRound: () => { busyEl.querySelector(".ag-cell-body").textContent = "agent is using tools…"; },
      });
      messages.push({ role: "assistant", content: text });
      busyEl.remove();
      transcript.splice(transcript.indexOf(busy), 1);
      pushCell({ kind: "agent", text });
      invoke("agent", "archivist", "log_turn", {
        venue: "agent-app", ask: message.slice(0, 4000),
        reply: (text ?? "").slice(0, 4000), tools: "", author: "agent-app",
      }).catch(() => {});
      refreshArchivist();
    } catch (e) {
      busyEl.remove();
      transcript.splice(transcript.indexOf(busy), 1);
      let text = `error: ${e.message || "the request failed"}`;
      if (/Key '(agent|VLLM_URL|VLLM_MODEL)' not found/.test(e.message ?? "")) {
        text += "\n\nThis instance's agent app isn't configured for chat: add " +
          "`agent` to config.properties apps=, put VLLM_URL and VLLM_MODEL in " +
          "runtime/agent/botd.properties, and restart.";
      }
      pushCell({ kind: "agent", error: true, text });
    }
    sendBtn.disabled = false;
    input.focus();
  });
  input.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter" && !ev.shiftKey) {
      ev.preventDefault();
      form.requestSubmit();
    }
  });
  host.querySelector(".ag-clear").addEventListener("click", newSession);

  // ── attachments: 📎 button, drag & drop, pasted screenshots ──────────
  // Files upload through agent.chat.upload via a direct JSON POST to
  // ../app/exec — the websocket/JSONP invoke path can't carry megabytes.
  // The message gets an [ATTACHED FILES] block of saved paths (small text
  // files inlined); images also ride the message's `images`, which chat_llm
  // renders as image blocks for vision models. The agent opens the rest
  // with its tools.
  const attachStrip = host.querySelector(".ag-attach");
  const fileInput = host.querySelector(".ag-file");
  const attachments = [];
  const fmtSize = (n) => n < 1024 ? `${n} B`
    : n < 1048576 ? `${(n / 1024).toFixed(1)} KB` : `${(n / 1048576).toFixed(1)} MB`;
  function renderAttachments() {
    attachStrip.innerHTML = "";
    attachStrip.hidden = attachments.length === 0;
    attachments.forEach((a, i) => {
      const chip = document.createElement("span");
      chip.className = "ag-chip";
      chip.textContent = `${a.name} (${fmtSize(a.size)}) `;
      const x = document.createElement("button");
      x.type = "button";
      x.className = "ag-chip-x";
      x.textContent = "✕";
      x.title = "remove this attachment";
      x.addEventListener("click", () => { attachments.splice(i, 1); renderAttachments(); });
      chip.appendChild(x);
      attachStrip.appendChild(chip);
    });
  }
  function addFiles(list) {
    for (const f of Array.from(list || [])) {
      if (!f || !f.size) continue;
      if (f.size > 20 * 1048576) {
        pushCell({ kind: "agent", error: true,
          text: `${f.name} is ${fmtSize(f.size)} — attachments are capped at 20 MB` });
        continue;
      }
      attachments.push(f);
    }
    renderAttachments();
  }
  let uploadCmdId = null;
  async function uploadOne(file) {
    const b64 = await new Promise((res, rej) => {
      const r = new FileReader();
      r.onload = () => res(String(r.result).split(",")[1] || "");
      r.onerror = () => rej(new Error(`cannot read ${file.name}`));
      r.readAsDataURL(file);
    });
    if (!uploadCmdId) {
      const ctlId = await new Promise((res) => lookupID("agent", "chat", res));
      const rec = await jsonP("../app/read", "lib=agent&id=" + encodeURIComponent(ctlId));
      const entry = (((rec || {}).data || {}).cmd || []).find((c) => c.name === "upload");
      if (!entry) throw new Error("the upload command is missing on agent.chat");
      uploadCmdId = entry.id;
    }
    const res = await fetch("../app/exec?sessionid=" + encodeURIComponent(sessionid), {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ lib: "agent", id: uploadCmdId,
        args: { filename: file.name, data_b64: b64 } }),
    });
    if (!res.ok) throw new Error(`upload of ${file.name}: HTTP ${res.status}`);
    const env = await res.json();
    const flat = env && env.data && typeof env.data === "object" ? env.data : env;
    if (!flat || flat.status !== "ok") {
      throw new Error((flat && flat.msg) || `upload of ${file.name} failed`);
    }
    return flat;
  }
  const texty = (f) => (f.type || "").startsWith("text/") || f.type === "application/json" ||
    /\.(txt|md|json|js|ts|css|html?|rs|py|toml|ya?ml|csv|log|sh|xml|svg|properties)$/i.test(f.name);
  async function uploadBatch(files) {
    const lines = ["[ATTACHED FILES — saved on this instance; open them with your tools]"];
    const inlined = [];
    const images = [];
    for (const f of files) {
      const r = await uploadOne(f);
      lines.push(`- ${r.path} (${f.type || "unknown type"}, ${fmtSize(f.size)})`);
      // chat_llm renders these paths as image blocks per provider dialect,
      // so vision models see the pixels, not just the path.
      if (/\.(png|jpe?g|gif|webp)$/i.test(r.path)) images.push(r.path);
      if (f.size <= 32768 && texty(f)) {
        try { inlined.push(`--- ${f.name} (inlined) ---\n` + await f.text()); } catch (e) {}
      }
    }
    return { block: lines.join("\n") + (inlined.length ? "\n\n" + inlined.join("\n\n") : ""),
             images };
  }
  host.querySelector(".ag-attachbtn").addEventListener("click", () => fileInput.click());
  fileInput.addEventListener("change", () => { addFiles(fileInput.files); fileInput.value = ""; });
  const chatMain = host.querySelector(".ag-chat-main");
  chatMain.addEventListener("dragover", (ev) => {
    ev.preventDefault();
    chatMain.classList.add("ag-drop-hot");
  });
  chatMain.addEventListener("dragleave", (ev) => {
    if (!chatMain.contains(ev.relatedTarget)) chatMain.classList.remove("ag-drop-hot");
  });
  chatMain.addEventListener("drop", (ev) => {
    ev.preventDefault();
    chatMain.classList.remove("ag-drop-hot");
    if (ev.dataTransfer) addFiles(ev.dataTransfer.files);
  });
  input.addEventListener("paste", (ev) => {
    const items = (ev.clipboardData && ev.clipboardData.items) || [];
    const files = [];
    for (const it of items) {
      if (it.kind !== "file") continue;
      const f = it.getAsFile();
      if (!f) continue;
      files.push(!f.name || f.name === "image.png"
        ? new File([f], `screenshot-${Date.now()}.png`, { type: f.type || "image/png" }) : f);
    }
    if (files.length) { ev.preventDefault(); addFiles(files); }
  });

  // ── memory: inspector + the owner's audit ────────────────────────────
  const domainsEl = host.querySelector(".ag-domains");
  const entriesEl = host.querySelector(".ag-entries");
  const memCap = host.querySelector(".ag-mem-cap");
  const journalEl = host.querySelector(".ag-mem-journal");
  let openDomainKey = null;   // "lib.ctl" of the open domain
  let openDomainLib = null;   // lib of the open domain
  let openDomainName = null;  // ctl name of the open domain

  // A "memory domain" is any control that carries a memory facet — the kb
  // knowledge domains AND control-resident memory (agent.llm, dev.code, …).
  // The list is grouped by library; the open key is "lib.ctl".
  async function loadDomains() {
    const libs = await jsonP("../app/libs", null);
    domainsEl.replaceChildren();
    if (libs.status !== "ok") {
      domainsEl.textContent = "library list unavailable: " + (libs.msg || "read failed");
      return;
    }
    const ids = (libs.data ?? []).map((l) => l.id).sort((a, b) => a.localeCompare(b));
    const groups = [];
    for (const lib of ids) {
      const controls = await controlsOf(lib);
      if (controls instanceof Error) continue;
      const doms = [];
      for (const c of controls.slice().sort((a, b) => a.name.localeCompare(b.name))) {
        const rec = await readRec(lib, c.id);
        if (rec instanceof Error || rec.memory === undefined || rec.memory === null) continue;
        let n = "?";
        try {
          const t = (rec.memory ?? "").trim();
          if (t.startsWith("[")) n = JSON.parse(t || "[]").length;
          else n = t ? t.split("\n").filter((x) => x.trim()).length : 0;
        } catch { n = "!"; }
        doms.push({ lib, name: c.name, n, desc: rec.desc ?? "" });
      }
      if (doms.length) groups.push({ lib, doms });
    }
    if (!groups.length) { domainsEl.textContent = "no memory facets found in any library"; return; }
    for (const g of groups) {
      const head = document.createElement("p");
      head.className = "ag-domlib lbl";
      head.textContent = g.lib;
      domainsEl.appendChild(head);
      for (const d of g.doms) {
        const btn = document.createElement("button");
        const key = `${g.lib}.${d.name}`;
        btn.className = "ag-domain" + (key === openDomainKey ? " on" : "");
        btn.dataset.key = key;
        btn.innerHTML = "";
        btn.textContent = `${d.name} (${d.n})`;
        btn.title = d.desc;
        btn.addEventListener("click", () => openDomain(g.lib, d.name));
        domainsEl.appendChild(btn);
      }
    }
  }

  async function openDomain(lib, name) {
    openDomainKey = `${lib}.${name}`;
    openDomainLib = lib;
    openDomainName = name;
    for (const b of domainsEl.querySelectorAll(".ag-domain")) {
      b.classList.toggle("on", b.dataset.key === openDomainKey);
    }
    const r = await readFacet(lib, name, "memory");
    entriesEl.replaceChildren();
    journalEl.replaceChildren();
    if (r.status !== "ok") { entriesEl.textContent = "read failed: " + r.msg; return; }
    let entries;
    try { entries = JSON.parse(r.source || "[]"); }
    catch { entriesEl.textContent = "this domain's memory facet is not valid JSON — repair it in the journal's history"; return; }
    memCap.textContent = `${lib}.${name} — ${entries.length} entr${entries.length === 1 ? "y" : "ies"}; every change below is a journaled patch`;

    const save = async (next, label) => {
      const body = JSON.stringify(next, null, 2) + "\n";
      const pr = await patchFacet(lib, name, "memory",
        { oldSnippet: r.source, newSnippet: body, base: r.hash, label });
      if (pr.status !== "ok") { note(memCap, `save failed: ${pr.msg}`, true); return false; }
      await openDomain(lib, name);
      await loadDomains();
      return true;
    };

    entries.forEach((e, i) => {
      const card = document.createElement("div");
      card.className = "ag-entry" + ((e.tags ?? "").split(",").some((t) => t.trim() === "unreviewed") ? " unreviewed" : "");
      const claim = document.createElement("div");
      claim.className = "ag-entry-claim";
      claim.textContent = e.claim ?? "(no claim)";
      card.appendChild(claim);
      if (e.detail) {
        const d = document.createElement("div");
        d.className = "ag-entry-detail";
        d.textContent = e.detail;
        card.appendChild(d);
      }
      const meta = document.createElement("div");
      meta.className = "ag-entry-meta lbl";
      const when = e.time ? new Date(e.time).toISOString().slice(0, 10) : "";
      meta.textContent = [e.tags ? `[${e.tags}]` : "", e.confidence ?? "", when,
        e.source?.doc ? `→ ${e.source.doc}` :
        e.source?.lib ? `→ ${e.source.lib}.${e.source.ctl}.${e.source.facet}` : ""]
        .filter(Boolean).join(" · ");
      card.appendChild(meta);
      if (e.source?.lib && e.source?.hash) {
        readFacet(e.source.lib, e.source.ctl, e.source.facet).then((ref) => {
          if (ref.status === "ok" && ref.hash !== e.source.hash) {
            const s = document.createElement("span");
            s.className = "ag-stale";
            s.textContent = "stale? referent changed";
            meta.appendChild(s);
          }
        });
      }
      const row = document.createElement("div");
      row.className = "ag-entry-actions";
      if ((e.tags ?? "").split(",").some((t) => t.trim() === "unreviewed")) {
        const bless = document.createElement("button");
        bless.textContent = "✓ reviewed";
        bless.title = "clear the unreviewed tag — the owner's audit blessing this entry";
        bless.addEventListener("click", async () => {
          const next = entries.map((x, j) => j !== i ? x : { ...x,
            tags: (x.tags ?? "").split(",").map((t) => t.trim())
              .filter((t) => t && t !== "unreviewed").join(",") });
          await save(next, `memory: reviewed — ${(e.claim ?? "").slice(0, 50)}`);
        });
        row.appendChild(bless);
      }
      const edit = document.createElement("button");
      edit.textContent = "edit";
      edit.addEventListener("click", () => {
        const ta = document.createElement("textarea");
        ta.className = "ag-entry-edit";
        ta.rows = 8;
        ta.value = JSON.stringify(e, null, 2);
        const ok = document.createElement("button");
        ok.textContent = "save entry";
        ok.addEventListener("click", async () => {
          let parsed;
          try { parsed = JSON.parse(ta.value); }
          catch { note(memCap, "that entry is not valid JSON", true); return; }
          await save(entries.map((x, j) => (j === i ? parsed : x)),
            `memory: edit — ${(parsed.claim ?? e.claim ?? "").slice(0, 50)}`);
        });
        card.append(ta, ok);
        edit.remove();
      });
      const del = document.createElement("button");
      del.textContent = "✕";
      del.title = "forget this entry (journaled — the history keeps it)";
      del.addEventListener("click", async () => {
        if (del.textContent !== "really forget?") {
          del.textContent = "really forget?";
          setTimeout(() => { del.textContent = "✕"; }, 2500);
          return;
        }
        await save(entries.filter((_, j) => j !== i),
          `memory: forget — ${(e.claim ?? "").slice(0, 50)}`);
      });
      row.append(edit, del);
      card.appendChild(row);
      entriesEl.appendChild(card);
    });

    const j = await listPatches(lib, name, 10);
    if (j.status === "ok") {
      for (const p of (j.patches ?? []).filter((p) => p.facet === "memory")) {
        const line = document.createElement("div");
        line.className = "ag-journal-line lbl";
        line.textContent = `${p.patch_id} · ${p.author} · ${p.label ?? ""}`;
        journalEl.appendChild(line);
      }
    }
  }

  host.querySelector(".ag-remember").addEventListener("submit", async (ev) => {
    ev.preventDefault();
    if (!openDomainName) { note(memCap, "open a domain first", true); return; }
    const g = (c) => host.querySelector(c).value.trim();
    const entry = { claim: g(".ag-rem-claim") };
    if (g(".ag-rem-detail")) entry.detail = g(".ag-rem-detail");
    if (g(".ag-rem-tags")) entry.tags = g(".ag-rem-tags");
    entry.confidence = host.querySelector(".ag-rem-conf").value;
    const user = await userP();
    const r = await invokeP("agent", "archivist", "remember", {
      lib: openDomainLib, domain: openDomainName, entry,
      author: user?.displayname || user?.id || "owner",
    });
    if (r.status !== "ok") { note(memCap, `remember failed: ${r.msg}`, true); return; }
    host.querySelector(".ag-rem-claim").value = "";
    host.querySelector(".ag-rem-detail").value = "";
    host.querySelector(".ag-rem-tags").value = "";
    await openDomain(openDomainLib, openDomainName);
    await loadDomains();
  });

  host.querySelector(".ag-newdomain").addEventListener("submit", async (ev) => {
    ev.preventDefault();
    const name = host.querySelector(".ag-newdomain-name").value.trim();
    if (!/^[a-z][a-z0-9-]*$/.test(name)) { note(memCap, "lowercase + dashes", true); return; }
    const r = await code("add_control", { lib: "kb", ctl: name });
    if (r.status !== "ok") { note(memCap, `add failed: ${r.msg}`, true); return; }
    host.querySelector(".ag-newdomain-name").value = "";
    await loadDomains();
  });

  // ── prompts: three journaled editors ─────────────────────────────────
  const PROMPTS = [
    { key: "core", lib: "agent", ctl: "prompts", facet: "prompt",
      cap: "the platform-knowledge CORE (agent.prompts) — every consumer assembles on this" },
    { key: "addendum", record: true,
      cap: "the OWNER ADDENDUM — appended to every notebook/chat prompt; your experiment surface" },
    { key: "archivist", lib: "agent", ctl: "archivist", facet: "prompt",
      cap: "the ARCHIVIST's extraction prompt — what gets remembered, and how conservatively" },
  ];
  let promptsLoaded = false;
  async function loadPrompts() {
    if (promptsLoaded) return;
    promptsLoaded = true;
    for (const p of PROMPTS) {
      const block = host.querySelector(`.ag-prompt[data-prompt="${p.key}"]`);
      const ta = block.querySelector("textarea");
      const status = block.querySelector(".ag-prompt-note");
      block.querySelector(".ag-prompt-cap").textContent = p.cap;
      // The addendum is a runtime DATA RECORD (id "agentchataddendum",
      // data.addendum), not a control facet: read via app.app.read, written via
      // app.app.write, and created on first save (the runtime library / record
      // may not exist yet on a fresh install). Other prompts stay facet patches.
      if (p.record) {
        const rec = await invokeP("app", "app", "read", { lib: "runtime", id: "agentchataddendum", nn_sessionid: null });
        ta.value = (rec.status === "ok" && rec.data && typeof rec.data.addendum === "string")
          ? rec.data.addendum : "";
        block.querySelector(".ag-prompt-save").addEventListener("click", async () => {
          const wr = await invokeP("app", "app", "write", {
            lib: "runtime", id: "agentchataddendum",
            data: { addendum: ta.value }, readers: [], writers: [], nn_sessionid: null });
          note(status, wr.status === "ok"
            ? "saved — takes effect on the next ask"
            : `save failed: ${wr.msg}`, wr.status !== "ok");
        });
        continue;
      }
      const r = await readFacet(p.lib, p.ctl, p.facet);
      if (r.status !== "ok") { status.textContent = "read failed: " + r.msg; ta.disabled = true; continue; }
      let text = r.source;
      if (p.extract) {
        const inner = p.extract(r.source);
        if (inner === null) {
          status.textContent = "unexpected facet shape — edit it in the Development app instead";
          ta.disabled = true;
          continue;
        }
        text = inner;
      }
      ta.value = text;
      block.querySelector(".ag-prompt-save").addEventListener("click", async () => {
        const cur = await readFacet(p.lib, p.ctl, p.facet);
        if (cur.status !== "ok") { note(status, "re-read failed: " + cur.msg, true); return; }
        const next = p.inject ? p.inject(cur.source, ta.value) : ta.value;
        const pr = await patchFacet(p.lib, p.ctl, p.facet,
          { oldSnippet: cur.source, newSnippet: next, base: cur.hash,
            label: `prompt: ${p.key} edited in the agent app` });
        note(status, pr.status === "ok"
          ? "saved (journaled) — takes effect on the next ask"
          : `save failed: ${pr.msg}`, pr.status !== "ok");
      });
    }
  }

  // ── the mind tab (docs/understandingloop.md; mind-tab-spec 2026-10-28):
  // a sectioned settings workspace. Status is polled every 5s while the
  // pane is open; the instant switches and service buttons call the same
  // platform commands as before; rates, settings and persona edits
  // collect in a DRAFT and are saved via Review changes → Apply. This
  // pane owns no state of its own beyond that draft. ───────────────────
  let mindWired = false;
  let mindTimer = null;
  const mEsc = (s) => { const d = document.createElement("span"); d.textContent = String(s ?? ""); return d.innerHTML; };
  // The HTTP exec envelope nests the command's own result under .data
  // ({status, pid, nn_return_type, data:{...}}); unwrap it, letting an
  // inner status (our commands carry one) win over the transport's.
  const mEnv = (r) => {
    if (r instanceof Error || !r || !r.envelope) return null;
    const e = r.envelope;
    return (e.data && typeof e.data === "object") ? { status: e.status, ...e.data } : e;
  };
  const mKv = (pairs, cls) => `<div class="kv ${cls || ""}">` + pairs.map(([k, v, c]) =>
    `<span class="kv-row"><span class="k lbl">${mEsc(k)}</span>` +
    `<span class="v ${c || ""}" title="${mEsc(v)}">${mEsc(v)}</span></span>`).join("") + `</div>`;
  const mChip = (txt, cls) => `<span class="chip ${cls || ""}">${mEsc(txt)}</span>`;
  const mChips = (s) => `<div class="chips">${s}</div>`;

  const M_LABEL = {
    LLM: "Chat provider", LLM_CTL: "Custom command", VLLM_URL: "vLLM server address", VLLM_MODEL: "vLLM model",
    OPENAI_MODEL: "OpenAI model", OPENAI_URL: "OpenAI endpoint", ANTHROPIC_MODEL: "Anthropic model", ANTHROPIC_URL: "Anthropic endpoint",
    GEMINI_MODEL: "Gemini model", GEMINI_URL: "Gemini endpoint", OLLAMA_URL: "Ollama address", OLLAMA_MODEL: "Ollama model",
    LLM_REMOTE: "Remote peer", LLM_REMOTE_ID: "Remote command id", LLM_REMOTE_SPLIT: "Split large requests",
    CLAUDE_CODE_MODEL: "Claude Code model", CLAUDE_CODE_PERMISSION_MODE: "Claude Code permission mode", CLAUDE_CODE_SYSTEM_MODE: "Claude Code system prompt",
    CLAUDE_CODE_TIMEOUT: "Claude Code timeout", CLAUDE_CODE_IDLE_TIMEOUT: "Claude Code idle timeout", CLAUDE_CODE_TOOLS: "Claude Code tools", CLAUDE_CODE_MCP: "Claude Code MCP servers",
    CLAUDE_CODE_CWD: "Claude Code working folder", CLAUDE_CODE_BIN: "Claude Code CLI path",
    "@drive": "Self-initiated activity", MAX_PLAN_STEPS: "Steps per plan item", PLAN_STEPS_PER_HOUR: "Plan steps per hour",
    SALIENCE: "Score what matters locally", MODEL_TRAIN: "Train the resident model", USER_GATE: "Release learned updates",
    MODEL_CHECKPOINT: "Resident model checkpoint", MODEL_SERVICE_PORT: "Local service port", MODEL_TRAIN_LR: "Learning rate",
    MODEL_MIX: "Training mix", MODEL_GATE: "Evaluation gate", MODEL_TRAIN_INTERVAL: "Training interval", USER_LORA: "Personality adapter",
    SALIENCE_STEER: "Steer by salience", SALIENCE_BANDS: "Salience bands", NANOCHAT_TRAIN_ARGS: "Base training arguments",
    NANOCHAT_REPO: "nanochat repository", NANOCHAT_DIST: "nanochat distribution",
  };
  const mSectionOf = (k) =>
    k === "LLM" || k.startsWith("LLM_") || /^(VLLM|OPENAI|ANTHROPIC|GEMINI|OLLAMA|CLAUDE_CODE)_/.test(k) ? "conversation"
    : ["@drive", "MAX_PLAN_STEPS", "PLAN_STEPS_PER_HOUR", "PLAN_AUTONOMY"].includes(k) ? "activity"
    : ["SALIENCE", "MODEL_TRAIN", "USER_GATE"].includes(k) ? "learning" : "advanced";
  const M_SECTIONS = [
    ["conversation", "Conversation", "The model that answers you in chat.", '<path d="M3 4h14v9H8l-4 3v-3H3z"/>'],
    ["activity", "Background activity", "Decide what happens between your messages.", '<polyline points="2,10 6,10 8,4 12,16 14,10 18,10"/>'],
    ["learning", "Local learning", "The resident model, its training, and what reaches chat.", '<path d="M10 17V9"/><path d="M10 9c0-3 2-5 6-5 0 4-2 5-6 5z"/><path d="M10 12c0-2-2-4-5-4 0 3 2 4 5 4z"/>'],
    ["personality", "Local personality", "Teach the local model through example exchanges.", '<circle cx="10" cy="7" r="3"/><path d="M4 17c0-3 3-5 6-5s6 2 6 5"/>'],
    ["insights", "Insights", "What the agent grew this week, and how its judge is doing.", '<path d="M4 16V10M10 16V5M16 16v-8"/>'],
    ["advanced", "Advanced", "Technical configuration, when you need it.", '<path d="M3 6h14M3 14h14"/><circle cx="7" cy="6" r="2"/><circle cx="13" cy="14" r="2"/>'],
  ];

  // ── draft model: baselines come from the 5s poll (get_settings + the
  // executive's live drive); a key counts as changed only while the draft
  // differs from the baseline, and the poll never overwrites a drafted or
  // focused field. ─────────────────────────────────────────────────────
  let mSection = "conversation";
  let mDraft = {};
  let mSet = {};                 // key -> {v, d, t, set, locked} from get_settings
  let mDrive = "";               // live drive baseline (executive.status)
  let mLiveState = { executive: false, sensor: false, plan: false };
  let mRepos = [];
  let mReposEnabled = 0;
  let mDots = {};
  let mShowKeys = lsGet("ag-mind-showkeys") === "1";
  const mBaseVal = (k) => k === "@drive" ? String(mDrive ?? "") : (mSet[k] ? String(mSet[k].v ?? "") : "");
  const mCur = (k) => (k in mDraft ? mDraft[k] : mBaseVal(k));
  const mSetDraft = (k, v) => { if (String(v) === mBaseVal(k)) delete mDraft[k]; else mDraft[k] = String(v); };
  const mEff = (k, dflt) => (String(mCur(k)).trim() || dflt);
  const mGateMode = (s) => (/(^|,)mode=([^,]*)/.exec(s || "") || [])[2] || "manual";
  const mWithGateMode = (s, m) => /(^|,)mode=/.test(s) ? s.replace(/(^|,)mode=[^,]*/, `$1mode=${m}`) : `mode=${m},${s}`;
  const M_TAKES = (k) => (mSet[k] && mSet[k].t) || (k === "@drive" ? "immediately"
    : /^(OPENAI|ANTHROPIC|GEMINI|OLLAMA|LLM_REMOTE|CLAUDE_CODE)/.test(k) ? "next frontier call"
    : /PLAN/.test(k) ? "as the plan worker next reads it" : "when its command next reads it");
  const mCmdOf = (k) => k === "@drive" ? "agent.executive.set_drive"
    : (k === "PLAN_STEPS_PER_HOUR" && !mDraft[k]) ? `agent.model.set_setting ${k} = "" (removes the key)`
    : `agent.model.set_setting ${k}`;
  function mPretty(k, v) {
    v = String(v ?? "");
    if (k === "USER_GATE") return mGateMode(v) === "auto" ? "release automatically" : "ask me to approve";
    if (k === "LLM") {
      const o = host.querySelector(`[data-provider] option[value="${v}"]`);
      return o ? o.textContent : (v || "VLLM (default)");
    }
    if (k === "@drive") return v + " acts/hr";
    if (k === "PLAN_STEPS_PER_HOUR" && (!v || v === "0")) return "no cap";
    if (v === "") return mSet[k] && mSet[k].d ? `default (${mSet[k].d})` : "not set";
    return v.length > 64 ? v.slice(0, 64) + "…" : v;
  }

  // ── the persona editor: the corpus the 8b adapter derives from.
  // Loaded once per session (the 5s poll never stomps edits); the empty
  // corpus shows the shipped default seed until the owner saves. Edits
  // join the draft; Apply serializes exactly as the old save did (raw
  // rows re-parsed, pairs as {user, assistant}, both-empty pairs
  // skipped - so an added empty row is not a pending change). ──────────
  let personaRows = null;
  let personaLoaded = null;       // deep clone of the last load (Discard target)
  let personaBaseStr = "";        // normalized serialization of the last load
  let pSeed = false, pInvalid = 0, pReadErr = "", pAdopt = false; // pAdopt: "Use this seed" marked the unedited default seed as a draft change
  const pClone = (x) => JSON.parse(JSON.stringify(x || []));
  const personaLines = (rows) => {
    const lines = []; let bad = false;
    for (const row of rows || []) {
      if (row.__rawText != null) {
        try { lines.push(JSON.stringify(JSON.parse(row.__rawText))); }
        catch { bad = true; lines.push("\u0000invalid"); }
      } else if ("user" in row || "assistant" in row) {
        if (!(row.user || "").trim() && !(row.assistant || "").trim()) continue;
        lines.push(JSON.stringify({ user: row.user || "", assistant: row.assistant || "" }));
      } else {
        lines.push(JSON.stringify(row));
      }
    }
    return { lines, bad };
  };
  const personaDirtyNow = () => personaRows !== null && personaLines(personaRows).lines.join("\n") !== personaBaseStr;
  const personaBadNow = () => personaRows !== null && personaLines(personaRows).bad;
  function renderPersona() {
    const box = host.querySelector("[data-persona]");
    if (!box) return;
    box.textContent = "";
    (personaRows || []).forEach((row, i) => {
      const div = document.createElement("div");
      const isPair = row && typeof row === "object" && !("messages" in row)
        && ("user" in row || "assistant" in row) && row.__rawText == null;
      div.className = "prow" + (isPair ? "" : " raw");
      if (i % 5 === 4) {
        const held = document.createElement("span");
        held.className = "lbl held";
        held.textContent = "held out for the check";
        div.append(held);
      }
      if (isPair) {
        const tu = document.createElement("textarea");
        tu.placeholder = "user says…";
        tu.setAttribute("aria-label", "User message");
        tu.value = row.user ?? "";
        tu.addEventListener("input", () => { row.user = tu.value; mindRefresh(); });
        const ta = document.createElement("textarea");
        ta.placeholder = "the agent answers…";
        ta.setAttribute("aria-label", "Agent answer");
        ta.value = row.assistant ?? "";
        ta.addEventListener("input", () => { row.assistant = ta.value; mindRefresh(); });
        div.append(tu, ta);
      } else {
        const tr = document.createElement("textarea");
        tr.className = "mono";
        tr.setAttribute("aria-label", "Raw exchange");
        tr.value = row.__rawText != null ? row.__rawText : JSON.stringify(row);
        tr.addEventListener("input", () => {
          row.__rawText = tr.value;
          let bad = false; try { JSON.parse(tr.value); } catch (e) { bad = true; }
          div.classList.toggle("bad", bad);
          mindRefresh();
        });
        div.append(tr);
      }
      const del = document.createElement("button");
      del.className = "del";
      del.textContent = "×";
      del.title = "remove this exchange";
      del.addEventListener("click", () => { personaRows.splice(i, 1); renderPersona(); mindRefresh(); });
      div.append(del);
      box.append(div);
    });
    renderPersonaState();
  }
  function renderPersonaState() {
    const state = host.querySelector("[data-pstate]");
    if (!state) return;
    state.textContent = pReadErr ? "persona read failed: " + pReadErr
      : (pSeed && !personaDirtyNow())
      ? "showing the DEFAULT SEED — nothing saved yet; edit to taste, then save to adopt it"
      : `${(personaRows || []).length} rows ${personaDirtyNow() ? "· edited (unsaved)" : "on disk"}`
        + (pInvalid ? ` · ${pInvalid} unparseable lines skipped` : "");
    // "Use this seed": only while the unedited default seed is showing; a
    // pending adopt hides it (the draft now carries the change — Discard
    // or Reload from disk brings it back).
    const adopt = host.querySelector("[data-padopt]");
    if (adopt) adopt.hidden = !(pSeed && !pReadErr && !personaDirtyNow() && !pAdopt);
  }
  async function loadPersona(force) {
    if ((personaDirtyNow() || pAdopt) && !force) return;
    const r = mEnv(await invoke("agent", "model", "persona_read", {}));
    if (!r || r.status !== "ok") {
      pReadErr = (r && r.msg) || "no reply";
      renderPersonaState();
      return;
    }
    pReadErr = "";
    pSeed = !!r.seed;
    pAdopt = false; // a fresh load (Reload from disk, the post-Apply reload) clears a pending adopt
    pInvalid = r.invalid || 0;
    personaRows = r.rows || [];
    personaLoaded = pClone(personaRows);
    personaBaseStr = personaLines(personaRows).lines.join("\n");
    renderPersona();
    mindRefresh();
  }

  // today's per-card note, grown the spec's way: a one-line message, red
  // on failure, with the command's full JSON reply behind a collapsed
  // "full reply" <details> when there is one.
  function mResult(where, msg, isErr, raw) {
    const el = host.querySelector(`[data-res="${where}"]`);
    if (!el) return;
    el.classList.toggle("err", !!isErr);
    el.innerHTML = `<div class="msg">${mEsc(msg)}</div>` +
      (raw ? `<details><summary>full reply</summary><pre>${mEsc(JSON.stringify(raw, null, 1))}</pre></details>` : "");
  }
  function mToast(txt) {
    const el = host.querySelector(".mx-toast");
    if (!el) return;
    el.textContent = txt || "";
    el.hidden = !txt;
  }

  // ── building the workspace: sidebar + panes from the section templates,
  // one delegated listener set for drafts, switches and actions. ───────
  function mindBuild() {
    const mx = host.querySelector('[data-pane="mind"] .mx');
    if (!mx) return;
    const side = mx.querySelector(".mx-side");
    const panes = mx.querySelector(".mx-panes");
    for (const [id, label, sub, icon] of M_SECTIONS) {
      const b = document.createElement("button");
      b.className = "mx-nav";
      b.dataset.sec = id;
      b.innerHTML = `<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">${icon}</svg><span>${label}</span><span class="dirty"></span><span class="dot"></span>`;
      side.append(b);
      const pane = document.createElement("section");
      pane.dataset.msec = id;
      pane.innerHTML = `<div class="mx-head"><span class="ico"><svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">${icon}</svg></span><div><h3>${label}</h3><p>${sub}</p></div></div>`;
      const t = host.querySelector(`template[data-mt="${id}"]`);
      if (t) pane.append(t.content.cloneNode(true));
      panes.append(pane);
    }

    const sk = mx.querySelector(".mx-showkeys");
    sk.checked = mShowKeys;
    mx.classList.toggle("showkeys", mShowKeys);
    sk.addEventListener("change", () => {
      mShowKeys = sk.checked;
      lsSet("ag-mind-showkeys", mShowKeys ? "1" : "0");
      mx.classList.toggle("showkeys", mShowKeys);
    });

    // drafts: every data-key field (friendly or raw) and the gate-mode view
    mx.addEventListener("input", (e) => {
      const el = e.target;
      if (el.classList.contains("mx-cfgfilter")) { renderCfg(); return; }
      if (el.dataset.key && !el.dataset.live) {
        const k = el.dataset.key;
        let v = el.type === "checkbox" ? (el.checked ? (el.dataset.on || "on") : (el.dataset.off || "off")) : el.value;
        if (!el.closest(".cfg")) {
          if (k === "PLAN_STEPS_PER_HOUR" && v.trim() === "0") v = "";       // 0 removes the key: unlimited is the default
          if (k === "MAX_PLAN_STEPS" && v.trim() === "20" && mBaseVal(k) === "") v = "";
        }
        mSetDraft(k, v);
        if (k === "LLM" && v === "CLAUDECODE" && mBaseVal("LLM_CTL") !== "agent:llm:claude_code") mSetDraft("LLM_CTL", "agent:llm:claude_code");
        if (k === "LLM" && v !== "CLAUDECODE" && v !== "CUSTOM") delete mDraft["LLM_CTL"];
        mToast(""); mBack(); mindRefresh();
      } else if (el.dataset.virtual === "gateMode") {
        mSetDraft("USER_GATE", mWithGateMode(mCur("USER_GATE"), el.value));
        mToast(""); mBack(); mindRefresh();
      }
    });

    // instant switches: the same commands and messages as the old buttons
    mx.addEventListener("change", async (e) => {
      const el = e.target;
      if (el.dataset.live) {
        const which = el.dataset.live;
        el.disabled = true;
        try {
          if (which === "executive" || which === "sensor") {
            const st = mEnv(await invoke("agent", which, "status", {}));
            const cmd = st && st.running ? "stop" : "start";
            const r = mEnv(await invoke("agent", which, cmd, {}));
            mResult("loop", r && r.status === "ok" ? `${which} ${cmd} ok` : `${which} ${cmd} failed`, !(r && r.status === "ok"));
          } else if (which === "plan") {
            const gs2 = mEnv(await invoke("agent", "model", "get_settings", {}));
            const row = ((gs2 && gs2.settings) || []).find((s) => s.key === "PLAN_AUTONOMY");
            const cur = row ? String(row.value || "").trim().toLowerCase() : "off";
            const on = ["on", "true", "1", "yes"].includes(cur);
            // armed -> disarm (set off); disarmed -> arm (set on). set_setting
            // updates the LIVE globals + persists botd.properties.
            const next = on ? "off" : "on";
            const r = mEnv(await invoke("agent", "model", "set_setting", { key: "PLAN_AUTONOMY", value: next }));
            // The plan worker follows PLAN_AUTONOMY live; arming only has to make
            // sure its thread exists (the worker_boot timer normally starts it).
            if (next === "on" && r && r.status === "ok") {
              const ws = mEnv(await invoke("agent", "plan", "worker_status", {}));
              if (!ws || !ws.alive) await invoke("agent", "plan", "worker_start", {});
            }
            mResult("plan", r && r.status === "ok" ? `plan autonomy ${next}` : "set_setting failed", !(r && r.status === "ok"));
          }
        } finally { el.disabled = false; }
        loadMind();
      } else if (el.dataset.repo != null) {
        const repo = el.dataset.repo;
        el.disabled = true;
        try {
          const cm = mEnv(await invoke("agent", "committer", "status", {}));
          const cur = (cm && (cm.all || []).find((r) => r.repo === repo)) || {};
          const turnOn = !cur.agent_commit;
          const r = mEnv(await invoke("agent", "committer", "set_enabled", { repo, enabled: turnOn }));
          let msg = r && r.status === "ok" ? `${repo} agent-commit ${turnOn ? "on" : "off"}` : "set_enabled failed";
          let err = !(r && r.status === "ok");
          // turning ON a repo dev still mechanically drives: hand it to the agent
          if (!err && turnOn && cur.dev_autocommit) {
            const d = mEnv(await invoke("dev", "git", "set_autocommit", { name: repo, autocommit: false }));
            msg += d && d.status === "ok" ? " · dev autocommit off (agent now drives it)" : " · WARN: dev autocommit still on - shadowed";
            if (!(d && d.status === "ok")) err = true;
          }
          mResult("committer", msg, err);
        } finally { el.disabled = false; }
        loadMind();
      }
    });

    // navigation, review, undo, instant action buttons
    mx.addEventListener("click", async (e) => {
      const t = e.target.closest("button, a");
      if (!t || !mx.contains(t)) return;
      if (t.classList.contains("mx-nav")) { mSection = t.dataset.sec; mBack(); mindRefresh(); return; }
      if (t.dataset.goto) { e.preventDefault(); mSection = t.dataset.goto; mBack(); mindRefresh(); return; }
      if (t.dataset.revert) {
        const k = t.dataset.revert;
        delete mDraft[k];
        if (k === "LLM") delete mDraft["LLM_CTL"];
        mindRefresh(); return;
      }
      if (t.classList.contains("mx-discard")) {
        mDraft = {};
        pAdopt = false;
        personaRows = pClone(personaLoaded);
        renderPersona(); mBack(); mToast(""); mindRefresh(); return;
      }
      if (t.classList.contains("mx-reviewbtn")) { mShowReview(); return; }
      const act = t.dataset.act;
      if (!act) return;
      if (act === "back") { mBack(); mindRefresh(); return; }
      if (act === "apply") { t.disabled = true; await mApply(); return; }
      if (act === "persona-add") {
        (personaRows = personaRows || []).push({ user: "", assistant: "" });
        renderPersona(); mindRefresh(); return;
      }
      if (act === "persona-adopt") { pAdopt = true; mToast(""); mindRefresh(); return; }
      if (act === "persona-reload") { loadPersona(true); return; }
      if (act === "sweep") {
        t.disabled = true;
        mResult("committer", "sweep running - grouping units, composing messages…");
        const r = mEnv(await invoke("agent", "committer", "sweep", {}));
        t.disabled = false;
        const ok = r && r.status === "ok";
        mResult("committer", ok
          ? `swept ${r.swept ?? 0} · committed ${r.units_committed ?? 0} units (${r.commits ?? 0} commits) · pushed ${r.pushed ?? 0}`
          : (r && r.msg) || "sweep failed", !ok);
        loadMind(); return;
      }
      if (act === "bootstrap") {
        t.disabled = true;
        mResult("judge", "bootstrap running — installs and training can take a while…");
        const r = mEnv(await invoke("agent", "model", "bootstrap", {}));
        t.disabled = false;
        mResult("judge", r ? (r.status === "ok" ? "bootstrap ok" : "bootstrap failed") : "bootstrap: no reply", !(r && r.status === "ok"), r);
        loadMind(); return;
      }
      if (act === "promote") {
        const r = mEnv(await invoke("agent", "model", "promote_pointer", {}));
        mResult("judge", r ? (r.status === "ok" ? "promote_pointer ok" : "promote failed") : "promote: no reply", !(r && r.status === "ok"), r);
        loadMind(); return;
      }
      if (act === "release") {
        const r = mEnv(await invoke("agent", "model", "service_stop", {}));
        mResult("judge", r && r.status === "ok"
          ? "GPU released - the judge is down by request; resume with bootstrap (it reloads the newest ring checkpoint)"
          : `release: ${r ? r.msg : "no reply"}`, !(r && r.status === "ok"));
        loadMind(); return;
      }
      if (act === "user-promote") {
        const r = mEnv(await invoke("agent", "model", "user_promote", {}));
        mResult("judge", r ? (r.status === "ok" ? "user_promote ok" : "user promote failed") : "user promote: no reply", !(r && r.status === "ok"), r);
        loadMind(); return;
      }
      if (act === "user-rollback") {
        const r = mEnv(await invoke("agent", "model", "user_rollback", {}));
        mResult("judge", r ? (r.status === "ok" ? "user_rollback ok" : "user rollback failed") : "user rollback: no reply", !(r && r.status === "ok"), r);
        loadMind(); return;
      }
      if (act === "rederive") {
        t.disabled = true;
        mResult("persona", "deriving the personality adapter — a LoRA training run; this can take a few minutes…");
        const r = mEnv(await invoke("agent", "model", "persona_rederive", {}));
        t.disabled = false;
        mResult("persona", r ? (r.status === "ok" ? "persona_rederive ok" : "re-derive failed") : "re-derive: no reply", !(r && r.status === "ok"), r);
        loadMind(); return;
      }
      if (act === "export") {
        const ts = new Date().toISOString().replace(/[-:T]/g, "").slice(0, 14);
        const r = mEnv(await invoke("agent", "model", "curriculum_export",
          { path: `runtime/agent/model/ingest/batch-${ts}.jsonl` }));
        mResult("flywheel", r && r.status === "ok"
          ? `exported ${r.total} samples (${r.salience_pairs} pairs, ${r.claims} claims, ${r.curation_traces} traces) — the trainer drains ingest within seconds`
          : `export failed: ${r ? r.msg : "no reply"}`, !(r && r.status === "ok"));
        loadMind(); return;
      }
    });
  }

  // one switch per dev-registered repo (never hardcoded names); skipped
  // while a switch is in flight so the rebuild can't eat its busy state.
  function renderRepos() {
    const box = host.querySelector("[data-repos]");
    if (!box) return;
    if ([...box.querySelectorAll("input")].some((i) => i.disabled)) return;
    const say = { active: "agent commits", shadowed_by_dev_autocommit: "shadowed by dev autocommit", refused_default_branch: "refused: on its default branch", "dev mechanical": "dev mechanical", off: "off" };
    box.innerHTML = mRepos.length ? mRepos.map((r) => {
      const st = r.state || (r.agent_commit ? "active" : (r.dev_autocommit ? "dev mechanical" : "off"));
      const cls = st === "active" ? "ok" : (st === "shadowed_by_dev_autocommit" ? "warn" : (st === "refused_default_branch" ? "err" : "off"));
      return `<div class="repo"><span class="name">${mEsc(r.repo)}</span>${mChip(say[st] || st, cls)}` +
        `<span class="tg"><input type="checkbox" data-repo="${mEsc(r.repo)}" ${r.agent_commit ? "checked" : ""} aria-label="Agent commits for ${mEsc(r.repo)}" title="on a repo dev still autocommits, turning this on also turns dev's mechanical autocommit OFF so the agent drives it"><span></span></span></div>`;
    }).join("") : mChip("no dev-registered repos", "off");
    const rc = host.querySelector("[data-repocount]");
    if (rc) rc.textContent = `agent-commit repos: ${mReposEnabled}`;
  }

  // the all-settings list: every key get_settings returns (known keys plus
  // whatever else the file holds) + any freshly drafted key; never rebuilt
  // while the user is typing in it.
  function renderCfg() {
    const mx = host.querySelector('[data-pane="mind"] .mx');
    if (!mx) return;
    const cfg = mx.querySelector(".cfg");
    if (!cfg || cfg.contains(document.activeElement)) return;
    const f = ((mx.querySelector(".mx-cfgfilter") || {}).value || "").toLowerCase();
    const keys = Object.keys(mSet);
    for (const k of Object.keys(mDraft)) if (!(k in mSet) && k !== "@drive") keys.push(k);
    cfg.innerHTML = keys.filter((k) => k.toLowerCase().includes(f)).map((k) => {
      const s = mSet[k] || { v: "", d: "", t: "", set: false, locked: false };
      return `<span class="ck ${s.set ? "set" : ""} ${k in mDraft ? "changed" : ""}" data-k="${mEsc(k)}" title="${s.set ? "explicitly set" : "default"}">${mEsc(k)}</span>` +
        (s.locked ? `<span class="locked">${mEsc(s.v)} (file-only)</span>`
          : `<input data-key="${mEsc(k)}" value="${mEsc(mCur(k))}" placeholder="${mEsc(s.d)}" aria-label="${mEsc(k)}">`) +
        `<span class="ct">${s.d ? "default " + mEsc(s.d.length > 44 ? s.d.slice(0, 44) + "…" : s.d) + (s.t ? " · " : "") : ""}${s.t ? "takes effect: " + mEsc(s.t) : ""}</span>`;
    }).join("");
  }

  // bind current values (draft over baseline) into every field; a focused
  // field is never touched, so the 5s poll can't stomp an edit in progress.
  function mBindValues() {
    for (const el of host.querySelectorAll('[data-pane="mind"] [data-key]')) {
      const k = el.dataset.key;
      if (el.type === "checkbox") el.checked = String(mCur(k)).toLowerCase() === (el.dataset.on || "on");
      else if (document.activeElement !== el) el.value =
        (k === "MAX_PLAN_STEPS" && !el.closest(".cfg")) ? mEff(k, "20")
        : (k === "PLAN_STEPS_PER_HOUR" && !el.closest(".cfg")) ? mEff(k, "0")
        : (k === "LLM") ? (mCur(k) || (mSet.LLM && mSet.LLM.d) || "VLLM")
        : mCur(k);
      const wrap = el.closest(".fld, .sw");
      if (wrap && !el.closest(".cfg")) mMarkChanged(wrap, k);
    }
    const gm = host.querySelector('[data-virtual="gateMode"]');
    if (gm) {
      if (document.activeElement !== gm) gm.value = mGateMode(mCur("USER_GATE"));
      mMarkChanged(gm.closest(".fld"), "USER_GATE");
    }
    for (const el of host.querySelectorAll('[data-pane="mind"] [data-live]'))
      if (!el.disabled) el.checked = !!mLiveState[el.dataset.live];
  }
  function mMarkChanged(wrap, k) {
    if (!wrap) return;
    const changed = k in mDraft;
    wrap.classList.toggle("changed", changed);
    let was = wrap.querySelector(":scope > .was");
    if (changed) {
      if (!was) { was = document.createElement("span"); was.className = "was"; wrap.append(was); }
      was.innerHTML = `was ${mEsc(mPretty(k, mBaseVal(k)))}<button data-revert="${mEsc(k)}">undo</button>`;
    } else if (was) was.remove();
  }

  function mProviderErrors() {
    const p = mCur("LLM") || (mSet.LLM && mSet.LLM.d) || "VLLM";
    const box = [...host.querySelectorAll('[data-pane="mind"] [data-when]')].find((d) => d.dataset.when.split(",").includes(p));
    return box ? [...box.querySelectorAll("[data-req]")].filter((i) => !String(mCur(i.dataset.key)).trim()).map((i) => M_LABEL[i.dataset.key] || i.dataset.key) : [];
  }
  function mRefreshProvider() {
    const p = mCur("LLM") || (mSet.LLM && mSet.LLM.d) || "VLLM";
    for (const d of host.querySelectorAll('[data-pane="mind"] [data-when]')) d.hidden = !d.dataset.when.split(",").includes(p);
    const need = { OPENAI: "needed unless a custom endpoint is keyless", ANTHROPIC: "required", GEMINI: "required", VLLM: "optional", OLLAMA: "optional, for hosted Ollama" }[p];
    const kn = host.querySelector("[data-keynote]");
    if (kn) kn.textContent = need ? `API key (${p}_KEY or ${p}_API_KEY, ${need}) and any extra headers (${p}_HEADERS) stay in botd.properties — secret keys can't be edited from the app.` : "";
    const perr = host.querySelector("[data-provider-err]");
    const miss = mProviderErrors();
    if (perr) perr.textContent = miss.length ? `Fill in ${miss.join(" and ")} before using this provider.` : "";
    for (const i of host.querySelectorAll('[data-pane="mind"] [data-req]'))
      i.closest(".fld").classList.toggle("bad", !i.closest("[hidden]") && !String(mCur(i.dataset.key)).trim());
    const an = host.querySelector("[data-activity-note]");
    if (an) an.textContent = !mLiveState.executive
      ? "Background thinking is paused. You can still chat with your agent."
      : (mLiveState.sensor ? "" : "The executive is on, but sensors are paused — no new events will be collected.");
  }

  // sidebar marks, footer count, field values - everything derived from
  // the draft and the last poll.
  function mindRefresh() {
    const mx = host.querySelector('[data-pane="mind"] .mx');
    if (!mx || !mindWired) return;
    for (const pn of mx.querySelectorAll("[data-msec]")) pn.hidden = pn.dataset.msec !== mSection;
    const ch = mChangesList();
    const dirtySecs = new Set(ch.map((c) => c.sec));
    for (const b of mx.querySelectorAll(".mx-nav")) {
      b.setAttribute("aria-current", String(b.dataset.sec === mSection));
      b.querySelector(".dirty").textContent = dirtySecs.has(b.dataset.sec) ? "•" : "";
      const d = b.querySelector(".dot");
      d.className = "dot " + (mDots[b.dataset.sec] || "");
      d.hidden = !mDots[b.dataset.sec];
    }
    const dirtyEl = mx.querySelector(".mx-dirty");
    dirtyEl.textContent = ch.length ? `${ch.length} pending ${ch.length === 1 ? "change" : "changes"}` : "No pending changes";
    dirtyEl.classList.toggle("dirty", !!ch.length);
    mx.querySelector(".mx-discard").disabled = mx.querySelector(".mx-reviewbtn").disabled = !ch.length;
    mBindValues(); mRefreshProvider(); renderPersonaState();
    for (const el of mx.querySelectorAll(".cfg .ck")) el.classList.toggle("changed", el.dataset.k in mDraft);
  }

  async function loadMind() {
    if (!mindWired) { mindWired = true; mindBuild(); }
    if (personaRows === null) { personaRows = []; personaLoaded = []; personaBaseStr = ""; loadPersona(true); }
    if (!mindTimer) mindTimer = setInterval(() => {
      const pane = host.querySelector('[data-pane="mind"]');
      if (pane && !pane.hidden) loadMind();
    }, 5000);

    const [exR, snR, svR, trR, slR, hvR, cmR, gsR, pwR, mtR] = await Promise.all([
      invoke("agent", "executive", "status", {}),
      invoke("agent", "sensor", "status", {}),
      invoke("agent", "model", "service_status", {}),
      invoke("agent", "model", "train_status", {}),
      invoke("agent", "executive", "salience_log", {}),
      invoke("agent", "model", "harvest_report", { window_days: 7 }),
      invoke("agent", "committer", "status", {}),
      invoke("agent", "model", "get_settings", {}),
      invoke("agent", "plan", "worker_status", {}),
      invoke("agent", "model", "metrics", {}),
    ]);
    const ex = mEnv(exR) || {}, sn = mEnv(snR) || {}, sv = mEnv(svR) || {};
    const tr = mEnv(trR) || {}, sl = mEnv(slR) || {}, hv = mEnv(hvR) || {};
    const cm = mEnv(cmR) || {}, gs = mEnv(gsR) || {}, pw = mEnv(pwR) || {};
    const mt = mEnv(mtR);

    // the draft's baselines: get_settings rows + the executive's live drive.
    // A drafted key whose baseline caught up is no longer a change.
    const newSet = {};
    for (const s of gs.settings || []) newSet[s.key] = {
      v: String(s.value ?? ""), d: String(s.default ?? ""), t: s.takes || "",
      set: !!s.set, locked: !!s.locked,
    };
    if (gs.settings && gs.settings.length) mSet = newSet;
    if (ex.drive != null) mDrive = String(ex.drive);
    for (const k of Object.keys(mDraft)) if (mDraft[k] === mBaseVal(k)) delete mDraft[k];

    // plan autonomy (kb.plan GATED plan-item autonomy): read the LIVE
    // effective values off get_settings - PLAN_AUTONOMY arms the plan worker
    // (agent.plan.worker_start, its own thread, not the executive's drive);
    // MAX_PLAN_STEPS is the per-item convergence guard; PLAN_STEPS_PER_HOUR
    // an optional sliding-hour cap (unset/0 = unlimited).
    const planAutonomyOn = ["on", "true", "1", "yes"].includes(((mSet.PLAN_AUTONOMY && mSet.PLAN_AUTONOMY.v) || "off").trim().toLowerCase());
    const maxPlanSteps = ((mSet.MAX_PLAN_STEPS && mSet.MAX_PLAN_STEPS.v) || "").trim() || "20";
    mLiveState = { executive: !!ex.running, sensor: !!sn.running, plan: planAutonomyOn };

    const strip = (n) => host.querySelector(`[data-status="${n}"]`);

    // conversation: what is saved and in use, from settings only
    {
      const arm = ((mSet.LLM && mSet.LLM.v) || "").trim() || (mSet.LLM && mSet.LLM.d) || "VLLM";
      const o = host.querySelector(`[data-provider] option[value="${arm}"]`);
      const armName = o ? o.textContent : arm;
      const armModel = arm === "CLAUDECODE" ? (mBaseVal("CLAUDE_CODE_MODEL") || "CLI default")
        : arm === "LOCAL" ? "released local version"
        : arm === "REMOTE" ? (mBaseVal("LLM_REMOTE") || "—")
        : arm === "CUSTOM" ? (mBaseVal("LLM_CTL") || "—")
        : (mBaseVal(arm + "_MODEL") || "—");
      const st = strip("conversation");
      if (st) st.innerHTML = mChips(mChip("in use: " + armName, "ok")) +
        mKv([[arm === "REMOTE" ? "peer" : arm === "CUSTOM" ? "command" : "model", armModel], ["provider key", "LLM=" + arm]]);
    }

    // background activity
    const ctx = ex.last_context || {};
    const planChip = !planAutonomyOn ? mChip("plan autonomy off", "off")
      : (pw.alive ? mChip(`plan worker ${pw.phase || "on"}`, "warn") : mChip("plan worker NOT RUNNING", "err"));
    {
      const st = strip("activity");
      if (st) st.innerHTML = mChips(
        mChip(ex.running ? `executive ${ex.phase || "on"}` : "executive stopped", ex.running ? "ok" : "off") +
        mChip(sn.running ? "sensor on" : "sensor stopped", sn.running ? "ok" : "off") + planChip) +
        mKv([
          ["perceived", ex.perceived_total ?? "—"],
          ["salience calls", ex.salience_calls ?? "—"],
          ["escalations / audits", `${ex.escalations ?? 0} / ${ex.audits ?? 0}`],
          ["disagreements", ex.disagreements ?? 0, (ex.disagreements ?? 0) > 0 ? "warn" : ""],
          ["unparseable verdicts", ex.unparsed_verdicts ?? 0, (ex.unparsed_verdicts ?? 0) > 0 ? "warn" : ""],
          ["drive (memory acts/hr)", ex.drive ?? "—"],
          ["epistemic acts", ex.acts_total ?? "—"],
          ["steer fast / deep", `${ex.fast_skips ?? 0} / ${ex.deep_orients ?? 0}`],
          ["last verdict", ctx.salience != null
            ? `${ctx.salience}${ctx.steer ? ` [${ctx.steer}]` : ""} — ${(ctx.salience_why || "").slice(0, 56)}`
            : "none"],
        ]);
    }
    {
      const st = strip("plan");
      if (st) st.innerHTML = mChips(planChip) + mKv([
        ["plan worker", pw.phase === "stepping"
          ? `stepping: ${(pw.current || "").slice(0, 60)}`
          : pw.phase === "rate_limited" ? `rate limited until ${new Date(pw.resume_at).toLocaleTimeString()}`
          : (pw.phase || "not started")],
        ["plan turns", `${pw.turns_total ?? 0} total · ${pw.turns_last_hour ?? 0} this hour · ${Number(pw.per_hour) > 0 ? `cap ${pw.per_hour}/hr` : "no hourly cap"} · ${maxPlanSteps} steps/item`],
        ["plan queue", `${pw.eligible ?? 0} ready (${(pw.coordinating || []).length} coordinating) · ${(pw.waiting_for_children || []).length} waiting for active children · ${(pw.parked || []).length} parked · ${(pw.awaiting_reapproval || []).length} need re-approval`,
          ((pw.parked || []).length + (pw.awaiting_reapproval || []).length) > 0 ? "warn" : ""],
        ["last plan step", pw.last ? `${pw.last.action} — ${(pw.last.claim || "").slice(0, 56)}` : "none"],
      ], "one");
    }

    // automatic commits - which repos the LLM committer drives
    {
      const all = cm.all || [], en = cm.repos || [];
      const enMap = {};
      en.forEach((r) => { enMap[r.repo] = r; });
      mRepos = all.map((r) => ({ repo: r.repo, agent_commit: !!r.agent_commit, dev_autocommit: !!r.dev_autocommit,
        state: enMap[r.repo] ? enMap[r.repo].state : "" }));
      mReposEnabled = cm.enabled ?? 0;
      renderRepos();
    }

    // local learning: the resident service
    const mode = sv.mode || "down";
    const modeCls = mode.startsWith("nanochat") ? "ok" : (mode === "stub" ? "warn" : (mode === "down" ? "err" : "warn"));
    {
      const st = strip("learning");
      if (st) st.innerHTML = mChips(mChip(mode, modeCls)) +
        (mode === "stub" ? `<div class="note warn" style="margin:0 0 8px">Stub mode is for testing the service. It is not a trained model — set a real checkpoint in <b>Advanced</b>.</div>` : "") +
        mKv([
          ["checkpoint", sv.checkpoint || "—"],
          ["live slot / pid", `${sv.live_slot || "—"} / ${sv.pid || "—"}`],
          ["verdicts served", sv.scored ?? "—"],
          ["pointer promotions", sv.promotions ?? "—"],
          ["uptime", sv.uptime_s != null ? `${Math.floor(sv.uptime_s / 3600)}h ${Math.floor((sv.uptime_s % 3600) / 60)}m` : "—"],
          ["load error", sv.boot_error || "none", sv.boot_error ? "err" : ""],
        ]);
    }

    // the user-facing pointer (8a): the stricter lane's state
    const u = sv.user || {};
    const soak = u.soak || {};
    const uCls = u.serving ? "ok" : (u.ready ? "warn" : "off");
    const uTxt = u.serving ? `serving ${u.name}` : (u.ready ? `READY: ${u.ready}` : "lagging (by design)");
    const soakTxt = u.soaking
      ? `${u.soaking} · ${Math.floor((soak.since_s ?? 0) / 3600)}h ${Math.floor(((soak.since_s ?? 0) % 3600) / 60)}m · ${soak.verdicts ?? 0} verdicts`
      : "—";
    const ev = u.eval || {};
    {
      const st = host.querySelector("[data-userptr]");
      if (st) st.innerHTML = mChips(mChip(uTxt, uCls)) + mKv([
        ["soaking on the fast lane", soakTxt],
        ["ready candidate", u.ready ? `${u.ready} (evals attached)` : "none"],
        ["last promotion eval", ev.agree != null || ev.std != null
          ? `agree ${ev.agree ?? "—"} · std ${ev.std?.toFixed?.(4) ?? ev.std ?? "—"}` : "—"],
        ["watchdog re-audit", ev.watch_agree != null ? `agree ${ev.watch_agree}` : "not yet"],
        ["promotions / rollbacks", `${u.promotions ?? 0} / ${u.rollbacks ?? 0}`],
        ["rollback target", u.last_good || "none"],
      ]);
    }
    const upBtn = host.querySelector('[data-act="user-promote"]');
    if (upBtn) upBtn.hidden = !u.ready;
    const urBtn = host.querySelector('[data-act="user-rollback"]');
    if (urBtn) urBtn.hidden = !u.last_good;

    // the personality adapter (8b)
    const p = sv.persona || {};
    const pa = p.adapter || {};
    const drift = (p.probe != null && p.baseline != null && p.baseline > 0)
      ? ((p.probe / p.baseline - 1) * 100) : null;
    {
      const st = strip("personality");
      if (st) st.innerHTML = mKv([
        ["persona corpus", p.corpus ? `${p.corpus} rows` : "none (persona/persona.jsonl)"],
        ["adapter", p.deriving ? "deriving…"
          : (p.adapter ? `on ${pa.derived_from} (rank ${pa.rank ?? "—"})` : "none")],
        ["probe / baseline", p.probe != null
          ? `${p.probe} / ${p.baseline ?? "—"}` + (drift != null ? ` (${drift >= 0 ? "+" : ""}${drift.toFixed(1)}%)` : "")
          : "—", drift != null && drift > 10 ? "warn" : ""],
        ["re-derivations", p.rederivations ?? 0],
        ["last derivation", p.last_result
          ? `${p.last_result.verdict} · persona ${p.last_result.heldout_base ?? "—"} → ${p.last_result.heldout_adapted ?? "—"} · ${p.last_result.seconds ?? "—"}s`
          : "never"],
      ]);
    }
    const rdBtn = host.querySelector('[data-act="rederive"]');
    if (rdBtn) rdBtn.hidden = !(u.serving && p.corpus >= 5 && !p.deriving);

    // continuous training (the flywheel)
    const t = sv.trainer || {};
    {
      const g = t.last_gate;
      const gateLine = `<div class="gateline">` + (!g ? "no gate yet" :
        `last gate @${g.step}: <b class="${g.verdict === "promote" ? "ok" : "warn"}">${mEsc(g.verdict)}</b>` +
        ` · standard cand ${g.cand_std?.toFixed?.(4) ?? "—"} vs live ${g.live_std?.toFixed?.(4) ?? "—"}` +
        (g.cand_agree != null
          ? ` · AGREEMENT cand ${g.cand_agree?.toFixed?.(3)} vs live ${g.live_agree?.toFixed?.(3) ?? "—"} (${g.pairs ?? 0} held-out pairs)`
          : ` · curriculum-loss cand ${g.cand_fresh?.toFixed?.(4) ?? "—"} vs live ${g.live_fresh?.toFixed?.(4) ?? "—"}`)) + `</div>`;
      const st = host.querySelector("[data-flywheel]");
      if (st) st.innerHTML = mChips(mChip(t.active ? "training" : "idle", t.active ? "ok" : "off")) + mKv([
        ["steps", t.steps ?? 0],
        ["loss ema", t.loss_ema ?? "—"],
        ["replay reservoir", t.replay_size ?? 0],
        ["fresh pending", t.fresh_pending ?? 0],
        ["gates / promotions / resets", `${t.gates ?? 0} / ${t.promotions ?? 0} / ${t.resets ?? 0}`],
        ["mix", t.mix ? Object.entries(t.mix).map(([k2, v2]) => `${k2} ${v2}`).join(" · ") : "—"],
      ]) + gateLine;
    }

    // base training (the forge) - block appears only when a run exists
    {
      const forge = host.querySelector("[data-forge]");
      const hasForge = tr.running || tr.done || (tr.log && tr.log.length);
      if (forge) forge.hidden = !hasForge;
      const body = host.querySelector("[data-forge-body]");
      if (body && hasForge) body.innerHTML =
        mChips(mChip(tr.running ? "training running" : (tr.done ? "train_done" : "stopped"),
          tr.running ? "ok" : (tr.done ? "ok" : "warn"))) +
        mKv([["pid", tr.pid || "—"]]) +
        `<pre class="trainlog">${mEsc((tr.log || []).join("\n"))}</pre>`;
    }

    // insights: the harvest + the salience audit trail (the agent's own
    // files, so they render whatever the service state)
    {
      const dom = Object.entries(hv.claims_by_domain || {})
        .sort((a, b) => b[1] - a[1]).slice(0, 5)
        .map(([k, v]) => `${k} ${v}`).join(" · ");
      const actsTxt = Object.entries(hv.acts || {})
        .map(([k, v]) => `${k} ${v}`).join(" · ") || "none yet";
      const banksTxt = (hv.banks || [])
        .map(b => `${b.name} ${b.rows}`).join(" · ") || "—";
      const gauge = Object.entries(hv.context_by_purpose || {})
        .map(([p2, r2]) => `${p2} ${r2.tokens_avg}`).join(" · ");
      const venues = Object.entries(hv.messages_by_venue || {})
        .map(([k, v]) => `${k} ${v}`).join(" · ");
      const notes = (hv.wonderings || []).slice(-2).map(q => `? ${q}`).join("\n");
      const st = strip("harvest");
      if (st) st.innerHTML = mChips(
        mChip(`${hv.claims ?? 0} claims / ${hv.window_days ?? 7}d`, "ok") +
        mChip(`${hv.notions_pending ?? 0} notions await audit`, (hv.notions_pending ?? 0) > 0 ? "warn" : "off")) +
        mKv([
          ["by domain (top)", dom || "—"],
          ["the garden's acts", actsTxt],
          ["banks (rows)", banksTxt],
          ["messages", `${hv.messages ?? 0} (${venues || "—"})`],
          ["capture rows", hv.capture_rows ?? 0],
          ["context tokens avg", gauge || "no calls yet"],
        ]) +
        (notes ? `<div class="res"><div class="msg">${mEsc(notes)}</div></div>` : "");
    }
    {
      const rows = sl.rows || [];
      const dis = rows.filter((r2) => r2.disagree).length;
      const gap = rows.length ? rows.reduce((a2, r2) => a2 + Math.abs((r2.local ?? 0) - (r2.frontier ?? 0)), 0) / rows.length : null;
      let html = mKv([
        ["audit rows (total)", sl.total ?? 0],
        ["recent disagreement rate", rows.length ? `${dis}/${rows.length}` : "—", dis > rows.length / 2 ? "warn" : ""],
        ["recent |local − frontier|", gap != null ? gap.toFixed(3) : "—"],
        ["band escalations dropped", ex.esc_dropped ?? 0],
        ["newest row", rows.length ? `${(rows[rows.length - 1].input || "").slice(0, 50)} → local ${rows[rows.length - 1].local} / frontier ${rows[rows.length - 1].frontier}` : "—"],
      ]);
      if (mt && mt.status === "ok") {
        const pts = (mt.loss || []).map((p2) => p2.loss);
        let sparkHtml = `<span class="lbl">trainer loss</span><svg class="spark" viewBox="0 0 100 34" preserveAspectRatio="none"></svg>`;
        if (pts.length > 1) {
          const lo = Math.min(...pts), hi = Math.max(...pts), rng = (hi - lo) || 1;
          const path = pts.map((v2, i2) =>
            `${(i2 / (pts.length - 1) * 100).toFixed(1)},${(32 - (v2 - lo) / rng * 28).toFixed(1)}`).join(" ");
          sparkHtml = `<span class="lbl">trainer loss ${pts[pts.length - 1].toFixed(3)} (${pts.length} samples)</span>` +
            `<svg class="spark" viewBox="0 0 100 34" preserveAspectRatio="none"><polyline points="${path}" fill="none" stroke="currentColor" stroke-width="1.2"/></svg>`;
        }
        const hist = mt.verdict_hist || [];
        const hmax = Math.max(1, ...hist);
        const histHtml = `<span class="lbl">served verdicts (0 → 1)</span><div class="hist">` +
          hist.map((n2, i2) => `<span style="height:${Math.max(2, n2 / hmax * 34)}px" title="${(i2 / 10).toFixed(1)}–${((i2 + 1) / 10).toFixed(1)}: ${n2}"></span>`).join("") + `</div>`;
        const gateHtml = `<span class="lbl">gates</span><div class="gatebar">` +
          (mt.gate_total ? `${mt.gate_pass}/${mt.gate_total} promoted<span class="marks">` +
            (mt.gates || []).slice(-12).map((g2) => g2.verdict === "promote" ? "▲" : "·").join("") + `</span>` : "no gates yet") + `</div>`;
        html += `<div class="trends"><div class="trend">${sparkHtml}</div><div class="trend">${histHtml}</div><div class="trend">${gateHtml}</div></div>`;
        // perception rate from the verdict journal's span, frontier spend
        // from the executive's counters, per-sensor rows as reported
        const hrs = (mt.span_ms || 0) / 3600000;
        const sc = ex.sensor_counts || {};
        html += mKv([
          ["perception rate", hrs > 0.01 ? `${(mt.verdict_total / hrs).toFixed(1)}/hr` : "—"],
          ["frontier calls (esc + audit)", `${(ex.escalations ?? 0) + (ex.audits ?? 0)}` +
            (hrs > 0.01 ? ` · ${(((ex.escalations ?? 0) + (ex.audits ?? 0)) / hrs).toFixed(1)}/hr` : "")],
          ...Object.entries(sc).map(([n2, r2]) =>
            [`sensor ${n2}`, `${r2.count ?? 0} perceived · ${r2.fast ?? 0} fast · ${r2.deep ?? 0} deep`]),
        ]);
      }
      const st = strip("stats");
      if (st) st.innerHTML = html;
    }

    // sidebar status dots (spec §2)
    mDots = {
      conversation: "ok",
      activity: !ex.running ? "off" : ((planAutonomyOn && !pw.alive) ? "err" : "ok"),
      learning: mode.startsWith("nanochat") ? "ok" : (mode === "stub" ? "warn" : "err"),
      personality: (mode !== "down" && mode !== "stub") ? "ok" : "off",
      insights: "", advanced: "",
    };

    renderCfg();
    mindRefresh();
  }

  // ── draft → Review → Apply (spec §5). One entry per changed key plus
  // the persona corpus; each change runs as its own existing command, in
  // review order with the persona last; failures stay in the draft. ────
  function mChangesList() {
    const out = Object.keys(mDraft).map((k) => ({
      k, label: M_LABEL[k] || k,
      from: mPretty(k, mBaseVal(k)), to: mPretty(k, mDraft[k]),
      when: M_TAKES(k), cmd: mCmdOf(k), sec: mSectionOf(k),
    }));
    if (personaDirtyNow() || (pSeed && pAdopt)) {
      const nb = personaBaseStr ? personaBaseStr.split("\n").length : 0;
      const nd = personaLines(personaRows).lines.length;
      out.push({ k: "@persona", label: "Personality examples",
        from: pSeed ? "default seed" : `${nb} rows`,
        to: personaDirtyNow() ? `${nd} rows` + (nb === nd ? " (edited)" : "") : `${nd} rows (adopt)`,
        when: "the service's next gate tick", cmd: "agent.model.persona_write (whole corpus, atomic)", sec: "personality" });
    }
    return out;
  }
  function mErrors() {
    const e = [];
    const isInt = (v) => /^\d+$/.test(String(v).trim());
    if ("@drive" in mDraft && !isInt(mDraft["@drive"])) e.push("drive must be a number ≥ 0");
    if ("MAX_PLAN_STEPS" in mDraft && mDraft.MAX_PLAN_STEPS !== "" && !(isInt(mDraft.MAX_PLAN_STEPS) && Number(mDraft.MAX_PLAN_STEPS) >= 1)) e.push("max steps must be a number ≥ 1");
    if ("PLAN_STEPS_PER_HOUR" in mDraft && mDraft.PLAN_STEPS_PER_HOUR !== "" && !isInt(mDraft.PLAN_STEPS_PER_HOUR)) e.push("plan steps/hr must be a number ≥ 0 (0 = unlimited)");
    for (const k of Object.keys(mDraft)) if (/[\r\n]/.test(mDraft[k])) e.push(`${M_LABEL[k] || k}: value must be a single line (set_setting rejects newlines)`);
    const miss = mProviderErrors();
    if (miss.length) e.push(`The chat provider needs ${miss.join(" and ")}.`);
    if (personaBadNow()) e.push("a raw row is not valid JSON — fix it before saving");
    return e;
  }
  function mShowReview() {
    const rev = host.querySelector(".mx-review");
    if (!rev) return;
    const ch = mChangesList(), errs = mErrors();
    rev.innerHTML = `<h3>Review your changes</h3><p>Check what changes and when it takes effect. Nothing is saved until you apply.</p>` +
      (errs.length ? `<div class="note err">${errs.map(mEsc).join("<br>")}</div>` : "") +
      ch.map((c) => `<div class="chg"><strong>${mEsc(c.label)}</strong><span class="k">${mEsc(c.cmd)}</span>` +
        `<div class="vals"><span>${mEsc(c.from)}</span><span aria-hidden="true">→</span><b>${mEsc(c.to)}</b></div>` +
        `<small>Takes effect: ${mEsc(c.when)} · <a href="#" data-goto="${c.sec}" style="color:var(--text-dim)">edit</a></small></div>`).join("") +
      `<div class="btnrow" style="margin-top:20px"><button data-act="back">Keep editing</button>` +
      `<button class="primary" data-act="apply" ${errs.length ? "disabled" : ""}>Apply ${ch.length} ${ch.length === 1 ? "change" : "changes"}</button></div>`;
    host.querySelector(".mx-panes").hidden = true;
    rev.hidden = false;
  }
  function mBack() {
    const rev = host.querySelector(".mx-review");
    if (rev) rev.hidden = true;
    const panes = host.querySelector(".mx-panes");
    if (panes) panes.hidden = false;
  }
  async function mApply() {
    const ch = mChangesList();
    const out = [];
    for (const c of ch) {
      let ok = false, msg = "";
      if (c.k === "@persona") {
        const pl = personaLines(personaRows);
        if (pl.bad) { msg = "a raw row is not valid JSON — fix it before saving"; }
        else {
          const r = mEnv(await invoke("agent", "model", "persona_write", { content: pl.lines.join("\n") }));
          ok = !!(r && r.status === "ok");
          msg = ok ? `saved ${r.rows} rows (${r.heldout} held out for the probe)` : "save failed: " + ((r && r.msg) || "no reply");
        }
      } else if (c.k === "@drive") {
        const n = parseInt(mDraft[c.k], 10);
        const r = mEnv(await invoke("agent", "executive", "set_drive", { acts_per_hour: n }));
        ok = !!(r && r.status === "ok");
        msg = ok ? `drive set to ${n}/hr` : "set_drive failed";
        if (ok) { mDrive = String(n); delete mDraft[c.k]; }
      } else {
        const v = mDraft[c.k];
        const r = mEnv(await invoke("agent", "model", "set_setting", { key: c.k, value: v }));
        ok = !!(r && r.status === "ok");
        if (c.k === "MAX_PLAN_STEPS") msg = ok ? (v ? `max plan steps set to ${v}/item` : "max plan steps back to the default (20/item)") : "set_setting failed";
        else if (c.k === "PLAN_STEPS_PER_HOUR") msg = ok ? (v ? `plan worker capped at ${v} steps/hr` : "plan worker hourly cap removed (unlimited)") : "set_setting failed";
        else msg = ok ? `${c.k} ${(r && r.action) || "updated"} — ${r && r.action === "removed" ? "reverted to default" : "saved"}` : "save failed: " + ((r && r.msg) || "no reply");
        if (ok) {
          if (mSet[c.k]) { mSet[c.k].v = v; mSet[c.k].set = v !== ""; }
          else mSet[c.k] = { v, d: "", t: "", set: v !== "", locked: false };
          delete mDraft[c.k];
        }
      }
      out.push({ c, ok, msg });
    }
    const good = out.filter((o) => o.ok).length;
    const rev = host.querySelector(".mx-review");
    rev.innerHTML = `<h3>${good === out.length ? "All changes applied" : `Applied ${good} of ${out.length} changes`}</h3>` +
      `<p>${good === out.length ? "Each one takes effect as listed." : "What failed is still in your draft, so you can fix it and try again."}</p>` +
      out.map((o) => `<div class="chg"><strong>${o.ok ? '<span style="color:#6c9">✓</span>' : '<span style="color:#c66">✗</span>'} ${mEsc(o.c.label)}</strong><span class="k">${mEsc(o.c.cmd)}</span>` +
        `<div class="vals" style="${o.ok ? "" : "color:#c66"}">${mEsc(o.msg)}</div>${o.ok ? `<small>Takes effect: ${mEsc(o.c.when)}</small>` : ""}</div>`).join("") +
      `<div class="btnrow" style="margin-top:20px"><button class="primary" data-act="back">Done</button></div>`;
    mToast(good ? `Applied ${good} ${good === 1 ? "change" : "changes"}` : "");
    if (out.some((o) => o.c.k === "@persona" && o.ok)) await loadPersona(true);
    loadMind();
  }

  function note(el, text, isErr) {
    el.textContent = text;
    el.classList.toggle("err", !!isErr);
    setTimeout(() => { if (el.textContent === text) el.textContent = ""; }, 6000);
  }

  return { dispose() { window.removeEventListener("resize", onWinResize); } };
}

  return init(ME, ME.DATA || {});
}).catch(function (e) {
  console.log("chat failed to start: " + (e && e.message ? e.message : e));
  return null;
});
readyP.then(function (api) { if (api) Object.assign(me, api); });
me.waitReady = function (cb) { readyP.then(function () { cb(me); }); };
