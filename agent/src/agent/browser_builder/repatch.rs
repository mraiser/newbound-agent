use ndata::dataobject::DataObject;
use flowlang::datastore::DataStore;
pub fn execute(_: DataObject) -> DataObject {
    use std::panic;
    let ax = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        repatch()
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

pub fn repatch() -> DataObject {
// The update path: apply_patch is idempotent on the NoobscapeStartWatcher
// marker, so a tree that already carries an OLD block answers `already`
// and keeps it. repatch restores the pristine .noobscape.orig backup
// first, refreshes the kit mirror, then applies the CURRENT block asset.
fn prop(key: &str, dflt: &str) -> String {
    (|| -> Option<String> {
        let r = DataStore::globals().try_get_object("system").ok()?
            .try_get_object("apps").ok()?.try_get_object("agent").ok()?
            .try_get_object("runtime").ok()?;
        match r.try_get_string(key) { Ok(v) if !v.trim().is_empty() => Some(v.trim().to_string()), _ => None }
    })().unwrap_or_else(|| dflt.to_string())
}
fn err(m: String) -> DataObject { let mut o = DataObject::new(); o.put_string("status","err"); o.put_string("msg", &m); o }

let workspace = prop("NOOBSCAPE_WORKSPACE", "/newbound/runtime/agent/noobscape-build");
let version   = prop("NOOBSCAPE_VERSION", "128.0esr");
let cpp_path = format!("{}/work/firefox-{}/docshell/base/nsDocShell.cpp", workspace, version);
let orig = format!("{}.noobscape.orig", cpp_path);

let src = match std::fs::read_to_string(&cpp_path) {
    Ok(s) => s,
    Err(e) => return err(format!("cannot read {} (extract the source first): {}", cpp_path, e)),
};

let mut restored = false;
if src.contains("NoobscapeStartWatcher") {
    let pristine = match std::fs::read_to_string(&orig) {
        Ok(p) => p,
        Err(e) => return err(format!("source is patched but no pristine backup at {} ({}); re-extract the source and patch again", orig, e)),
    };
    if pristine.contains("NoobscapeStartWatcher") {
        return err(format!("backup {} itself carries the mechanism - it is not pristine; re-extract the source", orig));
    }
    if let Err(e) = std::fs::write(&cpp_path, &pristine) {
        return err(format!("restore {} from backup: {}", cpp_path, e));
    }
    restored = true;
}

// Keep the workspace mirror of the kit in step with the store assets.
let m = crate::agent::browser_builder::materialize_kit::materialize_kit();
if m.get_string("status") != "ok" { return m; }

let mut r = crate::agent::browser_builder::apply_patch::apply_patch();
r.put_boolean("restored", restored);
if r.get_string("status") == "ok" {
    let action = if r.has("action") { r.get_string("action") } else { String::new() };
    r.put_string("msg", &format!("{}source patched with the current v2 block ({})",
        if restored { "pristine source restored; " } else { "" }, action));
}
r
}
