// epistemic_work (understandingloop.md Phase 4): the work queue derives
// from CLAIM STATE - no LLM, no randomness, nothing like the donor
// spawner's random injection (deleted for good by never being built).
// Kinds, by priority:
//   stale      (3): source hash drifted and not yet acknowledged -
//                   evidence the claim may be out of date; the one kind
//                   the executive may act on autonomously (decay).
//   review     (2): confidence at the low floor, or a drift already
//                   acknowledged - a human or a better extraction
//                   decides its fate; the executive only surfaces it.
//   unpromoted (1): subject-bearing claims still in the brain -
//                   promotion pressure. Surfaced, never auto-promoted:
//                   that channel stays curated (owner's ritual).
// "Gapped" (knowledge gaps) needs semantic judgment and waits for the
// salience tier (Phase 5). Read-only; safe at any tick rate.
// One-memory-cycle (A1/A2/B2): each domain is read as the UNION of its
// facet (legacy array or JSONL) and the instance-local overlay
// (runtime/agent/memory-overlay/<lib>.<ctl>.jsonl) - a later overlay
// line with the same claim supersedes, so the queue sees curation the
// moment it happens, before any promote ships it.
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
fn repo_base(repo: &str) -> String {
    // registered repo -> its path (runtime/dev/repos.json) - matches
    // remember's stamping side; empty = unknown repo, which is drift.
    if let Ok(txt) = std::fs::read_to_string("runtime/dev/repos.json") {
        if let Ok(rj) = DataObject::try_from_string(&txt) {
            if let Ok(r) = rj.try_get_object(repo) {
                if r.has("path") { return r.get_string("path"); }
            }
        }
    }
    String::new()
}
fn overlay_file(lib: &str, ctl: &str) -> String {
    format!("runtime/agent/memory-overlay/{}.{}.jsonl", lib, ctl)
}
fn parse_entries(src: &str) -> DataArray {
    // both facet formats: legacy pretty JSON array, or JSONL (B2)
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

let _ = esc; // shared helper block; serializer unused here
let store = DataStore::new();
let mut libs: Vec<String> = Vec::new();
if let Ok(rd) = std::fs::read_dir(&store.root) {
    for e in rd.flatten() {
        if e.path().is_dir() {
            if let Ok(n) = e.file_name().into_string() { libs.push(n); }
        }
    }
}
libs.sort();
let mut items: Vec<(i64, i64, DataObject)> = Vec::new();
let mut n_stale = 0i64;
let mut n_review = 0i64;
let mut n_unpromoted = 0i64;
let mut n_plan = 0i64;
for lib in libs {
    if !store.exists(&lib, "controls") { continue; }
    let list = store.get_data(&lib, "controls").get_object("data").get_array("list");
    for i in 0..list.len() {
        let item = list.get_object(i);
        if !item.has("name") || !item.has("id") { continue; }
        let name = item.get_string("name");
        let id = item.get_string("id");
        if !store.exists(&lib, &id) { continue; }
        let dd = store.get_data(&lib, &id).get_object("data");
        let msrc = if dd.has("memory") { dd.get_string("memory") } else { String::new() };
        let a = apply_overlay(parse_entries(&msrc), &lib, &name);
        if a.len() == 0 { continue; }
        for j in 0..a.len() {
            let e = match a.try_get_object(j) { Ok(e) => e, Err(_) => continue };
            if !e.has("claim") || e.has("superseded") { continue; }
            let claim = e.get_string("claim");
            let t0 = if e.has("time") { e.get_int("time") } else { 0 };
            let conf = if e.has("confidence") { e.get_string("confidence") } else { "medium".to_string() };
            let mut pushed = false;
            if e.has("source") {
                if let Ok(src) = e.try_get_object("source") {
                    // Two checkable shapes, one drift test: a store facet
                    // pointer or a registered-repo file pointer (brick 3).
                    let is_ptr = src.has("lib") && src.has("ctl") && src.has("facet") && src.has("hash");
                    let is_repo = !is_ptr && src.has("repo") && src.has("path") && src.has("hash");
                    if is_ptr || is_repo {
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
                            let base = repo_base(&repo);
                            if !base.is_empty() {
                                if let Ok(c) = std::fs::read_to_string(format!("{}/{}", base, path)) {
                                    current = content_hash(&c.replace("\r", ""));
                                }
                            }
                        }
                        if current != src.get_string("hash") {
                            let acknowledged = e.has("decayed_hash")
                                && e.get_string("decayed_hash") == current;
                            let mut it = DataObject::new();
                            it.put_string("lib", &lib);
                            it.put_string("domain", &name);
                            it.put_string("claim", &claim);
                            if acknowledged {
                                it.put_string("kind", "review");
                                it.put_int("priority", 2);
                                it.put_string("why", &format!("source {} drift acknowledged; awaiting review", srcdesc));
                                n_review += 1;
                                items.push((2, t0, it));
                            } else {
                                it.put_string("kind", "stale");
                                it.put_int("priority", 3);
                                it.put_string("why", &format!("source {} drifted since this claim was stamped", srcdesc));
                                n_stale += 1;
                                items.push((3, t0, it));
                            }
                            pushed = true;
                        }
                    }
                }
            }
            // plan (brick 5): kb.plan entries are INTENT, not belief -
            // the lifecycle rides the tags (proposed | accepted |
            // in-progress | done | abandoned). Active intent surfaces
            // as derived work; finished or dropped intent surfaces as
            // nothing. A plan entry whose source drifts still takes
            // the stale path above - evidence decay re-opens planning
            // by mechanism. Plan entries never fall through to the
            // belief kinds: low confidence on intent is not review
            // pressure, and intent is never promotion material.
            if lib == "kb" && name == "plan" {
                if !pushed {
                    let mut tags = String::new();
                    if e.has("tags") {
                        match e.get_property("tags") {
                            Data::DString(s) => tags = s,
                            Data::DArray(r) => {
                                let ta = DataArray::get(r);
                                for k in 0..ta.len() {
                                    if let Data::DString(s) = ta.get_property(k) {
                                        tags.push_str(&s);
                                        tags.push(',');
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    let tags = tags.to_lowercase();
                    let toks: Vec<&str> = tags.split(',').map(|t| t.trim()).collect();
                    let has_tag = |s: &str| toks.iter().any(|t| *t == s);
                    if !has_tag("done") && !has_tag("abandoned") {
                        let (pr, stage) = if has_tag("in-progress") { (2i64, "in-progress") }
                            else if has_tag("accepted") { (2i64, "accepted") }
                            else { (1i64, "proposed") };
                        let mut it = DataObject::new();
                        it.put_string("kind", "plan");
                        it.put_int("priority", pr);
                        it.put_string("lib", &lib);
                        it.put_string("domain", &name);
                        it.put_string("claim", &claim);
                        it.put_string("why", &format!("plan item ({}) - active intent in kb.plan, surfaced never auto-executed", stage));
                        n_plan += 1;
                        items.push((pr, t0, it));
                    }
                }
                continue;
            }
            if !pushed && conf == "low" {
                let mut it = DataObject::new();
                it.put_string("kind", "review");
                it.put_int("priority", 2);
                it.put_string("lib", &lib);
                it.put_string("domain", &name);
                it.put_string("claim", &claim);
                it.put_string("why", "confidence at the low floor - contradicted or decayed down; needs review");
                n_review += 1;
                items.push((2, t0, it));
                pushed = true;
            }
            if !pushed && e.has("subject") {
                let mut it = DataObject::new();
                it.put_string("kind", "unpromoted");
                it.put_int("priority", 1);
                it.put_string("lib", &lib);
                it.put_string("domain", &name);
                it.put_string("claim", &claim);
                it.put_string("why", &format!("subject '{}' claim still in the brain - awaits promote", e.get_string("subject")));
                n_unpromoted += 1;
                items.push((1, t0, it));
            }
        }
    }
}
items.sort_by(|x, y| y.0.cmp(&x.0).then(x.1.cmp(&y.1)));
let mut out = DataArray::new();
for (_, _, it) in items.iter().take(20) { out.push_object(it.clone()); }
let total = items.len() as i64;
let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_array("items", out);
o.put_int("stale", n_stale);
o.put_int("review", n_review);
o.put_int("unpromoted", n_unpromoted);
o.put_int("plan", n_plan);
o.put_int("total", total);
o