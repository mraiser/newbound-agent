use ndata::dataobject::DataObject;
use ndata::dataarray::DataArray;
use ndata::data::Data;
use std::sync::{Mutex, OnceLock};
use std::collections::{HashMap, VecDeque};
use crate::agent::plan::store;

pub const LIB: &str = "kb";
pub const NS: &str = "plan";

// First use on an instance: copy the legacy kb.plan memory facet into
// records (verified by the importer). The facet itself is never modified.
pub fn ensure() {
    if store::has_index(LIB, NS) { return; }
    static ONCE: OnceLock<Mutex<()>> = OnceLock::new();
    let _g = ONCE.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|e| e.into_inner());
    if store::has_index(LIB, NS) { return; }
    if let Some(e) = store::legacy_entries() { let _ = store::import_entries(LIB, NS, &e, false); }
}

fn cobj(o: &DataObject) -> String { store::canon(&Data::DObject(o.data_ref)) }
fn carr(a: &DataArray) -> String { store::canon(&Data::DArray(a.data_ref)) }

// The last few boards handed out, as one fingerprint per entry, keyed by
// board hash (+ the import stamp, so a re-imported board never matches an
// old key). A whole-board writer passes back the hash it read: an entry
// whose fingerprint still equals the one it was handed was not changed by
// that writer and is skipped, whatever has happened to the record since.
// Process-local; without a snapshot every entry is compared with its record.
fn snaps() -> &'static Mutex<VecDeque<(String, HashMap<String, String>)>> {
    static S: OnceLock<Mutex<VecDeque<(String, HashMap<String, String>)>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(VecDeque::new()))
}
fn snap_key(hash: &str) -> String {
    format!("{}@{}", hash, store::read_index(LIB, NS).try_get_int("imported").unwrap_or(0))
}
fn remember_snapshot(b: &DataObject) {
    let key = snap_key(&b.try_get_string("hash").unwrap_or_default());
    {
        let s = snaps().lock().unwrap_or_else(|e| e.into_inner());
        if s.iter().any(|x| x.0 == key) { return; }
    }
    let mut m = HashMap::new();
    if let Ok(a) = b.try_get_array("entries") {
        for i in 0..a.len() {
            if let Ok(e) = a.try_get_object(i) { m.insert(e.try_get_string("item_id").unwrap_or_default(), store::fnv(&cobj(&e))); }
        }
    }
    let mut s = snaps().lock().unwrap_or_else(|e| e.into_inner());
    s.push_back((key, m));
    while s.len() > 8 { s.pop_front(); }
}
fn snapshot(hash: &str) -> Option<HashMap<String, String>> {
    if hash.trim().is_empty() { return None; }
    let key = snap_key(hash.trim());
    let s = snaps().lock().unwrap_or_else(|e| e.into_inner());
    s.iter().find(|x| x.0 == key).map(|x| x.1.clone())
}

pub fn board() -> DataObject { ensure(); let b = store::board(LIB, NS); remember_snapshot(&b); b }
pub fn entries() -> DataArray { board().get_array("entries") }
pub fn hash() -> String { ensure(); format!("r{}", store::read_index(LIB, NS).try_get_int("rev").unwrap_or(0)) }

// The board in the shape dev.code.read_control_facet gave for the old
// facet: {exists, hash, source} with source = the entry array as JSON.
pub fn facet_read() -> DataObject {
    let b = board();
    let mut o = DataObject::new();
    o.put_string("status", "ok");
    o.put_boolean("exists", true);
    o.put_string("hash", &b.get_string("hash"));
    o.put_string("source", &b.get_array("entries").to_string());
    o
}

fn strip(e: &DataObject) -> DataObject {
    let mut i = e.deep_copy();
    for k in ["notes", "step_log", "item_id", "rev"] { if i.has(k) { i.remove_property(k); } }
    i
}
fn meta(item: &DataObject, rev: i64, notes: i64, steps: i64) -> DataObject {
    let mut m = DataObject::new();
    m.put_string("claim", &item.try_get_string("claim").unwrap_or_default());
    m.put_string("lifecycle", &store::lifecycle_of(&item.try_get_string("tags").unwrap_or_default()));
    m.put_string("parent", &item.try_get_string("parent").unwrap_or_default());
    m.put_int("time", item.try_get_int("time").unwrap_or(0));
    m.put_int("rev", rev); m.put_int("notes", notes); m.put_int("steps", steps);
    m
}
fn write_list(kind: &str, id: &str, list: DataArray) {
    let mut d = DataObject::new(); d.put_array("list", list);
    store::write_rec(LIB, &store::rec_id(NS, kind, id), d);
}

// Same signature as dev.code.patch_control_facet, so a writer that used to
// replace the whole facet switches by changing the function name. `source`
// is the whole entry array and `base` the board hash it was read at. Only
// the entries the writer CHANGED are written: an entry equal to what the
// writer was handed (see snaps) is skipped, and one equal to its record is
// a no-op. Concurrency is per item: a changed entry must carry the `rev` it
// was read with, else stale_base - so a write to one item never invalidates
// a writer of another. Notes are append-only: stored notes beyond the
// incoming list are kept. Entries absent from `source` are left alone.
pub fn facet_write(_lib: String, _ctl: String, _facet: String, _old: String, source: String, base: String, _label: String, _author: String, _sid: String) -> DataObject {
    let arr = match DataObject::try_from_string(&format!("{{\"a\": {} }}", source)).ok().and_then(|o| o.try_get_array("a").ok()) {
        Some(a) => a,
        None => return store::err("plan write: the board did not parse as a JSON array"),
    };
    ensure();
    let snap = snapshot(&base);
    let _g = store::lock();
    let mut idx = store::read_index(LIB, NS);
    let mut items = idx.get_object("items");
    let mut order = idx.get_array("order");
    let cur_hash = format!("r{}", idx.try_get_int("rev").unwrap_or(0));
    let mut writes: Vec<(String, Option<DataObject>, Option<DataArray>, Option<DataArray>, bool)> = Vec::new();
    let mut new_claims: Vec<String> = Vec::new();
    for i in 0..arr.len() {
        let e = match arr.try_get_object(i) { Ok(e) => e, Err(_) => continue };
        let claim = e.try_get_string("claim").unwrap_or_default();
        if claim.trim().is_empty() { continue; }
        let hinted = e.try_get_string("item_id").unwrap_or_default();
        let id = if !hinted.is_empty() && items.has(&hinted) { Some(hinted) } else { store::find_id(&idx, &claim) };
        match id {
            None => {
                if new_claims.contains(&claim) { continue; }
                new_claims.push(claim.clone());
                writes.push((String::new(), Some(strip(&e)),
                    e.try_get_array("notes").ok().map(|a| a.deep_copy()),
                    e.try_get_array("step_log").ok().map(|a| a.deep_copy()), true));
            }
            Some(id) => {
                if let Some(sn) = &snap {
                    if sn.get(&id).map(|f| *f == store::fnv(&cobj(&e))).unwrap_or(false) { continue; }
                }
                let clean = strip(&e);
                let item_changed = match store::read_item(LIB, NS, &id) { Some(s) => cobj(&s) != cobj(&clean), None => true };
                let steps = match e.try_get_array("step_log") {
                    Ok(a) => if carr(&a) != carr(&store::read_steps(LIB, NS, &id)) { Some(a.deep_copy()) } else { None },
                    Err(_) => None,
                };
                let notes = match e.try_get_array("notes") {
                    Ok(a) => {
                        let cur = store::read_notes(LIB, NS, &id);
                        let mut merged = a.deep_copy();
                        for k in a.len()..cur.len() { if let Ok(o) = cur.try_get_object(k) { merged.push_object(o.deep_copy()); } }
                        if carr(&merged) != carr(&cur) { Some(merged) } else { None }
                    }
                    Err(_) => None,
                };
                if !item_changed && steps.is_none() && notes.is_none() { continue; }
                let heavy = item_changed || steps.is_some();
                if heavy {
                    let cur_rev = items.get_object(&id).try_get_int("rev").unwrap_or(0);
                    if let Ok(r) = e.try_get_int("rev") {
                        if r != cur_rev {
                            let mut o = store::err("stale_base");
                            o.put_string("current_hash", &cur_hash);
                            o.put_string("item_id", &id);
                            o.put_string("claim", &claim);
                            return o;
                        }
                    }
                }
                writes.push((id, if heavy { Some(clean) } else { None }, notes, steps, false));
            }
        }
    }
    let mut out = DataObject::new();
    out.put_string("status", "ok"); out.put_boolean("ok", true);
    out.put_int("changed", writes.len() as i64);
    if writes.is_empty() { out.put_string("hash", &cur_hash); return out; }
    for (id, item, notes, steps, is_new) in writes {
        let id = if is_new {
            let c = item.as_ref().map(|i| i.try_get_string("claim").unwrap_or_default()).unwrap_or_default();
            let nid = store::mint_id(&idx, &c);
            order.push_string(&nid);
            nid
        } else { id };
        let (rev0, at0) = match items.try_get_object(&id) {
            Ok(m) => (m.try_get_int("rev").unwrap_or(0), m.try_get_int("at").unwrap_or(0)),
            Err(_) => (0, 0),
        };
        let (mut rev, mut n, mut s) = match items.try_get_object(&id) {
            Ok(m) => (m.try_get_int("rev").unwrap_or(0), m.try_get_int("notes").unwrap_or(0), m.try_get_int("steps").unwrap_or(0)),
            Err(_) => (0, 0, 0),
        };
        if let Some(a) = notes { n = a.len() as i64; write_list("notes", &id, a); }
        if let Some(a) = steps { s = a.len() as i64; write_list("steps", &id, a); }
        let rec = match item {
            Some(it) => { rev += 1; store::write_rec(LIB, &store::rec_id(NS, "item", &id), it.clone()); it }
            None => store::read_item(LIB, NS, &id).unwrap_or_else(DataObject::new),
        };
        // `at`: the index rev at which the item's record last changed (see store::put_item).
        let mut m = meta(&rec, rev, n, s);
        m.put_int("at", if rev != rev0 { idx.try_get_int("rev").unwrap_or(0) + 1 } else { at0 });
        items.put_object(&id, m);
    }
    let rev = idx.try_get_int("rev").unwrap_or(0) + 1;
    idx.put_int("rev", rev);
    store::write_rec(LIB, &store::index_id(NS), idx);
    out.put_string("hash", &format!("r{}", rev));
    out
}

// Files one new item (agent.archivist.remember's kb.plan path). An exact
// duplicate claim is refused, as remember refuses it in every domain.
pub fn file_new(entry: &DataObject) -> DataObject {
    ensure();
    let claim = entry.try_get_string("claim").unwrap_or_default().trim().to_string();
    if claim.is_empty() { return store::err("claim is required"); }
    let idx = store::read_index(LIB, NS);
    if store::find_id(&idx, &claim).is_some() { return store::err("an entry with this exact claim already exists in kb.plan"); }
    let id = store::mint_id(&idx, &claim);
    let r = store::put_item(LIB, NS, &id, entry, 0);
    if r.try_get_string("status").unwrap_or_default() != "ok" { return r; }
    let mut o = DataObject::new();
    o.put_string("status", "ok"); o.put_string("domain", "plan"); o.put_string("claim", &claim);
    o.put_boolean("stamped", false);
    o.put_int("entries", idx.get_array("order").len() as i64 + 1);
    o.put_string("item_id", &id); o.put_string("hash", &r.get_string("hash"));
    o
}
pub fn execute(_: DataObject) -> DataObject {
    use std::panic;
    let ax = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        compat()
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

pub fn compat() -> DataObject {
let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_boolean("imported", store::has_index(LIB, NS));
o.put_string("hash", &format!("r{}", store::read_index(LIB, NS).try_get_int("rev").unwrap_or(0)));
o.put_int("items", store::read_index(LIB, NS).get_array("order").len() as i64);
o
}
