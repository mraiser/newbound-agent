use ndata::dataobject::DataObject;
use ndata::dataarray::DataArray;
use ndata::data::Data;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};
use flowlang::datastore::DataStore;
use flowlang::flowlang::system::time::time;

pub const LIB: &str = "kb";
pub const NS: &str = "plan";
const LIFECYCLES: [&str; 7] = ["proposed", "accepted", "in-progress", "blocked", "completed", "done", "abandoned"];

// One process-wide mutex serializes every store write. Writes are small and
// never span a model call, so this is not a contention point; long
// read-think-write cycles are guarded per item by `rev` instead.
pub fn lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|e| e.into_inner())
}

pub fn fnv(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
    format!("{:016x}", h)
}

pub fn index_id(ns: &str) -> String { format!("{}_index", ns) }
pub fn rec_id(ns: &str, kind: &str, id: &str) -> String { format!("{}_{}_{}", ns, kind, id) }

pub fn read_rec(lib: &str, id: &str) -> Option<DataObject> {
    let store = DataStore::new();
    if !store.exists(lib, id) { return None; }
    store.get_data(lib, id).try_get_object("data").ok()
}

// The kb library is usable only while its controls index exists. A pull
// across newbound-agent bdc31ba ("Untrack data/kb") deletes that index and
// every once-tracked kb file while leaving the untracked ones (kb.plan's
// facet among them) on disk. Nothing may be imported or written over a kb
// in that state: an empty index written then would hide the surviving
// facet from the importer for good.
pub fn kb_ok() -> bool { DataStore::new().exists(LIB, "controls") }
pub const KB_DAMAGED: &str = "the kb library has no controls index (data/kb is missing or damaged); the plan board refuses to import or write until it is restored";

// Atomic: the record is written beside its final path and renamed into
// place, so a reader (this process or another) never sees a partial file.
// DataStore::set_data writes in place, which a concurrent reader can catch
// truncated.
pub fn write_rec(lib: &str, id: &str, data: DataObject) {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    if lib == LIB && !kb_ok() { panic!("plan store: {}", KB_DAMAGED); }
    let mut r = DataObject::new();
    r.put_string("id", id); r.put_string("username", "system");
    r.put_array("readers", DataArray::new()); r.put_array("writers", DataArray::new());
    r.put_int("time", time()); r.put_object("data", data);
    let f = DataStore::new().get_data_file(lib, id);
    if let Some(p) = f.parent() { let _ = std::fs::create_dir_all(p); }
    let tmp = f.with_file_name(format!(".{}.{}.{}.tmp", id, std::process::id(), SEQ.fetch_add(1, Ordering::Relaxed)));
    std::fs::write(&tmp, r.to_string()).expect("plan store: unable to write record");
    std::fs::rename(&tmp, &f).expect("plan store: unable to place record");
}

pub fn delete_rec(lib: &str, id: &str) -> bool {
    let f = DataStore::new().get_data_file(lib, id);
    f.exists() && std::fs::remove_file(f).is_ok()
}

pub fn lifecycle_of(tags: &str) -> String {
    let mut out = String::new();
    for t in tags.split(',') { let t = t.trim(); if LIFECYCLES.contains(&t) { out = t.to_string(); } }
    out
}

fn empty_index() -> DataObject {
    let mut i = DataObject::new(); i.put_int("rev", 0);
    i.put_array("order", DataArray::new()); i.put_object("items", DataObject::new()); i
}

pub fn has_index(lib: &str, ns: &str) -> bool { DataStore::new().exists(lib, &index_id(ns)) }

pub fn read_index(lib: &str, ns: &str) -> DataObject {
    let i = read_rec(lib, &index_id(ns)).unwrap_or_else(empty_index);
    if i.has("order") && i.has("items") { i } else { empty_index() }
}

pub fn find_id(index: &DataObject, claim: &str) -> Option<String> {
    for (k, v) in index.get_object("items").objects() {
        if v.is_object() && v.object().try_get_string("claim").unwrap_or_default() == claim { return Some(k); }
    }
    None
}

// A new id for a claim: the claim's hash, salted on the (unlikely) collision
// with a different claim. Minted once; a later claim edit keeps the id.
pub fn mint_id(index: &DataObject, claim: &str) -> String {
    let items = index.get_object("items");
    let mut n = 0;
    loop {
        let id = if n == 0 { fnv(claim) } else { fnv(&format!("{}#{}", claim, n)) };
        if !items.has(&id) { return id; }
        n += 1;
    }
}

fn read_list_nl(lib: &str, ns: &str, kind: &str, id: &str) -> DataArray {
    match read_rec(lib, &rec_id(ns, kind, id)) {
        Some(d) => d.try_get_array("list").unwrap_or_else(|_| DataArray::new()),
        None => DataArray::new(),
    }
}
fn write_list_nl(lib: &str, ns: &str, kind: &str, id: &str, list: DataArray) {
    let mut d = DataObject::new(); d.put_array("list", list);
    write_rec(lib, &rec_id(ns, kind, id), d);
}

pub fn read_item(lib: &str, ns: &str, id: &str) -> Option<DataObject> { read_rec(lib, &rec_id(ns, "item", id)) }
pub fn read_notes(lib: &str, ns: &str, id: &str) -> DataArray { read_list_nl(lib, ns, "notes", id) }
pub fn read_steps(lib: &str, ns: &str, id: &str) -> DataArray { read_list_nl(lib, ns, "steps", id) }

fn meta_from(item: &DataObject, rev: i64, notes: i64, steps: i64) -> DataObject {
    let mut m = DataObject::new();
    m.put_string("claim", &item.try_get_string("claim").unwrap_or_default());
    m.put_string("lifecycle", &lifecycle_of(&item.try_get_string("tags").unwrap_or_default()));
    m.put_string("parent", &item.try_get_string("parent").unwrap_or_default());
    m.put_int("time", item.try_get_int("time").unwrap_or(0));
    m.put_int("rev", rev); m.put_int("notes", notes); m.put_int("steps", steps);
    m
}

fn strip(item: &DataObject) -> DataObject {
    let mut i = item.deep_copy();
    for k in ["notes", "step_log", "item_id", "rev"] { if i.has(k) { i.remove_property(k); } }
    i
}

fn entry_nl(lib: &str, ns: &str, index: &DataObject, id: &str) -> Option<DataObject> {
    let items = index.get_object("items");
    let meta = items.try_get_object(id).ok()?;
    let mut e = read_item(lib, ns, id)?.deep_copy();
    if meta.try_get_int("notes").unwrap_or(0) > 0 { e.put_array("notes", read_notes(lib, ns, id)); }
    if meta.try_get_int("steps").unwrap_or(0) > 0 { e.put_array("step_log", read_steps(lib, ns, id)); }
    e.put_string("item_id", id); e.put_int("rev", meta.try_get_int("rev").unwrap_or(0));
    Some(e)
}

// The legacy-shaped entry: the item record plus its notes and step_log,
// with item_id and rev added. None when the item is not in the index. Read
// under the store lock, so the item and the rev it is returned with belong
// together.
pub fn entry(lib: &str, ns: &str, id: &str) -> Option<DataObject> {
    let _g = lock();
    entry_nl(lib, ns, &read_index(lib, ns), id)
}

// FLAT {status, hash, entries, storage}: the board in index order, one
// consistent snapshot. hash is the index revision, which moves on every
// write to any item, note or step.
pub fn board(lib: &str, ns: &str) -> DataObject {
    let _g = lock();
    let idx = read_index(lib, ns);
    let order = idx.get_array("order");
    let mut entries = DataArray::new();
    let mut missing = DataArray::new();
    for i in 0..order.len() {
        let id = order.get_string(i);
        match entry_nl(lib, ns, &idx, &id) { Some(e) => entries.push_object(e), None => missing.push_string(&id) }
    }
    let mut out = DataObject::new();
    out.put_string("status", "ok"); out.put_string("storage", "records");
    out.put_string("hash", &format!("r{}", idx.try_get_int("rev").unwrap_or(0)));
    out.put_array("entries", entries);
    if missing.len() > 0 { out.put_array("missing", missing); }
    if lib == LIB && !kb_ok() { out.put_string("warning", KB_DAMAGED); }
    out
}

pub fn err(msg: &str) -> DataObject { let mut o = DataObject::new(); o.put_string("status", "err"); o.put_string("msg", msg); o }

// Create (id not in the index: appended to the order) or replace one item.
// base_rev < 0 writes unguarded; otherwise it must equal the item's current
// rev (0 for a new item) or the write is refused with stale_base. Notes and
// step_log on the passed object are ignored - they have their own records.
pub fn put_item(lib: &str, ns: &str, id: &str, item: &DataObject, base_rev: i64) -> DataObject {
    if lib == LIB && !kb_ok() { return err(KB_DAMAGED); }
    let _g = lock();
    let mut idx = read_index(lib, ns);
    let mut items = idx.get_object("items");
    let old = items.try_get_object(id).ok();
    let cur = old.as_ref().map(|m| m.try_get_int("rev").unwrap_or(0)).unwrap_or(0);
    if base_rev >= 0 && base_rev != cur {
        let mut o = err("stale_base"); o.put_int("current_rev", cur); o.put_string("item_id", id); return o;
    }
    let clean = strip(item);
    if clean.try_get_string("claim").unwrap_or_default().trim().is_empty() { return err("item needs a claim"); }
    let (n, s) = match &old { Some(m) => (m.try_get_int("notes").unwrap_or(0), m.try_get_int("steps").unwrap_or(0)), None => (0, 0) };
    // `at` = the index rev at which this item's record last changed (its
    // rev moved). move_item uses it to accept a base older than the board
    // hash as long as THIS item has not changed since. Notes never move it.
    let mut meta = meta_from(&clean, cur + 1, n, s);
    meta.put_int("at", idx.try_get_int("rev").unwrap_or(0) + 1);
    write_rec(lib, &rec_id(ns, "item", id), clean);
    items.put_object(id, meta);
    if old.is_none() { let mut order = idx.get_array("order"); order.push_string(id); }
    let rev = idx.try_get_int("rev").unwrap_or(0) + 1;
    idx.put_int("rev", rev);
    write_rec(lib, &index_id(ns), idx);
    let mut o = DataObject::new(); o.put_string("status", "ok"); o.put_string("item_id", id);
    o.put_int("rev", cur + 1); o.put_string("hash", &format!("r{}", rev)); o.put_boolean("created", old.is_none()); o
}

// kind = "notes" | "steps". append pushes one object; replace swaps the whole
// list (step_item stamping seen_at). Neither moves the item's rev, so a note
// never invalidates a concurrent item writer; both move the board hash.
fn list_write(lib: &str, ns: &str, kind: &str, id: &str, one: Option<DataObject>, all: Option<DataArray>) -> DataObject {
    if kind != "notes" && kind != "steps" { return err("kind must be notes or steps"); }
    if lib == LIB && !kb_ok() { return err(KB_DAMAGED); }
    let _g = lock();
    let mut idx = read_index(lib, ns);
    let mut items = idx.get_object("items");
    let mut meta = match items.try_get_object(id) { Ok(m) => m, Err(_) => return err("unknown item") };
    let mut list = match all { Some(a) => a, None => read_list_nl(lib, ns, kind, id) };
    if let Some(o) = one { list.push_object(o); }
    let n = list.len() as i64;
    write_list_nl(lib, ns, kind, id, list);
    meta.put_int(kind, n);
    items.put_object(id, meta);
    let rev = idx.try_get_int("rev").unwrap_or(0) + 1;
    idx.put_int("rev", rev);
    write_rec(lib, &index_id(ns), idx);
    let mut o = DataObject::new(); o.put_string("status", "ok"); o.put_string("item_id", id);
    o.put_int("count", n); o.put_string("hash", &format!("r{}", rev)); o
}
pub fn append(lib: &str, ns: &str, kind: &str, id: &str, obj: DataObject) -> DataObject { list_write(lib, ns, kind, id, Some(obj), None) }
pub fn replace_list(lib: &str, ns: &str, kind: &str, id: &str, list: DataArray) -> DataObject { list_write(lib, ns, kind, id, None, Some(list)) }

// Canonical text of a value (sorted keys, integral floats as ints) for
// order-independent comparison.
pub fn canon(d: &Data) -> String {
    if d.is_object() {
        let o = d.object(); let mut keys = o.get_keys(); keys.sort();
        let parts: Vec<String> = keys.iter().map(|k| format!("{:?}:{}", k, canon(&o.get_property(k)))).collect();
        format!("{{{}}}", parts.join(","))
    } else if d.is_array() {
        let a = d.array();
        let parts: Vec<String> = (0..a.len()).map(|i| canon(&a.get_property(i))).collect();
        format!("[{}]", parts.join(","))
    } else if d.is_string() { format!("{:?}", d.string()) }
    else if d.is_int() { format!("{}", d.int()) }
    else if d.is_float() { let f = d.float(); if f.fract() == 0.0 && f.abs() < 9.0e15 { format!("{}", f as i64) } else { format!("{}", f) } }
    else if d.is_boolean() { format!("{}", d.boolean()) }
    else if d.is_null() { "null".to_string() }
    else { "<bytes>".to_string() }
}

// An entry as compared across storages: without item_id/rev, and with empty
// notes/step_log treated as absent.
pub fn canon_entry(e: &DataObject) -> String {
    let mut c = e.deep_copy();
    for k in ["item_id", "rev"] { if c.has(k) { c.remove_property(k); } }
    for k in ["notes", "step_log"] {
        if let Ok(a) = c.try_get_array(k) { if a.len() == 0 { c.remove_property(k); } }
    }
    canon(&Data::DObject(c.data_ref))
}

// Compares the records under (lib, ns) with a legacy entry array, in order.
pub fn compare(lib: &str, ns: &str, entries: &DataArray) -> DataObject {
    let b = board(lib, ns);
    let got = b.get_array("entries");
    let mut mismatches = DataArray::new();
    let n = entries.len().max(got.len());
    for i in 0..n {
        let a = if i < entries.len() { entries.try_get_object(i).ok() } else { None };
        let g = if i < got.len() { got.try_get_object(i).ok() } else { None };
        let same = match (&a, &g) { (Some(a), Some(g)) => canon_entry(a) == canon_entry(g), _ => false };
        if !same {
            let mut m = DataObject::new(); m.put_int("index", i as i64);
            m.put_string("facet_claim", &a.map(|x| x.try_get_string("claim").unwrap_or_default()).unwrap_or_default());
            m.put_string("record_claim", &g.map(|x| x.try_get_string("claim").unwrap_or_default()).unwrap_or_default());
            mismatches.push_object(m);
        }
    }
    let mut o = DataObject::new();
    o.put_int("facet_entries", entries.len() as i64); o.put_int("record_entries", got.len() as i64);
    o.put_boolean("equal", mismatches.len() == 0); o.put_array("mismatches", mismatches);
    o.put_string("hash", &b.get_string("hash")); o
}

// Imports a legacy entry array into records under (lib, ns), entry by entry,
// then reads everything back and compares. Refuses when an index already
// exists. The source is never modified. dry_run writes nothing.
pub fn import_entries(lib: &str, ns: &str, entries: &DataArray, dry_run: bool) -> DataObject {
    if has_index(lib, ns) { let mut o = err("already imported: an index exists"); o.put_boolean("already", true); return o; }
    if lib == LIB && !dry_run && !kb_ok() { return err(KB_DAMAGED); }
    let mut out = DataObject::new();
    let (mut notes_n, mut steps_n, mut bytes) = (0i64, 0i64, 0i64);
    let mut skipped = DataArray::new();
    {
        let _g = lock();
        let mut idx = empty_index();
        let mut items = idx.get_object("items");
        let mut order = idx.get_array("order");
        for i in 0..entries.len() {
            let e = match entries.try_get_object(i) { Ok(e) => e, Err(_) => { skipped.push_int(i as i64); continue; } };
            let claim = e.try_get_string("claim").unwrap_or_default();
            if claim.trim().is_empty() || find_id(&idx, &claim).is_some() { skipped.push_int(i as i64); continue; }
            let id = mint_id(&idx, &claim);
            let clean = strip(&e);
            let notes = e.try_get_array("notes").map(|a| a.deep_copy()).unwrap_or_else(|_| DataArray::new());
            let steps = e.try_get_array("step_log").map(|a| a.deep_copy()).unwrap_or_else(|_| DataArray::new());
            let (n, s) = (notes.len() as i64, steps.len() as i64);
            notes_n += n; steps_n += s; bytes += e.to_string().len() as i64;
            items.put_object(&id, meta_from(&clean, 1, n, s));
            order.push_string(&id);
            if !dry_run {
                write_rec(lib, &rec_id(ns, "item", &id), clean);
                if n > 0 { write_list_nl(lib, ns, "notes", &id, notes); }
                if s > 0 { write_list_nl(lib, ns, "steps", &id, steps); }
            }
        }
        out.put_int("items", order.len() as i64);
        idx.put_int("rev", 1); idx.put_int("imported", time());
        if !dry_run { write_rec(lib, &index_id(ns), idx); }
    }
    out.put_string("status", "ok"); out.put_boolean("dry_run", dry_run);
    out.put_int("notes", notes_n); out.put_int("steps", steps_n); out.put_int("source_bytes", bytes);
    out.put_array("skipped", skipped);
    if !dry_run {
        let c = compare(lib, ns, entries);
        out.put_boolean("verified", c.get_boolean("equal") && out.get_array("skipped").len() == 0);
        out.put_object("compare", c);
    }
    out
}

// Removes every record under (lib, ns): the index and each item's records.
pub fn drop_all(lib: &str, ns: &str) -> i64 {
    let _g = lock();
    let idx = read_index(lib, ns);
    let order = idx.get_array("order");
    let mut n = 0;
    for i in 0..order.len() {
        let id = order.get_string(i);
        for k in ["item", "notes", "steps"] { if delete_rec(lib, &rec_id(ns, k, &id)) { n += 1; } }
    }
    if delete_rec(lib, &index_id(ns)) { n += 1; }
    n
}

// The kb.plan memory facet as an entry array (the legacy monolith). None
// means UNKNOWN - the kb library is damaged, the read failed, or the facet
// does not parse - and the caller must not import. Only a positively absent
// control or facet on a healthy kb is an empty board (a fresh instance).
pub fn legacy_entries() -> Option<DataArray> {
    if !kb_ok() { return None; }
    let r = crate::api::new().dev.code.read_control_facet("kb".to_string(), "plan".to_string(), "memory".to_string());
    if r.try_get_string("status").unwrap_or_default() == "err" {
        let m = r.try_get_string("msg").unwrap_or_default();
        return if m.contains("not found in library") { Some(DataArray::new()) } else { None };
    }
    match r.try_get_boolean("exists") {
        Ok(true) => {},
        Ok(false) => return Some(DataArray::new()),
        Err(_) => return None,
    }
    let wrapped = format!("{{\"a\": {} }}", r.try_get_string("source").unwrap_or_default());
    DataObject::try_from_string(&wrapped).ok().and_then(|o| o.try_get_array("a").ok())
}
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["op", "lib", "ns", "dry_run"] {
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
        let arg_0: String = o.get_string("op");
        let arg_1: String = o.get_string("lib");
        let arg_2: String = o.get_string("ns");
        let arg_3: bool = o.get_boolean("dry_run");
        store(arg_0, arg_1, arg_2, arg_3)
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

pub fn store(op: String, lib: String, ns: String, dry_run: bool) -> DataObject {
let lib = if lib.trim().is_empty() { LIB.to_string() } else { lib.trim().to_string() };
let ns = if ns.trim().is_empty() { NS.to_string() } else { ns.trim().to_string() };
if lib != "kb" && lib != "runtime" { return err("lib must be kb or runtime"); }
if ns.is_empty() || ns.len() > 40 || !ns.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') { return err("ns must be [A-Za-z0-9_], max 40 chars"); }
match op.as_str() {
    "status" => {
        let idx = read_index(&lib, &ns);
        let mut o = DataObject::new(); o.put_string("status", "ok");
        o.put_string("lib", &lib); o.put_string("ns", &ns);
        o.put_boolean("imported", has_index(&lib, &ns));
        o.put_int("items", idx.get_array("order").len() as i64);
        o.put_string("hash", &format!("r{}", idx.try_get_int("rev").unwrap_or(0)));
        let mut lc = DataObject::new();
        for (_k, v) in idx.get_object("items").objects() {
            if !v.is_object() { continue; }
            let l = v.object().try_get_string("lifecycle").unwrap_or_default();
            let l = if l.is_empty() { "none".to_string() } else { l };
            let n = lc.try_get_int(&l).unwrap_or(0); lc.put_int(&l, n + 1);
        }
        o.put_object("lifecycles", lc);
        o
    },
    "board" => board(&lib, &ns),
    "import" => match legacy_entries() {
        Some(e) => import_entries(&lib, &ns, &e, dry_run),
        None => err("kb.plan memory facet did not parse as a JSON array"),
    },
    "verify" => match legacy_entries() {
        Some(e) => { let mut c = compare(&lib, &ns, &e); c.put_string("status", "ok"); c },
        None => err("kb.plan memory facet did not parse as a JSON array"),
    },
    // Retire the legacy monolith once the records are the board: back the
    // kb.plan memory facet up to runtime/agent/plan-legacy/, empty it to [],
    // and delete the control's patch journal (whole-facet snapshots of it).
    "retire" => {
        if lib != LIB || ns != NS { return err("retire applies to kb/plan only"); }
        if !has_index(&lib, &ns) { return err("the board has not been imported into records; nothing may be retired"); }
        let api = crate::api::new();
        let r = api.dev.code.read_control_facet("kb".to_string(), "plan".to_string(), "memory".to_string());
        let exists = r.try_get_boolean("exists").unwrap_or(false);
        let src = if exists { r.try_get_string("source").unwrap_or_default() } else { String::new() };
        let legacy = legacy_entries().map(|a| a.len() as i64).unwrap_or(-1);
        // Called by lookup: this crate's generated api may predate patch_journal.
        let journal = |op: &str, dry: bool| -> DataObject {
            let mut a = DataObject::new();
            a.put_string("op", op); a.put_string("lib", "kb"); a.put_string("ctl", "plan");
            a.put_string("to_lib", ""); a.put_object("entry", DataObject::new());
            a.put_int("limit", 0); a.put_boolean("dry_run", dry); a.put_string("nn_sessionid", "");
            let c = flowlang::command::Command::lookup("dev", "code", "patch_journal");
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c.execute(a))) {
                Ok(Ok(r)) => if r.has("a") && r.get_property("a").is_object() { r.get_object("a") } else { r },
                _ => err("dev.code.patch_journal failed"),
            }
        };
        let js = journal("status", true);
        let mut o = DataObject::new(); o.put_string("status", "ok");
        o.put_boolean("dry_run", dry_run);
        o.put_int("records", read_index(&lib, &ns).get_array("order").len() as i64);
        o.put_boolean("facet_exists", exists);
        o.put_int("facet_bytes", src.len() as i64);
        o.put_int("facet_entries", legacy);
        o.put_int("journal_patches", js.try_get_int("total").unwrap_or(0));
        o.put_string("journal_storage", &js.try_get_string("storage").unwrap_or_default());
        o.put_boolean("journal_legacy_present", js.try_get_boolean("legacy_present").unwrap_or(false));
        let already = !exists || src.trim() == "[]";
        o.put_boolean("facet_already_empty", already);
        if dry_run { return o; }
        if !already {
            let dir = "runtime/agent/plan-legacy";
            let path = format!("{}/kb-plan-memory-{}.json", dir, time());
            if std::fs::create_dir_all(dir).is_err() || std::fs::write(&path, src.as_bytes()).is_err() {
                return err("could not write the backup; nothing was changed");
            }
            let back = std::fs::read_to_string(&path).unwrap_or_default();
            if back != src { return err("the backup did not read back equal; nothing was changed"); }
            o.put_string("backup", &path);
            let w = api.dev.code.patch_control_facet("kb".to_string(), "plan".to_string(), "memory".to_string(), String::new(), "[]".to_string(), r.try_get_string("hash").unwrap_or_default(), "retire the legacy plan monolith (records are the board)".to_string(), "plan-store".to_string(), String::new());
            if w.try_get_string("status").unwrap_or_default() != "ok" {
                let mut e = err("emptying the facet failed; the backup was written, the journal was left alone");
                e.put_object("write", w); e.put_string("backup", &path);
                return e;
            }
            o.put_boolean("facet_emptied", true);
        }
        let d = journal("delete", false);
        o.put_boolean("journal_deleted", d.try_get_string("status").unwrap_or_default() == "ok");
        o.put_object("journal", d);
        o
    },
    _ => err("op must be status, board, import, verify or retire"),
}
}
