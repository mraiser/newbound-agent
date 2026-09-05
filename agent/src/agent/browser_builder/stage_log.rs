use ndata::dataobject::DataObject;
use flowlang::datastore::DataStore;
use std::path::Path;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["tail_lines"] {
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
        let arg_0: i64 = o.get_int("tail_lines");
        stage_log(arg_0)
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

pub fn stage_log(tail_lines: i64) -> DataObject {
// Light polling payload: the running flag + a tail of build.log. Cheaper
// than builder_status for the wizard's log pane while a stage runs.
fn prop(key: &str, dflt: &str) -> String {
    (|| -> Option<String> {
        let r = DataStore::globals().try_get_object("system").ok()?
            .try_get_object("apps").ok()?.try_get_object("agent").ok()?
            .try_get_object("runtime").ok()?;
        match r.try_get_string(key) { Ok(v) if !v.trim().is_empty() => Some(v.trim().to_string()), _ => None }
    })().unwrap_or_else(|| dflt.to_string())
}
let workspace = prop("NOOBSCAPE_WORKSPACE", "/newbound/runtime/agent/noobscape-build");
let n: usize = if tail_lines > 0 { tail_lines as usize } else { 60 };

let pid = std::fs::read_to_string(format!("{}/build.pid", workspace)).ok()
    .and_then(|s| s.trim().parse::<i64>().ok()).unwrap_or(0);
let running = pid > 0 && std::fs::read_to_string(format!("/proc/{}/stat", pid)).ok()
    .and_then(|st| st.rsplit(')').next().map(|r| r.trim_start().chars().next() != Some('Z')))
    .unwrap_or(false);
let stage = std::fs::read_to_string(format!("{}/build.stage", workspace)).unwrap_or_default().trim().to_string();
// Bounded tail: read only the last 256KB and treat \r as a newline, so a
// giant log or a progress-meter mega-line can never blow up the response.
let log = {
    use std::io::{Read, Seek, SeekFrom};
    match std::fs::File::open(format!("{}/build.log", workspace)) {
        Ok(mut f) => {
            let len = f.metadata().map(|m| m.len()).unwrap_or(0);
            let cap: u64 = 262144;
            if len > cap { let _ = f.seek(SeekFrom::Start(len - cap)); }
            let mut buf = Vec::new();
            let _ = f.read_to_end(&mut buf);
            String::from_utf8_lossy(&buf).replace('\r', "\n")
        }
        Err(_) => String::new(),
    }
};
let lines: Vec<&str> = log.lines().filter(|l| !l.trim().is_empty()).collect();
let tail: String = lines.iter().rev().take(n).rev().cloned().collect::<Vec<_>>().join("\n");
let tail: String = if tail.len() > 8000 {
    let mut i = tail.len() - 8000;
    while !tail.is_char_boundary(i) { i += 1; }
    tail[i..].to_string()
} else { tail };
let last_exit = lines.iter().rev().find_map(|l| l.strip_prefix("===NOOBSCAPE_STAGE_EXIT ").map(|s| s.trim_end_matches('=').trim().to_string())).unwrap_or_default();

let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_boolean("running", running);
o.put_int("pid", pid);
o.put_string("stage", &stage);
o.put_string("last_exit", &last_exit);
o.put_string("log_tail", &tail);
o
}
