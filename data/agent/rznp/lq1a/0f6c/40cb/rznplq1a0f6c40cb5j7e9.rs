// Opens one session record for the session's user. The store's check_auth
// gates the record itself (runtime is admin-only by default); this command
// adds the ownership binding (the record's data.username must BE the asking
// user) and, on success, upserts the per-user index row — which is what
// keeps the sidebar (the index) in step with the store (truth) when a row
// was lost, and what mints the very first index entry for a new session.
fn err(msg: &str) -> DataObject {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", msg);
    o
}

let id = id.trim().to_string();
if id.is_empty() { return err("id is required"); }

// the asking user, server-side
let system = DataStore::globals().get_object("system");
let sessions = system.get_object("sessions");
if !sessions.has(&nn_sessionid) { return err("no session for nn_sessionid"); }
let user = sessions.get_object(&nn_sessionid).get_object("user");
let username = if user.has("username") { user.get_string("username") }
               else if user.has("id") { user.get_string("id") }
               else { "anonymous".to_string() };
let username = if username.trim().is_empty() { "anonymous".to_string() } else { username };

// the record — truth
let store = DataStore::new();
if !store.exists("runtime", &id) { return err("no such session"); }
let rec = store.get_data("runtime", &id);
let data = rec.get_object("data");
if data.get_string("username") != username { return err("not your session"); }

// upsert this user's index row (self-healing, and first-save mint)
let safe: String = username.chars()
    .map(|c| if c.is_ascii_alphanumeric() || c=='-' || c=='_' || c=='.' { c } else { '_' })
    .collect();
if let Some(base) = store.root.canonicalize().ok().and_then(|r| r.parent().map(|p| p.to_path_buf())) {
    let dir = base.join("runtime").join("agent").join("chat");
    let file = dir.join(format!("{}.jsonl", safe));
    let title = if data.has("title") { data.get_string("title") } else { "untitled session".to_string() };
    let t = if data.has("time") { data.get_int("time") } else { time() };
    // only append if no live row for this id already exists
    let mut have = false;
    if let Ok(text) = std::fs::read_to_string(&file) {
        for line in text.lines() {
            if let Ok(row) = DataObject::try_from_string(line.trim()) {
                if row.has("id") && row.get_string("id") == id
                   && !(row.has("deleted") && row.get_boolean("deleted")) { have = true; break; }
            }
        }
    }
    if !have && std::fs::create_dir_all(&dir).is_ok() {
        use std::io::Write;
        let mut row = DataObject::new();
        row.put_string("id", &id);
        row.put_string("title", &title);
        row.put_int("time", t);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&file) {
            let _ = writeln!(f, "{}", row.to_string().replace('\n', " "));
        }
    }
}

let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_string("id", &id);
o.put_object("data", data);
o