use ndata::dataobject::DataObject;
use flowlang::datastore::DataStore;
use flowlang::flowlang::system::time::time;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["claim"] {
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
        let arg_0: String = o.get_string("claim");
        continuity_status(arg_0)
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

pub fn continuity_status(claim: String) -> DataObject {
// Read-only item-level continuity inspector. The item's active pointer and
// structured checkpoint are durable; runtime run history is an optional,
// bounded detail facet. Never changes lifecycle or treats recovery as consent.
let mut out = DataObject::new();
let b = crate::api::new().agent.plan.board();
let mut found = false;
for x in b.get_array("entries").objects() {
    let e = x.object();
    if e.try_get_string("claim").unwrap_or_default() != claim { continue; }
    found = true;
    out.put_string("claim", &claim);
    if let Ok(a) = e.try_get_object("active_run") { out.put_object("active_run", a); }
    if let Ok(a) = e.try_get_object("last_run") { out.put_object("last_run", a); }
    if let Ok(a) = e.try_get_object("checkpoint") { out.put_object("checkpoint", a); }
    if let Ok(a) = e.try_get_object("continuity") { out.put_object("continuity", a); }
    let active_id = e.try_get_object("active_run").ok()
        .and_then(|a| a.try_get_string("run_id").ok()).unwrap_or_default();
    let last_id = e.try_get_object("last_run").ok()
        .and_then(|a| a.try_get_string("run_id").ok()).unwrap_or_default();
    let rid = if !active_id.is_empty() { active_id.clone() } else { last_id.clone() };
    let store = DataStore::new();
    let mut run_found = false;
    if !rid.is_empty() && store.exists("runtime", "agent_plan_runs") {
        let runs = store.get_data("runtime", "agent_plan_runs").get_object("data").get_array("runs");
        for i in 0..runs.len() {
            let r = runs.get_object(i);
            if r.try_get_string("run_id").unwrap_or_default() != rid { continue; }
            run_found = true;
            out.put_object("run", r.deep_copy());
            let boot = DataStore::globals().try_get_object("AGENT_PLAN_ACTIVITY").ok()
                .and_then(|g| g.try_get_int("boot").ok()).unwrap_or(0);
            let ps = std::fs::read_to_string("/proc/self/stat").ok().and_then(|s| {
                let tail = s.rsplit_once(") ")?.1;
                tail.split_whitespace().nth(19)?.parse::<i64>().ok()
            }).unwrap_or(0);
            let same_process = r.try_get_int("process_start").unwrap_or(0) == 0 ||
                (ps > 0 && r.try_get_int("process_start").unwrap_or(0) == ps);
            let stale = r.try_get_boolean("active").unwrap_or(false) &&
                (r.try_get_int("pid").unwrap_or(0) != std::process::id() as i64 || !same_process ||
                 (r.try_get_int("boot").unwrap_or(0) != 0 && boot != 0 && r.try_get_int("boot").unwrap_or(0) != boot));
            out.put_boolean("active", r.try_get_boolean("active").unwrap_or(false) && !stale);
            out.put_boolean("stale", stale);
            if stale { out.put_string("state", "interrupted"); out.put_boolean("interrupted", true); }
            if let Ok(control) = r.try_get_object("control") { out.put_object("control", control); }
            out.put_string("control_state", &r.try_get_string("control_state").unwrap_or_default());
            out.put_string("control_note", &r.try_get_string("control_note").unwrap_or_default());
            out.put_boolean("paused", r.try_get_boolean("paused").unwrap_or(false));
            break;
        }
    }
    if !run_found {
        // A durable active pointer with no live record is an interrupted run,
        // not an invisible one. The plan lifecycle remains human/gate-owned.
        if !active_id.is_empty() && active_id != last_id {
            out.put_boolean("interrupted", true); out.put_boolean("active", false);
            out.put_string("state", "interrupted");
            out.put_string("outcome", "The last run did not reach its final journal boundary; its durable active pointer and checkpoint remain available for recovery.");
        } else { out.put_boolean("active", false); }
    }
    out.put_boolean("history_available", run_found);
    out.put_string("run_id", &rid);
    break;
}
out.put_int("now", time());
out.put_string("status", if found { "ok" } else { "err" });
if !found { out.put_string("msg", "plan item not found"); }
out
}
