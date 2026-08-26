use ndata::dataobject::DataObject;
use flowlang::datastore::DataStore;
use ndata::dataarray::DataArray;
use std::os::unix::fs::PermissionsExt;
pub fn execute(_: DataObject) -> DataObject {
    use std::panic;
    let ax = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        materialize_kit()
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

pub fn materialize_kit() -> DataObject {
// Copy the build kit + the v2 nsDocShell block from the library assets into
// the configured workspace, and make the scripts executable. Idempotent.
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
let assets = DataStore::new().root.join("agent").join("_ASSETS").join("browser_builder");

if let Err(e) = std::fs::create_dir_all(&workspace) { return err(format!("cannot create workspace {}: {}", workspace, e)); }
let _ = std::fs::create_dir_all(format!("{}/patches", workspace));

// (asset relative path, destination name under workspace, executable)
let files = [
    ("kit/build.sh", "build.sh", true),
    ("kit/apply-patches.sh", "apply-patches.sh", true),
    ("kit/mozconfig", "mozconfig", false),
    ("kit/dotgitignore", ".gitignore", false),
    ("kit/README.md", "README.md", false),
    ("nsDocShell_v2_block.cpp", "nsDocShell_v2_block.cpp", false),
];
let mut written = DataArray::new();
for (rel, dest, exec) in files.iter() {
    let src = assets.join(rel);
    let dst = std::path::Path::new(&workspace).join(dest);
    if let Err(e) = std::fs::copy(&src, &dst) {
        return err(format!("copy {} -> {}: {}", src.display(), dst.display(), e));
    }
    if *exec {
        if let Ok(md) = std::fs::metadata(&dst) {
            let mut p = md.permissions(); p.set_mode(0o755); let _ = std::fs::set_permissions(&dst, p);
        }
    }
    written.push_string(dest);
}

let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_string("workspace", &workspace);
o.put_array("written", written);
o
}
