use ndata::dataobject::DataObject;
use flowlang::datastore::DataStore;
use std::path::Path;
pub fn execute(_: DataObject) -> DataObject {
    use std::panic;
    let ax = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        builder_status()
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

pub fn builder_status() -> DataObject {
// Read-only snapshot of the Noobscape build workspace + install state.
// Drives the browser_builder wizard UI.
fn prop(key: &str, dflt: &str) -> String {
    (|| -> Option<String> {
        let r = DataStore::globals().try_get_object("system").ok()?
            .try_get_object("apps").ok()?
            .try_get_object("agent").ok()?
            .try_get_object("runtime").ok()?;
        match r.try_get_string(key) {
            Ok(v) if !v.trim().is_empty() => Some(v.trim().to_string()),
            _ => None,
        }
    })().unwrap_or_else(|| dflt.to_string())
}

let workspace = prop("NOOBSCAPE_WORKSPACE", "/newbound/runtime/agent/noobscape-build");
let version   = prop("NOOBSCAPE_VERSION", "128.0esr");
let bin       = prop("NOOBSCAPE_BIN", "/noobscape/bin/firefox");
let dir       = prop("NOOBSCAPE_DIR", "/noobscape");
let display   = prop("BROWSER_DISPLAY", "");

let src_dir = format!("{}/work/firefox-{}", workspace, version);
let cpp = format!("{}/docshell/base/nsDocShell.cpp", src_dir);
let built_binary = format!("{}/obj-firefox/dist/bin/firefox", src_dir);

let workspace_exists = Path::new(&workspace).is_dir();
let kit_present = Path::new(&format!("{}/build.sh", workspace)).is_file();
let src_extracted = Path::new(&src_dir).is_dir();
let patched = std::fs::read_to_string(&cpp).map(|c| c.contains("NoobscapeStartWatcher")).unwrap_or(false);
let built = Path::new(&built_binary).is_file();

// Install state: is NOOBSCAPE_BIN present, and where does it point?
let bin_exists = Path::new(&bin).exists();
let bin_target = std::fs::read_link(&bin).map(|p| p.display().to_string()).unwrap_or_default();
let installed = bin_exists && (!bin_target.is_empty() || Path::new(&bin).is_file());

// Build process liveness + current stage.
let pid = std::fs::read_to_string(format!("{}/build.pid", workspace)).ok()
    .and_then(|s| s.trim().parse::<i64>().ok()).unwrap_or(0);
let build_running = pid > 0 && std::fs::read_to_string(format!("/proc/{}/stat", pid)).ok()
    .and_then(|st| st.rsplit(')').next().map(|r| r.trim_start().chars().next() != Some('Z')))
    .unwrap_or(false);
let stage = std::fs::read_to_string(format!("{}/build.stage", workspace)).unwrap_or_default().trim().to_string();

// Log tail (last ~40 lines) + last stage exit code if the marker is present.
let log = std::fs::read_to_string(format!("{}/build.log", workspace)).unwrap_or_default();
let lines: Vec<&str> = log.lines().collect();
let tail: String = lines.iter().rev().take(40).rev().cloned().collect::<Vec<_>>().join("\n");
let last_exit = lines.iter().rev().find_map(|l| l.strip_prefix("===NOOBSCAPE_STAGE_EXIT ").map(|s| s.trim_end_matches('=').trim().to_string())).unwrap_or_default();

let mut cfg = DataObject::new();
cfg.put_string("workspace", &workspace);
cfg.put_string("version", &version);
cfg.put_string("bin", &bin);
cfg.put_string("dir", &dir);
cfg.put_string("display", &display);

let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_object("config", cfg);
o.put_string("src_dir", &src_dir);
o.put_string("built_binary", &built_binary);
o.put_boolean("workspace_exists", workspace_exists);
o.put_boolean("kit_present", kit_present);
o.put_boolean("src_extracted", src_extracted);
o.put_boolean("patched", patched);
o.put_boolean("built", built);
o.put_boolean("installed", installed);
o.put_string("bin_target", &bin_target);
o.put_boolean("build_running", build_running);
o.put_int("build_pid", pid);
o.put_string("stage", &stage);
o.put_string("last_exit", &last_exit);
o.put_string("log_tail", &tail);
o
}
