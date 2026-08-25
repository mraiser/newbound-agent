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
entry.put_string("tags", "plan,proposed");
entry.put_string("confidence", "medium");
let api = crate::api::new();
api.agent.archivist.remember("kb".to_string(), "plan".to_string(), entry, author)