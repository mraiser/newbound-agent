// Cheap change probe: the plan index revision (one small record), the same
// token board returns. It moves on every write to any item, note or step.
let mut out = DataObject::new();
out.put_string("status", "ok");
out.put_string("hash", &crate::agent::plan::compat::hash());
out