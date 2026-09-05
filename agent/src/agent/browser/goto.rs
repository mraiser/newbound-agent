use ndata::dataobject::DataObject;
use flowlang::datastore::DataStore;
use flowlang::flowlang::system::time::time;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["url"] {
        if !o.has(p) {
            let mut e = DataObject::new();
            e.put_string("status", "err");
            e.put_string("msg", &format!("missing required parameter: {}", p));
            let mut result_obj = DataObject::new();
            result_obj.put_object("a", e);
            return result_obj;
        }
    }
    let ax = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let arg_0: String = o.get_string("url");
        goto(arg_0)
    }));
    match ax {
        Ok(ax) => {
            let mut result_obj = DataObject::new();
    result_obj.put_object("a", ax);
            result_obj
        }
        Err(err) => {
            let mut err_obj = DataObject::new();
            err_obj.put_string("status", "err");

            let msg = if let Some(s) = err.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = err.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic occurred".to_string()
            };

            err_obj.put_string("msg", &msg);
            // Wrapped in the same `a` envelope a successful return uses.
            // Unwrapped, callers that unpack the envelope (newbound's
            // format_result, for one) report an opaque 500 — "Not an object:
            // DString(\"err\")" — instead of this message.
            let mut result_obj = DataObject::new();
            result_obj.put_object("a", err_obj);
            result_obj
        }
    }
}

pub fn goto(url: String) -> DataObject {
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
}
