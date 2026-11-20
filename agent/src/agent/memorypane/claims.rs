use ndata::dataobject::DataObject;
use ndata::dataarray::DataArray;
use flowlang::datastore::DataStore;
use flowlang::flowlang::system::time::time;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["lib", "ctl"] {
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
        let arg_0: String = o.get_string("lib");
        let arg_1: String = o.get_string("ctl");
        claims(arg_0, arg_1)
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

pub fn claims(lib: String, ctl: String) -> DataObject {
// Every claim of ONE memory domain (lib.ctl), as recall would see it: the
// shipped facet (legacy array or line-oriented JSON) with the
// instance-local overlay applied, latest line per claim winning. Unlike
// recall there is no query and no cap. Read-only.
fn content_hash(s: &str) -> String {
    // FNV-1a over the \r-normalized source - in sync with remember/recall.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
    format!("{:016x}", h)
}
fn repo_base(repo: &str) -> String {
    if let Ok(txt) = std::fs::read_to_string("runtime/dev/repos.json") {
        if let Ok(rj) = DataObject::try_from_string(&txt) {
            if let Ok(r) = rj.try_get_object(repo) {
                if r.has("path") { return r.get_string("path"); }
            }
        }
    }
    String::new()
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
fn parse_entries(src: &str) -> Vec<DataObject> {
    let t = src.trim();
    let mut out = Vec::new();
    if t.is_empty() { return out; }
    if t.starts_with('[') {
        if let Ok(w) = DataObject::try_from_string(&format!("{{\"a\":{}}}", t)) {
            if let Ok(a) = w.try_get_array("a") {
                for i in 0..a.len() {
                    if let Ok(o) = a.try_get_object(i) { out.push(o); }
                }
            }
        }
        return out;
    }
    for ln in t.lines() {
        let ln = ln.trim();
        if ln.is_empty() || !ln.starts_with('{') { continue; }
        if let Ok(o) = DataObject::try_from_string(ln) { out.push(o); }
    }
    out
}

let mut o = DataObject::new();
let store = DataStore::new();
let id = ctl_id(&store, &lib, &ctl);
if id.is_empty() || !store.exists(&lib, &id) {
    o.put_string("status", "err");
    o.put_string("msg", &format!("no control {}.{}", lib, ctl));
    return o;
}
let dd = store.get_data(&lib, &id).get_object("data");
let facet_exists = dd.has("memory");
let msrc = if facet_exists { dd.get_string("memory") } else { String::new() };
// (entry, origin): shipped | overlay (overlay only) | both (overlay line supersedes a shipped one)
let mut v: Vec<(DataObject, &str)> = Vec::new();
for e in parse_entries(&msrc) {
    if e.has("claim") { v.push((e, "shipped")); }
}
let shipped = v.len() as i64;
let otxt = std::fs::read_to_string(format!("runtime/agent/memory-overlay/{}.{}.jsonl", lib, ctl)).unwrap_or_default();
let mut overlay_lines: i64 = 0;
for ln in otxt.lines() {
    let ln = ln.trim();
    if ln.is_empty() || !ln.starts_with('{') { continue; }
    if let Ok(e) = DataObject::try_from_string(ln) {
        if !e.has("claim") { continue; }
        overlay_lines += 1;
        let c = e.get_string("claim");
        let mut hit = false;
        for slot in v.iter_mut() {
            if slot.0.get_string("claim").trim() == c.trim() {
                let org = if slot.1 == "overlay" { "overlay" } else { "both" };
                *slot = (e.clone(), org);
                hit = true;
                break;
            }
        }
        if !hit { v.push((e, "overlay")); }
    }
}
let now = time();
let mut claims = DataArray::new();
let mut stale_n: i64 = 0;
let mut live: i64 = 0;
for (e, origin) in v {
    let mut out = DataObject::new();
    out.put_string("claim", &e.get_string("claim"));
    for k in ["detail", "tags", "confidence", "author", "subject"] {
        if e.has(k) { out.put_string(k, &ndata::data::Data::as_string(e.get_property(k))); }
    }
    let t0 = if e.has("time") { e.try_get_int("time").unwrap_or(0) } else { 0 };
    out.put_int("time", t0);
    out.put_int("age_days", if t0 > 0 { (now - t0) / 86_400_000 } else { -1 });
    out.put_string("origin", origin);
    out.put_boolean("promoted", e.has("promoted"));
    let superseded = e.has("superseded");
    out.put_boolean("superseded", superseded);
    if !superseded { live += 1; }
    let mut stale = false;
    let mut checked = false;
    if let Ok(src) = e.try_get_object("source") {
        out.put_object("source", src.deep_copy());
        if src.has("lib") && src.has("ctl") && src.has("facet") && src.has("hash") {
            checked = true;
            let slib = src.get_string("lib");
            let sid = ctl_id(&store, &slib, &src.get_string("ctl"));
            stale = true;
            if !sid.is_empty() && store.exists(&slib, &sid) {
                let sdata = store.get_data(&slib, &sid).get_object("data");
                let sfacet = src.get_string("facet");
                if sdata.has(&sfacet) {
                    stale = content_hash(&sdata.get_string(&sfacet).replace("\r", "")) != src.get_string("hash");
                }
            }
        } else if src.has("repo") && src.has("path") && src.has("hash") {
            checked = true;
            let base = repo_base(&src.get_string("repo"));
            stale = true;
            if !base.is_empty() {
                if let Ok(c) = std::fs::read_to_string(format!("{}/{}", base, src.get_string("path"))) {
                    stale = content_hash(&c.replace("\r", "")) != src.get_string("hash");
                }
            }
        }
    }
    out.put_boolean("stale", stale);
    out.put_boolean("stale_checked", checked);
    if stale && !superseded { stale_n += 1; }
    claims.push_object(out);
}
o.put_string("status", "ok");
o.put_string("home", &format!("{}.{}", lib, ctl));
o.put_boolean("facet_exists", facet_exists);
o.put_int("shipped", shipped);
o.put_int("overlay_lines", overlay_lines);
o.put_int("count", live);
o.put_int("stale", stale_n);
o.put_array("claims", claims);
o
}
