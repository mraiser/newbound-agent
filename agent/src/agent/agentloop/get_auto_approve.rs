use ndata::dataobject::DataObject;
use flowlang::datastore::DataStore;

pub fn execute(_: DataObject) -> DataObject {
    use std::panic;
    let ax = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        get_auto_approve()
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

pub fn get_auto_approve() -> DataObject {
// The agent's global approval kill-switch, read LIVE from
// runtime/agent/botd.properties (system.apps.agent.runtime — same map
// chat_llm reads LLM=/VLLM_URL from, re-read per call, so an edit to
// botd.properties takes effect without a restart). Truthy: 1/true/yes/on
// (case-insensitive). Absent or anything else = the normal per-call gate.
let mut out = DataObject::new();
let mut enabled = false;
let system = DataStore::globals().get_object("system");
if system.has("apps") {
    let apps = system.get_object("apps");
    if apps.has("agent") {
        let agent = apps.get_object("agent");
        if agent.has("runtime") {
            let rt = agent.get_object("runtime");
            if rt.has("AUTO_APPROVE_TOOLS") {
                let v = rt.get_string("AUTO_APPROVE_TOOLS").trim().to_lowercase();
                enabled = matches!(v.as_str(), "1" | "true" | "yes" | "on");
            }
        }
    }
}
out.put_boolean("enabled", enabled);
out
}
