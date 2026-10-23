let mut out = DataObject::new();
let text = note.trim().to_string();
if claim.trim().is_empty() || text.is_empty() {
    out.put_string("status", "err");
    out.put_string("msg", "claim and note are both required");
    return out;
}
if text.chars().count() > 8000 {
    out.put_string("status", "err");
    out.put_string("msg", "note too long (max 8000 chars) - a note is guidance, put large material in the item's detail or an asset");
    return out;
}
// Author = the calling session's user (same resolution as add_item), with a
// fixed provenance fallback.
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
let api = crate::api::new();
let now = flowlang::flowlang::system::time::time();
// A note is an APPEND, which commutes with any other write - so instead of
// bouncing stale_base back to the human (and losing what they typed), re-read
// and re-apply onto the CURRENT state. Every write is still base-hashed
// against the read just made, so nothing is ever forced or lost; a write that
// races the read simply retries.
let mut last_msg = String::new();
for _attempt in 0..4 {
    let r = api.dev.code.read_control_facet("kb".to_string(), "plan".to_string(), "memory".to_string());
    if !r.try_get_boolean("exists").unwrap_or(false) {
        out.put_string("status", "err");
        out.put_string("msg", "kb.plan has no memory facet");
        return out;
    }
    let hash = r.get_string("hash");
    let parsed = match DataObject::try_from_string(&format!("{{\"a\": {} }}", r.get_string("source"))) {
        Ok(o) => o,
        Err(_) => {
            out.put_string("status", "err");
            out.put_string("msg", "kb.plan memory facet did not parse as a JSON array");
            return out;
        }
    };
    let arr = parsed.get_array("a");
    let mut found = false;
    let mut count: i64 = 0;
    let mut lifecycle = String::new();
    for i in 0..arr.len() {
        let mut e = match arr.try_get_object(i) { Ok(o) => o, Err(_) => continue };
        if e.try_get_string("claim").unwrap_or_default() != claim { continue; }
        let mut notes = match e.try_get_array("notes") { Ok(a) => a, Err(_) => DataArray::new() };
        let mut n = DataObject::new();
        n.put_string("text", &text);
        n.put_string("author", &author);
        n.put_int("time", now);
        notes.push_object(n);
        count = notes.len() as i64;
        if !e.has("notes") { e.put_array("notes", notes); }
        let tags = e.try_get_string("tags").unwrap_or_default().to_lowercase();
        for l in ["proposed", "accepted", "in-progress", "blocked", "completed", "done", "abandoned"] {
            if tags.split(',').any(|t| t.trim() == l) { lifecycle = l.to_string(); }
        }
        found = true;
        break;
    }
    if !found {
        out.put_string("status", "err");
        out.put_string("msg", "no kb.plan entry with that claim - the board may be stale, refresh it");
        return out;
    }
    let short: String = claim.chars().take(50).collect();
    let label = format!("plan note: '{}' (+1, {} total)", short, count);
    let pw = api.dev.code.patch_control_facet(
        "kb".to_string(), "plan".to_string(), "memory".to_string(),
        "".to_string(), arr.to_string(), hash, label, author.clone(), nn_sessionid.clone());
    let ok = pw.try_get_boolean("ok").unwrap_or(false)
        || pw.try_get_string("status").map(|s| s == "ok").unwrap_or(false);
    if ok {
        out.put_string("status", "ok");
        out.put_int("notes", count);
        out.put_int("time", now);
        out.put_string("author", &author);
        out.put_string("lifecycle", &lifecycle);
        out.put_string("hash", &pw.try_get_string("hash").unwrap_or_default());
        if pw.has("patch_id") { out.put_string("patch_id", &pw.get_string("patch_id")); }
        return out;
    }
    last_msg = pw.try_get_string("msg").unwrap_or_default();
    if last_msg != "stale_base" { break; }
}
out.put_string("status", "err");
out.put_string("msg", &format!("note write failed: {}", if last_msg.is_empty() { "unknown error".to_string() } else { last_msg }));
out