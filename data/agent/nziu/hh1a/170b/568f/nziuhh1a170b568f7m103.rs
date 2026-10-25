// Read a specific bounded run for the Plan tab or a later agent turn. Raw
// arguments are not emitted. Handoffs and tool outcomes are bounded; provider
// transcripts and raw arguments are not retained in these run records.
// After boot, active is recomputed from the run PID/boot rather than trusted
// from a stale record.
let now = flowlang::flowlang::system::time::time();
let boot_now = DataStore::globals().try_get_object("AGENT_PLAN_ACTIVITY").ok()
    .and_then(|a| a.try_get_int("boot").ok()).unwrap_or(0);
let mut out = DataObject::new();
let store = DataStore::new();
if !store.exists("runtime", "agent_plan_runs") {
    out.put_string("status", "err"); out.put_string("msg", "run history is not available"); return out;
}
let runs = store.get_data("runtime", "agent_plan_runs").get_object("data").get_array("runs");
for i in 0..runs.len() {
    let r = runs.get_object(i);
    if r.try_get_string("run_id").unwrap_or_default() != run_id { continue; }
    let all = r.try_get_array("events").unwrap_or_else(|_| DataArray::new());
    let mut events = DataArray::new();
    for j in 0..all.len() {
        let e = all.get_object(j);
        if e.try_get_int("seq").unwrap_or(0) <= after { continue; }
        events.push_object(e);
    }
    let cap = limit.clamp(1, 400) as usize;
    let n = events.len();
    let mut page = DataArray::new();
    for j in n.saturating_sub(cap)..n { page.push_object(events.get_object(j)); }
    out.put_string("status", "ok"); out.put_string("run_id", &run_id);
    let process_start = std::fs::read_to_string("/proc/self/stat").ok().and_then(|s| {
        let tail = s.rsplit_once(") ")?.1;
        tail.split_whitespace().nth(19)?.parse::<i64>().ok()
    }).unwrap_or(0);
    let same_process = r.try_get_int("process_start").unwrap_or(0) == 0 ||
        (process_start > 0 && r.try_get_int("process_start").unwrap_or(0) == process_start);
    let stale = r.try_get_boolean("active").unwrap_or(false) &&
        (r.try_get_int("pid").unwrap_or(0) != std::process::id() as i64 || !same_process ||
         (r.try_get_int("boot").unwrap_or(0) != 0 && boot_now != 0 && r.try_get_int("boot").unwrap_or(0) != boot_now));
    out.put_boolean("active", r.try_get_boolean("active").unwrap_or(false) && !stale);
    out.put_boolean("stale", stale); out.put_int("now", now);
    out.put_boolean("live_pid", r.try_get_int("pid").unwrap_or(0) == std::process::id() as i64);
    out.put_int("boot_now", boot_now);
    if stale { out.put_string("state", "interrupted"); }
    out.put_array("events", page); out.put_boolean("truncated", n > cap);
    for k in ["claim", "turn", "started", "updated", "ended", "pid", "boot", "outcome", "round", "max_rounds", "tool_calls", "errors"] {
        if let Ok(v) = r.try_get_property(k) { out.set_property(k, v); }
    }
    for k in ["context", "handoff", "pending_tool", "delivery"] {
        if let Ok(v) = r.try_get_object(k) { out.put_object(k, v); }
    }
    if let Ok(v) = r.try_get_string("checkpoint") { out.put_string("checkpoint", &v); }
    return out;
}
out.put_string("status", "err"); out.put_string("msg", "run not found (history is bounded and instance-local)"); out