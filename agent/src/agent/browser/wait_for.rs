use ndata::dataobject::DataObject;
use flowlang::flowlang::system::time::time;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["selector", "timeout_ms"] {
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
        let arg_0: String = o.get_string("selector");
        let arg_1: i64 = o.get_int("timeout_ms");
        wait_for(arg_0, arg_1)
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

pub fn wait_for(selector: String, timeout_ms: i64) -> DataObject {
// Poll until an element matching `selector` exists (or timeout). This is
// the stable primitive the run-ui skill's gotchas call for: wait on a
// real selector, never a fixed sleep. Returns {status, found}.
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
let tmo: i64 = if timeout_ms > 0 { timeout_ms } else { 10000 };
let js = format!("!!document.querySelector({})", js_string(&selector));
let deadline = time() + tmo;
let mut found = false;
while time() < deadline {
    let r = crate::agent::browser::eval::eval(js.clone(), 2000);
    if r.try_get_string("status").map(|s| s == "ok").unwrap_or(false) {
        if r.try_get_boolean("value").unwrap_or(false) { found = true; break; }
    }
    std::thread::sleep(std::time::Duration::from_millis(200));
}
let mut o = DataObject::new();
o.put_string("status", if found { "ok" } else { "err" });
o.put_boolean("found", found);
if !found { o.put_string("msg", &format!("selector not found within {} ms: {}", tmo, selector)); }
o
}
