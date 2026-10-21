use ndata::dataobject::DataObject;

pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["claim", "detail", "parent", "nn_sessionid"] {
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
        let arg_1: String = o.get_string("detail");
        let arg_2: String = o.get_string("parent");
        let arg_3: String = o.get_string("nn_sessionid");
        add_item(arg_0, arg_1, arg_2, arg_3)
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

pub fn add_item(claim: String, detail: String, parent: String, nn_sessionid: String) -> DataObject {
let mut out = DataObject::new();
if claim.trim().is_empty() {
    out.put_string("status", "err");
    out.put_string("msg", "claim is required");
    return out;
}
// Author = the calling session's user (the platform injects nn_sessionid
// on web calls), falling back to a fixed provenance name.
let mut author = String::new();
if !nn_sessionid.is_empty() {
    let system = flowlang::datastore::DataStore::globals().get_object("system");
    if system.has("sessions") {
        let sessions = system.get_object("sessions");
        if sessions.has(&nn_sessionid) {
            let session = sessions.get_object(&nn_sessionid);
            if session.has("user") {
                author = session.get_object("user").try_get_string("displayname").unwrap_or_default();
            }
            if author.trim().is_empty() {
                author = session.try_get_string("username").unwrap_or_default();
            }
        }
    }
}
if author.trim().is_empty() { author = "plan-board".to_string(); }
let mut entry = DataObject::new();
entry.put_string("claim", claim.trim());
if !detail.trim().is_empty() { entry.put_string("detail", detail.trim()); }
// (A) SUB-ITEMS: an optional `parent` (the exact claim of another kb.plan
// entry) files this as a CHILD phase of that parent. A phased plan is filed as
// N children under a parent claim; step_item/epistemic_work treat the children
// as the unit of work and the parent reports done only when all children are
// done. Accept-at-phase granularity = accept each CHILD separately.
if !parent.trim().is_empty() { entry.put_string("parent", parent.trim()); }
entry.put_string("tags", "plan,proposed");
entry.put_string("confidence", "medium");
// Mark this as a GENUINE TODO (owner, 2026-10): the archivist.remember guard
// refuses kb.plan deposits to stop the insight-sweep polluting the board, and
// it admits ONLY entries flagged work:true. agent.plan.add_item is the todo
// front-door, so it stamps the flag; insights never carry it and stay refused.
entry.put_boolean("work", true);
let api = crate::api::new();
api.agent.archivist.remember("kb".to_string(), "plan".to_string(), entry, author)
}
