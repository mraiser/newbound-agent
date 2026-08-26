// Poll until an element matching `selector` exists (or timeout). This is
// the stable primitive the run-ui skill's gotchas call for: wait on a
// real selector, never a fixed sleep. Returns {status, found}.
fn js_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
let tmo: i64 = if timeout_ms > 0 { timeout_ms } else { 10000 };
let js = format!("!!document.querySelector({})", js_string(&selector));
let deadline = time() + tmo;
let mut found = false;
while time() < deadline {
    let r = crate::agent::browser::eval::eval(js.clone(), 2000);
    if r.try_get_string("status").map(|s| s == "ok").unwrap_or(false) {
        if r.try_get_boolean("value").unwrap_or(false) { found = true; break; }
    }
    std::thread::sleep(std::time::Duration::from_millis(200));
}
let mut o = DataObject::new();
o.put_string("status", if found { "ok" } else { "err" });
o.put_boolean("found", found);
if !found { o.put_string("msg", &format!("selector not found within {} ms: {}", tmo, selector)); }
o