// Terminate the running build: TERM the whole process group (setsid made
// the recorded pid a session/group leader, so -pid reaches mach and its
// children). Targeted — never a blanket pkill.
fn prop(key: &str, dflt: &str) -> String {
    (|| -> Option<String> {
        let r = DataStore::globals().try_get_object("system").ok()?
            .try_get_object("apps").ok()?.try_get_object("agent").ok()?
            .try_get_object("runtime").ok()?;
        match r.try_get_string(key) { Ok(v) if !v.trim().is_empty() => Some(v.trim().to_string()), _ => None }
    })().unwrap_or_else(|| dflt.to_string())
}
let workspace = prop("NOOBSCAPE_WORKSPACE", "/newbound/runtime/agent/noobscape-build");
let pidfile = format!("{}/build.pid", workspace);
let pid = std::fs::read_to_string(&pidfile).ok().and_then(|s| s.trim().parse::<i64>().ok()).unwrap_or(0);

let mut o = DataObject::new();
o.put_string("status", "ok");
if pid <= 0 || !Path::new(&format!("/proc/{}", pid)).exists() {
    o.put_boolean("killed", false);
    o.put_string("msg", "no running build");
    return o;
}
// Negative pid = process group.
let _ = Command::new("kill").arg("-TERM").arg(format!("-{}", pid)).status();
std::thread::sleep(std::time::Duration::from_millis(500));
let still = Path::new(&format!("/proc/{}", pid)).exists();
if still { let _ = Command::new("kill").arg("-KILL").arg(format!("-{}", pid)).status(); }
o.put_boolean("killed", true);
o.put_int("pid", pid);
o.put_string("msg", &format!("terminated build pid group {}", pid));
o