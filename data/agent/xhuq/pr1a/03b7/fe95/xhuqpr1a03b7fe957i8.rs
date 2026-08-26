// Noobscape v2 primitive: evaluate `js` in the live page as the system
// principal and return its JSON.stringify'd result. Writes a v2 request
// envelope (//NOOBSCAPE / id / source) to <dir>/inject.js atomically,
// then polls <dir>/inject.out for a response whose id matches ours.
// Requires a Noobscape-v2 browser running against the same NOOBSCAPE_DIR
// (see _ASSETS/browser/noobscape-v2.md). Returns {status, value} or
// {status:err, msg}.
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

let dir = prop("NOOBSCAPE_DIR", "/noobscape");
let dir = dir.trim_end_matches('/').to_string();
let req = format!("{}/inject.js", dir);
let out = format!("{}/inject.out", dir);
let tmo: i64 = if timeout_ms > 0 { timeout_ms } else { 15000 };

let id = format!("nb{}_{}", time(), rand_range(0, 1_000_000));

// Clear any stale response so we never read one from a prior request.
let _ = std::fs::remove_file(&out);

// Write the v2 envelope tmp+rename so the browser never sees a partial file.
let envelope = format!("//NOOBSCAPE\n{}\n{}", id, js);
let tmp = format!("{}.tmp", req);
if std::fs::write(&tmp, envelope.as_bytes()).is_err() {
    return errobj(&format!("cannot write request under {} (writable?)", dir));
}
if std::fs::rename(&tmp, &req).is_err() {
    let _ = std::fs::remove_file(&tmp);
    return errobj("cannot place request file");
}

let deadline = time() + tmo;
let mut result = errobj("timeout waiting for browser response");
while time() < deadline {
    if let Ok(s) = std::fs::read_to_string(&out) {
        // <id>\n<OK|ERR>\n<payload> ; payload is compact JSON (no raw newlines)
        let mut it = s.splitn(3, '\n');
        let rid = it.next().unwrap_or("");
        if rid == id {
            let tag = it.next().unwrap_or("");
            let payload = it.next().unwrap_or("");
            let _ = std::fs::remove_file(&out);
            if tag == "OK" {
                let pv = if payload.trim().is_empty() { "null" } else { payload };
                let wrap = format!("{{\"status\":\"ok\",\"value\":{}}}", pv);
                result = match DataObject::try_from_string(&wrap) {
                    Ok(o) => o,
                    Err(_) => {
                        let mut o = DataObject::new();
                        o.put_string("status", "ok");
                        o.put_string("value_raw", payload);
                        o
                    }
                };
            } else {
                result = errobj(payload);
            }
            break;
        }
        // else: a stale response from an earlier request; keep polling.
    }
    std::thread::sleep(std::time::Duration::from_millis(50));
}
result