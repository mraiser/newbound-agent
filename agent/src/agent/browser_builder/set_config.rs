use ndata::dataobject::DataObject;

pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["key", "value"] {
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
        let arg_0: String = o.get_string("key");
        let arg_1: String = o.get_string("value");
        set_config(arg_0, arg_1)
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

pub fn set_config(key: String, value: String) -> DataObject {
// Persist one Noobscape builder config key. Whitelisted so the wizard can
// only touch the browser keys, then delegates to agent.model.set_setting
// (which updates live globals + rewrites botd.properties). Empty value
// reverts the key to its default.
let allowed = ["NOOBSCAPE_WORKSPACE", "NOOBSCAPE_VERSION", "NOOBSCAPE_BIN", "NOOBSCAPE_DIR", "BROWSER_DISPLAY"];
let key = key.trim().to_string();
if !allowed.contains(&key.as_str()) {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", &format!("key not settable here: {} (allowed: {})", key, allowed.join(", ")));
    return o;
}
crate::agent::model::set_setting::set_setting(key, value.trim().to_string())
}
