use ndata::dataobject::DataObject;
use flowlang::datastore::DataStore;
use std::process::Command;
use std::path::Path;
pub fn execute(_: DataObject) -> DataObject {
    use std::panic;
    let ax = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        stop_build()
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

pub fn stop_build() -> DataObject {
// Terminate the running build: TERM the whole process group (setsid made
// the recorded pid a session/group leader, so -pid reaches mach and its
// children). Targeted — never a blanket pkill.
fn prop(key: &str, dflt: &str) -> String {
    (|| -> Option<String> {
        let r = DataStore::globals().try_get_object("system").ok()?
            .try_get_object("apps").ok()?.try_get_object("agent").ok()?
            .try_get_object("runtime").ok()?;
        match r.try_get_string(key) { Ok(v) if !v.trim().is_empty() => Some(v.trim().to_string()), _ => None }
    })().unwrap_or_else(|| dflt.to_string())
}
let workspace = prop("NOOBSCAPE_WORKSPACE", "/newbound/runtime/agent/noobscape-build");
let pidfile = format!("{}/build.pid", workspace);
let pid = std::fs::read_to_string(&pidfile).ok().and_then(|s| s.trim().parse::<i64>().ok()).unwrap_or(0);

let mut o = DataObject::new();
o.put_string("status", "ok");
if pid <= 0 || !Path::new(&format!("/proc/{}", pid)).exists() {
    o.put_boolean("killed", false);
    o.put_string("msg", "no running build");
    return o;
}
// Negative pid = process group.
let _ = Command::new("kill").arg("-TERM").arg(format!("-{}", pid)).status();
std::thread::sleep(std::time::Duration::from_millis(500));
let still = Path::new(&format!("/proc/{}", pid)).exists();
if still { let _ = Command::new("kill").arg("-KILL").arg(format!("-{}", pid)).status(); }
o.put_boolean("killed", true);
o.put_int("pid", pid);
o.put_string("msg", &format!("terminated build pid group {}", pid));
o
}
