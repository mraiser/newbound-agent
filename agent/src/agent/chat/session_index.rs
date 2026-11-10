use ndata::dataobject::DataObject;
use ndata::dataarray::DataArray;
use flowlang::datastore::DataStore;
use flowlang::flowlang::system::time::time;
use flowlang::command::Command;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["op", "id", "arg", "nn_sessionid"] {
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
        let arg_0: String = o.get_string("op");
        let arg_1: String = o.get_string("id");
        let arg_2: String = o.get_string("arg");
        let arg_3: String = o.get_string("nn_sessionid");
        session_index(arg_0, arg_1, arg_2, arg_3)
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

pub fn session_index(op: String, id: String, arg: String, nn_sessionid: String) -> DataObject {
// The session index owns two things the generic store-writer (app.write)
// can't: the per-user index FILE, and the server-side binding of an index
// row to the session's username. The session records themselves are read
// and written through the platform's gated paths (app.read / app.write),
// which already tie them to the user via check_auth; this command never
// touches them.
fn err(msg: &str) -> DataObject {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", msg);
    o
}
fn ok_list(list: DataArray) -> DataObject {
    let mut o = DataObject::new();
    o.put_string("status", "ok");
    o.put_array("list", list);
    o
}

// who is asking — the session's user, server-side. The framework always
// resolves a user; no logged-in account means the literal `anonymous` user.
let system = DataStore::globals().get_object("system");
let sessions = system.get_object("sessions");
if !sessions.has(&nn_sessionid) { return err("no session for nn_sessionid"); }
let user = sessions.get_object(&nn_sessionid).get_object("user");
let username = if user.has("username") { user.get_string("username") }
               else if user.has("id") { user.get_string("id") }
               else { "anonymous".to_string() };
let username = if username.trim().is_empty() { "anonymous".to_string() } else { username };

// the index file: runtime/agent/chat/<sanitized-username>.jsonl, beside the
// other user-files (msg.put's index.jsonl, uploads). Sanitize defensively —
// a username becomes a filename.
let safe: String = username.chars()
    .map(|c| if c.is_ascii_alphanumeric() || c=='-' || c=='_' || c=='.' { c } else { '_' })
    .collect();
let store = DataStore::new();
let base = match store.root.canonicalize().ok().and_then(|r| r.parent().map(|p| p.to_path_buf())) {
    Some(b) => b,
    None => return err("cannot resolve the runtime root"),
};
let dir = base.join("runtime").join("agent").join("chat");
let file = dir.join(format!("{}.jsonl", safe));

// fold the append-only rows into current sessions: latest row per id wins,
// a {deleted:true} row removes the id. Order, not truth — the session
// records in the store are truth; this file is rebuilt meaning, never data.
let fold = || -> DataArray {
    let mut order: Vec<String> = Vec::new();            // first-seen order, for stability
    let mut ord: Vec<String> = Vec::new();              // the user's saved ordering: the LATEST order row wins
    let mut latest: Vec<(String, DataObject)> = Vec::new();
    if let Ok(text) = std::fs::read_to_string(&file) {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }
            let row = match DataObject::try_from_string(line) {
                Ok(r) => r, Err(_) => continue,
            };
            if row.has("order") {
                // an order row: {order:[ids], time} — replaces the saved ordering
                // wholesale, so the latest order row in the file wins.
                if let ndata::data::Data::DArray(_) = row.get_property("order") {
                    ord = row.get_array("order").objects().into_iter()
                        .filter_map(|d| match d { ndata::data::Data::DString(s) => Some(s), _ => None })
                        .collect();
                }
                continue;
            }
            if !row.has("id") { continue; }
            let id = row.get_string("id");
            if let Some(pos) = latest.iter().position(|(i, _)| *i == id) {
                latest[pos].1 = row;                     // newer row replaces
            } else {
                order.push(id.clone());
                latest.push((id, row));
            }
        }
    }
    let mut out = DataArray::new();
    for (id, row) in latest {
        if row.has("deleted") && row.get_boolean("deleted") { continue; }
        let title = if row.has("title") { row.get_string("title") } else { "untitled session".to_string() };
        let t = if row.has("time") { row.get_int("time") } else { 0 };
        let mut e = DataObject::new();
        e.put_string("id", &id);
        e.put_string("title", &title);
        e.put_int("time", t);
        out.push_object(e);
    }
    // newest first, by the row's recorded time
    let mut v: Vec<DataObject> = out.objects().into_iter().map(|d| d.object()).collect();
    v.sort_by(|a, b| b.get_int("time").cmp(&a.get_int("time")));
    // apply the user's saved ordering: ids NOT in it come FIRST in their
    // newest-first relative order (brand-new sessions land on top), then the
    // ordered ids in sequence; deleted or absent ids are simply skipped.
    if !ord.is_empty() {
        let mut arranged: Vec<DataObject> = Vec::new();
        for o in &v {
            if !ord.contains(&o.get_string("id")) { arranged.push(o.clone()); }
        }
        for want in &ord {
            if let Some(o) = v.iter().find(|o| o.get_string("id") == *want) { arranged.push(o.clone()); }
        }
        v = arranged;
    }
    let mut out2 = DataArray::new();
    for o in v { out2.push_object(o); }
    out2
};

let op_l = op.trim().to_lowercase();
if op_l == "list" {
    return ok_list(fold());
}
if op_l == "delete" {
    let id = id.trim();
    if id.is_empty() { return err("id is required for delete"); }
    if std::fs::create_dir_all(&dir).is_err() { return err("cannot create the index dir"); }
    use std::io::Write;
    let mut row = DataObject::new();
    row.put_string("id", id);
    row.put_int("time", time());
    row.put_boolean("deleted", true);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&file) {
        let _ = writeln!(f, "{}", row.to_string().replace('\n', " "));
    }
    return ok_list(fold());
}
if op_l == "rename" {
    let id = id.trim();
    if id.is_empty() { return err("id is required for rename"); }
    let new_title = arg.replace('\n', " ").trim().to_string();
    if new_title.is_empty() { return err("arg (the new title) is required for rename"); }
    // truth first: stamp the custom title on the session record itself, ownership-checked,
    // through the same gated door the chat uses (app.write) — openSession restores it, and
    // session_touch can rebuild a lost index row with the rename intact
    if !store.exists("runtime", id) { return err("no such session"); }
    let mut data = store.get_data("runtime", id).get_object("data");
    if !data.has("username") || data.get_string("username") != username { return err("not your session"); }
    data.put_string("custom_title", &new_title);
    data.put_string("title", &new_title);
    let mut a = DataObject::new();
    a.put_string("lib", "runtime");
    a.put_string("id", id);
    a.put_object("data", data);
    a.put_array("readers", DataArray::new());
    a.put_array("writers", DataArray::new());
    a.put_string("nn_sessionid", &nn_sessionid);
    if Command::lookup("app", "app", "write").execute(a).is_err() {
        return err("could not update the session record");
    }
    // then meaning: append an index row with the new title, REUSING the old row's time
    // so a rename never changes the sidebar's order
    let mut t = 0i64;
    for d in fold().objects() {
        let o = d.object();
        if o.get_string("id") == id { t = o.get_int("time"); break; }
    }
    if t == 0 { t = time(); }
    if std::fs::create_dir_all(&dir).is_err() { return err("cannot create the index dir"); }
    use std::io::Write;
    let mut row = DataObject::new();
    row.put_string("id", id);
    row.put_string("title", &new_title);
    row.put_int("time", t);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&file) {
        let _ = writeln!(f, "{}", row.to_string().replace('\n', " "));
    }
    return ok_list(fold());
}
if op_l == "reorder" {
    // `id` is ignored; `arg` is a JSON array of session ids, top to bottom.
    // House idiom for non-object JSON: wrap it in an object and read it back
    // (try_from_string PANICS on well-formed non-object JSON, so never parse
    // the bare array).
    let wrapped = match DataObject::try_from_string(&format!("{{\"a\":{}}}", arg)) {
        Ok(o) => o,
        Err(_) => return err("arg must be a JSON array of session ids"),
    };
    let mut idvec: Vec<String> = Vec::new();
    if let ndata::data::Data::DArray(_) = wrapped.get_property("a") {
        for d in wrapped.get_array("a").objects() {
            if let ndata::data::Data::DString(s) = d { idvec.push(s); }
        }
    }
    if idvec.is_empty() { return err("arg must be a JSON array of session ids"); }
    if std::fs::create_dir_all(&dir).is_err() { return err("cannot create the index dir"); }
    use std::io::Write;
    let mut ids = DataArray::new();
    for s in &idvec { ids.push_string(s); }
    let mut row = DataObject::new();
    row.put_array("order", ids);
    row.put_int("time", time());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&file) {
        let _ = writeln!(f, "{}", row.to_string().replace('\n', " "));
    }
    return ok_list(fold());
}
err("op must be list | delete | rename | reorder")
}
