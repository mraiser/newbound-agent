use ndata::dataobject::DataObject;
use flowlang::datastore::DataStore;
use std::process::Command;
pub fn execute(_: DataObject) -> DataObject {
    use std::panic;
    let ax = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        close()
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

pub fn close() -> DataObject {
// Close the current Noobscape session: first ask the browser to quit
// itself (the v2 QUIT sentinel), then, as a backstop, TERM the exact pid
// recorded by open() - never a blanket `pkill firefox`, so a second
// browser survives. Returns {status, killed}.
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
let dir = prop("NOOBSCAPE_DIR", "/noobscape");
let dir = dir.trim_end_matches('/').to_string();

// Polite quit (best effort; the browser dies before it can answer).
let _ = crate::agent::browser::eval::eval("QUIT".to_string(), 2000);
std::thread::sleep(std::time::Duration::from_millis(500));

// Backstop: TERM the recorded pid.
let mut killed = false;
let pidfile = format!("{}/firefox.pid", dir);
if let Ok(pids) = std::fs::read_to_string(&pidfile) {
    let pid = pids.trim().to_string();
    if !pid.is_empty() && pid.chars().all(|c| c.is_ascii_digit()) {
        let mut c = Command::new("kill");
        c.arg("-TERM").arg(&pid);
        killed = c.status().map(|s| s.success()).unwrap_or(false);
    }
}
let _ = std::fs::remove_file(&pidfile);
let _ = std::fs::remove_file(format!("{}/inject.out", dir));

let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_boolean("killed", killed);
o
}
