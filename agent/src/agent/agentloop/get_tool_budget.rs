use ndata::dataobject::DataObject;
use flowlang::datastore::DataStore;

pub fn execute(_: DataObject) -> DataObject {
    use std::panic;
    let ax = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        get_tool_budget()
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

pub fn get_tool_budget() -> DataObject {
// The agent's per-turn tool budget, read from runtime/agent/botd.properties
// (system.apps.agent.runtime — the same boot-time snapshot chat_llm reads
// LLM= from, so an edit takes effect on the next service restart, not
// live). TOOL_BUDGET=<positive integer>; absent, zero, negative, or
// non-numeric falls back to the default of 10. Capped at 1000 as a guard.
// Returns FLAT {budget: i64}. The agentloop js reads this once at ready and
// uses it as the chatTurn round limit in place of the hard-coded 10.
let mut out = DataObject::new();
let mut budget: i64 = 10;
let system = DataStore::globals().get_object("system");
if system.has("apps") {
    let apps = system.get_object("apps");
    if apps.has("agent") {
        let agent = apps.get_object("agent");
        if agent.has("runtime") {
            let rt = agent.get_object("runtime");
            if rt.has("TOOL_BUDGET") {
                if let Ok(n) = rt.get_string("TOOL_BUDGET").trim().parse::<i64>() {
                    if n > 0 { budget = n.min(1000); }
                }
            }
        }
    }
}
out.put_int("budget", budget);
out
}
