// Capture a PNG. Two modes:
//  - url non-empty: shoot that URL via Firefox's own --headless --screenshot
//    against a throwaway profile (fresh anonymous load; logged-out).
//  - url empty: capture the LIVE driven tab's real pixels. A chrome-targeted
//    request (//NOOBSCAPE-CHROME) is answered by a parent-process chrome
//    docshell, which calls drawSnapshot on the bound browsing context
//    (bind.id) - so the actual current document, mutations and login state
//    included, is rendered - encodes PNG via OffscreenCanvas, and writes it
//    with IOUtils. The driver polls shot.out for the async chain's verdict.
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
    // Require a running session: bind.id is the bound tab's browsing-context id.
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

    // The chrome-side async capture. Runs in a parent-process chrome global:
    // BrowsingContext/IOUtils/OffscreenCanvas/DOMRect are all available there,
    // and drawSnapshot lives on WindowGlobalParent. It writes shot.out at the
    // end of the chain (ok or err) - that is the driver's completion signal.
    let js_tpl = r#"(async () => {
  const DIR = "__DIR__";
  const OUT = "__OUT__";
  const BINDID = __BINDID__;
  const W = __W__, H = __H__;
  const done = (ok, err) => { try { IOUtils.writeUTF8(DIR + "/shot.out", JSON.stringify({ok: ok, err: err || ""})); } catch (e) {} };
  try {
    const bc = BrowsingContext.get(BINDID);
    if (!bc) { done(false, "no browsing context " + BINDID); return "started"; }
    const wgp = bc.currentWindowGlobal;
    if (!wgp) { done(false, "no currentWindowGlobal for " + BINDID); return "started"; }
    const rect = new DOMRect(0, 0, W, H);
    const bitmap = await wgp.drawSnapshot(rect, 1.0, "rgb(255,255,255)", false);
    if (!bitmap) { done(false, "drawSnapshot returned null"); return "started"; }
    const oc = new OffscreenCanvas(bitmap.width, bitmap.height);
    const ctx = oc.getContext("2d");
    ctx.drawImage(bitmap, 0, 0);
    const blob = await oc.convertToBlob({type: "image/png"});
    const buf = await blob.arrayBuffer();
    await IOUtils.write(OUT, new Uint8Array(buf));
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
    let _ = std::fs::remove_file(&shot_out);

    // Write the chrome request envelope tmp+rename so no watcher sees a partial.
    let id = format!("nbshot{}_{}", time(), rand_range(0, 1_000_000));
    let envelope = format!("//NOOBSCAPE-CHROME\n{}\n{}", id, js);
    let req = format!("{}/inject.js", dir);
    let tmp = format!("{}.tmp", req);
    if std::fs::write(&tmp, envelope.as_bytes()).is_err() {
        return errobj(&format!("cannot write capture request under {} (writable?)", dir));
    }
    if std::fs::rename(&tmp, &req).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return errobj("cannot place capture request file");
    }

    // Poll shot.out for the async chain's verdict (watcher fires within ~1s).
    let deadline = time() + 30000;
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
            // else: partial write - keep polling.
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    match verdict {
        None => return errobj("timeout waiting for live capture (no shot.out; is a Noobscape-v2 session running?)"),
        Some((false, err)) => return errobj(&format!("live capture failed: {}", if err.is_empty() { "unknown".to_string() } else { err })),
        Some((true, _)) => {
            let bytes = std::fs::metadata(&abs).map(|m| m.len()).unwrap_or(0);
            if bytes == 0 {
                let _ = std::fs::remove_file(&abs);
                return errobj("live capture reported ok but produced no bytes");
            }
            let mut o = DataObject::new();
            o.put_string("status", "ok");
            o.put_string("mode", "live");
            o.put_string("path", &abs);
            o.put_int("bytes", bytes as i64);
            o.put_string("url", &live_url);
            o.put_int("width", w);
            o.put_int("height", h);
            return o;
        }
    }
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