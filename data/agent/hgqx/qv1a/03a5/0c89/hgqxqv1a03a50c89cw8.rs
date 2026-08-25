let api = crate::api::new();
let r = api.dev.code.read_control_facet("kb".to_string(), "plan".to_string(), "memory".to_string());
let mut out = DataObject::new();
if !r.try_get_boolean("exists").unwrap_or(false) {
    out.put_string("status", "ok");
    out.put_string("hash", "");
    out.put_array("entries", DataArray::new());
    return out;
}
let src = r.get_string("source");
// The facet is a JSON array (legacy pretty-printed or line-oriented);
// wrap it so try_from_string sees an object - the house idiom for
// parsing json whose top level is not guaranteed to be an object.
let wrapped = format!("{{\"a\": {} }}", src);
match DataObject::try_from_string(&wrapped) {
    Ok(o) => {
        out.put_string("status", "ok");
        out.put_string("hash", &r.get_string("hash"));
        out.put_array("entries", o.get_array("a"));
    },
    Err(_) => {
        out.put_string("status", "err");
        out.put_string("msg", "kb.plan memory facet did not parse as a JSON array");
    }
}
out