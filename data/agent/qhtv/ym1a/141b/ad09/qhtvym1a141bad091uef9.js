// stage — the in-chat UI demo/test harness. Mounts live controls into chat
// thread cells (specs the agent sends via its `stage` tool) and evaluates
// in-page JS against them (the agent.stage.eval command polls requests /
// posts results — see eval_pump below). window.NB_STAGE.registry maps
// stage0, stage1, ... to {el, api, spec} for eval code to drive.

var me = this;
var ME = document.getElementById(me.UUID);

var readyP = new Promise(function (res) { me.ready = res; }).then(function () {
  const REG = (window.NB_STAGE = window.NB_STAGE || { registry: {}, n: 0 });

  // ---- mount a control into this control's own host (the mount API takes
  // (host, lib, ctl, cb, props) — see app.app's installControl call sites).
  // Returns a Promise of {el, api}; rejects with .status "unknown_control".
  function mount(spec) {
    return new Promise(function (resolve, reject) {
      let lib, ctl, props;
      try {
        const i = spec.indexOf(":"), j = spec.lastIndexOf(":");
        if (i <= 0 || j <= i) throw new Error("spec must be lib:ctl:{json}");
        lib = spec.slice(0, i);
        ctl = spec.slice(i + 1, j);
        props = JSON.parse(spec.slice(j + 1));
      } catch (e) { reject(e); return; }
      json(
        "../app/read",
        "lib=" + encodeURIComponent(lib) + "&id=controls",
        function (r) {
          const list = (r && r.status === "ok" && r.data && r.data.list) || [];
          const ok = list.some(function (c) {
            try { return (typeof c === "string" ? JSON.parse(c) : c).name === ctl; }
            catch (e) { return false; }
          });
          if (!ok) {
            const e = new Error("unknown control " + lib + ":" + ctl);
            e.status = "unknown_control";
            reject(e);
            return;
          }
          const el = document.createElement("div");
          el.className = "nb-stage-instance";
          ME.querySelector(".nb-stage-host").appendChild(el);
          installControl(el, lib, ctl, function (api) {
            resolve({ el: el, api: api });
          }, props);
        }
      );
    });
  }

  // ---- the eval bridge: a request queue + a pending long-poll. The backend
  // command (agent.stage.eval) enqueues, this page executes, and whichever
  // side holds the poll delivers the result.
  let nextReq = null;                 // a request waiting for the page
  let pending = null;                 // {res, timer} — the held long-poll
  const TIMEOUT_PAD = 5000;           // page-side grace past the cmd timeout

  function settle(value) {
    if (!pending) return false;
    clearTimeout(pending.timer);
    const p = pending;
    pending = null;
    p.res(value);
    return true;
  }

  function pump(req) {
    const reply = function (value) {
      if (!settle(value)) nextReq = { t: "res", id: req.id, value: value };
    };
    (async function () {
      try {
        const fn = new Function(
          "stage",
          '"use strict"; return (async () => { ' + req.js + "\n})();"
        );
        let v = await fn(REG.registry);
        reply(JSON.stringify(v === undefined ? null : v));
      } catch (e) {
        reply(JSON.stringify({ __err: String((e && e.stack) || e) }));
      }
    })();
  }

  // Called with {} by the command: claim a queued request, a queued result,
  // or hold the poll. Called with {id, value} by the page (never — the page
  // replies through pump's settle instead; the id/value path exists for a
  // direct HTTP post if needed).
  me.eval_pump = function (a) {
    if (a && a.id !== undefined && a.value !== undefined) {
      // a result arriving out of band — queue it for the matching poll
      nextReq = { t: "res", id: a.id, value: a.value };
      return { status: "ok" };
    }
    if (nextReq) {
      const r = nextReq;
      nextReq = null;
      if (r.t === "req") {
        pump(r);
        return { status: "accepted" };
      }
      return r; // a result for a poll that already left — deliver now
    }
    return new Promise(function (res) {
      pending = {
        res: res,
        timer: setTimeout(function () {
          pending = null;
          res({ status: "timeout" });
        }, (a && a.timeout_ms ? a.timeout_ms : 15000) + TIMEOUT_PAD),
      };
    });
  };

  // ---- public API (consumed by the chat control, which renders the cells)
  me.mount = mount;
  me.eval_pump = me.eval_pump;
  me.registry = REG.registry;
});
