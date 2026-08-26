use ndata::dataobject::DataObject;
use flowlang::datastore::DataStore;
use std::process::Command;
use std::path::Path;
use std::os::unix::fs::symlink;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["mode"] {
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
        let arg_0: String = o.get_string("mode");
        install(arg_0)
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

pub fn install(mode: String) -> DataObject {
// Place the freshly built Noobscape where agent.browser looks
// (NOOBSCAPE_BIN). mode=symlink points at the built binary in its dist/bin
// (Firefox finds libxul beside the real path); mode=copy clones the whole
// dist/bin so the install survives a workspace clean. Also ensures the
// channel dir (NOOBSCAPE_DIR) exists.
fn prop(key: &str, dflt: &str) -> String {
    (|| -> Option<String> {
        let r = DataStore::globals().try_get_object("system").ok()?
            .try_get_object("apps").ok()?.try_get_object("agent").ok()?
            .try_get_object("runtime").ok()?;
        match r.try_get_string(key) { Ok(v) if !v.trim().is_empty() => Some(v.trim().to_string()), _ => None }
    })().unwrap_or_else(|| dflt.to_string())
}
fn err(m: String) -> DataObject { let mut o = DataObject::new(); o.put_string("status","err"); o.put_string("msg", &m); o }

let mode = { let m = mode.trim().to_lowercase(); if m.is_empty() { "symlink".to_string() } else { m } };
if mode != "symlink" && mode != "copy" { return err(format!("mode must be symlink or copy, got {:?}", mode)); }

let workspace = prop("NOOBSCAPE_WORKSPACE", "/newbound/runtime/agent/noobscape-build");
let version   = prop("NOOBSCAPE_VERSION", "128.0esr");
let bin       = prop("NOOBSCAPE_BIN", "/noobscape/bin/firefox");
let dir       = prop("NOOBSCAPE_DIR", "/noobscape");

let dist_bin = format!("{}/work/firefox-{}/obj-firefox/dist/bin", workspace, version);
let built_binary = format!("{}/firefox", dist_bin);
if !Path::new(&built_binary).is_file() {
    return err(format!("built binary not found at {} — build the browser first", built_binary));
}

let bin_dir = match Path::new(&bin).parent() { Some(p) => p.to_path_buf(), None => return err(format!("NOOBSCAPE_BIN has no parent dir: {}", bin)) };
if let Err(e) = std::fs::create_dir_all(&bin_dir) { return err(format!("cannot create {}: {}", bin_dir.display(), e)); }
let _ = std::fs::create_dir_all(&dir); // channel dir

// Remove whatever is currently at the target path (stale symlink or file).
let _ = std::fs::remove_file(&bin);

if mode == "symlink" {
    if let Err(e) = symlink(&built_binary, &bin) {
        return err(format!("symlink {} -> {}: {}", bin, built_binary, e));
    }
} else {
    // copy the whole dist/bin tree into the target's parent dir
    let st = Command::new("cp").arg("-a").arg(format!("{}/.", dist_bin)).arg(format!("{}/", bin_dir.display())).status();
    match st { Ok(s) if s.success() => {}, Ok(s) => return err(format!("cp exited {}", s)), Err(e) => return err(format!("cp failed: {}", e)) }
    if !Path::new(&bin).is_file() { return err(format!("after copy, {} is still missing (is NOOBSCAPE_BIN's basename 'firefox'?)", bin)); }
}

let target = std::fs::read_link(&bin).map(|p| p.display().to_string()).unwrap_or_else(|_| bin.clone());
let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_string("mode", &mode);
o.put_string("bin", &bin);
o.put_string("target", &target);
o.put_string("dir", &dir);
o.put_string("msg", &format!("installed ({}) at {}", mode, bin));
o
}
