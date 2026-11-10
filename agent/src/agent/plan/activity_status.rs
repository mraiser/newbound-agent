use ndata::dataobject::DataObject;
use ndata::dataarray::DataArray;
use flowlang::datastore::DataStore;
use flowlang::flowlang::system::time::time;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["after", "limit"] {
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
        let arg_0: i64 = o.get_int("after");
        let arg_1: i64 = o.get_int("limit");
        activity_status(arg_0, arg_1)
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

pub fn activity_status(after: i64, limit: i64) -> DataObject {
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
              "errors", "failure_streak", "duplicate_streak", "pending_tool", "checkpoint", "paused", "stop_requested", "control_state", "control_note", "control",
              "context", "finish", "handoff", "ended", "outcome", "seq", "delivery", "process_start", "interrupted", "previous_run_id", "recovered", "inherited_checkpoint", "checkpoint_seq"] {
        if let Ok(v) = src.try_get_property(k) { r.set_property(k, v); }
    }
    if let Ok(v) = src.try_get_object("pending_tool") { r.put_object("pending_tool", v); }
    if let Ok(v) = src.try_get_object("delivery") { r.put_object("delivery", v); }
    if let Ok(v) = src.try_get_object("control") { r.put_object("control", v); }
    if let Ok(v) = src.try_get_object("inherited_checkpoint") { r.put_object("inherited_checkpoint", v); }
    if let Ok(v) = src.try_get_object("checkpoint") { r.put_object("checkpoint", v); }
    if let Ok(v) = src.try_get_object("context") { r.put_object("context", v); }
    if let Ok(v) = src.try_get_object("handoff") {
        // A diagnostic transcript is explicit history-only data. Do not send
        // it every three seconds in the ordinary live/recent status poll.
        let mut h = v.deep_copy(); h.remove_property("transcript_excerpt");
        r.put_object("handoff", h);
    }
    r
}
let now = time();
let g = DataStore::globals();
let boot = g.try_get_object("AGENT_PLAN_ACTIVITY").ok()
    .and_then(|a| a.try_get_int("boot").ok()).unwrap_or(0);
let store = DataStore::new();
let mut runs: Vec<DataObject> = Vec::new();
if store.exists("runtime", "agent_plan_runs") {
    let d = store.get_data("runtime", "agent_plan_runs").get_object("data");
    if let Ok(a) = d.try_get_array("runs") {
        for i in 0..a.len() { if let Ok(r) = a.try_get_object(i) { runs.push(r.deep_copy()); } }
    }
}
// A live run from before this service boot is interrupted, never active. Do
// not mutate stored state merely because somebody opened the Plan tab. PID is
// a same-process supplement to the boot token, not a cross-restart identity.
for r in runs.iter_mut() {
    let active = r.try_get_boolean("active").unwrap_or(false);
    let run_boot = r.try_get_int("boot").unwrap_or(0);
    let boot_mismatch = boot != 0 && run_boot != 0 && run_boot != boot;
    let recorded_pid = r.try_get_int("pid").unwrap_or(0);
    let live_pid = recorded_pid > 0 && recorded_pid == std::process::id() as i64;
    let process_start = std::fs::read_to_string("/proc/self/stat").ok().and_then(|s| {
        let tail = s.rsplit_once(") ")?.1;
        tail.split_whitespace().nth(19)?.parse::<i64>().ok()
    }).unwrap_or(0);
    let same_process = r.try_get_int("process_start").unwrap_or(0) == 0 ||
        (process_start > 0 && r.try_get_int("process_start").unwrap_or(0) == process_start);
    if active && (!live_pid || !same_process || boot_mismatch) {
        r.put_boolean("active", false);
        r.put_boolean("stale", true);
        r.put_boolean("interrupted", true);
        r.put_string("state", "interrupted");
        r.put_string("outcome", "The service or worker stopped before this run reached its finish boundary.");
    }
}
runs.sort_by(|a, b| b.try_get_int("updated").unwrap_or(0).cmp(&a.try_get_int("updated").unwrap_or(0)));
let mut recent = DataArray::new();
for r in runs.iter().take(8) {
    let mut summary = copy_summary(r);
    // The selected run carries its full bounded handoff below. Recent cards
    // only need identity, lifecycle and counters; fetch history on selection.
    for key in ["checkpoint", "handoff", "context", "inherited_checkpoint", "control_note"] { summary.remove_property(key); }
    if let Ok(h) = r.try_get_object("handoff") {
        summary.put_string("next", &h.try_get_string("next").unwrap_or_default());
        summary.put_string("summary", &h.try_get_string("summary").unwrap_or_default().chars().take(240).collect::<String>());
    }
    recent.push_object(summary);
}
let current_process_start = std::fs::read_to_string("/proc/self/stat").ok().and_then(|s| {
    let tail = s.rsplit_once(") ")?.1;
    tail.split_whitespace().nth(19)?.parse::<i64>().ok()
}).unwrap_or(0);
let active = runs.iter().find(|r| r.try_get_boolean("active").unwrap_or(false)
    && r.try_get_int("pid").unwrap_or(0) == std::process::id() as i64
    && (r.try_get_int("process_start").unwrap_or(0) == 0 || (current_process_start > 0 && r.try_get_int("process_start").unwrap_or(0) == current_process_start))
    && (boot == 0 || r.try_get_int("boot").unwrap_or(0) == 0 || r.try_get_int("boot").unwrap_or(0) == boot));
let selected = active.or_else(|| runs.first());
if let Some(r) = selected {
    let mut selected_summary = copy_summary(r);
    // A handoff's inherited checkpoint is a provenance pointer for display,
    // not another full transcript. Fetch the prior run only when requested.
    if let Ok(ic) = selected_summary.try_get_object("inherited_checkpoint") {
        let mut refonly=DataObject::new();refonly.put_string("run_id",&ic.try_get_string("run_id").unwrap_or_default());
        refonly.put_string("next",&ic.try_get_string("next").unwrap_or_default());
        refonly.put_string("status",&ic.try_get_string("status").unwrap_or_default());
        selected_summary.put_object("inherited_checkpoint",refonly);
    }
    let mut events = DataArray::new();
    let all = r.try_get_array("events").unwrap_or_else(|_| DataArray::new());
    let cap = limit.clamp(1, 400) as usize;
    let mut n = 0usize;
    for i in 0..all.len() {
        let e = all.get_object(i);
        if e.try_get_int("seq").unwrap_or(0) <= after { continue; }
        let mut v = DataObject::new();
        for k in ["seq", "time", "kind", "phase", "round", "tool", "target", "ok", "streak", "detail", "outcome"] {
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
    out.put_object("run", selected_summary);
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
}
