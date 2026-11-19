use ndata::dataobject::DataObject;
use crate::agent::plan::{compat, store};
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["claim", "note", "nn_sessionid"] {
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
        let arg_1: String = o.get_string("note");
        let arg_2: String = o.get_string("nn_sessionid");
        add_note(arg_0, arg_1, arg_2)
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

pub fn add_note(claim: String, note: String, nn_sessionid: String) -> DataObject {
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
let now = flowlang::flowlang::system::time::time();
// A note is an append to the item's own notes record, made under the store
// lock: it cannot go stale and never invalidates a writer of the item or of
// any other item, so there is no base and no retry.
compat::ensure();
let idx = store::read_index(compat::LIB, compat::NS);
let id = match store::find_id(&idx, &claim) {
    Some(id) => id,
    None => {
        out.put_string("status", "err");
        out.put_string("msg", "no kb.plan entry with that claim - the board may be stale, refresh it");
        return out;
    }
};
let lifecycle = idx.get_object("items").get_object(&id).try_get_string("lifecycle").unwrap_or_default();
let mut n = DataObject::new();
n.put_string("text", &text);
n.put_string("author", &author);
n.put_int("time", now);
let r = store::append(compat::LIB, compat::NS, "notes", &id, n);
if r.try_get_string("status").unwrap_or_default() != "ok" {
    out.put_string("status", "err");
    out.put_string("msg", &format!("note write failed: {}", r.try_get_string("msg").unwrap_or_default()));
    return out;
}
out.put_string("status", "ok");
out.put_int("notes", r.try_get_int("count").unwrap_or(0));
out.put_int("time", now);
out.put_string("author", &author);
out.put_string("lifecycle", &lifecycle);
out.put_string("hash", &r.try_get_string("hash").unwrap_or_default());
out
}
