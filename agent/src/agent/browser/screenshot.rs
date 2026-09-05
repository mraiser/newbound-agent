use ndata::dataobject::DataObject;
use flowlang::datastore::DataStore;
use flowlang::flowlang::system::time::time;
use flowlang::rand::rand_range;
use std::process::Command;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["url", "path", "width", "height"] {
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
        let arg_0: String = o.get_string("url");
        let arg_1: String = o.get_string("path");
        let arg_2: i64 = o.get_int("width");
        let arg_3: i64 = o.get_int("height");
        screenshot(arg_0, arg_1, arg_2, arg_3)
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

pub fn screenshot(url: String, path: String, width: i64, height: i64) -> DataObject {
// Capture a PNG. Two modes:
//  - url non-empty: shoot that URL via Firefox's own --headless --screenshot
//    against a throwaway profile (fresh anonymous load; logged-out).
//  - url empty: capture the LIVE driven tab's real pixels. A chrome-targeted
//    request (//NOOBSCAPE-CHROME) is answered by a parent-process chrome
//    docshell, which calls drawSnapshot on the bound browsing context
//    (bind.id) - the actual current document, mutations and login state
//    included - encodes PNG via OffscreenCanvas, and writes it with IOUtils.
//    The capture JS bounds each async step with an internal timeout and the
//    driver retries, so an occasional drawSnapshot stall recovers instead of
//    hanging. The driver polls shot.out for each attempt's verdict.
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
// Safe to embed inside a JS double-quoted string literal?
fn js_safe(p: &str) -> bool {
    !p.contains('"') && !p.contains('\\') && !p.contains('\n') && !p.contains('\r')
}

let dir = prop("NOOBSCAPE_DIR", "/noobscape");
let dir = dir.trim_end_matches('/').to_string();
let bin = prop("NOOBSCAPE_BIN", "/noobscape/bin/firefox");

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

let target = url.trim().to_string();

// ===================================================================
// LIVE MODE: url empty -> capture the bound tab's real pixels.
// ===================================================================
if target.is_empty() {
    let bind_id_path = format!("{}/bind.id", dir);
    let bindid = match std::fs::read_to_string(&bind_id_path) {
        Ok(s) => s.trim().to_string(),
        Err(_) => return errobj("no live session (bind.id absent) - open() one first, or pass an explicit url"),
    };
    if bindid.is_empty() || !bindid.chars().all(|c| c.is_ascii_digit()) {
        return errobj("bind.id is empty or malformed");
    }
    if !js_safe(&dir) || !js_safe(&abs) {
        return errobj("channel dir or output path contains characters unsafe to embed in the capture script");
    }

    // Best-effort: the live tab's URL, for the return value only.
    let mut live_url = String::new();
    let r = crate::agent::browser::eval::eval("location.href".to_string(), 4000);
    if r.try_get_string("status").map(|s| s == "ok").unwrap_or(false) {
        if let Ok(v) = r.try_get_string("value") {
            if !v.is_empty() && v != "null" { live_url = v; }
        }
    }

    // Chrome-side async capture, run in a parent-process chrome global.
    // withTimeout bounds each await so a stalled drawSnapshot rejects fast
    // (the driver then retries) rather than hanging the whole call.
    let js_tpl = r#"(async () => {
  const DIR = "__DIR__";
  const OUT = "__OUT__";
  const BINDID = __BINDID__;
  const W = __W__, H = __H__;
  const done = (ok, err) => { try { IOUtils.writeUTF8(DIR + "/shot.out", JSON.stringify({ok: ok, err: err || ""})); } catch (e) {} };
  const withTimeout = (p, ms, label) => Promise.race([p, new Promise((_, rej) => setTimeout(() => rej(new Error("TIMEOUT " + label)), ms))]);
  try {
    const bc = BrowsingContext.getCurrentTopByBrowserId(BINDID);
    if (!bc) { done(false, "no tab for browser id " + BINDID); return "started"; }
    const wgp = bc.currentWindowGlobal;
    if (!wgp) { done(false, "no currentWindowGlobal for " + BINDID); return "started"; }
    const rect = new DOMRect(0, 0, W, H);
    const bitmap = await withTimeout(wgp.drawSnapshot(rect, 1.0, "rgb(255,255,255)", false), 5000, "drawSnapshot");
    if (!bitmap) { done(false, "drawSnapshot returned null"); return "started"; }
    const oc = new OffscreenCanvas(bitmap.width, bitmap.height);
    const ctx = oc.getContext("2d");
    ctx.drawImage(bitmap, 0, 0);
    const blob = await withTimeout(oc.convertToBlob({type: "image/png"}), 10000, "convertToBlob");
    const buf = await blob.arrayBuffer();
    await withTimeout(IOUtils.write(OUT, new Uint8Array(buf)), 10000, "write");
    done(true, "");
  } catch (e) { done(false, String(e)); }
  return "started";
})()"#;
    let js = js_tpl
        .replace("__DIR__", &dir)
        .replace("__OUT__", &abs)
        .replace("__BINDID__", &bindid)
        .replace("__W__", &w.to_string())
        .replace("__H__", &h.to_string());

    let shot_out = format!("{}/shot.out", dir);
    let req = format!("{}/inject.js", dir);

    // Up to 3 attempts; each internally bounded to ~10s, polled to 15s.
    let mut last_reason = String::from("no attempt ran");
    for attempt in 0..3 {
        let _ = std::fs::remove_file(&shot_out);
        let id = format!("nbshot{}_{}", time(), rand_range(0, 1_000_000));
        let envelope = format!("//NOOBSCAPE-CHROME\n{}\n{}", id, js);
        let tmp = format!("{}.tmp", req);
        if std::fs::write(&tmp, envelope.as_bytes()).is_err() {
            return errobj(&format!("cannot write capture request under {} (writable?)", dir));
        }
        if std::fs::rename(&tmp, &req).is_err() {
            let _ = std::fs::remove_file(&tmp);
            return errobj("cannot place capture request file");
        }

        let deadline = time() + 15000;
        let mut verdict: Option<(bool, String)> = None;
        while time() < deadline {
            if let Ok(s) = std::fs::read_to_string(&shot_out) {
                let wrap = format!("{{\"v\":{}}}", s.trim());
                if let Ok(o) = DataObject::try_from_string(&wrap) {
                    if let Ok(v) = o.try_get_object("v") {
                        let ok = v.try_get_boolean("ok").unwrap_or(false);
                        let err = v.try_get_string("err").unwrap_or_default();
                        let _ = std::fs::remove_file(&shot_out);
                        verdict = Some((ok, err));
                        break;
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        match verdict {
            Some((true, _)) => {
                let bytes = std::fs::metadata(&abs).map(|m| m.len()).unwrap_or(0);
                if bytes == 0 {
                    let _ = std::fs::remove_file(&abs);
                    last_reason = "capture reported ok but produced no bytes".to_string();
                } else {
                    let mut o = DataObject::new();
                    o.put_string("status", "ok");
                    o.put_string("mode", "live");
                    o.put_string("path", &abs);
                    o.put_int("bytes", bytes as i64);
                    o.put_string("url", &live_url);
                    o.put_int("width", w);
                    o.put_int("height", h);
                    o.put_int("attempts", (attempt + 1) as i64);
                    return o;
                }
            }
            Some((false, err)) => {
                last_reason = if err.is_empty() { "unknown".to_string() } else { err };
            }
            None => {
                last_reason = "no verdict (watcher silent)".to_string();
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(400));
    }
    return errobj(&format!("live capture failed after 3 attempts: {}", last_reason));
}

// ===================================================================
// URL MODE: explicit url -> throwaway-profile CLI shot (logged-out).
// ===================================================================
if target.contains('\'') {
    return errobj("url may not contain a single quote");
}
let profdir = absolutize(&format!("tmp/noobscape-shot-{}", time()));
if std::fs::create_dir_all(&profdir).is_err() {
    return errobj("cannot create temp profile dir");
}
// NOOBSCAPE_DIR points the mechanism's file-watcher at the empty throwaway
// dir so a leftover live-channel QUIT can't force-quit the screenshot browser.
let line = format!(
    "NOOBSCAPE_DIR='{}' MOZ_DISABLE_JEMALLOC=1 MOZ_DISABLE_CONTENT_SANDBOX=1 LIBGL_ALWAYS_SOFTWARE=1 timeout 90 '{}' --headless --no-remote --profile '{}' --window-size={},{} --screenshot '{}' '{}'",
    profdir, bin, profdir, w, h, abs, target
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
        o.put_string("mode", "url");
        o.put_string("path", &abs);
        o.put_int("bytes", bytes as i64);
        o.put_string("url", &target);
        o.put_int("width", w);
        o.put_int("height", h);
    }
}
o
}
