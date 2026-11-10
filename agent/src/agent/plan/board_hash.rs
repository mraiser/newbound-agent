use ndata::dataobject::DataObject;

pub fn execute(_: DataObject) -> DataObject {
    use std::panic;
    let ax = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        board_hash()
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

pub fn board_hash() -> DataObject {
// Cheap change probe for the plan board: just the kb.plan memory facet's
// content hash (the same token board returns and move_item takes), without
// shipping the entries. The kanban polls this and reloads the full board
// only when the hash moves.
let api = crate::api::new();
let r = api.dev.code.read_control_facet("kb".to_string(), "plan".to_string(), "memory".to_string());
let mut out = DataObject::new();
out.put_string("status", "ok");
if r.try_get_boolean("exists").unwrap_or(false) {
    out.put_string("hash", &r.try_get_string("hash").unwrap_or_default());
} else {
    out.put_string("hash", "");
}
out
}
