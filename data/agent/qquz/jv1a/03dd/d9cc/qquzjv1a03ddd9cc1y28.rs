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

// The assets are the kit's source of truth: refresh the workspace copy at
// every launch so an asset fix can never lose to a stale materialization.
let m = crate::agent::browser_builder::materialize_kit::materialize_kit();
if m.get_string("status") != "ok" { return m; }

if !Path::new(&format!("{}/build.sh", workspace)).is_file() {
    return err(format!("build kit not materialized in {} (run materialize_kit first)", workspace));
}

// Refuse to stack: is a previous stage still alive?
let pidfile = format!("{}/build.pid", workspace);
if let Ok(s) = std::fs::read_to_string(&pidfile) {
    if let Ok(pid) = s.trim().parse::<i64>() {
        // A finished stage can linger as a zombie (/proc entry still
        // present), so liveness must exclude state Z.
        let alive = pid > 0 && std::fs::read_to_string(format!("/proc/{}/stat", pid)).ok()
            .and_then(|st| st.rsplit(')').next().map(|r| r.trim_start().chars().next() != Some('Z')))
            .unwrap_or(false);
        if alive {
            return err(format!("a build stage is already running (pid {}); wait for it or stop it first", pid));
        }
    }
}

let logpath = format!("{}/build.log", workspace);
let log = match File::create(&logpath) { Ok(f) => f, Err(e) => return err(format!("cannot open log {}: {}", logpath, e)) };
let logerr = match log.try_clone() { Ok(f) => f, Err(e) => return err(format!("log clone: {}", e)) };

// setsid detaches into a new session so the build survives this call
// returning (and even a newbound restart). stdbuf keeps the log live.
// 'all' must carry the Noobscape mechanism, which lives in the Rust
// apply_patch command — build.sh's own patch stage only applies patches/.
// Chain it between deps and build via `newbound exec`, and gate the build
// on the mechanism marker actually being present in the source.
let script = if stage == "all" {
    let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_else(|_| "newbound".to_string());
    let root = std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_else(|_| ".".to_string());
    let cpp = format!("{}/work/firefox-{}/docshell/base/nsDocShell.cpp", workspace, version);
    format!(
        "cd '{ws}' && FIREFOX_VERSION='{v}' WORKDIR='{ws}/work' stdbuf -oL -eL ./build.sh download extract patch deps && (cd '{root}' && '{exe}' exec agent browser_builder apply_patch '{{}}') && grep -q NoobscapeStartWatcher '{cpp}' && cd '{ws}' && FIREFOX_VERSION='{v}' WORKDIR='{ws}/work' stdbuf -oL -eL ./build.sh build; echo \"===NOOBSCAPE_STAGE_EXIT $?===\"",
        ws = workspace, v = version, root = root, exe = exe, cpp = cpp
    )
} else {
    format!(
        "cd '{ws}' && FIREFOX_VERSION='{v}' WORKDIR='{ws}/work' stdbuf -oL -eL ./build.sh {stage}; echo \"===NOOBSCAPE_STAGE_EXIT $?===\"",
        ws = workspace, v = version, stage = stage
    )
};
let child = Command::new("setsid").arg("bash").arg("-c").arg(&script)
    .stdin(Stdio::null()).stdout(Stdio::from(log)).stderr(Stdio::from(logerr))
    .spawn();
let mut child = match child { Ok(c) => c, Err(e) => return err(format!("spawn failed: {}", e)) };
let pid = child.id();
// Reap the child when it exits — without this it stays a zombie whose
// /proc entry makes every liveness check read the stage as running.
std::thread::spawn(move || { let _ = child.wait(); });

let _ = std::fs::write(&pidfile, pid.to_string());
let _ = std::fs::write(format!("{}/build.stage", workspace), &stage);

let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_string("stage", &stage);
o.put_int("pid", pid as i64);
o.put_string("log", &logpath);
o.put_string("msg", &format!("stage '{}' started (pid {})", stage, pid));
o