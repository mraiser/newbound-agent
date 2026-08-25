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