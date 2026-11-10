// agent-msg-expand_clipped - recover the full text behind a UI-clipped
// tool cell. The chat/agentloop clamp renderer emits
// "…[N chars clipped]" after a fixed-length prefix of the real output; the
// same output was captured verbatim by chat_llm's seam when LLM_CAPTURE=on.
// The clipped fragment the UI still shows is therefore an EXACT PREFIX of
// the captured content, so no ids need to cross the wire: we scan today's
// capture rows, resolve their msg ids, and return the first captured
// message that starts with the fragment.
//
// Gate mirrors the capture seam exactly: absent LLM_CAPTURE or any value
// other than "on" answers status err (the caller hides the expander), so
// the front-end can probe cheaply and never offer expansion when there is
// nothing to pull from.
fn err(msg: String) -> DataObject {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", &msg);
    o
}
// same gate the seam reads (chat_llm): system.apps.agent.runtime.LLM_CAPTURE
let cap_on = (|| -> bool {
    let g = DataStore::globals();
    let s = match g.try_get_object("system") { Ok(x) => x, _ => return false };
    let a = match s.try_get_object("apps") { Ok(x) => x, _ => return false };
    let ag = match a.try_get_object("agent") { Ok(x) => x, _ => return false };
    let r = match ag.try_get_object("runtime") { Ok(x) => x, _ => return false };
    r.try_get_string("LLM_CAPTURE").map(|v| v.trim().eq_ignore_ascii_case("on")).unwrap_or(false)
})();
if !cap_on {
    return err("LLM_CAPTURE is not on - there is no captured full text to expand into".to_string());
}
let frag = fragment.trim_end().to_string();
if frag.len() < 8 {
    return err("fragment too short to match safely".to_string());
}
let store = DataStore::new();
let root2 = match store.root.canonicalize().ok().and_then(|r| r.parent().map(|p| p.to_path_buf())) {
    Some(p) => p,
    None => return err("cannot resolve runtime root".to_string()),
};
let dir = root2.join("runtime").join("agent").join("model").join("capture");
if !dir.is_dir() {
    return err("no capture directory yet - no LLM turns have been recorded today".to_string());
}
// scan the most recent capture day-files first, newest lines first, so the
// match is the turn the user is looking at rather than a stale duplicate.
let mut files: Vec<std::path::PathBuf> = Vec::new();
if let Ok(rd) = std::fs::read_dir(&dir) {
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().map(|x| x == "jsonl").unwrap_or(false) { files.push(p); }
    }
}
files.sort();
files.reverse();
for f in files {
    let text = match std::fs::read_to_string(&f) { Ok(t) => t, _ => continue };
    let lines: Vec<&str> = text.lines().collect();
    for line in lines.iter().rev() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let row = match DataObject::try_from_string(line) { Ok(r) => r, _ => continue };
        // gather this row's message ids + reply id
        let mut ids: Vec<String> = Vec::new();
        if let Ok(arr) = row.try_get_array("msg_ids") {
            for i in 0..arr.len() { ids.push(arr.get_string(i)); }
        }
        if let Ok(rid) = row.try_get_string("reply_id") { if !rid.is_empty() { ids.push(rid); } }
        for oid in ids {
            if oid.is_empty() || !store.exists("runtime", &oid) { continue; }
            let occ = store.get_data("runtime", &oid).get_object("data");
            let cid = if occ.has("content_id") { occ.get_string("content_id") } else { String::new() };
            if cid.is_empty() || !store.exists("runtime", &cid) { continue; }
            let cd = store.get_data("runtime", &cid).get_object("data");
            let full = if cd.has("text") { cd.get_string("text") } else { String::new() };
            if full.len() > frag.len() && full.starts_with(&frag) {
                let mut o = DataObject::new();
                o.put_string("status", "ok");
                o.put_string("id", &oid);
                o.put_string("full", &full);
                o.put_int("clipped_chars", (full.len() - frag.len()) as i64);
                return o;
            }
        }
    }
}
err("no captured message starts with that fragment - this cell may predate capture, or the text was not an LLM turn".to_string())