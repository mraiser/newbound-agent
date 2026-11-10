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