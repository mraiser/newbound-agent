// Read-only live/recent autonomous run activity. Activity is deliberately not
// stored in kb.plan: tool events must not churn its concurrency hash or make
// every board poll ship a transcript. The runtime records are instance-local;
// this read API is the UI and next-turn agent's common observation boundary.
fn preview(s: &str, max: usize) -> String {
    let mut out = String::new();
    for c in s.chars().take(max) { if c != '\n' && c != '\r' { out.push(c); } else { out.push(' '); } }
    out
}
fn copy_summary(src: &DataObject) -> DataObject {
    let mut r = DataObject::new();
    for k in ["run_id", "claim", "turn", "phase", "state", "active", "stale", "pid",
              "started", "updated", "last_event_at", "round", "max_rounds", "tool_calls",
              "errors", "failure_streak", "duplicate_streak", "pending_tool", "checkpoint",
              "context", "finish", "handoff", "ended", "outcome"] {
        if let Ok(v) = src.try_get_property(k) { r.set_property(k, v); }
    }
    if let Ok(v) = src.try_get_object("pending_tool") { r.put_object("pending_tool", v); }
    if let Ok(v) = src.try_get_object("checkpoint") { r.put_object("checkpoint", v); }
    if let Ok(v) = src.try_get_object("context") { r.put_object("context", v); }
    if let Ok(v) = src.try_get_object("handoff") { r.put_object("handoff", v); }
    r
}
let now = time();
let mut g = DataStore::globals();
let boot = g.try_get_object("AGENT_PLAN_ACTIVITY").ok()
    .and_then(|a| a.try_get_int("boot").ok()).unwrap_or(0);
let store = DataStore::new();
let mut runs: Vec<DataObject> = Vec::new();
if store.exists("runtime", "agent_plan_runs") {
    let d = store.get_data("runtime", "agent_plan_runs").get_object("data");
    if let Ok(a) = d.try_get_array("runs") {
        for i in 0..a.len() { if let Ok(r) = a.try_get_object(i) { runs.push(r); } }
    }
}
// A live run from before this service boot is interrupted, never active. Do
// not mutate stored state merely because somebody opened the Plan tab.
for r in runs.iter_mut() {
    let active = r.try_get_boolean("active").unwrap_or(false);
    let run_boot = r.try_get_int("boot").unwrap_or(0);
    let recorded_pid = r.try_get_int("pid").unwrap_or(0);
    let live_pid = recorded_pid > 0 && recorded_pid == std::process::id() as i64;
    if active && (!live_pid || (run_boot != 0 && boot != 0 && run_boot != boot)) {
        r.put_boolean("active", false);
        r.put_boolean("stale", true);
        r.put_string("state", "interrupted");
        r.put_string("outcome", "The service or worker stopped before this run reached its finish boundary.");
    }
}
runs.sort_by(|a, b| b.try_get_int("updated").unwrap_or(0).cmp(&a.try_get_int("updated").unwrap_or(0)));
let mut summaries = DataArray::new();
let mut recent = DataArray::new();
for r in runs.iter().take(12) { recent.push_object(copy_summary(r)); }
let active = runs.iter().find(|r| r.try_get_boolean("active").unwrap_or(false)
    && r.try_get_int("pid").unwrap_or(0) == std::process::id() as i64
    && (r.try_get_int("boot").unwrap_or(0) == 0 || boot == 0 || r.try_get_int("boot").unwrap_or(0) == boot));
if let Some(r) = active {
    summaries.push_object(copy_summary(r));
    let mut events = DataArray::new();
    let all = r.try_get_array("events").unwrap_or_else(|_| DataArray::new());
    let cap = limit.clamp(1, 400) as usize;
    let mut n = 0usize;
    for i in 0..all.len() {
        let e = all.get_object(i);
        if e.try_get_int("seq").unwrap_or(0) <= after { continue; }
        let mut v = DataObject::new();
        for k in ["seq", "time", "kind", "phase", "round", "tool", "target", "ok", "streak", "detail"] {
            if let Ok(x) = e.try_get_property(k) { v.set_property(k, x); }
        }
        if let Ok(x) = e.try_get_string("detail") { v.put_string("detail", &preview(&x, 4000)); }
        events.push_object(v);
        n += 1;
    }
    let mut out = DataObject::new();
    out.put_string("status", "ok");
    out.put_int("now", now);
    out.put_int("boot", boot);
    out.put_int("event_cap", 400);
    out.put_boolean("truncated", n > cap);
    if n > cap {
        let mut kept = DataArray::new();
        for i in (n-cap)..n { kept.push_object(events.get_object(i)); }
        out.put_array("events", kept);
    } else { out.put_array("events", events); }
    out.put_array("recent", recent);
    return out;
}
let mut out = DataObject::new();
out.put_string("status", "ok");
out.put_int("now", now);
out.put_int("boot", boot);
out.put_int("event_cap", 400);
out.put_array("events", DataArray::new());
out.put_array("recent", recent);
out