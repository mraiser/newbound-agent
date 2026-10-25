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