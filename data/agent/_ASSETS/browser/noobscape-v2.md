# Noobscape v2 — a chrome-privileged browser driver for the agent

Noobscape is Marc's Firefox fork: a `nsDocShell` hook that watches a file
and evaluates its contents **as the system principal, inside the live
page** — bypassing CSP/CORS with no automation surface to fingerprint
(unlike geckodriver's content-sandbox `executeScript`). That is the moat.

v1 was one-way: it evaluated `/noobscape/inject.js` and **discarded the
result**, so callers (grabmore) had the injected JS write its own ad-hoc
result files, correlating by naming convention and 500 ms sleeps.

v2 keeps the moat and the one-file channel, adds three things, and stays
**fully back-compatible** with the v1 grabmore flow:

1. **Capture the result.** After `JS::Evaluate`, `JSON.stringify(rval)`
   is written to a response file — so injected JS is now just *an
   expression*, no self-written result files.
2. **Correlate.** A request carries an `id`; the response echoes it, so
   the reader knows the response is *its* response (kills the mtime race
   and the sleeps).
3. **Parameterize the channel dir** via env `NOOBSCAPE_DIR` (default
   `/noobscape`), so per-job/per-session browsers can coexist.

With the system principal, the verb set stays **two** (`eval`, `QUIT`):
navigate/click/read/type are all just `eval` of the right JS.

## The matched-pair protocol

Channel dir `D` = `$NOOBSCAPE_DIR` or `/noobscape`.

- **Request** — `D/inject.js`. A **v2** request is an `inject.js` whose
  **first line is exactly `//NOOBSCAPE`**; then line 2 is the correlation
  `id` and the remainder is the JS source. Any `inject.js` **without**
  that sentinel is treated as **v1 legacy** — evaluated as before, result
  discarded, `QUIT` force-quits — so grabmore is untouched.
- **Response** — `D/inject.out`, written atomically (tmp+rename), three
  lines: `id` / `OK`|`ERR` / payload. On `OK` the payload is
  `JSON.stringify(result)` (compact, single line — `undefined`->`null`,
  non-serialisable->`String(result)`); on `ERR` the payload is the thrown
  message.
- **Dedup**: v2 requests run at most once per `id` (a process-global
  `gNoobscapeLastId`; each Noobscape browser is its own process, so this
  is correctly per-session). This absorbs the immediate re-inject that
  fires on every page load / navigation.

The Rust side (`agent.browser`) writes the request tmp+rename, clears any
stale `inject.out` first, then polls for a response whose id matches.

---

## The C++ patch (against the attached `nsDocShell.cpp`)

Three edits. Line numbers are from the uploaded file; match on the
surrounding code, not the numbers.

### 1) Replace the `#define SCRIPT_FILE_PATH` block (~line 6194)

    // Noobscape v2 — channel dir from env, one watched request file.
    #include "mozilla/dom/Document.h"
    #include <fstream>
    #include <sstream>
    #include <cstdlib>   // getenv
    #include <cstdio>    // rename

    static std::string NoobscapeDir() {
      const char* e = getenv("NOOBSCAPE_DIR");
      std::string d = (e && *e) ? e : "/noobscape";
      if (!d.empty() && d.back() == '/') d.pop_back();
      return d;
    }
    static std::string NoobscapeReqPath() { return NoobscapeDir() + "/inject.js"; }
    static std::string NoobscapeOutPath() { return NoobscapeDir() + "/inject.out"; }

    // v2 request dedup — per-process, i.e. per Noobscape browser.
    static std::string gNoobscapeLastId;

    static void NoobscapeWriteOut(const std::string& id, bool ok,
                                  const std::string& payload) {
      std::string outPath = NoobscapeOutPath();
      std::string tmp = outPath + ".tmp";
      { std::ofstream f(tmp.c_str(), std::ios::binary | std::ios::trunc);
        if (!f) return;
        f << id << "\n" << (ok ? "OK" : "ERR") << "\n" << payload; }
      rename(tmp.c_str(), outPath.c_str());
    }

Then, in `FileWatcher::WatchFile()` and `nsDocShell::StartFileWatcher()`,
replace every `nsCString nativePath(SCRIPT_FILE_PATH);` with
`nsCString nativePath(NoobscapeReqPath().c_str());` and the
`printf(... SCRIPT_FILE_PATH)` with `NoobscapeReqPath().c_str()`.

### 2) In `StartFileWatcher` (~line 6496) — start the watcher even if the request file does not exist yet

The v2 request file won't exist until the driver writes one, so drop the
"skip if not exists" early return; keep the immediate inject guarded by
existence:

      bool exists = false;
      rv = file->Exists(&exists);      // keep, but DO NOT early-return on !exists
      // ... start the watcher unconditionally (WatchFile already no-ops on a
      // missing file) ...
      if (NS_SUCCEEDED(startResult)) {
        gFileWatchers[this] = watcher;
        if (exists) { InjectScriptIntoDOM(file); }   // only inject if present
      }

### 3) Replace `InjectScriptIntoDOM` (lines 6399–6465) wholesale

    #include "jsapi.h"
    #include "js/CompilationAndEvaluation.h"
    #include "js/SourceText.h"
    #include "js/JSON.h"            // JS_Stringify
    #include "js/Conversions.h"    // JS::ToString

    static bool NoobscapeJSONWrite(const char16_t* buf, uint32_t len, void* data) {
      static_cast<nsAutoString*>(data)->Append(buf, len);
      return true;
    }

    void nsDocShell::InjectScriptIntoDOM(nsIFile* file) {
      nsCString nativePath = file->NativePath();
      std::ifstream infile(nativePath.get(), std::ios::binary);
      if (!infile) return;
      std::stringstream ss; ss << infile.rdbuf();
      std::string raw = ss.str();

      // v2 sentinel: first line exactly "//NOOBSCAPE".
      bool v2 = false; std::string id, jsUtf8;
      {
        size_t nl1 = raw.find('\n');
        std::string l1 = (nl1 == std::string::npos) ? raw : raw.substr(0, nl1);
        while (!l1.empty() && l1.back() == '\r') l1.pop_back();
        if (l1 == "//NOOBSCAPE" && nl1 != std::string::npos) {
          v2 = true;
          size_t nl2 = raw.find('\n', nl1 + 1);
          if (nl2 == std::string::npos) { id = raw.substr(nl1 + 1); }
          else { id = raw.substr(nl1 + 1, nl2 - (nl1 + 1)); jsUtf8 = raw.substr(nl2 + 1); }
          while (!id.empty() && (id.back() == '\r' || id.back() == '\n')) id.pop_back();
        }
      }

      if (v2) {
        if (id == gNoobscapeLastId) return;   // run each id at most once
        gNoobscapeLastId = id;
      }

      nsAutoString scriptContent;
      scriptContent.Assign(NS_ConvertUTF8toUTF16((v2 ? jsUtf8 : raw).c_str()));
      scriptContent.Trim(" \t\r\n");

      if (scriptContent.EqualsLiteral("QUIT")) {
        if (v2) NoobscapeWriteOut(id, true, "null");
        ForceQuitBrowser();
        return;
      }

      nsIScriptSecurityManager* ssm = nsContentUtils::GetSecurityManager();
      if (!ssm) { if (v2) NoobscapeWriteOut(id, false, "no security manager"); return; }
      nsCOMPtr<nsIPrincipal> systemPrincipal;
      ssm->GetSystemPrincipal(getter_AddRefs(systemPrincipal));

      Document* doc = GetDocument();
      if (!doc) { if (v2) NoobscapeWriteOut(id, false, "no document"); return; }
      nsCOMPtr<nsPIDOMWindowOuter> outerWindow = GetWindow();
      if (!outerWindow) { if (v2) NoobscapeWriteOut(id, false, "no window"); return; }
      nsCOMPtr<nsPIDOMWindowInner> innerWindow = outerWindow->GetCurrentInnerWindow();
      if (!innerWindow) { if (v2) NoobscapeWriteOut(id, false, "no inner window"); return; }

      AutoJSAPI jsapi;
      if (!jsapi.Init(innerWindow)) { if (v2) NoobscapeWriteOut(id, false, "jsapi init failed"); return; }
      JSContext* cx = jsapi.cx();
      JS::RootedValue rval(cx);
      JS::CompileOptions options(cx);
      options.setFileAndLine("injected-script.js", 1);

      JS::SourceText<char16_t> source;
      if (!source.init(cx, scriptContent.get(), scriptContent.Length(),
                       JS::SourceOwnership::Borrowed)) {
        if (v2) NoobscapeWriteOut(id, false, "source init failed");
        return;
      }

      bool okEval = JS::Evaluate(cx, options, source, &rval);
      if (!v2) { if (!okEval) printf("Script execution failed.\n"); return; }  // legacy

      if (!okEval) {
        std::string msg = "script threw";
        JS::RootedValue exc(cx);
        if (JS_GetPendingException(cx, &exc)) {
          JS_ClearPendingException(cx);
          JS::RootedString es(cx, JS::ToString(cx, exc));
          if (es) { JS::UniqueChars ec = JS_EncodeStringToUTF8(cx, es);
                    if (ec) msg = ec.get(); }
        }
        NoobscapeWriteOut(id, false, msg);
        return;
      }

      if (rval.isUndefined()) { NoobscapeWriteOut(id, true, "null"); return; }
      nsAutoString jsonOut;
      JS::RootedValue space(cx);
      if (JS_Stringify(cx, &rval, nullptr, space, NoobscapeJSONWrite, &jsonOut)) {
        NoobscapeWriteOut(id, true, NS_ConvertUTF16toUTF8(jsonOut).get());
        return;
      }
      // Not JSON-serialisable (e.g. a DOM node): fall back to a JSON string of String(rval).
      JS_ClearPendingException(cx);
      JS::RootedString s(cx, JS::ToString(cx, rval));
      if (s) {
        JS::RootedValue sv(cx, JS::StringValue(s));
        nsAutoString sout;
        if (JS_Stringify(cx, &sv, nullptr, space, NoobscapeJSONWrite, &sout)) {
          NoobscapeWriteOut(id, true, NS_ConvertUTF16toUTF8(sout).get());
          return;
        }
      }
      NoobscapeWriteOut(id, true, "null");
    }

**Notes / drift risks.** `JS_Stringify` / `JSONWriteCallback`
(`bool(*)(const char16_t*, uint32_t, void*)`) and the `AutoJSAPI` /
`JS::Evaluate` signatures are the parts most likely to move between
Firefox releases; if this fork bumps, those three includes and two calls
are where to look. Everything else is plain std::string/file I/O.

**Screenshot** is deliberately NOT in this patch. A live-session
compositor capture is a proper future third verb (`SCREENSHOT <path>`
via `nsIDocShell`->presShell->gfx), worth adding once the eval core is
proven — that is what unlocks the screenshot->`agent-llm-chat_llm` vision
loop against the *logged-in* page. Draw-to-canvas from JS is the stopgap.

---

## The Rust side: the `agent.browser` control

One primitive, everything else composed from it:

- `eval(js, timeout_ms)` — write the v2 envelope (fresh id, tmp+rename),
  poll `inject.out` for the matching id, return `{status, value}` (value =
  the parsed JSON result) or `{status:err, msg}`.
- `open(url)` — seed the channel, spawn `exec firefox` (headless unless
  `BROWSER_DISPLAY` is set), record its pid, poll `eval` until the page
  is ready. Returns the pid.
- `goto`, `wait_for`, `text`, `click`, `type` — thin `eval` wrappers.
- `close()` — polite `QUIT`, then `kill -TERM` the recorded pid (targeted;
  never `pkill -9 firefox`).

Config keys (botd.properties / `system.apps.agent.runtime`):
`NOOBSCAPE_DIR` (default `/noobscape`), `NOOBSCAPE_BIN`
(default `/noobscape/bin/firefox`), `BROWSER_DISPLAY` (empty = headless).
