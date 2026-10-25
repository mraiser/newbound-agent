use ndata::dataobject::DataObject;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["entry"] {
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
        let arg_0: DataObject = o.get_object("entry");
        continuity_context(arg_0)
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

pub fn continuity_context(entry: DataObject) -> DataObject {
// Pure, bounded continuity assembly for step_item. Keeping this separate
// makes the across-turn contract executable against synthetic entries without
// spending an LLM turn or touching the real board.
let logs = entry.try_get_array("step_log").unwrap_or_else(|_| ndata::dataarray::DataArray::new());
let n = logs.len();
let mut latest = -1i64;
for i in 0..n {
    let le = logs.get_object(i);
    if le.has("checkpoint") || le.try_get_string("result").unwrap_or_default().starts_with("[budget checkpoint") {
        latest = i as i64;
    }
}
// Legacy entries predate explicit checkpoints. Their latest final response is
// still the best available handoff; it must not fall back to a 200-char hint.
if latest < 0 && n > 0 { latest = n as i64 - 1; }
let mut parts: Vec<String> = Vec::new();
let mut previous_run_id = String::new();
let mut handoff_received = String::new();
let mut included = 0i64;
for i in 0..n {
    let le = logs.get_object(i);
    let is_latest = i as i64 == latest;
    let result = le.try_get_string("result").unwrap_or_default();
    let checkpoint = le.try_get_string("checkpoint").unwrap_or_default();
    let body = if !checkpoint.trim().is_empty() { checkpoint } else { result };
    let explicit = le.has("checkpoint") || body.starts_with("[budget checkpoint");
    let recent = i + 3 >= n;
    if !is_latest && (explicit || !recent) { continue; }
    let text: String = if is_latest { body.chars().take(8000).collect() } else { body.chars().take(1200).collect() };
    if text.trim().is_empty() { continue; }
    let home = le.try_get_string("home").unwrap_or_else(|_| "previous plan turn".to_string());
    let ok = le.try_get_boolean("ok").unwrap_or(false);
    let rid = le.try_get_string("run_id").unwrap_or_default();
    let suffix = if rid.is_empty() { String::new() } else { format!(" [run_id: {}]", rid) };
    parts.push(format!("- [{}] {}{} -> {}", if ok { "ok" } else { "err" }, home, suffix, text));
    included += 1;
    if is_latest {
        previous_run_id = rid;
        handoff_received = text;
    }
}
let mut out = DataObject::new();
out.put_string("prior_steps", &parts.join("\n"));
out.put_string("previous_run_id", &previous_run_id);
out.put_string("handoff_received", &handoff_received);
out.put_int("included_steps", included);
out.put_int("available_steps", n as i64);
out.put_int("chars", parts.join("\n").chars().count() as i64);
out
}
