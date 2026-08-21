// agent-archivist-reverify - the curator the memory system has waited
// for (harvest H5). Picks up to `limit` STALE claims (source referent
// drifted since the hash was stamped), re-reads each referent's
// CURRENT content, and asks the frontier: does the claim still hold?
//   confirm -> the one write this act owns: re-stamp source.hash to
//     the current referent, mark reverified, step confidence up one
//     (drift checked and survived is evidence) - the stale flag
//     clears because the hash now matches. On a SHIPPED domain the
//     re-stamped entry is a superseding overlay line (A1); on a local
//     one it rewrites the facet with a journal entry, as ever.
//   amend  -> decay the old claim (drift acknowledged) + remember the
//     corrected text with a fresh source pointer - two governed
//     writes through the standing commands, which route themselves.
//   retire -> decay alone; at the low floor the claim waits in the
//     review queue for the owner. Autonomy never deletes.
// Traces go to the local traces log unconditionally (A2). An
// unparseable verdict skips the claim (counted); the arm erroring
// aborts with the count spent. Frontier spend rides the drive budget.
fn err(msg: String) -> DataObject {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", &msg);
    o
}
fn esc(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}
fn val(d: Data, ind: usize) -> String {
    match d {
        Data::DString(s) => format!("\"{}\"", esc(&s)),
        Data::DInt(i) => format!("{}", i),
        Data::DFloat(f) => format!("{}", f),
        Data::DBoolean(b) => format!("{}", b),
        Data::DNull => "null".to_string(),
        Data::DObject(r) => obj(DataObject::get(r), ind),
        Data::DArray(r) => {
            let a = DataArray::get(r);
            if a.len() == 0 { return "[]".to_string(); }
            let pad = "  ".repeat(ind + 1);
            let mut out = String::from("[");
            for i in 0..a.len() {
                if i > 0 { out.push(','); }
                out.push_str(&format!("\n{}{}", pad, val(a.get_property(i), ind + 1)));
            }
            out.push_str(&format!("\n{}]", "  ".repeat(ind)));
            out
        }
        _ => "null".to_string(),
    }
}
fn obj(o: DataObject, ind: usize) -> String {
    let canon = ["claim", "detail", "tags", "source", "confidence", "time",
                 "lib", "ctl", "facet", "hash", "doc", "repo", "path", "commit"];
    let mut keys: Vec<String> = canon.iter().filter(|k| o.has(k)).map(|s| s.to_string()).collect();
    let mut extra: Vec<String> = o.get_keys().into_iter()
        .filter(|k| !canon.contains(&k.as_str())).collect();
    extra.sort();
    keys.extend(extra);
    if keys.is_empty() { return "{}".to_string(); }
    let pad = "  ".repeat(ind + 1);
    let mut out = String::from("{");
    let mut first = true;
    for k in &keys {
        if !first { out.push(','); }
        first = false;
        out.push_str(&format!("\n{}\"{}\": {}", pad, esc(k), val(o.get_property(k), ind + 1)));
    }
    out.push_str(&format!("\n{}}}", "  ".repeat(ind)));
    out
}
fn jval(d: Data) -> String {
    match d {
        Data::DString(s) => format!("\"{}\"", esc(&s)),
        Data::DInt(i) => format!("{}", i),
        Data::DFloat(f) => format!("{}", f),
        Data::DBoolean(b) => format!("{}", b),
        Data::DNull => "null".to_string(),
        Data::DObject(r) => jobj(DataObject::get(r)),
        Data::DArray(r) => {
            let a = DataArray::get(r);
            let mut out = String::from("[");
            for i in 0..a.len() {
                if i > 0 { out.push_str(", "); }
                out.push_str(&jval(a.get_property(i)));
            }
            out.push(']');
            out
        }
        _ => "null".to_string(),
    }
}
fn jobj(o: DataObject) -> String {
    // obj()'s JSONL twin: same canonical field order, one line (B2).
    let canon = ["claim", "detail", "tags", "source", "confidence", "time",
                 "lib", "ctl", "facet", "hash", "doc", "repo", "path", "commit"];
    let mut keys: Vec<String> = canon.iter().filter(|k| o.has(k)).map(|s| s.to_string()).collect();
    let mut extra: Vec<String> = o.get_keys().into_iter()
        .filter(|k| !canon.contains(&k.as_str())).collect();
    extra.sort();
    keys.extend(extra);
    let mut out = String::from("{");
    let mut first = true;
    for k in &keys {
        if !first { out.push_str(", "); }
        first = false;
        out.push_str(&format!("\"{}\": {}", esc(k), jval(o.get_property(k))));
    }
    out.push('}');
    out
}
fn overlay_file(lib: &str, ctl: &str) -> String {
    format!("runtime/agent/memory-overlay/{}.{}.jsonl", lib, ctl)
}
fn parse_entries(src: &str) -> DataArray {
    let t = src.trim();
    let mut out = DataArray::new();
    if t.is_empty() { return out; }
    if t.starts_with('[') {
        if let Ok(w) = DataObject::try_from_string(&format!("{{\"a\":{}}}", t)) {
            if let Ok(a) = w.try_get_array("a") {
                for i in 0..a.len() {
                    if let Ok(o) = a.try_get_object(i) { out.push_object(o); }
                }
            }
        }
        return out;
    }
    for ln in t.lines() {
        let ln = ln.trim();
        if ln.is_empty() || !ln.starts_with('{') { continue; }
        if let Ok(o) = DataObject::try_from_string(ln) { out.push_object(o); }
    }
    out
}
fn apply_overlay(base: DataArray, lib: &str, ctl: &str) -> DataArray {
    let txt = std::fs::read_to_string(overlay_file(lib, ctl)).unwrap_or_default();
    if txt.trim().is_empty() { return base; }
    let mut v: Vec<DataObject> = Vec::new();
    for i in 0..base.len() {
        if let Ok(o) = base.try_get_object(i) { v.push(o); }
    }
    for ln in txt.lines() {
        let ln = ln.trim();
        if ln.is_empty() || !ln.starts_with('{') { continue; }
        if let Ok(o) = DataObject::try_from_string(ln) {
            if !o.has("claim") { continue; }
            let c = o.get_string("claim");
            let mut hit = false;
            for e in v.iter_mut() {
                if e.has("claim") && e.get_string("claim").trim() == c.trim() {
                    *e = o.clone();
                    hit = true;
                    break;
                }
            }
            if !hit { v.push(o); }
        }
    }
    let mut out = DataArray::new();
    for o in v { out.push_object(o); }
    out
}
fn overlay_append(lib: &str, ctl: &str, entry: DataObject) {
    let _ = std::fs::create_dir_all("runtime/agent/memory-overlay");
    let f = overlay_file(lib, ctl);
    let mut txt = std::fs::read_to_string(&f).unwrap_or_default();
    if !txt.is_empty() && !txt.ends_with('\n') { txt.push('\n'); }
    txt.push_str(&jobj(entry));
    txt.push('\n');
    let _ = std::fs::write(&f, txt);
}
fn trace_local(lib: &str, ctl: &str, trace: DataObject) {
    // A2: traces are operational exhaust - instance-local, capped.
    let _ = std::fs::create_dir_all("runtime/agent/memory-overlay");
    let f = format!("runtime/agent/memory-overlay/{}.{}.traces.jsonl", lib, ctl);
    let mut lines: Vec<String> = std::fs::read_to_string(&f).unwrap_or_default()
        .lines().map(|s| s.to_string()).collect();
    lines.push(jobj(trace));
    while lines.len() > 200 { lines.remove(0); }
    let _ = std::fs::write(&f, lines.join("\n") + "\n");
}
fn shipped(lib: &str) -> bool {
    // SHIPPED = data/<lib> resolves inside a registered repo's working
    // tree (longest match) AND git tracks bytes there.
    let dd = match std::fs::canonicalize(format!("data/{}", lib)) {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return false,
    };
    let mut best = String::new();
    if let Ok(txt) = std::fs::read_to_string("runtime/dev/repos.json") {
        if let Ok(rj) = DataObject::try_from_string(&txt) {
            for (_n, v) in rj.objects() {
                if let Data::DObject(r) = v {
                    let ro = DataObject::get(r);
                    if !ro.has("path") { continue; }
                    if let Ok(b) = std::fs::canonicalize(ro.get_string("path")) {
                        let b = b.to_string_lossy().to_string();
                        if (dd == b || dd.starts_with(&format!("{}/", b))) && b.len() > best.len() {
                            best = b;
                        }
                    }
                }
            }
        }
    }
    if best.is_empty() { return false; }
    let rel = if dd == best { ".".to_string() } else { dd[best.len() + 1..].to_string() };
    let mut a = DataArray::new();
    for s in ["git", "--no-optional-locks", "-C", best.as_str(), "ls-files", "--", rel.as_str()] {
        a.push_string(s);
    }
    let r = system_call(a);
    !r.try_get_string("out").unwrap_or_default().trim().is_empty()
}
fn content_hash(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", h)
}
fn ctl_id(store: &DataStore, lib: &str, name: &str) -> String {
    if !store.exists(lib, "controls") { return String::new(); }
    let rec = store.get_data(lib, "controls").get_object("data");
    if !rec.has("list") { return String::new(); }
    for c in rec.get_array("list").objects() {
        let c = c.object();
        if c.has("name") && c.get_string("name") == name { return c.get_string("id"); }
    }
    String::new()
}
fn repo_base(repo: &str) -> String {
    // registered repo -> its path (runtime/dev/repos.json) - matches
    // remember's stamping side; empty = unknown repo.
    if let Ok(txt) = std::fs::read_to_string("runtime/dev/repos.json") {
        if let Ok(rj) = DataObject::try_from_string(&txt) {
            if let Ok(r) = rj.try_get_object(repo) {
                if r.has("path") { return r.get_string("path"); }
            }
        }
    }
    String::new()
}
fn repo_head(base: &str) -> String {
    // HEAD sha via .git files (remember's resolver): ref file, then
    // packed-refs; empty when unresolvable - commit is provenance only.
    let head = std::fs::read_to_string(format!("{}/.git/HEAD", base)).unwrap_or_default();
    let head = head.trim().to_string();
    if let Some(r) = head.strip_prefix("ref: ") {
        if let Ok(s) = std::fs::read_to_string(format!("{}/.git/{}", base, r)) {
            return s.trim().to_string();
        }
        if let Ok(pk) = std::fs::read_to_string(format!("{}/.git/packed-refs", base)) {
            for line in pk.lines() {
                if line.starts_with('#') || line.starts_with('^') { continue; }
                if let Some((sha, name)) = line.split_once(' ') {
                    if name.trim() == r { return sha.to_string(); }
                }
            }
        }
        return String::new();
    }
    if head.len() == 40 && head.chars().all(|c| c.is_ascii_hexdigit()) { return head; }
    String::new()
}
fn ensure_key(data_obj: &mut DataObject, name: &str) {
    let mut keys = if data_obj.has("attachmentkeynames") {
        data_obj.get_array("attachmentkeynames")
    } else { DataArray::new() };
    let mut hasit = false;
    for i in 0..keys.len() {
        if let Data::DString(s) = keys.get_property(i) {
            if s == name { hasit = true; break; }
        }
    }
    if !hasit { keys.push_string(name); }
    data_obj.put_array("attachmentkeynames", keys);
}
fn journal(store: &DataStore, lib: &str, ctlid: &str, facet: &str, old: &str, new: &str, author: &str, label: &str) -> String {
    let jid = format!("{}_patches", ctlid);
    let mut jrec;
    let mut jdata;
    let mut jlist;
    if store.exists(lib, &jid) {
        jrec = store.get_data(lib, &jid);
        jdata = jrec.get_object("data");
        jlist = if jdata.has("list") { jdata.get_array("list") } else { DataArray::new() };
    } else {
        jrec = DataObject::new();
        jrec.put_string("id", &jid);
        jrec.put_string("username", "system");
        jrec.put_array("readers", DataArray::new());
        jrec.put_array("writers", DataArray::new());
        jdata = DataObject::new();
        jlist = DataArray::new();
    }
    let patch_id = format!("p{}", jlist.len() + 1);
    let mut jentry = DataObject::new();
    jentry.put_string("patch_id", &patch_id);
    jentry.put_string("author", author);
    jentry.put_string("facet", facet);
    jentry.put_string("cmd", "");
    jentry.put_string("old", old);
    jentry.put_string("new", new);
    jentry.put_int("time", time());
    let mut label = label.to_string();
    if label.chars().count() > 72 {
        label = label.chars().take(69).collect::<String>() + "...";
    }
    jentry.put_string("label", &label);
    jlist.push_object(jentry);
    jdata.put_array("list", jlist);
    jrec.put_object("data", jdata);
    jrec.put_int("time", time());
    store.set_data(lib, &jid, jrec);
    patch_id
}
fn conf_up(c: &str) -> &'static str {
    match c { "low" => "medium", _ => "high" }
}

if limit < 1 || limit > 10 {
    return err("limit must be 1..=10 - re-verification spends one frontier call per claim".to_string());
}
let store = DataStore::new();
let work = epistemic_work();
if !work.has("items") {
    return err("epistemic_work returned no items array".to_string());
}
let items = work.get_array("items");
let mut confirmed = 0i64;
let mut amended = 0i64;
let mut retired = 0i64;
let mut unparseable = 0i64;
let mut spent = 0i64;
for i in 0..items.len() {
    if spent >= limit { break; }
    let it = match items.try_get_object(i) { Ok(x) => x, Err(_) => continue };
    if !it.has("kind") || it.get_string("kind") != "stale" { continue; }
    let (wlib, wdom, wclaim) = (it.get_string("lib"), it.get_string("domain"), it.get_string("claim"));
    // the entry's source pointer, from the domain's effective entries
    // (facet in either format + overlay's superseding lines)
    let ctlid = ctl_id(&store, &wlib, &wdom);
    if ctlid.is_empty() || !store.exists(&wlib, &ctlid) { continue; }
    let mut record = store.get_data(&wlib, &ctlid);
    let mut data_obj = record.get_object("data");
    let old_source = if data_obj.has("memory") { data_obj.get_string("memory").replace("\r", "") } else { String::new() };
    let arr = apply_overlay(parse_entries(&old_source), &wlib, &wdom);
    let mut idx: i64 = -1;
    for j in 0..arr.len() {
        if let Ok(e) = arr.try_get_object(j) {
            if e.has("claim") && e.get_string("claim").trim() == wclaim.trim()
                && !e.has("superseded") { idx = j as i64; break; }
        }
    }
    if idx < 0 { continue; }
    let mut entry = arr.get_object(idx as usize);
    let src = match entry.try_get_object("source") { Ok(s) => s, Err(_) => continue };
    // Two checkable shapes (brick 3): a store facet pointer or a
    // registered-repo file pointer. Either way the curator re-reads the
    // referent's CURRENT bytes; a vanished referent is left for decay.
    let is_ptr = src.has("lib") && src.has("ctl") && src.has("facet");
    let is_repo = !is_ptr && src.has("repo") && src.has("path");
    if !is_ptr && !is_repo { continue; }
    let content;
    let srcdesc;
    let mut rcommit = String::new();
    if is_ptr {
        let (slib, sctl, sfacet) = (src.get_string("lib"), src.get_string("ctl"), src.get_string("facet"));
        let scid = ctl_id(&store, &slib, &sctl);
        if scid.is_empty() || !store.exists(&slib, &scid) { continue; }
        let sdata = store.get_data(&slib, &scid).get_object("data");
        if !sdata.has(&sfacet) { continue; }
        content = sdata.get_string(&sfacet).replace("\r", "");
        srcdesc = format!("{}.{} facet {}", slib, sctl, sfacet);
    } else {
        let repo = src.get_string("repo");
        let path = src.get_string("path");
        srcdesc = format!("{}:{}", repo, path);
        let base = repo_base(&repo);
        if base.is_empty() { continue; }
        match std::fs::read_to_string(format!("{}/{}", base, path)) {
            Ok(c) => { content = c.replace("\r", ""); }
            Err(_) => { continue; }
        }
        rcommit = repo_head(&base);
    }
    let cur_hash = content_hash(&content);
    let excerpt: String = content.chars().take(3000).collect();
    let detail = if entry.has("detail") { entry.get_string("detail") } else { String::new() };
    let prompt = format!(
        "You are the agent re-verifying one of its own memories against reality. The claim below was stamped against an older version of the referent; the referent has since changed. Decide whether the claim still holds of the CURRENT content.\nCLAIM: {}\nDETAIL: {}\nREFERENT ({}, current):\n{}\nReply with ONLY one JSON object, no fences:\n{{\"verdict\": \"confirm\" | \"amend\" | \"retire\", \"claim\": \"<the corrected claim if amend, else empty>\", \"why\": \"<one sentence>\"}}",
        wclaim, detail, srcdesc, excerpt);
    let reply = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ask_llm(prompt, Data::DNull)
    })).unwrap_or_else(|_| "ERROR: ask_llm panicked".to_string());
    if reply.starts_with("ERROR") {
        return err(format!("the frontier arm failed after {} claims: {}", spent,
            reply.chars().take(200).collect::<String>()));
    }
    spent += 1;
    let fd = reply.find('{').and_then(|s0| reply.rfind('}').map(|e0| (s0, e0)))
        .filter(|(s0, e0)| e0 > s0)
        .and_then(|(s0, e0)| DataObject::try_from_string(&reply[s0..=e0]).ok());
    let fd = match fd { Some(f) => f, None => { unparseable += 1; continue; } };
    let verdict = if fd.has("verdict") { fd.get_string("verdict") } else { String::new() };
    let why = if fd.has("why") { fd.get_string("why") } else { String::new() };
    let now = time();
    if verdict == "confirm" {
        // the curator's write: re-stamp the hash, mark reverified,
        // step confidence up - drift checked and survived is evidence
        let mut nsrc = src.deep_copy();
        nsrc.put_string("hash", &cur_hash);
        // the hash is the load-bearing re-stamp; on a repo source the
        // commit provenance moves with it
        if is_repo && !rcommit.is_empty() { nsrc.put_string("commit", &rcommit); }
        entry.put_object("source", nsrc);
        entry.put_int("reverified", now);
        let c = if entry.has("confidence") { entry.get_string("confidence") } else { "medium".to_string() };
        let up = conf_up(&c).to_string();
        entry.put_string("confidence", &up);
        // trace - local unconditionally (A2), adjudicate's shape
        let mut trace = DataObject::new();
        trace.put_string("input_claim", &wclaim);
        trace.put_string("relation", "reverified");
        trace.put_string("action", "confirmed");
        trace.put_string("before", &c);
        trace.put_string("after", &up);
        trace.put_string("reasoning", &why);
        trace.put_int("time", now);
        trace.put_string("author", "reverify");
        trace_local(&wlib, &wdom, trace);
        if shipped(&wlib) {
            // A1: the re-stamped entry is a superseding overlay line;
            // shipped bytes wait for promote.
            overlay_append(&wlib, &wdom, entry.clone());
        } else {
            let new_source = val(Data::DArray(arr.data_ref), 0) + "\n";
            data_obj.put_string("memory", &new_source);
            ensure_key(&mut data_obj, "memory");
            record.put_object("data", data_obj);
            record.put_int("time", now);
            store.set_data(&wlib, &ctlid, record);
            journal(&store, &wlib, &ctlid, "memory", &old_source, &new_source, "reverify",
                &format!("reverify: confirmed {}", wclaim));
        }
        confirmed += 1;
    } else if verdict == "amend" {
        let new_claim = if fd.has("claim") { fd.get_string("claim") } else { String::new() };
        if new_claim.trim().is_empty() { unparseable += 1; continue; }
        // acknowledge the drift on the old claim, insert the corrected
        // one with a FRESH source stamp - both through standing commands,
        // which route shipped domains to the overlay themselves
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            decay(wlib.clone(), wdom.clone(), wclaim.clone(), "reverify".to_string())
        }));
        let mut ne = DataObject::new();
        ne.put_string("claim", new_claim.trim());
        ne.put_string("detail", &format!("amended by reverify: {}", why));
        ne.put_string("tags", "reverified,amended");
        ne.put_string("confidence", "medium");
        // fresh pointer of the SAME shape as the old source; remember
        // stamps hash (and commit, for repo sources) at write time
        let mut nsrc = DataObject::new();
        if is_ptr {
            nsrc.put_string("lib", &src.get_string("lib"));
            nsrc.put_string("ctl", &src.get_string("ctl"));
            nsrc.put_string("facet", &src.get_string("facet"));
        } else {
            nsrc.put_string("repo", &src.get_string("repo"));
            nsrc.put_string("path", &src.get_string("path"));
        }
        ne.put_object("source", nsrc);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            remember(wlib.clone(), wdom.clone(), ne.deep_copy(), "reverify".to_string())
        }));
        amended += 1;
    } else if verdict == "retire" {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            decay(wlib.clone(), wdom.clone(), wclaim.clone(), "reverify".to_string())
        }));
        retired += 1;
    } else {
        unparseable += 1;
    }
}
let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_int("spent", spent);
o.put_int("confirmed", confirmed);
o.put_int("amended", amended);
o.put_int("retired", retired);
o.put_int("unparseable", unparseable);
o