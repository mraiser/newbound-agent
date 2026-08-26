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