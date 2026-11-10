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
// PARENT VALIDATION (2026-11): the link is the parent's exact claim, so an
// unchecked string silently orphans the child. "null"/"undefined"/"none"
// (a UI serializing an absent value) mean no parent; any other parent must
// name an existing kb.plan entry, and never the item itself.
let parent_n = {
    let p = parent.trim();
    let pl = p.to_lowercase();
    if pl == "null" || pl == "undefined" || pl == "none" { String::new() } else { p.to_string() }
};
if !parent_n.is_empty() {
    if parent_n == claim.trim() {
        out.put_string("status", "err");
        out.put_string("msg", "an item cannot be its own parent");
        return out;
    }
    let rf = crate::agent::plan::compat::facet_read();
    let fsrc = rf.try_get_string("source").unwrap_or_default();
    let fsrc = if fsrc.trim().is_empty() { "[]".to_string() } else { fsrc };
    let found = match DataObject::try_from_string(&format!("{{\"a\": {} }}", fsrc)) {
        Ok(p) => {
            let a = p.get_array("a");
            (0..a.len()).any(|i| a.try_get_object(i).ok()
                .map(|e| e.try_get_string("claim").unwrap_or_default() == parent_n).unwrap_or(false))
        }
        Err(_) => false,
    };
    if !found {
        out.put_string("status", "err");
        out.put_string("msg", "no kb.plan entry has that exact parent claim - pass the parent's claim verbatim, or empty for a top-level item");
        return out;
    }
    entry.put_string("parent", &parent_n);
}
entry.put_string("tags", "plan,proposed");
entry.put_string("confidence", "medium");
// Mark this as a GENUINE TODO (owner, 2026-10): the archivist.remember guard
// refuses kb.plan deposits to stop the insight-sweep polluting the board, and
// it admits ONLY entries flagged work:true. agent.plan.add_item is the todo
// front-door, so it stamps the flag; insights never carry it and stay refused.
entry.put_boolean("work", true);
let api = crate::api::new();
api.agent.archivist.remember("kb".to_string(), "plan".to_string(), entry, author)