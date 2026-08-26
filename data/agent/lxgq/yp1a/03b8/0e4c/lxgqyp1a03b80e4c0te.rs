// Return the textContent of the first element matching `selector`
// (value is null when nothing matches). {status, value} from eval.
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
    "(function(){{var e=document.querySelector({});return e?e.textContent:null;}})()",
    js_string(&selector)
);
crate::agent::browser::eval::eval(js, 5000)