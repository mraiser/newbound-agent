use ndata::dataobject::DataObject;

pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["filename", "data_b64"] {
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
        let arg_0: String = o.get_string("filename");
        let arg_1: String = o.get_string("data_b64");
        upload(arg_0, arg_1)
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

pub fn upload(filename: String, data_b64: String) -> DataObject {
fn err(msg: &str) -> DataObject {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", msg);
    o
}

// Basename only, safe chars only — uploads never choose their directory.
let base = filename.rsplit(['/', '\\']).next().unwrap_or("").trim().to_string();
let mut clean: String = base.chars()
    .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
    .collect();
while clean.starts_with('.') { clean.remove(0); }
if clean.is_empty() { clean = "file".to_string(); }

let mut bytes: Vec<u8> = Vec::with_capacity(data_b64.len() / 4 * 3);
let mut acc: u32 = 0;
let mut nbits: u32 = 0;
for &c in data_b64.as_bytes() {
    let v: i32 = match c {
        b'A'..=b'Z' => (c - b'A') as i32,
        b'a'..=b'z' => (c - b'a') as i32 + 26,
        b'0'..=b'9' => (c - b'0') as i32 + 52,
        b'+' => 62,
        b'/' => 63,
        b'=' | b'\r' | b'\n' | b' ' | b'\t' => -1,
        _ => -2,
    };
    if v == -2 { return err("data_b64 is not valid base64"); }
    if v < 0 { continue; }
    acc = (acc << 6) | v as u32;
    nbits += 6;
    if nbits >= 8 {
        nbits -= 8;
        bytes.push((acc >> nbits) as u8);
    }
}
if bytes.is_empty() { return err("empty upload"); }

let dir = std::path::Path::new("runtime").join("agent").join("uploads");
if let Err(e) = std::fs::create_dir_all(&dir) {
    return err(&format!("cannot create {}: {}", dir.display(), e));
}
let ts = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_millis())
    .unwrap_or(0);
let target = dir.join(format!("{}-{}", ts, clean));
if let Err(e) = std::fs::write(&target, &bytes) {
    return err(&format!("write failed: {}", e));
}
let abs = std::fs::canonicalize(&target).unwrap_or(target.clone());

let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_string("path", abs.to_str().unwrap_or(""));
o.put_string("name", &clean);
o.put_int("size", bytes.len() as i64);
o
}
