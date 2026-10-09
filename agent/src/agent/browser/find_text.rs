use ndata::dataobject::DataObject;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["query"] {
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
        let arg_0: String = o.get_string("query");
        find_text(arg_0)
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

pub fn find_text(query: String) -> DataObject {
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
}
