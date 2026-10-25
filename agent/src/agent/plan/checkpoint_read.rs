use ndata::dataobject::DataObject;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["checkpoint", "fallback_text"] {
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
        let arg_0: DataObject = o.get_object("checkpoint");
        let arg_1: String = o.get_string("fallback_text");
        checkpoint_read(arg_0, arg_1)
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

pub fn checkpoint_read(checkpoint: DataObject, fallback_text: String) -> DataObject {
// Compatibility boundary: an empty checkpoint selects the legacy text. A v2
// checkpoint is already a DataObject throughout, with no Any conversion.
let mut out = DataObject::new();
let mut cp = checkpoint;
if cp.has("handoff") && !cp.has("summary") && !cp.has("next") { cp = cp.get_object("handoff"); }
let structured = cp.has("summary") || cp.has("next") || cp.has("text") || cp.has("checkpoint");
if structured {
    for k in ["version", "time", "kind", "status", "complete", "reason", "guidance", "summary", "next", "blockers", "verified", "verification_pending", "last_action", "last_result", "run_id", "previous_run_id", "turn", "round", "max_rounds", "tool_calls", "errors", "text", "checkpoint"] {
        if let Ok(v) = cp.try_get_property(k) { out.set_property(k, v); }
    }
    for k in ["observations", "changes", "evidence"] {
        if let Ok(v) = cp.try_get_array(k) { out.put_array(k, v); }
    }
    if !out.has("text") {
        let text = if cp.has("checkpoint") { cp.get_string("checkpoint") } else {
            let mut s = format!("PROGRESS: {}\nNEXT: {}", cp.try_get_string("summary").unwrap_or_default(), cp.try_get_string("next").unwrap_or_default());
            if let Ok(v) = cp.try_get_string("verified") { if !v.is_empty() { s.push_str(&format!("\nVERIFIED: {}", v)); } }
            if let Ok(v) = cp.try_get_string("verification_pending") { if !v.is_empty() { s.push_str(&format!("\nVERIFICATION PENDING: {}", v)); } }
            s
        };
        out.put_string("text", &text.chars().take(8000).collect::<String>());
    }
} else {
    out.put_int("version", 1); out.put_string("text", &fallback_text);
    out.put_string("summary", &fallback_text);
    out.put_string("next", "Read the legacy checkpoint and continue from its evidence; do not assume missing fields were verified.");
}
out.put_boolean("structured", structured);
out
}
