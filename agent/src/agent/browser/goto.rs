use ndata::dataobject::DataObject;
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
// Navigate the current Noobscape session to `url` and wait for the new
// page to finish loading. Returns {status, url} (url = the final
// location after any redirects), or {status:err, msg}.
fn js_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// Navigation cannot go through the DOM (location.href / assign / pushState):
// the injected script carries the SYSTEM principal, so those APIs fail their
// same-origin / subject-principal check and silently veto. The mechanism
// recognizes this directive and performs a real top-level docshell load with
// a system triggering principal (the URL-bar path). js_string stays used by
// the escaping above; the directive body is a raw URL line, trimmed in C++.
let _ = js_string("");
let set = format!("NOOBSCAPE_NAV {}", url);
let r = crate::agent::browser::eval::eval(set, 5000);
if !r.try_get_string("status").map(|s| s == "ok").unwrap_or(false) {
    return r; // propagate the eval error
}

// The navigation tears down the inner window; give it a moment, then
// poll until the fresh document reports complete.
std::thread::sleep(std::time::Duration::from_millis(400));
let deadline = time() + 30000;
let mut loaded = false;
while time() < deadline {
    let s = crate::agent::browser::eval::eval("document.readyState".to_string(), 2000);
    if s.try_get_string("status").map(|x| x == "ok").unwrap_or(false) {
        if let Ok(v) = s.try_get_string("value") {
            if v == "complete" { loaded = true; break; }
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
}

let here = crate::agent::browser::eval::eval("location.href".to_string(), 2000);
let mut o = DataObject::new();
o.put_string("status", if loaded { "ok" } else { "err" });
o.put_string("url", &here.try_get_string("value").unwrap_or_else(|_| url.clone()));
if !loaded { o.put_string("msg", "navigation did not reach readyState=complete within timeout"); }
o
}
