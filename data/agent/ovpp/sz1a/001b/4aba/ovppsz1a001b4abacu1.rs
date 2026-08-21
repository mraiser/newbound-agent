// promote (one-memory-cycle A1 + B2): THE door to shipped memory bytes.
// Two feeds, one deliberate act:
//   1. the instance-local OVERLAY (runtime/agent/memory-overlay/
//      <lib>.<ctl>.jsonl) - every deposit and curation delta the instance
//      accumulated against this library's shipped manuals - is FOLDED
//      into the facets and the folded files deleted (their content now
//      lives in tracked bytes);
//   2. the brain's unpromoted subject claims (kb entries whose `subject`
//      names this library) are union-merged in on exact-claim identity,
//      and the brain copies stamped `promoted` (journaled patch on the
//      kb domain). A bare-library subject ("dev") files onto the
//      eponymous control (dev.dev).
// The facet is (re)written as JSONL - one canonical object per line (B2) -
// so parallel branches' promotes union-merge in git without textual
// conflict (.gitattributes: *.memory merge=union). Explicit trigger only;
// run it while preparing a branch commit - this is the deliberate act
// that makes shipped-manual diffs authored and reviewable.
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
    // Canonical field order: the entry shape first, then pointer fields,
    // then anything else sorted - hash-backed ndata loses file order, so
    // a fixed order is what makes rewrites diff cleanly.
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
    // obj()'s JSONL twin: same canonical field order, one line (B2) -
    // the unit shipped facets are made of from this cycle on.
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
    // both facet formats: legacy pretty JSON array, or JSONL
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
// load one target control's state: (id, raw old facet, effective entries
// after overlay, overlay line count). None = control not found.
fn load_target(store: &DataStore, lib: &str, ctl: &str) -> Option<(String, String, DataArray, i64)> {
    let id = ctl_id(store, lib, ctl);
    if id.is_empty() || !store.exists(lib, &id) { return None; }
    let dd = store.get_data(lib, &id).get_object("data");
    let old = if dd.has("memory") { dd.get_string("memory") } else { String::new() };
    let base = parse_entries(&old.replace("\r", ""));
    let mut folded = 0i64;
    let mut v: Vec<DataObject> = Vec::new();
    for i in 0..base.len() {
        if let Ok(o) = base.try_get_object(i) { v.push(o); }
    }
    let txt = std::fs::read_to_string(overlay_file(lib, ctl)).unwrap_or_default();
    for ln in txt.lines() {
        let ln = ln.trim();
        if ln.is_empty() || !ln.starts_with('{') { continue; }
        if let Ok(o) = DataObject::try_from_string(ln) {
            if !o.has("claim") { continue; }
            folded += 1;
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
    let mut eff = DataArray::new();
    for o in v { eff.push_object(o); }
    Some((id, old, eff, folded))
}

let store = DataStore::new();
let mut promoted: i64 = 0;
let mut already: i64 = 0;
let mut folded_total: i64 = 0;
let mut failures = DataArray::new();
// (ctl, id, old facet, effective entries, write_needed, overlay lines)
let mut loaded: Vec<(String, String, String, DataArray, bool, i64)> = Vec::new();

// ── feed 1: the overlay files accumulated against this library ──────────
let pre = format!("{}.", lib);
if let Ok(rd) = std::fs::read_dir("runtime/agent/memory-overlay") {
    for ent in rd.flatten() {
        if let Ok(n) = ent.file_name().into_string() {
            if n.starts_with(&pre) && n.ends_with(".jsonl") && !n.ends_with(".traces.jsonl") {
                let ctl = n[pre.len()..n.len() - 6].to_string();
                if ctl.is_empty() || loaded.iter().any(|t| t.0 == ctl) { continue; }
                match load_target(&store, &lib, &ctl) {
                    Some((id, old, eff, folded)) => {
                        let wn = folded > 0;
                        loaded.push((ctl, id, old, eff, wn, folded));
                    }
                    None => {
                        let mut f = DataObject::new();
                        f.put_string("domain", &ctl);
                        f.put_string("msg", &format!("overlay file exists but control {}.{} was not found - overlay left in place", lib, ctl));
                        failures.push_object(f);
                    }
                }
            }
        }
    }
}

// ── feed 2: the brain's unpromoted subject claims naming this library ───
if store.exists("kb", "controls") {
    let list = store.get_data("kb", "controls").get_object("data").get_array("list");
    for ci in 0..list.len() {
        let item = list.get_object(ci);
        if !item.has("name") || !item.has("id") { continue; }
        let ctl_name = item.get_string("name");
        let kb_id = item.get_string("id");
        if !store.exists("kb", &kb_id) { continue; }
        let dd = store.get_data("kb", &kb_id).get_object("data");
        if !dd.has("memory") { continue; }
        let old_source = dd.get_string("memory");
        if old_source.trim().is_empty() { continue; }
        let arr = parse_entries(&old_source.replace("\r", ""));
        if arr.len() == 0 { continue; }
        let mut stamped: i64 = 0;
        for i in 0..arr.len() {
            let e = match arr.try_get_object(i) { Ok(x) => x, Err(_) => continue };
            if !e.has("claim") || !e.has("subject") || e.has("promoted") { continue; }
            let subject = e.get_string("subject");
            let (tlib, tctl) = match subject.find('.') {
                Some(p) => (subject[..p].to_string(), subject[p + 1..].to_string()),
                None => (subject.clone(), subject.clone()),
            };
            if tlib != lib { continue; }
            let mut ti: i64 = -1;
            for (n, t) in loaded.iter().enumerate() {
                if t.0 == tctl { ti = n as i64; break; }
            }
            if ti < 0 {
                match load_target(&store, &lib, &tctl) {
                    Some((id, old, eff, folded)) => {
                        let wn = folded > 0;
                        loaded.push((tctl.clone(), id, old, eff, wn, folded));
                        ti = (loaded.len() - 1) as i64;
                    }
                    None => {
                        let mut f = DataObject::new();
                        f.put_string("domain", &ctl_name);
                        f.put_string("claim", &e.get_string("claim"));
                        f.put_string("msg", &format!("subject control {}.{} not found", lib, tctl));
                        failures.push_object(f);
                        continue;
                    }
                }
            }
            let claim = e.get_string("claim");
            let t = &mut loaded[ti as usize];
            let mut dup = false;
            for j in 0..t.3.len() {
                if let Ok(x) = t.3.try_get_object(j) {
                    if x.has("claim") && x.get_string("claim").trim() == claim.trim() {
                        dup = true;
                        break;
                    }
                }
            }
            if dup {
                already += 1;
            } else {
                let mut copy = e.deep_copy();
                copy.remove_property("subject");
                t.3.push_object(copy);
                t.4 = true;
                promoted += 1;
            }
            // A claim already present in the target still gets its brain
            // copy stamped: the union holds either way.
            let mut e = e;
            e.put_int("promoted", time());
            stamped += 1;
        }
        if stamped > 0 {
            let new_source = val(Data::DArray(arr.data_ref), 0) + "\n";
            let pc = Command::lookup("dev", "code", "patch_control_facet");
            let mut args = DataObject::new();
            args.put_string("lib", "kb");
            args.put_string("ctl", &ctl_name);
            args.put_string("facet", "memory");
            args.put_string("old_snippet", &old_source);
            args.put_string("new_snippet", &new_source);
            args.put_string("base", "");
            args.put_string("label", &format!("promote: {} claim(s) -> {}", stamped, lib));
            args.put_string("author", "promote");
            args.put_string("nn_sessionid", "");
            let stamp_ok = match pc.execute(args) {
                Ok(r) => r.has("a") && r.get_object("a").has("status")
                         && r.get_object("a").get_string("status") == "ok",
                Err(_) => false,
            };
            if !stamp_ok {
                let mut f = DataObject::new();
                f.put_string("domain", &ctl_name);
                f.put_string("msg", "filed to target but stamping the brain copy failed - a re-promote will report these as already_present");
                failures.push_object(f);
            }
        }
    }
}

// ── the one deliberate write to shipped bytes: JSONL facets, journaled ──
for t in &loaded {
    let (ctl, id, _old, eff, write_needed, folded) = (&t.0, &t.1, &t.2, &t.3, t.4, t.5);
    if write_needed {
        let mut new_source = String::new();
        for j in 0..eff.len() {
            if let Ok(o) = eff.try_get_object(j) {
                new_source.push_str(&jobj(o));
                new_source.push('\n');
            }
        }
        let mut record = store.get_data(&lib, id);
        let mut dd = record.get_object("data");
        let old_facet = if dd.has("memory") { dd.get_string("memory") } else { String::new() };
        dd.put_string("memory", &new_source);
        ensure_key(&mut dd, "memory");
        record.put_object("data", dd);
        record.put_int("time", time());
        store.set_data(&lib, id, record);
        journal(&store, &lib, id, "memory", &old_facet, &new_source, "promote",
            &format!("promote: fold -> {}.{}", lib, ctl));
        folded_total += folded;
        let _ = std::fs::remove_file(overlay_file(&lib, ctl));
    } else if folded == 0 {
        // an empty overlay file is residue, not knowledge
        let _ = std::fs::remove_file(overlay_file(&lib, ctl));
    }
}

let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_int("promoted", promoted);
o.put_int("already_present", already);
o.put_int("folded", folded_total);
o.put_array("failures", failures);
o