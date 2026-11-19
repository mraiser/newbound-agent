use ndata::dataobject::DataObject;
use ndata::dataarray::DataArray;
use flowlang::flowlang::system::time::time;
use crate::agent::plan::store as ps;
pub fn execute(_: DataObject) -> DataObject {
    use std::panic;
    let ax = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        test_store_contract()
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

pub fn test_store_contract() -> DataObject {
let lib = "runtime";
let t = time();
let ns_real = format!("plantest{}a", t);
let ns = format!("plantest{}b", t);
let mut checks = DataArray::new();
let mut all_ok = true;
let mut check = |name: &str, ok: bool, detail: String| {
    let mut c = DataObject::new(); c.put_string("check", name); c.put_boolean("ok", ok); c.put_string("detail", &detail);
    checks.push_object(c); if !ok { all_ok = false; }
};

// T1: the real kb.plan facet imports into a disposable namespace and reads
// back entry-for-entry equal. The facet itself is only read.
match ps::legacy_entries() {
    Some(e) => {
        let r = ps::import_entries(lib, &ns_real, &e, false);
        let ok = r.try_get_boolean("verified").unwrap_or(false);
        check("real facet imports and compares equal", ok, format!("items {} notes {} steps {} mismatches {}",
            r.try_get_int("items").unwrap_or(-1), r.try_get_int("notes").unwrap_or(-1), r.try_get_int("steps").unwrap_or(-1),
            r.try_get_object("compare").map(|c| c.get_array("mismatches").to_string()).unwrap_or_default()));
    },
    None => check("real facet imports and compares equal", false, "facet did not parse".to_string()),
}

// T2: a synthetic board with notes, step_log, a parent link and an empty
// notes array survives the split.
let src = DataObject::from_string(r#"{"a":[
 {"claim":"Parent goal","detail":"d0","tags":"plan,accepted","time":1,"work":true,"notes":[]},
 {"claim":"Child one","detail":"d1 é \"q\"","tags":"plan,in-progress","time":2,"parent":"Parent goal",
  "notes":[{"text":"n1","author":"a","time":3}],"step_log":[{"turn":1,"outcome":"IN_PROGRESS"},{"turn":2,"outcome":"IN_PROGRESS"}],
  "checkpoint":{"version":2,"progress":["p"]},"confidence":"medium"},
 {"claim":"Child two","detail":"","tags":"plan,proposed","time":4,"parent":"Parent goal","source":{"lib":"x","hash":"h"}}
]}"#).get_array("a");
let r = ps::import_entries(lib, &ns, &src, false);
check("synthetic board imports and compares equal", r.try_get_boolean("verified").unwrap_or(false), r.to_string());
let again = ps::import_entries(lib, &ns, &src, false);
check("a second import is refused", again.try_get_boolean("already").unwrap_or(false), again.to_string());
let idx = ps::read_index(lib, &ns);
let id1 = ps::find_id(&idx, "Child one").unwrap_or_default();
let id2 = ps::find_id(&idx, "Child two").unwrap_or_default();
let m1 = idx.get_object("items").try_get_object(&id1).unwrap_or_else(|_| DataObject::new());
check("index carries lifecycle, parent and counts",
    m1.try_get_string("lifecycle").unwrap_or_default() == "in-progress" && m1.try_get_string("parent").unwrap_or_default() == "Parent goal"
    && m1.try_get_int("notes").unwrap_or(0) == 1 && m1.try_get_int("steps").unwrap_or(0) == 2, m1.to_string());

// T3: two writers on different items, each a read-rev -> write cycle, never
// see stale_base.
let rounds = 20i64;
let mut handles = Vec::new();
for id in [id1.clone(), id2.clone()] {
    let ns_t = ns.clone();
    handles.push(std::thread::spawn(move || {
        let mut stale = 0i64;
        for k in 0..rounds {
            let e = match ps::entry("runtime", &ns_t, &id) { Some(e) => e, None => { stale += 1000; continue; } };
            let rev = e.get_int("rev");
            let mut e2 = e.deep_copy(); e2.put_string("detail", &format!("round {}", k));
            let w = ps::put_item("runtime", &ns_t, &id, &e2, rev);
            if w.get_string("status") != "ok" { stale += 1; }
        }
        stale
    }));
}
let stale: i64 = handles.into_iter().map(|h| h.join().unwrap_or(999)).sum();
let idx = ps::read_index(lib, &ns);
let r1 = idx.get_object("items").get_object(&id1).get_int("rev");
let r2 = idx.get_object("items").get_object(&id2).get_int("rev");
check("concurrent writers on different items never go stale", stale == 0 && r1 == rounds + 1 && r2 == rounds + 1,
    format!("stale {} rev1 {} rev2 {}", stale, r1, r2));

// T4: the same item with an old base is refused, and nothing is written.
let w = ps::put_item(lib, &ns, &id2, &ps::entry(lib, &ns, &id2).unwrap(), 1);
check("an old base on the same item is refused", w.try_get_string("msg").unwrap_or_default() == "stale_base"
    && w.try_get_int("current_rev").unwrap_or(0) == rounds + 1
    && ps::read_item(lib, &ns, &id2).map(|i| i.get_string("detail")).unwrap_or_default() == format!("round {}", rounds - 1), w.to_string());

// T5: notes appended while the item is being rewritten are all kept, and
// they never make the item writer stale.
let (ns_a, ns_b, ida, idb) = (ns.clone(), ns.clone(), id1.clone(), id1.clone());
let ha = std::thread::spawn(move || { let mut bad = 0i64; for k in 0..25 {
    let mut n = DataObject::new(); n.put_string("text", &format!("note {}", k)); n.put_int("time", k);
    if ps::append("runtime", &ns_a, "notes", &ida, n).get_string("status") != "ok" { bad += 1; } } bad });
let hb = std::thread::spawn(move || { let mut bad = 0i64; for k in 0..25 {
    let e = ps::entry("runtime", &ns_b, &idb).unwrap(); let rev = e.get_int("rev");
    let mut e2 = e.deep_copy(); e2.put_string("detail", &format!("w {}", k));
    if ps::put_item("runtime", &ns_b, &idb, &e2, rev).get_string("status") != "ok" { bad += 1; } } bad });
let (ba, bb) = (ha.join().unwrap_or(999), hb.join().unwrap_or(999));
let e1 = ps::entry(lib, &ns, &id1).unwrap();
let notes = e1.get_array("notes");
let mut texts: Vec<String> = (0..notes.len()).map(|i| notes.get_object(i).get_string("text")).collect();
texts.sort(); texts.dedup();
check("notes appended during item writes are all kept", ba == 0 && bb == 0 && notes.len() == 26 && texts.len() == 26
    && e1.get_int("rev") == rounds + 1 + 25 && e1.get_array("step_log").len() == 2 && e1.get_string("detail") == "w 24",
    format!("append errs {} writer errs {} notes {} distinct {} rev {}", ba, bb, notes.len(), texts.len(), e1.get_int("rev")));

// T6: the board hash moves on a note; a new item joins the end of the order;
// a claim edit keeps the id.
let h0 = ps::board(lib, &ns).get_string("hash");
let mut n = DataObject::new(); n.put_string("text", "one more");
ps::append(lib, &ns, "notes", &id2, n);
let h1 = ps::board(lib, &ns).get_string("hash");
let mut fresh = DataObject::new(); fresh.put_string("claim", "A new item"); fresh.put_string("tags", "plan,proposed");
let idn = ps::mint_id(&ps::read_index(lib, &ns), "A new item");
let wn = ps::put_item(lib, &ns, &idn, &fresh, 0);
let mut renamed = ps::entry(lib, &ns, &id2).unwrap(); renamed.put_string("claim", "Child two, renamed");
let wr = ps::put_item(lib, &ns, &id2, &renamed, renamed.get_int("rev"));
let b = ps::board(lib, &ns); let es = b.get_array("entries");
check("hash moves, new items append, a claim edit keeps the id",
    h0 != h1 && wn.try_get_boolean("created").unwrap_or(false) && wr.get_string("status") == "ok" && es.len() == 4
    && es.get_object(3).get_string("claim") == "A new item" && es.get_object(2).get_string("item_id") == id2
    && es.get_object(2).get_string("claim") == "Child two, renamed" && !b.has("missing"),
    format!("{} -> {} entries {}", h0, h1, es.len()));
let unk = ps::append(lib, &ns, "notes", "nope", DataObject::new());
check("a note for an unknown item is refused", unk.get_string("status") == "err", unk.to_string());

// Cleanup: both namespaces leave nothing behind.
let d1 = ps::drop_all(lib, &ns_real); let d2 = ps::drop_all(lib, &ns);
check("the test namespaces are removed", !ps::has_index(lib, &ns_real) && !ps::has_index(lib, &ns) && ps::read_item(lib, &ns, &id1).is_none(),
    format!("removed {} + {} records", d1, d2));

let mut out = DataObject::new();
out.put_string("status", "ok"); out.put_boolean("all_ok", all_ok); out.put_array("checks", checks);
out
}
