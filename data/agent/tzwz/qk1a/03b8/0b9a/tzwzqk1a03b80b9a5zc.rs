// Navigate the bound Noobscape session to `url` IN PLACE: evaluate a
// location.assign() in the driven tab and wait for the new document.
// This works because the mechanism evaluates under AutoEntryScript
// (script-initiated navigation commits) and the bind latch keys on the
// tab's stable BrowserId, so the eval channel follows the tab across the
// cross-process swap a navigation may cause. The old document's JS state
// is gone afterwards — that is what a navigation is — but the browser
// process, its history, and same-document (hash / SPA-route) state
// survive, and no relaunch is paid.
// Falls back to the legacy rebind path (close + relaunch at `url`) when
// no live session answers or the navigation never commits.
// Returns {status, pid, url}.
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
fn rebind(url: String) -> DataObject {
    let _ = crate::agent::browser::close::close();
    // close() TERMs the recorded pid; let the profile's single-instance lock
    // release before relaunch so firefox does not refuse it as already running.
    std::thread::sleep(std::time::Duration::from_millis(800));
    crate::agent::browser::open::open(url)
}

let dir = prop("NOOBSCAPE_DIR", "/noobscape");
let dir = dir.trim_end_matches('/').to_string();

// Encode the url as a JS string literal.
let mut esc = String::with_capacity(url.len() + 2);
esc.push('"');
for c in url.chars() {
    match c {
        '"' => esc.push_str("\\\""),
        '\\' => esc.push_str("\\\\"),
        '\n' => esc.push_str("\\n"),
        '\r' => esc.push_str("\\r"),
        c if (c as u32) < 0x20 => esc.push_str(&format!("\\u{:04x}", c as u32)),
        c => esc.push(c),
    }
}
esc.push('"');

// No live session answering? Rebind is the only way to get anywhere.
let probe = crate::agent::browser::eval::eval("1".to_string(), 2500);
if !probe.try_get_string("status").map(|s| s == "ok").unwrap_or(false) {
    return rebind(url);
}

// Plant a token on the OLD document, then navigate. The token dies with the
// old document; its absence marks the new one's arrival.
let nav = format!("window.__nbNavToken = true; location.assign({}); \"nav\"", esc);
let r = crate::agent::browser::eval::eval(nav, 8000);
if !r.try_get_string("status").map(|s| s == "ok").unwrap_or(false) {
    return rebind(url);
}

// Wait for the new document: token gone (or href already at the target —
// the same-document hash/route case, where nothing reloads) and readyState
// usable. Evals may time out mid process-swap; that is polling, not failure.
let poll = format!(
    "(window.__nbNavToken && location.href !== {}) ? \"old\" : document.readyState",
    esc
);
let deadline = time() + 30000;
let mut ready = false;
while time() < deadline {
    let r = crate::agent::browser::eval::eval(poll.clone(), 2000);
    if r.try_get_string("status").map(|s| s == "ok").unwrap_or(false) {
        if let Ok(v) = r.try_get_string("value") {
            if v == "complete" || v == "interactive" { ready = true; break; }
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
}
if !ready {
    // The assignment ran but no new document ever fronted the tab — the
    // load was blocked or hung. The rebind path still gets us there.
    return rebind(url);
}

let pid = std::fs::read_to_string(format!("{}/firefox.pid", dir))
    .ok().and_then(|s| s.trim().parse::<i64>().ok()).unwrap_or(0);
let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_int("pid", pid);
o.put_string("url", &url);
o