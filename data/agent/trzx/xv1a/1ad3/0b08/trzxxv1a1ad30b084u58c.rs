// Read-only probe of the plan step lock (helpers live in this module's
// imports: acquire / holder / StepGuard). The token is internal and omitted.
let h = holder();
let mut out = DataObject::new();
out.put_string("status", "ok");
out.put_boolean("held", h.has("token"));
if h.has("token") {
    let mut o = DataObject::new();
    for k in ["claim", "caller", "run_id"] {
        if let Ok(v) = h.try_get_string(k) { o.put_string(k, &v); }
    }
    let since = h.try_get_int("since").unwrap_or(0);
    o.put_int("since", since);
    o.put_int("age_ms", time() - since);
    out.put_object("holder", o);
}
out