// Set the value of the first element matching `selector` to `value` and
// fire input+change events (so page frameworks notice). value is true
// when the element was found. {status, value}. (Named `fill` because
// `type` is a Rust keyword the codegen cannot use for a module.)
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
let js = format!(
    "(function(){{var e=document.querySelector({});if(!e)return false;e.focus();e.value={};e.dispatchEvent(new Event('input',{{bubbles:true}}));e.dispatchEvent(new Event('change',{{bubbles:true}}));return true;}})()",
    js_string(&selector),
    js_string(&value)
);
crate::agent::browser::eval::eval(js, 5000)