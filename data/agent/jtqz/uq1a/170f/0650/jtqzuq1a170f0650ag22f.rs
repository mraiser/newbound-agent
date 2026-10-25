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