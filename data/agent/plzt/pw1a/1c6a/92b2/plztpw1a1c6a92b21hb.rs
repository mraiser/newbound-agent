// Removes one plan item for good: its index entry and its item, notes and
// steps records. For mistakes and test items - finished or dropped work stays
// on the board as done/abandoned. Refuses a parent that still has children and
// an item a plan step is running on.
let _ = nn_sessionid;
compat::ensure();
let holder = crate::agent::plan::step_lock::holder();
if holder.has("claim") && holder.get_string("claim") == claim {
    return store::err("a plan step is running on this item; wait for it to finish");
}
let _g = store::lock();
let mut idx = store::read_index("kb", "plan");
let id = match store::find_id(&idx, &claim) { Some(id) => id, None => return store::err("no plan item with that claim") };
let mut items = idx.get_object("items");
let order = idx.get_array("order");
let mut children = 0;
for i in 0..order.len() {
    let oid = order.get_string(i);
    if oid == id || !items.has(&oid) { continue; }
    let it = items.get_object(&oid);
    if it.has("parent") && it.get_string("parent") == claim { children += 1; }
}
if children > 0 {
    let mut e = store::err("the item has subtasks; remove them first");
    e.put_int("children", children);
    return e;
}
let lifecycle = { let it = items.get_object(&id); if it.has("lifecycle") { it.get_string("lifecycle") } else { String::new() } };
let mut keep = DataArray::new();
for i in 0..order.len() { let oid = order.get_string(i); if oid != id { keep.push_string(&oid); } }
items.remove_property(&id);
idx.put_array("order", keep);
let rev = idx.get_int("rev") + 1;
idx.put_int("rev", rev);
let remaining = idx.get_array("order").len() as i64;
store::write_rec("kb", &store::index_id("plan"), idx);
let mut records = 0;
for k in ["item", "notes", "steps"] { if store::delete_rec("kb", &store::rec_id("plan", k, &id)) { records += 1; } }
let mut out = DataObject::new();
out.put_string("status", "ok");
out.put_boolean("removed", true);
out.put_string("item_id", &id);
out.put_string("lifecycle", &lifecycle);
out.put_int("records", records);
out.put_int("items", remaining);
out.put_string("hash", &format!("r{}", rev));
out