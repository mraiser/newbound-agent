// Internal shared lock facet. The meaningful API is run_state::lock(), used
// by activity_emit and run_control around runtime read/modify/write cycles.
let mut out = DataObject::new();
out.put_string("status", "ok");
out.put_string("scope", "runtime.agent_plan_runs transaction lock");
out