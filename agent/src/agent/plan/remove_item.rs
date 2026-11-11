use ndata::dataobject::DataObject;
use ndata::dataarray::DataArray;
use crate::agent::plan::{compat, store};
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["claim", "nn_sessionid"] {
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
        let arg_0: String = o.get_string("claim");
        let arg_1: String = o.get_string("nn_sessionid");
        remove_item(arg_0, arg_1)
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

pub fn remove_item(claim: String, nn_sessionid: String) -> DataObject {
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
}
