// Searches the asking user's previous chat sessions. The list of sessions is
// session_index's folded list (sidebar order, tombstones gone); each record is
// read from the store and ownership-checked exactly as session_open does.
fn err(msg: &str) -> DataObject {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", msg);
    o
}
// a one-line excerpt around the first hit; char-based so it never splits UTF-8
fn snippet(text: &str, low: &str, pos: usize, qchars: usize) -> String {
    let at = low[..pos].chars().count();
    let chars: Vec<char> = text.chars().collect();
    let from = at.saturating_sub(40);
    let to = (at + qchars + 80).min(chars.len());
    let mid: String = chars[from.min(chars.len())..to].iter().collect();
    let mid = mid.split_whitespace().collect::<Vec<&str>>().join(" ");
    format!("{}{}{}", if from > 0 { "…" } else { "" }, mid, if to < chars.len() { "…" } else { "" })
}

let q = query.trim().to_lowercase();
if q.is_empty() { return err("query is required"); }
let qchars = q.chars().count();

let system = DataStore::globals().get_object("system");
let sessions = system.get_object("sessions");
if !sessions.has(&nn_sessionid) { return err("no session for nn_sessionid"); }
let user = sessions.get_object(&nn_sessionid).get_object("user");
let username = if user.has("username") { user.get_string("username") }
               else if user.has("id") { user.get_string("id") }
               else { "anonymous".to_string() };
let username = if username.trim().is_empty() { "anonymous".to_string() } else { username };

let ix = crate::agent::chat::session_index::session_index(
    "list".to_string(), "".to_string(), "".to_string(), nn_sessionid.clone());
if !ix.has("list") { return ix; }

let store = DataStore::new();
let mut matches = DataArray::new();
let mut searched = 0i64;
for d in ix.get_array("list").objects() {
    let row = d.object();
    let id = row.get_string("id");
    let title = row.get_string("title");
    let mut hits = 0i64;
    let mut snip = String::new();
    if title.to_lowercase().contains(&q) { hits += 1; }
    if store.exists("runtime", &id) {
        let rec = store.get_data("runtime", &id);
        if let Data::DObject(_) = rec.get_property("data") {
            let data = rec.get_object("data");
            let mine = match data.try_get_property("username") {
                Ok(Data::DString(u)) => u == username,
                _ => false,
            };
            if mine {
                searched += 1;
                if let Ok(Data::DArray(_)) = data.try_get_property("messages") {
                    for m in data.get_array("messages").objects() {
                        let m = match m { Data::DObject(_) => m.object(), _ => continue };
                        let role = match m.try_get_property("role") { Ok(Data::DString(r)) => r, _ => continue };
                        if role != "user" && role != "assistant" { continue; }
                        let text = match m.try_get_property("content") { Ok(Data::DString(t)) => t, _ => continue };
                        if text.starts_with("[CONTEXT]") { continue; }
                        let low = text.to_lowercase();
                        let n = low.matches(&q).count() as i64;
                        if n == 0 { continue; }
                        if snip.is_empty() {
                            if let Some(pos) = low.find(&q) {
                                snip = format!("{}: {}", if role == "user" { "you" } else { "agent" },
                                               snippet(&text, &low, pos, qchars));
                            }
                        }
                        hits += n;
                    }
                }
            }
        }
    }
    if hits > 0 {
        let mut e = DataObject::new();
        e.put_string("id", &id);
        e.put_string("title", &title);
        e.put_int("time", row.get_int("time"));
        e.put_int("hits", hits);
        e.put_string("snippet", &snip);
        matches.push_object(e);
    }
}

let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_string("query", query.trim());
o.put_int("searched", searched);
o.put_int("count", matches.len() as i64);
o.put_array("matches", matches);
o