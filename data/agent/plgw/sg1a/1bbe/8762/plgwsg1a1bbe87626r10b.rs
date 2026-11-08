// Durable checkpoints are a bounded continuation facet, not a transcript.
// Whitelist rather than recursively copying: inherited checkpoints, boards,
// provider transcripts and arbitrary tool payloads must never re-enter here.
fn bounded(s: &str, n: usize) -> String { s.chars().take(n).collect() }
fn compact_observations(input: &DataObject, key: &str) -> DataArray {
    let mut out = DataArray::new();
    let source = match input.try_get_array(key) { Ok(a) => a, Err(_) => return out };
    let start = source.len().saturating_sub(8);
    for i in start..source.len() {
        let item = source.get_object(i);
        let mut row = DataObject::new();
        row.put_string("tool", &bounded(&item.try_get_string("tool").unwrap_or_default(), 240));
        row.put_string("target", &bounded(&item.try_get_string("target").unwrap_or_default(), 400));
        row.put_boolean("ok", item.try_get_boolean("ok").unwrap_or(false));
        row.put_string("result", &bounded(&item.try_get_string("result").unwrap_or_default(), 700));
        out.push_object(row);
    }
    out
}
let mut out = DataObject::new();
out.put_int("version", 2);
out.put_boolean("compact", true);
for (key, cap) in [
    ("kind", 80usize), ("status", 80usize), ("reason", 1000usize),
    ("guidance", 2000usize), ("summary", 4000usize), ("next", 2000usize),
    ("blockers", 2000usize), ("verified", 2000usize),
    ("verification_pending", 2000usize), ("last_action", 600usize),
    ("last_result", 1200usize), ("run_id", 180usize),
    ("previous_run_id", 180usize), ("text", 8000usize), ("checkpoint", 8000usize)
] {
    if let Ok(value) = checkpoint.try_get_string(key) {
        out.put_string(key, &bounded(&value, cap));
    }
}
for key in ["turn", "round", "max_rounds", "tool_calls", "errors"] {
    out.put_int(key, checkpoint.try_get_int(key).unwrap_or(0));
}
out.put_int("time", checkpoint.try_get_int("time").unwrap_or(0));
out.put_boolean("complete", checkpoint.try_get_boolean("complete").unwrap_or(false));
for key in ["observations", "changes", "evidence"] {
    out.put_array(key, compact_observations(&checkpoint, key));
}
out