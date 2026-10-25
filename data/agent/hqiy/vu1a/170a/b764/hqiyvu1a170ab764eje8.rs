// Internal activity sink. The model is not given this command as a tool; the
// execution loop emits facts at real model/tool boundaries. The runtime store
// is instance-local and bounded; kb.plan remains the durable intent/journal.
if run_id.trim().is_empty() || run_id.len() > 160 {
    let mut o = DataObject::new(); o.put_string("status", "err");
    o.put_string("msg", "activity_emit requires a bounded run_id"); return o;
}
let now = time();
let mut globals = DataStore::globals();
let boot = if let Ok(a) = globals.try_get_object("AGENT_PLAN_ACTIVITY") {
    a.try_get_int("boot").unwrap_or(0)
} else { 0 };
let boot = if boot == 0 {
    let mut a = globals.try_get_object("AGENT_PLAN_ACTIVITY").unwrap_or_else(|_| DataObject::new());
    a.put_int("boot", now); globals.put_object("AGENT_PLAN_ACTIVITY", a); now
} else { boot };
let store = DataStore::new();
let mut rec = if store.exists("runtime", "agent_plan_runs") {
    store.get_data("runtime", "agent_plan_runs")
} else {
    let mut r = DataObject::new(); r.put_string("id", "agent_plan_runs");
    r.put_string("username", "system"); r.put_array("readers", DataArray::new());
    r.put_array("writers", DataArray::new());
    r.put_object("data", { let mut d=DataObject::new(); d.put_array("runs", DataArray::new()); d }); r
};
let mut d = rec.get_object("data");
let mut runs = if d.has("runs") { d.get_array("runs") } else { DataArray::new() };
let mut found = -1i64;
for i in 0..runs.len() {
    if runs.get_object(i).try_get_string("run_id").unwrap_or_default() == run_id { found=i as i64; break; }
}
let mut r = if found >= 0 { runs.get_object(found as usize) } else {
    let mut r = DataObject::new(); r.put_string("run_id", &run_id);
    r.put_int("started", now); r.put_int("seq", 0); r.put_int("round", 0);
    r.put_int("tool_calls", 0); r.put_int("errors", 0); r.put_int("failure_streak", 0);
    r.put_int("duplicate_streak", 0); r.put_array("events", DataArray::new());
    r.put_boolean("active", true); r.put_int("boot", boot);
    runs.push_object(r.clone()); r
};
let kind = event.try_get_string("kind").unwrap_or_else(|_| "activity".to_string());
if kind == "start" {
    r.put_boolean("active", true); r.put_boolean("stale", false); r.put_int("boot", boot);
    r.put_int("started", now); r.put_int("pid", std::process::id() as i64);
    r.put_string("state", "preparing");
}
for k in ["claim", "phase", "state", "pending_tool", "outcome", "checkpoint"] {
    if event.has(k) { r.put_string(k, &event.get_string(k)); }
}
for k in ["turn", "round", "max_rounds", "tool_calls", "errors", "failure_streak", "duplicate_streak"] {
    if event.has(k) { r.put_int(k, event.get_int(k)); }
}
for k in ["context", "handoff", "pending_tool"] {
    if let Ok(v) = event.try_get_object(k) { r.put_object(k, v); }
}
if kind == "tool_result" || kind == "tool_skipped" {
    r.remove_property("pending_tool");
}
if kind == "finish" || kind == "checkpoint" {
    r.put_boolean("active", false); r.put_int("ended", now); r.remove_property("pending_tool");
}
r.put_int("updated", now); r.put_int("last_event_at", now);
if kind == "finish" || kind == "checkpoint" { r.put_string("phase", "finished"); }
let seq = r.try_get_int("seq").unwrap_or(0) + 1;
r.put_int("seq", seq);
let mut e = event.deep_copy();
e.put_int("seq", seq); e.put_int("time", now);
let mut events = r.get_array("events");
events.push_object(e);
while events.len() > 400 { events.remove_property(0); }
// Keep the newest eight runs. Transcript snapshots are separately bounded by
// tool_loop; expired run records may be reclaimed by runtime housekeeping.
while runs.len() > 8 { runs.remove_property(0); }
d.put_array("runs", runs); rec.put_object("data", d); rec.put_int("time", now);
store.set_data("runtime", "agent_plan_runs", rec);
let mut out = DataObject::new(); out.put_string("status", "ok");
out.put_string("run_id", &run_id); out.put_int("seq", seq); out.put_int("boot", boot); out