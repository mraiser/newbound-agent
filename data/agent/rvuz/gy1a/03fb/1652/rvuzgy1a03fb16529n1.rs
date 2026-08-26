// Capture `url` (or the live session's current page) as a PNG using
// Firefox's own --headless --screenshot mode - the same CLI launch path
// open() trusts. Runs against a throwaway profile so it never fights the
// live session's profile lock; live-page capture is impossible in this
// build (drawWindow is gone, content-process pixels are unreachable),
// and since goto() rebinds per navigation, shooting the URL is
// equivalent to shooting the current tab.
fn prop(key: &str, dflt: &str) -> String {
    (|| -> Option<String> {
        let s = DataStore::globals().try_get_object("system").ok()?;
        let a = s.try_get_object("apps").ok()?;
        let g = a.try_get_object("agent").ok()?;
        let r = g.try_get_object("runtime").ok()?;
        match r.try_get_string(key) {
            Ok(v) if !v.trim().is_empty() => Some(v.trim().to_string()),
            _ => None,
        }
    })().unwrap_or_else(|| dflt.to_string())
}
fn errobj(m: &str) -> DataObject {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", m);
    o
}
fn absolutize(p: &str) -> String {
    if p.starts_with('/') { return p.to_string(); }
    match std::env::current_dir() {
        Ok(d) => d.join(p).to_string_lossy().to_string(),
        Err(_) => p.to_string(),
    }
}

let bin = prop("NOOBSCAPE_BIN", "/noobscape/bin/firefox");

// Empty url = the live session's current page.
let mut target = url.trim().to_string();
if target.is_empty() {
    let r = crate::agent::browser::eval::eval("location.href".to_string(), 3000);
    if r.try_get_string("status").map(|s| s == "ok").unwrap_or(false) {
        if let Ok(v) = r.try_get_string("value") {
            if !v.is_empty() && v != "null" { target = v; }
        }
    }
    if target.is_empty() {
        return errobj("no url given and no live session to read one from");
    }
}
if target.contains('\'') {
    return errobj("url may not contain a single quote");
}

let w = if width <= 0 { 1280 } else { width };
let h = if height <= 0 { 1024 } else { height };

let mut out_path = path.trim().to_string();
if out_path.is_empty() {
    out_path = format!("runtime/agent/screenshots/shot-{}.png", time());
}
if out_path.contains('\'') {
    return errobj("path may not contain a single quote");
}
let abs = absolutize(&out_path);
if let Some(parent) = std::path::Path::new(&abs).parent() {
    if std::fs::create_dir_all(parent).is_err() {
        return errobj(&format!("cannot create output dir for {}", abs));
    }
}
let _ = std::fs::remove_file(&abs);

// Throwaway profile: deterministic, and no clash with the default
// profile's lock while a driven session is running.
let profdir = absolutize(&format!("tmp/noobscape-shot-{}", time()));
if std::fs::create_dir_all(&profdir).is_err() {
    return errobj("cannot create temp profile dir");
}

// timeout(1) backstops a hung page; firefox exits on its own after the shot.
let line = format!(
    "MOZ_DISABLE_JEMALLOC=1 MOZ_DISABLE_CONTENT_SANDBOX=1 LIBGL_ALWAYS_SOFTWARE=1 timeout 90 '{}' --headless --no-remote --profile '{}' --window-size={},{} --screenshot '{}' '{}'",
    bin, profdir, w, h, abs, target
);
let run = Command::new("bash").arg("-c").arg(&line).output();
let _ = std::fs::remove_dir_all(&profdir);

let mut o = DataObject::new();
match run {
    Err(e) => return errobj(&format!("spawn failed: {}", e)),
    Ok(out) => {
        let bytes = std::fs::metadata(&abs).map(|m| m.len()).unwrap_or(0);
        if bytes == 0 {
            let _ = std::fs::remove_file(&abs);
            let err = String::from_utf8_lossy(&out.stderr);
            let tail: String = err.chars().rev().take(400).collect::<String>().chars().rev().collect();
            return errobj(&format!("no screenshot produced (exit {:?}): {}", out.status.code(), tail.trim()));
        }
        o.put_string("status", "ok");
        o.put_string("path", &abs);
        o.put_int("bytes", bytes as i64);
        o.put_string("url", &target);
        o.put_int("width", w);
        o.put_int("height", h);
    }
}
o