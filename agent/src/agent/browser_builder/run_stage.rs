use ndata::dataobject::DataObject;
use flowlang::datastore::DataStore;
use std::process::{Command, Stdio};
use std::fs::File;
use std::path::Path;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["stage"] {
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
        let arg_0: String = o.get_string("stage");
        run_stage(arg_0)
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

pub fn run_stage(stage: String) -> DataObject {
// Run one build stage. `patch` runs synchronously (it is a text edit);
// every heavy stage is launched detached via setsid, logging to
// workspace/build.log, with its pid recorded in workspace/build.pid.
// Refuses to start a second stage while one is still running.
fn prop(key: &str, dflt: &str) -> String {
    (|| -> Option<String> {
        let r = DataStore::globals().try_get_object("system").ok()?
            .try_get_object("apps").ok()?.try_get_object("agent").ok()?
            .try_get_object("runtime").ok()?;
        match r.try_get_string(key) { Ok(v) if !v.trim().is_empty() => Some(v.trim().to_string()), _ => None }
    })().unwrap_or_else(|| dflt.to_string())
}
fn err(m: String) -> DataObject { let mut o = DataObject::new(); o.put_string("status","err"); o.put_string("msg", &m); o }

let stage = stage.trim().to_string();
let allowed = ["download","extract","patch","configure","deps","build","package","clean","all"];
if !allowed.contains(&stage.as_str()) {
    return err(format!("unknown stage {:?} (allowed: {})", stage, allowed.join(", ")));
}

let workspace = prop("NOOBSCAPE_WORKSPACE", "/newbound/runtime/agent/noobscape-build");
let version   = prop("NOOBSCAPE_VERSION", "128.0esr");

// patch is our Rust anchored patcher, run inline.
if stage == "patch" {
    let r = crate::agent::browser_builder::apply_patch::apply_patch();
    return r;
}

if !Path::new(&format!("{}/build.sh", workspace)).is_file() {
    return err(format!("build kit not materialized in {} (run materialize_kit first)", workspace));
}

// Refuse to stack: is a previous stage still alive?
let pidfile = format!("{}/build.pid", workspace);
if let Ok(s) = std::fs::read_to_string(&pidfile) {
    if let Ok(pid) = s.trim().parse::<i64>() {
        if pid > 0 && Path::new(&format!("/proc/{}", pid)).exists() {
            return err(format!("a build stage is already running (pid {}); wait for it or stop it first", pid));
        }
    }
}

let logpath = format!("{}/build.log", workspace);
let log = match File::create(&logpath) { Ok(f) => f, Err(e) => return err(format!("cannot open log {}: {}", logpath, e)) };
let logerr = match log.try_clone() { Ok(f) => f, Err(e) => return err(format!("log clone: {}", e)) };

// setsid detaches into a new session so the build survives this call
// returning (and even a newbound restart). stdbuf keeps the log live.
let script = format!(
    "cd '{ws}' && FIREFOX_VERSION='{v}' WORKDIR='{ws}/work' stdbuf -oL -eL ./build.sh {stage}; echo \"===NOOBSCAPE_STAGE_EXIT $?===\"",
    ws = workspace, v = version, stage = stage
);
let child = Command::new("setsid").arg("bash").arg("-c").arg(&script)
    .stdin(Stdio::null()).stdout(Stdio::from(log)).stderr(Stdio::from(logerr))
    .spawn();
let pid = match child { Ok(c) => c.id(), Err(e) => return err(format!("spawn failed: {}", e)) };

let _ = std::fs::write(&pidfile, pid.to_string());
let _ = std::fs::write(format!("{}/build.stage", workspace), &stage);

let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_string("stage", &stage);
o.put_int("pid", pid as i64);
o.put_string("log", &logpath);
o.put_string("msg", &format!("stage '{}' started (pid {})", stage, pid));
o
}
