// scratchpad — the agent chat's live UI harness (agent.stage) end to end,
// against a REAL instance with a SCRIPTED model: chat_llm is answered
// in-page from a queue of canned replies, so everything else is the
// production path — chatTurn's tool loop, the "pad" grant, stage / eval /
// snapshot, the PNG upload, and the snapshot image riding the next model
// request. No LLM configuration or credits needed.
//
//   node tools/smoke/scratchpad.mjs --base http://localhost:33199 --dir DIR
import { smoke, login } from "./harness.mjs";

const BASE = (process.argv.includes("--base")
  ? process.argv[process.argv.indexOf("--base") + 1] : null) || "http://127.0.0.1:33199";

/** dev.code's commands by name over HTTP (ids are per-instance). */
async function devCode(base, sid) {
  const read = async (lib, id) => (await fetch(`${base}/app/read?lib=${lib}&id=${id}`,
    { headers: { Cookie: `sessionid=${sid}` } })).json();
  const ctls = await read("dev", "controls");
  const codeId = ctls.data?.list?.find((c) => c.name === "code")?.id;
  if (!codeId) throw new Error("no dev.code control on this instance");
  const ids = Object.fromEntries((await read("dev", codeId)).data.cmd.map((c) => [c.name, c.id]));
  return async (name, a) => {
    const r = await fetch(`${base}/app/exec?lib=dev&id=${ids[name]}&args=` +
      encodeURIComponent(JSON.stringify(a)), { headers: { Cookie: `sessionid=${sid}` } });
    const text = await r.text();
    try { return JSON.parse(text); } catch { return { status: "err", http: r.status, msg: text.slice(0, 200) }; }
  };
}

const COUNTER = {
  html: '<div class="ctr"><span class="n">0</span><button type="button" class="inc">+1</button><p class="hint">clicks so far</p></div>\n',
  css: ".ctr { display:flex; align-items:center; gap:12px; padding:12px; background:#24303c; border-radius:10px; width:260px; }\n" +
    ".n { font-size:32px; font-weight:700; color:#f0a63c; min-width:2ch; }\n" +
    ".inc { padding:6px 14px; border-radius:6px; border:1px solid #556; background:#3a4a5a; color:#fff; }\n",
  js: "var me = this;\nvar ME = document.getElementById(me.UUID);\nvar count = 0;\n" +
    "me.ready = function () {\n  count = (ME.DATA && ME.DATA.start) || 0;\n  const n = ME.querySelector('.n');\n  n.textContent = count;\n" +
    "  ME.querySelector('.inc').addEventListener('click', function () { count++; n.textContent = count; });\n};\n" +
    "me.count = function () { return count; };\n",
};

// canned model replies, in chat_llm's FLAT shape
let callN = 0;
const tc = (name, args) => ({ id: "c" + (++callN), name, arguments: JSON.stringify(args) });
const calls = (...cs) => ({ kind: "tool_calls", tool_calls: cs, assistant_message: { role: "assistant", content: "",
  tool_calls: cs.map((c) => ({ id: c.id, type: "function", function: { name: c.name, arguments: c.arguments } })) } });
const reply = (t) => ({ kind: "text", content: t, assistant_message: { role: "assistant", content: t } });

await smoke("agent chat scratchpad (real instance, scripted model)", async (t, browser) => {
  const sid = await login(BASE, { dir: t.args.dir, user: t.args.user, password: t.args.password });
  const code = await devCode(BASE, sid);
  const LIB = t.args.lib || "smoketest";
  await code("add_library", { lib: LIB });
  await code("create_control", { lib: LIB, name: "counter", desc: "scratchpad smoke fixture" });
  for (const [facet, src] of Object.entries(COUNTER)) {
    const w = await code("patch_control_facet", { lib: LIB, ctl: "counter", facet, old_snippet: "",
      new_snippet: src, base: "", label: "scratchpad smoke fixture", author: "smoke", nn_sessionid: "" });
    if (w.status !== "ok") throw new Error("fixture write failed: " + JSON.stringify(w).slice(0, 200));
  }

  const page = await t.page(browser, BASE, sid, { live: false });
  const openChat = async () => {
    await page.goto(`${BASE}/agent/index.html`, { waitUntil: "domcontentloaded" });
    await page.waitForFunction(() => document.querySelector('[data-control="agent:stage"]')?.api?.mount,
      null, { timeout: 30000 });
    await page.waitForTimeout(1000);   // the session list settles
    await page.evaluate(() => {
      const real = window.invokeCommand;
      window.__llm = { queue: [], log: [] };
      window.invokeCommand = function (lib, ctl, cmd, a, cb) {
        if (lib === "agent" && ctl === "llm" && cmd === "chat_llm") {
          window.__llm.log.push(JSON.parse(JSON.stringify(a.messages)));
          const next = window.__llm.queue.shift() || { kind: "text", content: "(script exhausted)" };
          setTimeout(() => cb({ status: "ok", data: next }), 20);
          return;
        }
        return real.apply(this, arguments);
      };
    });
  };
  const turn = async (prompt, script, done) => {
    await page.evaluate((s) => window.__llm.queue.push(...s), script);
    await page.fill(".ag-input", prompt);
    await page.click(".ag-send");
    const until = Date.now() + 60000;
    while (Date.now() < until) {
      if (await page.locator(".ag-confirm:not([hidden]) .ag-confirm-input:not([hidden])").count()) {
        await page.fill(".ag-confirm-input", "pad");
        await page.press(".ag-confirm-input", "Enter");
      }
      if ((await page.evaluate(() => document.querySelector(".ag-thread")?.lastElementChild?.textContent || "")).includes(done)) return;
      await page.waitForTimeout(250);
    }
    throw new Error("the turn never finished: " + done);
  };
  // the tool result of round k is the newest tool message of model call k+1
  const toolAt = async (k) => (await page.evaluate(() => window.__llm.log))[k]
    .filter((m) => m.role === "tool").at(-1)?.content ?? "";
  const frame = '.nb-stage-frame[data-name="counter"]';

  await openChat();
  await page.click(".ag-newsession");
  t.check("the pad starts hidden", await page.locator(".ag-pad[hidden]").count() === 1);

  // ── turn 1: stage → drive → look (calls 0–3)
  await turn("show me a counter", [
    calls(tc("stage", { name: "counter", spec: `${LIB}:counter:{"start":3,"note":"a:b"}` })),
    calls(tc("eval", { js: "const b = pad.counter.el.querySelector('button.inc');\nb.click(); b.click();\nreturn pad.counter.api.count()" })),
    calls(tc("snapshot", { name: "counter" })),
    reply("turn one done"),
  ], "turn one done");
  const staged = await toolAt(1);
  t.check("stage answers with the rendered structure (props JSON may hold colons)",
    /mounted: pad\.counter/.test(staged) && /button\.inc/.test(staged) && /"3"/.test(staged), staged.split("\n")[0]);
  t.check("eval drives the control: two clicks from 3 → 5", (await toolAt(2)).trim() === "5", await toolAt(2));
  t.check("snapshot returns the outline", /Snapshot of pad\.counter \(image \d+×\d+ attached\)/.test(await toolAt(3)));
  const log = await page.evaluate(() => window.__llm.log);
  const pic = log[3].at(-1);
  t.check("the PNG rides one user message after the tool results",
    pic.role === "user" && /\.png$/.test(pic.images?.[0] ?? ""), JSON.stringify(pic).slice(0, 120));
  t.check("the live counter in the pad shows 5", await page.locator(`.ag-pad:not([hidden]) ${frame} .n`).textContent() === "5");
  t.check("the thread shows the snapshot thumbnail", await page.locator(".ag-snap img.ag-snap-img").count() === 1);

  // ── turn 2: the mount persists across turns (calls 4–5)
  await turn("what does it show?", [
    calls(tc("eval", { js: "pad.counter.el.querySelector('.n').textContent;" })),
    reply("turn two done"),
  ], "turn two done");
  t.check("a lone expression (trailing ;) returns its value from the same live mount", await toolAt(5) === '"5"', await toolAt(5));
  const old = (await page.evaluate(() => window.__llm.log))[5].find((m) => String(m.content).startsWith("[SNAPSHOT IMAGES"));
  t.check("an earlier turn's snapshot pixels are not re-sent", old && !old.images);

  // ── between turns the "agent" edits the control; turn 3 (calls 6–10)
  await code("patch_control_facet", { lib: LIB, ctl: "counter", facet: "html", old_snippet: ">+1<",
    new_snippet: ">add one<", base: "", label: "scratchpad smoke edit", author: "smoke", nn_sessionid: "" });
  await turn("rename the button, then poke at errors", [
    calls(tc("stage", { name: "counter", spec: `${LIB}:counter:{"start":1}` })),
    calls(tc("eval", { js: "pad.counter.api.nope()" })),
    calls(tc("eval", { js: "setTimeout(() => { throw new Error('late boom') }, 0); return 1" })),
    calls(tc("eval", { js: "pad.missing.el" })),
    reply("turn three done"),
  ], "turn three done");
  t.check("restaging after a facet edit mounts the NEW code", /add one/.test(await toolAt(7)));
  const threw = await toolAt(8);
  t.check("a throwing eval reports name: message, not the harness's stack",
    /^ERROR: TypeError: .*nope/.test(threw) && !/padEval/.test(threw), threw.split("\n")[0]);
  t.check("an async error after an eval surfaces as a PAGE ERROR",
    /late boom/.test(await toolAt(9)) || /late boom/.test(await toolAt(10)));
  t.check("a missing mount says what is mounted", /nothing is mounted as missing — mounted: counter/.test(await toolAt(10)));
  // that error was thrown on purpose — it is the one page error that isn't a failure
  for (let i = t.errors.length - 1; i >= 0; i--) if (/late boom/.test(t.errors[i])) t.errors.splice(i, 1);

  await page.click(`${frame} button.inc`);
  t.check("the user can use the staged control too", await page.locator(`${frame} .n`).textContent() === "2");

  // ── reload: replay never re-runs control code on its own
  await openChat();
  t.check("after a reload the pad is empty", await page.locator(".nb-stage-frame").count() === 0);
  const remount = page.locator(".ag-stage-remount");
  t.check("the thread offers re-mount", await remount.count() >= 1);
  await remount.last().click();
  await page.waitForSelector(`.ag-pad:not([hidden]) ${frame} .n`, { timeout: 15000 });
  t.check("re-mount brings the control back", await page.locator(`${frame} .n`).textContent() === "1");
});
