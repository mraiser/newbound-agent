use ndata::dataobject::DataObject;

pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["claim", "lifecycle", "base", "nn_sessionid"] {
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
        let arg_1: String = o.get_string("lifecycle");
        let arg_2: String = o.get_string("base");
        let arg_3: String = o.get_string("nn_sessionid");
        move_item(arg_0, arg_1, arg_2, arg_3)
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

pub fn move_item(claim: String, lifecycle: String, base: String, nn_sessionid: String) -> DataObject {
let lifecycles = ["proposed", "accepted", "in-progress", "done", "abandoned"];
let mut out = DataObject::new();
if !lifecycles.contains(&lifecycle.as_str()) {
    out.put_string("status", "err");
    out.put_string("msg", "lifecycle must be one of proposed/accepted/in-progress/done/abandoned");
    return out;
}
let api = crate::api::new();
let r = api.dev.code.read_control_facet("kb".to_string(), "plan".to_string(), "memory".to_string());
if !r.try_get_boolean("exists").unwrap_or(false) {
    out.put_string("status", "err");
    out.put_string("msg", "kb.plan has no memory facet");
    return out;
}
let src = r.get_string("source");
let hash = r.get_string("hash");
if !base.is_empty() && base != hash {
    out.put_string("status", "err");
    out.put_string("msg", "stale_base");
    out.put_string("current_hash", &hash);
    return out;
}
let wrapped = format!("{{\"a\": {} }}", src);
let parsed = match DataObject::try_from_string(&wrapped) {
    Ok(o) => o,
    Err(_) => {
        out.put_string("status", "err");
        out.put_string("msg", "kb.plan memory facet did not parse as a JSON array");
        return out;
    }
};
let arr = parsed.get_array("a");
let mut found = false;
for i in 0..arr.len() {
    let mut e = arr.get_object(i);
    if e.try_get_string("claim").unwrap_or_default() == claim {
        let tags = e.try_get_string("tags").unwrap_or_default();
        let mut kept: Vec<String> = tags.split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty() && !lifecycles.contains(&t.as_str()))
            .collect();
        kept.push(lifecycle.clone());
        e.put_string("tags", &kept.join(","));
        found = true;
        break;
    }
}
if !found {
    out.put_string("status", "err");
    out.put_string("msg", "no kb.plan entry with that claim - the board may be stale, refresh it");
    return out;
}
let short: String = claim.chars().take(60).collect();
let label = format!("plan board: '{}' -> {}", short, lifecycle);
api.dev.code.patch_control_facet(
    "kb".to_string(), "plan".to_string(), "memory".to_string(),
    "".to_string(), arr.to_string(), hash, label, "".to_string(), nn_sessionid)
}
