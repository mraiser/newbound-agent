// decay (understandingloop.md Phase 4): the ONE autonomous write
// initiative permits - an evidence-based confidence step-down on a claim
// whose source referent drifted. Same ladder as adjudicate-contradicts,
// but the evidence is the drift itself, so each distinct drift state
// decays AT MOST ONE step: the observed referent hash is recorded
// (decayed_hash) and a repeat call against the same drift is a no-op.
// At the low floor only the acknowledgment is written, once - the claim
// then waits in the review queue. Convergent by construction: bounded
// writes per drift state, zero on repeat.
// One-memory-cycle (A1/A2): on a SHIPPED domain the step-down is a
// superseding line in the instance-local overlay - committed bytes are
// never touched; promote folds it later. Traces go to the local traces
// log unconditionally: operational exhaust is never shipped bytes.
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
    // A2: traces are operational exhaust - instance-local, never shipped
    // bytes; capped like the old facet was.
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
    // FNV-1a over the \r-normalized source - MUST stay in sync with
    // remember/recall/adjudicate/read_control_facet/patch_control_facet.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", h)
}
fn lookup_ctl_id(lib: &str, name: &str) -> String {
    let store = DataStore::new();
    if !store.exists(lib, "controls") { return String::new(); }
    let rec = store.get_data(lib, "controls").get_object("data");
    if !rec.has("list") { return String::new(); }
    for c in rec.get_array("list").objects() {
        let c = c.object();
        if c.has("name") && c.get_string("name") == name {
            return c.get_string("id");
        }
    }
    String::new()
}

fn err(msg: String) -> DataObject {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", &msg);
    o
}
fn ensure_key(data_obj: &mut DataObject, name: &str) {
    let mut keys = if data_obj.has("attachmentkeynames") {
        data_obj.get_array("attachmentkeynames")
    } else {
        DataArray::new()
    };
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

let ctlid = lookup_ctl_id(&lib, &domain);
let store = DataStore::new();
if !store.exists(&lib, &ctlid) {
    return err(format!("Domain control '{}' not found in library '{}'", domain, lib));
}
let mut record = store.get_data(&lib, &ctlid);
let mut data_obj = record.get_object("data");
let old_source = if data_obj.has("memory") {
    data_obj.get_string("memory").replace("\r", "")
} else {
    String::new()
};
let routed = shipped(&lib);
let arr;
if routed {
    // union read: the facet (either format) plus the overlay's
    // superseding lines - drift state lives wherever it was last written
    arr = apply_overlay(parse_entries(&old_source), &lib, &domain);
    if arr.len() == 0 {
        return err(format!("{}.{} has no memory entries", lib, domain));
    }
} else {
    if old_source.trim().is_empty() {
        return err(format!("{}.{} has no memory facet", lib, domain));
    }
    arr = match DataObject::try_from_string(&format!("{{\"a\":{}}}", old_source)) {
        Ok(w) => match w.try_get_array("a") {
            Ok(a) => a,
            Err(_) => return err(format!("{}.{}'s memory facet is not a JSON ARRAY", lib, domain)),
        },
        Err(_) => return err(format!("{}.{}'s memory facet is not valid JSON", lib, domain)),
    };
}
let mut found: i64 = -1;
for i in 0..arr.len() {
    let e = match arr.try_get_object(i) { Ok(e) => e, Err(_) => continue };
    if e.has("claim") && !e.has("superseded") && e.get_string("claim").trim() == claim.trim() {
        found = i as i64;
        break;
    }
}
if found < 0 {
    return err(format!("no live entry with that exact claim in {}.{}", lib, domain));
}
let mut e = arr.get_object(found as usize);
// Two checkable shapes, one drift test: a store facet pointer or a
// registered-repo file pointer (brick 3). The repo path resolves through
// runtime/dev/repos.json - an unregistered repo reads as "missing", drift.
let src = match e.try_get_object("source") {
    Ok(s) if (s.has("lib") && s.has("ctl") && s.has("facet") && s.has("hash"))
          || (s.has("repo") && s.has("path") && s.has("hash")) => s,
    _ => return err("claim has no checkable source pointer - decay needs drift evidence".to_string()),
};
let is_ptr = src.has("lib") && src.has("ctl") && src.has("facet");
let mut current = "missing".to_string();
let srcdesc;
if is_ptr {
    let slib = src.get_string("lib");
    let sctl = src.get_string("ctl");
    let sfacet = src.get_string("facet");
    srcdesc = format!("{}.{}:{}", slib, sctl, sfacet);
    let sid = lookup_ctl_id(&slib, &sctl);
    if !sid.is_empty() && store.exists(&slib, &sid) {
        let sdata = store.get_data(&slib, &sid).get_object("data");
        if sdata.has(&sfacet) {
            current = content_hash(&sdata.get_string(&sfacet).replace("\r", ""));
        }
    }
} else {
    let repo = src.get_string("repo");
    let path = src.get_string("path");
    srcdesc = format!("{}:{}", repo, path);
    if let Ok(txt) = std::fs::read_to_string("runtime/dev/repos.json") {
        if let Ok(rj) = DataObject::try_from_string(&txt) {
            if let Ok(r) = rj.try_get_object(&repo) {
                if r.has("path") {
                    if let Ok(c) = std::fs::read_to_string(format!("{}/{}", r.get_string("path"), path)) {
                        current = content_hash(&c.replace("\r", ""));
                    }
                }
            }
        }
    }
}
if current == src.get_string("hash") {
    let mut o = DataObject::new();
    o.put_string("status", "ok");
    o.put_string("action", "fresh");
    return o;
}
if e.has("decayed_hash") && e.get_string("decayed_hash") == current {
    let mut o = DataObject::new();
    o.put_string("status", "ok");
    o.put_string("action", "already_acknowledged");
    return o;
}
let now = time();
let c = if e.has("confidence") { e.get_string("confidence") } else { "medium".to_string() };
let action;
let mut after = String::new();
if c == "low" {
    e.put_string("decayed_hash", &current);
    e.put_int("adjudicated", now);
    action = "at_floor".to_string();
} else {
    after = (if c == "high" { "medium" } else { "low" }).to_string();
    e.put_string("confidence", &after);
    e.put_string("decayed_hash", &current);
    e.put_int("adjudicated", now);
    action = "decayed".to_string();
}
// trace - local unconditionally (A2), adjudicate's shape
let mut trace = DataObject::new();
trace.put_string("input_claim", &claim);
trace.put_string("relation", "drift");
trace.put_string("action", &action);
trace.put_string("before", &c);
if !after.is_empty() { trace.put_string("after", &after); }
trace.put_string("reasoning", &format!("source {} drifted; hysteresis step", srcdesc));
trace.put_int("time", now);
trace.put_string("author", &author);
trace_local(&lib, &domain, trace);
let mut patch_id = String::new();
if routed {
    // A1: the superseding line goes to the overlay; committed bytes stay
    // untouched until promote folds them on a branch.
    overlay_append(&lib, &domain, e.clone());
} else {
    let new_source = val(Data::DArray(arr.data_ref), 0) + "\n";
    data_obj.put_string("memory", &new_source);
    ensure_key(&mut data_obj, "memory");
    record.put_object("data", data_obj);
    record.put_int("time", now);
    store.set_data(&lib, &ctlid, record);
    patch_id = journal(&store, &lib, &ctlid, "memory", &old_source, &new_source, &author,
        &format!("decay: {} {}", action, claim));
}
let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_string("action", &action);
o.put_string("before", &c);
if !after.is_empty() { o.put_string("after", &after); }
if routed { o.put_string("routed", "overlay"); }
if !patch_id.is_empty() { o.put_string("patch_id", &patch_id); }
o