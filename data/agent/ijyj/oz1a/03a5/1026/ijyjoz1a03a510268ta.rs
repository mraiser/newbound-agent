let lifecycles = ["proposed", "accepted", "in-progress", "completed", "done", "abandoned"];
let mut out = DataObject::new();
if !lifecycles.contains(&lifecycle.as_str()) {
    out.put_string("status", "err");
    out.put_string("msg", "lifecycle must be one of proposed/accepted/in-progress/done/abandoned");
    return out;
}
let api = crate::api::new();
let r = api.dev.code.read_control_facet("kb".to_string(), "plan".to_string(), "memory".to_string());
if !r.try_get_boolean("exists").unwrap_or(false) {
    out.put_string("status", "err");
    out.put_string("msg", "kb.plan has no memory facet");
    return out;
}
let src = r.get_string("source");
let hash = r.get_string("hash");
if !base.is_empty() && base != hash {
    out.put_string("status", "err");
    out.put_string("msg", "stale_base");
    out.put_string("current_hash", &hash);
    return out;
}
let wrapped = format!("{{\"a\": {} }}", src);
let parsed = match DataObject::try_from_string(&wrapped) {
    Ok(o) => o,
    Err(_) => {
        out.put_string("status", "err");
        out.put_string("msg", "kb.plan memory facet did not parse as a JSON array");
        return out;
    }
};
let arr = parsed.get_array("a");
let mut found = false;
for i in 0..arr.len() {
    let mut e = arr.get_object(i);
    if e.try_get_string("claim").unwrap_or_default() == claim {
        let tags = e.try_get_string("tags").unwrap_or_default();
        let mut kept: Vec<String> = tags.split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty() && !lifecycles.contains(&t.as_str()))
            .collect();
        kept.push(lifecycle.clone());
        e.put_string("tags", &kept.join(","));
        // Re-approval hook (convergence guard): moving an item TO `accepted`
        // is the human re-authorization. If its step_log has reached the cap
        // (i.e. step_item refused it with needs_reapproval), reset the log so
        // the agent may step it again. A normal accept leaves a non-exhausted
        // log untouched (history preserved). The reset itself is recorded in
        // the journal via the label below.
        if lifecycle == "accepted" && e.has("step_log") {
            let prior = match e.try_get_array("step_log") { Ok(a) => a.len() as i64, Err(_) => 0 };
            let mut cap: i64 = 20;
            let g = flowlang::datastore::DataStore::globals();
            if let Ok(sys) = g.try_get_object("system") {
                if let Ok(apps) = sys.try_get_object("apps") {
                    if let Ok(agent) = apps.try_get_object("agent") {
                        if let Ok(rt) = agent.try_get_object("runtime") {
                            if let Ok(v) = rt.try_get_string("MAX_PLAN_STEPS") {
                                if let Ok(p) = v.trim().parse::<i64>() { if p > 0 { cap = p.min(1000); } }
                            }
                        }
                    }
                }
            }
            if prior >= cap {
                e.put_array("step_log", ndata::dataarray::DataArray::new());
                e.put_int("steps_reset", prior);
            }
        }
        found = true;
        break;
    }
}
if !found {
    out.put_string("status", "err");
    out.put_string("msg", "no kb.plan entry with that claim - the board may be stale, refresh it");
    return out;
}
let short: String = claim.chars().take(60).collect();
let label = format!("plan board: '{}' -> {}", short, lifecycle);
api.dev.code.patch_control_facet(
    "kb".to_string(), "plan".to_string(), "memory".to_string(),
    "".to_string(), arr.to_string(), hash, label, "".to_string(), nn_sessionid)