// Launch a Noobscape-v2 browser at `url` and wait until the page is
// ready. Spawns firefox detached (via `exec`, so the recorded pid IS
// firefox), seeds the channel so the file-watcher starts on first load,
// and polls agent.browser.eval until document.readyState settles.
// Headless unless BROWSER_DISPLAY is set. Returns {status, pid, url}.
fn prop(key: &str, dflt: &str) -> String {
    (|| -> Option<String> {
        let s = DataStore::globals().try_get_object("system").ok()?;
        let a = s.try_get_object("apps").ok()?;
        let g = a.try_get_object("agent").ok()?;
        let r = g.try_get_object("runtime").ok()?;
        match r.try_get_string(key) {
            Ok(v) if !v.trim().is_empty() => Some(v.trim().to_string()),
            _ => None,
        }
    })().unwrap_or_else(|| dflt.to_string())
}
fn errobj(m: &str) -> DataObject {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", m);
    o
}

let dir = prop("NOOBSCAPE_DIR", "/noobscape");
let dir = dir.trim_end_matches('/').to_string();
let bin = prop("NOOBSCAPE_BIN", "/noobscape/bin/firefox");
let display = prop("BROWSER_DISPLAY", "");

// Ensure the channel dir exists and seed a no-op v2 request so the
// browser's file-watcher starts on the first page load.
if std::fs::create_dir_all(&dir).is_err() {
    return errobj(&format!("cannot create channel dir {}", dir));
}
let _ = std::fs::remove_file(format!("{}/inject.out", dir));
// Noobscape v2 bind channel: clear any prior latch and record the launch
// URL so the mechanism binds to the ONE tab that loads exactly this url.
let _ = std::fs::remove_file(format!("{}/bind.id", dir));
let _ = std::fs::write(format!("{}/bind.url", dir), &url);
let _ = std::fs::write(format!("{}/inject.js", dir), "//NOOBSCAPE\nseed\nvoid 0");

// Single-process profile. A chrome-principal docshell load (the goto
// mechanism) only COMMITS when the top content docshell is parent-owned,
// i.e. non-remote; on a multiprocess/Fission browser it queues in a content
// process and the parent drops it. Multiprocess also spawns the background
// tab-content docshells (thumbnailer, extensions, preallocated) that forced
// the original broadcast hack. One profile with remote + fission OFF
// collapses the browser to a single parent process: injection AND navigation
// both act on the real top docshell in-process, and the cross-process
// response race disappears (the bind latch becomes belt-and-suspenders).
let profile = format!("{}/profile", dir);
let _ = std::fs::create_dir_all(&profile);
// Clear any stale single-instance lock from a previously killed run so
// firefox does not refuse the profile as "already in use".
let _ = std::fs::remove_file(format!("{}/lock", profile));
let _ = std::fs::remove_file(format!("{}/.parentlock", profile));
let user_js = concat!(
    "user_pref(\"fission.autostart\", false);\n",
    "user_pref(\"browser.tabs.remote.autostart\", false);\n",
    "user_pref(\"dom.ipc.processCount\", 1);\n",
    "user_pref(\"browser.tabs.remote.separatePrivilegedContentProcess\", false);\n",
    "user_pref(\"toolkit.telemetry.enabled\", false);\n",
    "user_pref(\"datareporting.policy.dataSubmissionEnabled\", false);\n",
    "user_pref(\"browser.shell.checkDefaultBrowser\", false);\n",
    "user_pref(\"browser.startup.homepage_override.mstone\", \"ignore\");\n",
);
let _ = std::fs::write(format!("{}/user.js", profile), user_js);

// Build the launch line. `exec` replaces bash so the child pid is firefox.
let headless = if display.is_empty() { "-headless" } else { "" };
let disp = if display.is_empty() { String::new() } else { format!("DISPLAY='{}' ", display) };
// Assignments must precede `exec` (a builtin): `exec VAR=1 cmd` makes
// bash try to execute the file "VAR=1" and die with 127.
let line = format!(
    "{}MOZ_DISABLE_JEMALLOC=1 MOZ_DISABLE_CONTENT_SANDBOX=1 LIBGL_ALWAYS_SOFTWARE=1 exec '{}' {} -profile '{}' '{}'",
    disp, bin, headless, profile, url
);

let mut cmd = Command::new("bash");
cmd.arg("-c").arg(&line);
let pid = match cmd.spawn() {
    Ok(child) => child.id(),
    Err(e) => return errobj(&format!("spawn failed: {}", e)),
};
let _ = std::fs::write(format!("{}/firefox.pid", dir), pid.to_string());

// Poll for readiness: eval succeeds once the page's inner window exists,
// and readyState settles to interactive/complete when the DOM is usable.
let deadline = time() + 30000;
let mut ready = false;
while time() < deadline {
    let r = crate::agent::browser::eval::eval("document.readyState".to_string(), 2000);
    if r.try_get_string("status").map(|s| s == "ok").unwrap_or(false) {
        if let Ok(v) = r.try_get_string("value") {
            if v == "complete" || v == "interactive" { ready = true; break; }
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
}

let mut o = DataObject::new();
o.put_string("status", if ready { "ok" } else { "err" });
o.put_int("pid", pid as i64);
o.put_string("url", &url);
if !ready { o.put_string("msg", "browser did not become ready (is Noobscape v2 built and NOOBSCAPE_BIN correct?)"); }
o