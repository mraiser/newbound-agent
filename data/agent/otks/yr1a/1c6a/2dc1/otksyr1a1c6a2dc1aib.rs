let api = crate::api::new();
let mut out = DataObject::new();
let mut checks = DataArray::new();
let mut all_ok = true;
let mut check = |name: &str, ok: bool, detail: String| {
    let mut c = DataObject::new(); c.put_string("check", name); c.put_boolean("ok", ok);
    c.put_string("detail", &detail.chars().take(300).collect::<String>());
    checks.push_object(c); if !ok { all_ok = false; }
};
let st = |o: &DataObject| o.try_get_string("status").unwrap_or_default();
let msg = |o: &DataObject| o.try_get_string("msg").unwrap_or_default();
let b0 = api.agent.plan.board();
let n0 = b0.get_array("entries").len();
check("board reads from records", b0.try_get_string("storage").unwrap_or_default() == "records", format!("{} entries, hash {}", n0, b0.get_string("hash")));
let t = flowlang::flowlang::system::time::time();
let c1 = format!("plan-split cutover test item {}", t);
let c2 = format!("plan-split cutover test subtask {}", t);
let r = api.agent.plan.add_item(c1.clone(), "throwaway".to_string(), "".to_string(), "".to_string());
check("add_item files a record", st(&r) == "ok" && r.has("item_id"), r.to_string());
let r = api.agent.plan.add_item(c1.clone(), "".to_string(), "".to_string(), "".to_string());
check("duplicate claim refused", st(&r) == "err", msg(&r));
let h1 = api.agent.plan.board_hash().get_string("hash");
let r = api.agent.plan.add_note(c1.clone(), "first note".to_string(), "".to_string());
check("add_note appends", st(&r) == "ok" && r.try_get_int("notes").unwrap_or(0) == 1, r.to_string());
let r = api.agent.plan.move_item(c1.clone(), "accepted".to_string(), h1.clone(), "".to_string());
check("move_item refuses an old board hash", msg(&r) == "stale_base", r.to_string());
let snap = compat::facet_read();
let h2 = snap.get_string("hash");
let r = api.agent.plan.move_item(c1.clone(), "accepted".to_string(), h2.clone(), "".to_string());
check("move_item with the current hash", st(&r) == "ok", r.to_string());
let r = api.agent.plan.add_subtask(c1.clone(), c2.clone(), "".to_string(), "proposed".to_string(), "".to_string());
check("add_subtask under the accepted item", st(&r) == "ok", r.to_string());
let r = api.agent.plan.add_note(c1.clone(), "second note".to_string(), "".to_string());
check("second note", r.try_get_int("notes").unwrap_or(0) == 2, r.to_string());
// A writer holding the OLD snapshot changes a different item: must succeed.
let arr = DataObject::try_from_string(&format!("{{\"a\": {} }}", snap.get_string("source"))).unwrap().get_array("a");
let mut other_claim = String::new();
for i in 0..arr.len() { let mut e = arr.get_object(i); let c = e.get_string("claim");
    if c != c1 && c != c2 { other_claim = c; e.put_string("cutover_probe", "x"); break; } }
if other_claim.is_empty() {
    check("old snapshot, different item (skipped: board has no other item)", true, String::new());
} else {
    let w = compat::facet_write("".into(), "".into(), "".into(), "".into(), arr.to_string(), h2.clone(), "".into(), "".into(), "".into());
    check("old snapshot, different item: written", st(&w) == "ok" && w.try_get_int("changed").unwrap_or(0) == 1, w.to_string());
}
// The same old snapshot now touches the test item, which changed since: stale.
for i in 0..arr.len() { let mut e = arr.get_object(i); if e.get_string("claim") == c1 { e.put_string("detail", "clobber"); } }
let w = compat::facet_write("".into(), "".into(), "".into(), "".into(), arr.to_string(), h2.clone(), "".into(), "".into(), "".into());
check("old snapshot, same item: stale_base", msg(&w) == "stale_base", w.to_string());
let b = api.agent.plan.board();
let (mut ok1, mut ok2, mut d) = (false, false, String::new());
for x in b.get_array("entries").objects() { let e = x.object(); let c = e.get_string("claim");
    if c == c1 {
        let notes = e.try_get_array("notes").map(|a| a.len()).unwrap_or(0);
        ok1 = e.get_string("tags") == "plan,accepted" && e.try_get_string("detail").unwrap_or_default() == "throwaway" && notes == 2;
        d = format!("tags={} detail={} notes={}", e.get_string("tags"), e.try_get_string("detail").unwrap_or_default(), notes);
    }
    if c == c2 { ok2 = e.get_string("tags") == "plan,proposed" && e.get_string("parent") == c1; }
}
check("item kept its lifecycle, detail and both notes", ok1, d);
check("subtask linked to its parent", ok2, String::new());
let ew = api.agent.archivist.epistemic_work();
check("epistemic_work sees the accepted item", ew.try_get_int("plan").unwrap_or(0) >= 1, format!("plan={}", ew.try_get_int("plan").unwrap_or(-1)));
// Cleanup: remove the probe field and the two test items.
{
    if !other_claim.is_empty() {
        let fr = compat::facet_read();
        let a = DataObject::try_from_string(&format!("{{\"a\": {} }}", fr.get_string("source"))).unwrap().get_array("a");
        for i in 0..a.len() { let mut e = a.get_object(i); if e.get_string("claim") == other_claim { e.remove_property("cutover_probe"); } }
        let w = compat::facet_write("".into(), "".into(), "".into(), "".into(), a.to_string(), "".into(), "".into(), "".into(), "".into());
        check("probe field removed", st(&w) == "ok", w.to_string());
    }
    let _g = store::lock();
    let mut idx = store::read_index("kb", "plan");
    let mut items = idx.get_object("items");
    let order = idx.get_array("order");
    let mut keep = DataArray::new();
    let mut dropped = 0;
    for i in 0..order.len() { let id = order.get_string(i);
        let c = items.get_object(&id).get_string("claim");
        if c == c1 || c == c2 {
            items.remove_property(&id);
            for k in ["item", "notes", "steps"] { store::delete_rec("kb", &store::rec_id("plan", k, &id)); }
            dropped += 1;
        } else { keep.push_string(&id); }
    }
    idx.put_array("order", keep);
    let rev = idx.get_int("rev") + 1; idx.put_int("rev", rev);
    store::write_rec("kb", &store::index_id("plan"), idx);
    check("test items removed", dropped == 2, format!("{} removed", dropped));
}
let n1 = api.agent.plan.board().get_array("entries").len();
check("board back to its starting size", n1 == n0, format!("{} -> {}", n0, n1));
if let Some(le) = store::legacy_entries() {
    let v = store::compare("kb", "plan", &le);
    out.put_boolean("equal_to_legacy_facet", v.get_boolean("equal"));
}
out.put_string("status", "ok");
out.put_boolean("all_ok", all_ok);
out.put_string("hash", &api.agent.plan.board_hash().get_string("hash"));
out.put_array("checks", checks);
out