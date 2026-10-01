// Ensures this session id has a live row in the asking user's index. Called
// by the chat right after it writes a session record through app.write, so
// the sidebar (the index) always reflects the store (truth) — including the
// first save of a brand-new session, and any save whose index row was lost.
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

// the record must exist and belong to this user — the index points at truth,
// it never asserts a session the store doesn't hold for this user
let store = DataStore::new();
if !store.exists("runtime", &id) { return err("no such session"); }
let data = store.get_data("runtime", &id).get_object("data");
if data.get_string("username") != username { return err("not your session"); }
let title = if data.has("title") { data.get_string("title") } else { "untitled session".to_string() };
let t = if data.has("time") { data.get_int("time") } else { time() };

// upsert the row only if no live row for this id exists
let safe: String = username.chars()
    .map(|c| if c.is_ascii_alphanumeric() || c=='-' || c=='_' || c=='.' { c } else { '_' })
    .collect();
let mut touched = false;
if let Some(base) = store.root.canonicalize().ok().and_then(|r| r.parent().map(|p| p.to_path_buf())) {
    let dir = base.join("runtime").join("agent").join("chat");
    let file = dir.join(format!("{}.jsonl", safe));
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
            touched = writeln!(f, "{}", row.to_string().replace('\n', " ")).is_ok();
        }
    }
}

let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_string("id", &id);
o.put_boolean("touched", touched);
o