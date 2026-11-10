// The kanban's one autonomy switch: persist the SAME live setting the worker
// reads. Enabling also starts the worker; disabling prevents NEW steps and
// deliberately lets an already-running step finish (see worker_start).
let api = crate::api::new();
let saved = api.agent.model.set_setting("PLAN_AUTONOMY".to_string(), if enabled { "on".to_string() } else { "off".to_string() });
if saved.try_get_string("status").unwrap_or_default() != "ok" {
    return saved;
}
let actual = DataStore::globals()
    .try_get_object("system").ok()
    .and_then(|s| s.try_get_object("apps").ok())
    .and_then(|a| a.try_get_object("agent").ok())
    .and_then(|g| g.try_get_object("runtime").ok())
    .and_then(|r| r.try_get_string("PLAN_AUTONOMY").ok())
    .map(|v| { let v = v.to_lowercase(); v == "on" || v == "true" || v == "1" || v == "yes" })
    .unwrap_or(false);
if actual != enabled {
    let mut e = DataObject::new();
    e.put_string("status", "err");
    e.put_string("msg", "PLAN_AUTONOMY did not retain the requested value");
    e.put_boolean("plan_autonomy", actual);
    return e;
}
if enabled {
    let started = api.agent.plan.worker_start();
    if started.try_get_string("status").unwrap_or_default() != "ok" {
        let mut e = DataObject::new();
        e.put_string("status", "err");
        e.put_string("msg", &format!("Autonomy was enabled, but the plan worker did not start: {}",
            started.try_get_string("msg").unwrap_or_else(|_| "unknown error".to_string())));
        e.put_boolean("plan_autonomy", true);
        return e;
    }
}
let mut out = DataObject::new();
out.put_string("status", "ok");
out.put_boolean("plan_autonomy", enabled);
out.put_string("msg", if enabled {
    "Plan autonomy enabled; the worker is starting. Disabling later stops new steps but lets a running step finish."
} else {
    "Plan autonomy disabled; no new steps will start. A step already running will finish."
});
out