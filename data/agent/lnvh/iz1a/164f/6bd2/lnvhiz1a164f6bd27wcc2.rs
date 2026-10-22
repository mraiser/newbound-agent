// agent.plan.worker_status - read-only view of the plan worker (see
// worker_start): its mirrored state plus the LIVE settings it obeys.
// alive = the thread beat within 20s, or it is mid-step (a step can run
// minutes and does not beat).
fn rprop(key: &str) -> Option<String> {
    let s = DataStore::globals().try_get_object("system").ok()?;
    let a = s.try_get_object("apps").ok()?;
    let g = a.try_get_object("agent").ok()?;
    let r = g.try_get_object("runtime").ok()?;
    match r.try_get_string(key) {
        Ok(v) if !v.trim().is_empty() => Some(v.trim().to_string()),
        _ => None,
    }
}
let now = time();
let g = DataStore::globals();
let mut o = match g.try_get_object("AGENT_PLAN_WORKER") {
    Ok(w) => w.deep_copy(),
    Err(_) => {
        let mut w = DataObject::new();
        w.put_string("phase", "not started");
        w
    }
};
let phase = o.try_get_string("phase").unwrap_or_default();
let beat = o.try_get_int("beat").unwrap_or(0);
o.put_boolean("alive", phase == "stepping" || (beat > 0 && now - beat < 20_000));
o.put_boolean("plan_autonomy", rprop("PLAN_AUTONOMY").map(|v| {
    let v = v.to_lowercase();
    v == "on" || v == "true" || v == "1" || v == "yes"
}).unwrap_or(false));
o.put_int("per_hour", rprop("PLAN_STEPS_PER_HOUR").and_then(|v| v.parse::<i64>().ok()).filter(|n| *n > 0).unwrap_or(0));
o.put_int("max_steps", rprop("MAX_PLAN_STEPS").and_then(|v| v.parse::<i64>().ok()).filter(|n| *n > 0).map(|n| n.min(1000)).unwrap_or(20));
o.put_int("now", now);
o.put_string("status", "ok");
o