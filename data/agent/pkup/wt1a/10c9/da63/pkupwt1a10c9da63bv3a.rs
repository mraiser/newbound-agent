// Search the LIVE browser page's visible text for `query` via one
// agent.browser.eval. Walks leaf elements, matches case-insensitively,
// and returns up to 50 hits as {el, text} plus a total count.
// Requires a running Noobscape-v2 browser (see agent.browser.open).
//
// Encode a Rust string as a JS string literal (for embedding `query` in eval JS).
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

if query.is_empty() {
    let mut e = DataObject::new();
    e.put_string("status", "err");
    e.put_string("msg", "find_text needs `query`");
    return e;
}

let jq = js_string(&query);
let js = format!(
    "(function(){{var q={jq}.toLowerCase();var out=[];var all=document.querySelectorAll('*');for(var i=0;i<all.length;i++){{var e=all[i];if(e.children.length)continue;var t=(e.textContent||'').replace(/\\s+/g,' ').trim();if(t&&t.toLowerCase().indexOf(q)>=0){{var d=e.tagName.toLowerCase();var id=e.id?'#'+e.id:'';var cn=(''+(e.className||'')).trim().split(/\\s+/).filter(Boolean).slice(0,2).map(function(x){{return '.'+x;}}).join('');out.push({{el:d+id+cn,text:t.slice(0,200)}});}}}}return {{count:out.length,matches:out.slice(0,50)}};}})()",
    jq = jq
);
crate::api::new().agent.browser.eval(js, 15000)