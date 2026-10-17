// harness.mjs — shared plumbing for the browser smokes.
//
// These used to live in a session scratchpad and died with it, so every
// session paid to rewrite them. Moving them here meant fixing the three
// things that made them un-portable:
//
//   · hardcoded ports and absolute playwright/chromium paths → resolved
//     from flags or the environment, with a readable error when missing;
//   · a hardcoded session id → each run logs in for itself (sessions expire
//     in 15 minutes and a disposable instance restarts often);
//   · hardcoded record IDS — the killer. Command and control ids are
//     generated PER INSTANCE, so the scratchpad copies only ever worked on
//     the machine that produced them. Everything here resolves by NAME.
//
// Usage from a smoke:
//   import { smoke } from "./harness.mjs";
//   await smoke("my check", async (t) => { ... });
//
// Flags (all optional): --base http://host:port   running instance
//                       --dir  /path/to/instance  to read admin.properties
//                       --user / --password       instead of --dir
//                       --chromium /path/to/chrome
//                       --keep                    leave the browser open
import { readFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";

export function args() {
  const a = { _: [] };
  const argv = process.argv.slice(2);
  for (let i = 0; i < argv.length; i++) {
    if (argv[i].startsWith("--")) {
      const k = argv[i].slice(2);
      const v = argv[i + 1] && !argv[i + 1].startsWith("--") ? argv[++i] : true;
      a[k] = v;
    } else a._.push(argv[i]);
  }
  return a;
}

/** Playwright lives wherever the host put it; try the package first, then
    common locations, then an explicit --playwright path. */
export async function playwright(opts = {}) {
  const tries = [
    opts.playwright || process.env.PLAYWRIGHT_MODULE,
    "playwright",
    "/opt/node22/lib/node_modules/playwright/index.mjs",
    "/usr/lib/node_modules/playwright/index.mjs",
  ].filter(Boolean);
  for (const t of tries) {
    try { return await import(t); } catch { /* next */ }
  }
  throw new Error("playwright not found — install it, or pass --playwright " +
    "/path/to/playwright/index.mjs (tried: " + tries.join(", ") + ")");
}

/** A chromium that can do WebGL headlessly. The sandbox ships one; elsewhere
    playwright's own download is used by leaving executablePath unset. */
export function chromiumOpts(opts = {}) {
  const explicit = opts.chromium || process.env.SMOKE_CHROMIUM;
  const known = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
  const exe = explicit || (existsSync(known) ? known : null);
  return {
    ...(exe ? { executablePath: exe } : {}),
    args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
  };
}

// ── talking to an instance ──────────────────────────────────────────────────

export function adminPassword(dir) {
  const p = resolve(dir, "users/admin.properties");
  if (!existsSync(p)) {
    throw new Error(`no ${p} — point --dir at an instance directory (the file ` +
      `is generated on that instance's first boot), or pass --user/--password`);
  }
  return readFileSync(p, "utf8").match(/^password=(.*)$/m)[1].trim();
}

/** Log in and return a session id. Each run gets its own. */
export async function login(base, { dir, user = "admin", password } = {}) {
  const pass = password || adminPassword(dir);
  const sid = "smoke" + process.pid + Math.floor(Date.now() % 100000);
  const r = await fetch(`${base}/app/login?user=${encodeURIComponent(user)}` +
    `&pass=${encodeURIComponent(pass)}`, { headers: { Cookie: `sessionid=${sid}` } })
    .then((x) => x.text());
  if (!/now logged in/.test(r)) throw new Error("login failed: " + r.slice(0, 160));
  return sid;
}

/** Resolve dev.code's command ids BY NAME — ids are per-instance, so this
    is the difference between a portable smoke and one that only ran here.
    (The editing surface moved from the agent library into the platform's
    static dev library; the name agentApi predates the move.) */
export async function agentApi(base, sid) {
  const read = async (lib, id) => (await fetch(
    `${base}/app/read?lib=${lib}&id=${id}`,
    { headers: { Cookie: `sessionid=${sid}` } })).json();
  const ctls = await read("dev", "controls");
  if (ctls.status !== "ok") throw new Error("cannot read dev controls: " + JSON.stringify(ctls).slice(0, 160));
  const codeId = ctls.data.list.find((c) => c.name === "code")?.id;
  if (!codeId) throw new Error("no dev.code control on this instance");
  const code = await read("dev", codeId);
  const ids = Object.fromEntries(code.data.cmd.map((c) => [c.name, c.id]));
  const call = async (name, a = {}) => {
    if (!ids[name]) throw new Error(`dev.code.${name} missing on this instance`);
    const r = await fetch(`${base}/app/exec?lib=dev&id=${ids[name]}&args=` +
      encodeURIComponent(JSON.stringify(a)), { headers: { Cookie: `sessionid=${sid}` } });
    const text = await r.text();
    // a command that panics server-side answers with an HTML 500, not JSON —
    // surface that as an error value instead of exploding the whole run
    try { return JSON.parse(text); }
    catch { return { status: "err", http: r.status, msg: text.slice(0, 200) }; }
  };
  return { ids, call, read };
}

// ── the smoke wrapper: checks, error capture, exit code ─────────────────────

export async function smoke(title, body) {
  const a = args();
  const pw = await playwright(a);
  const errors = [];
  const results = [];
  const t = {
    args: a,
    errors,
    check(name, cond, extra = "") {
      results.push({ name, ok: !!cond, extra });
      console.log(`${cond ? "PASS" : "FAIL"}  ${name}${extra ? "  — " + extra : ""}`);
      if (!cond) errors.push(name);
    },
    note(msg) { console.log("      " + msg); },
    /** A page with the session cookie set and console/page errors
        collected as failures. */
    async page(browser, base, sid, { viewport } = {}) {
      const ctx = await browser.newContext({ viewport: viewport || { width: 1440, height: 900 } });
      if (sid) await ctx.addCookies([{ name: "sessionid", value: sid, url: base }]);
      const page = await ctx.newPage();
      page.on("pageerror", (e) => { errors.push(`pageerror: ${e.message}`); console.log("  !! " + e.message); });
      // The browser asks for /favicon.ico unprompted and the repo has none.
      // Answer it here rather than filtering the resulting console error:
      // that message carries no url, so text-matching it would also swallow
      // real 404s — which are exactly what these smokes exist to catch.
      await page.route("**/favicon.ico", (r) => r.fulfill({ status: 204, body: "" }));
      page.on("console", (m) => {
        if (m.type() !== "error") return;
        const text = m.text();
        const url = m.location()?.url || "";
        errors.push(`console: ${text}${url ? ` (${url})` : ""}`);
        console.log("  !! " + text + (url ? ` (${url})` : ""));
      });
      return page;
    },
  };
  console.log(`\n── ${title} ──`);
  const browser = await pw.chromium.launch(chromiumOpts(a));
  let thrown = null;
  try {
    await body(t, browser, pw);
  } catch (e) {
    thrown = e;
    errors.push(`threw: ${e.message}`);
    console.log("FAIL  " + e.message);
  } finally {
    if (!a.keep) await browser.close();
  }
  const passed = results.filter((r) => r.ok).length;
  console.log(errors.length
    ? `\n${title}: ${passed}/${results.length} checks, ${errors.length} problem(s)`
    : `\n${title}: ${passed}/${results.length} checks — NO ERRORS`);
  if (thrown && process.env.SMOKE_TRACE) console.error(thrown);
  process.exitCode = errors.length ? 1 : 0;
  return !errors.length;
}
