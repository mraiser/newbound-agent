// agent-llm-last_request - rebuild the most recent LLM request from the
// LLM_CAPTURE seam into one viewable object: arm/model/kind/timing, every
// message in order with full content (system prompt first), the assistant
// reply, and the tools array.
// Rows written before tools-capture carry only a tool COUNT; for those the
// viewer substitutes the current tool_loop definitions, LABELED, so the
// request is still inspectable whole. Rows after record tools_id ("tc"+
// FNV of the definitions), which resolves to the exact array that was sent.

fn err(msg: String) -> DataObject {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", &msg);
    o
}

// the same gate the seam and expand_clipped read
let cap_on = (|| -> bool {
    let g = DataStore::globals();
    let s = match g.try_get_object("system") { Ok(x) => x, _ => return false };
    let a = match s.try_get_object("apps") { Ok(x) => x, _ => return false };
    let ag = match a.try_get_object("agent") { Ok(x) => x, _ => return false };
    let r = match ag.try_get_object("runtime") { Ok(x) => x, _ => return false };
    r.try_get_string("LLM_CAPTURE").map(|v| v.trim().eq_ignore_ascii_case("on")).unwrap_or(false)
})();
if !cap_on {
    return err("LLM_CAPTURE is not on - no request is recorded. Set LLM_CAPTURE=on in runtime/agent/botd.properties and restart.".to_string());
}

let store = DataStore::new();
let root2 = match store.root.canonicalize().ok().and_then(|r| r.parent().map(|p| p.to_path_buf())) {
    Some(p) => p,
    None => return err("cannot resolve runtime root".to_string()),
};
let dir = root2.join("runtime").join("agent").join("model").join("capture");
if !dir.is_dir() {
    return err("no capture directory yet - no LLM turn has been recorded".to_string());
}
let mut files: Vec<std::path::PathBuf> = Vec::new();
if let Ok(rd) = std::fs::read_dir(&dir) {
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().map(|x| x == "jsonl").unwrap_or(false) { files.push(p); }
    }
}
files.sort();
if files.is_empty() {
    return err("capture directory is empty - no LLM turn has been recorded".to_string());
}
let file = files.last().unwrap().clone();
let text = match std::fs::read_to_string(&file) { Ok(t) => t, _ => return err("cannot read capture file".to_string()) };
let last = match text.lines().filter(|l| !l.trim().is_empty()).last() {
    Some(l) => l.to_string(),
    None => return err("capture file has no rows".to_string()),
};
let row = DataObject::from_string(&format!("{{\"a\":{}}}", last)).get_object("a");

let mut out = DataObject::new();
out.put_string("status", "ok");
out.put_string("captured_at_ms", &row.try_get_int("t").map(|v| v.to_string()).unwrap_or_default());
out.put_string("capture_file", &file.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
out.put_string("arm", &row.try_get_string("arm").unwrap_or_default());
out.put_string("model", &row.try_get_string("model").unwrap_or_default());
out.put_string("kind", &row.try_get_string("kind").unwrap_or_default());
if let Ok(c) = row.try_get_float("cost_usd") { out.put_float("cost_usd", c); }

// messages, in request order; [0] is the system prompt on a well-formed request
let mut msgs = DataArray::new();
if let Ok(ids) = row.try_get_array("msg_ids") {
    for i in 0..ids.len() {
        let oid = ids.get_string(i);
        let mut m = DataObject::new();
        m.put_string("id", &oid);
        let od = store.get_data("runtime", &oid).get_object("data");
        let role = od.try_get_string("role").unwrap_or_default();
        let cid = od.try_get_string("content_id").unwrap_or_default();
        m.put_string("role", &role);
        m.put_string("content_id", &cid);
        let mut content = String::new();
        if !cid.is_empty() {
            content = store.get_data("runtime", &cid).get_object("data")
                .try_get_string("text").unwrap_or_default();
        }
        m.put_int("content_chars", content.len() as i64);
        m.put_string("content", &content);
        msgs.push_object(m);
    }
}
out.put_array("messages", msgs);

// the assistant reply that came back on this turn
let reply_id = row.try_get_string("reply_id").unwrap_or_default();
if !reply_id.is_empty() {
    let rd = store.get_data("runtime", &reply_id).get_object("data");
    let cid = rd.try_get_string("content_id").unwrap_or_default();
    let content = if cid.is_empty() { String::new() } else {
        store.get_data("runtime", &cid).get_object("data")
            .try_get_string("text").unwrap_or_default()
    };
    let mut r = DataObject::new();
    r.put_string("id", &reply_id);
    r.put_string("content", &content);
    out.put_object("reply", r);
}

// tools: exact definitions where the row captured them (tools_id), else the
// current MCP list converted to the OpenAI wire shape as a labeled stand-in
out.put_int("tools_count", row.try_get_int("tools").unwrap_or(0));
let tools_id = row.try_get_string("tools_id").unwrap_or_default();
let mut have_exact = false;
if !tools_id.is_empty() {
    let td = store.get_data("runtime", &tools_id).get_object("data");
    if let Ok(tarr) = td.try_get_array("tools") {
        out.put_string("tools_from", "capture");
        out.put_array("tools", tarr);
        have_exact = true;
    }
}
if !have_exact {
    let lt = crate::agent::plugin::list_tools::list_tools();
    if let Ok(t) = lt.try_get_array("tools") {
        // MCP shape ({name, description, inputSchema}) -> OpenAI function shape
        let mut fns = DataArray::new();
        for i in 0..t.len() {
            let tool = t.get_object(i);
            let mut f = DataObject::new();
            f.put_string("type", "function");
            let mut fnobj = DataObject::new();
            fnobj.put_string("name", &tool.try_get_string("name").unwrap_or_default().replace('_', "-"));
            fnobj.put_string("description", &tool.try_get_string("description").unwrap_or_default());
            if let Ok(schema) = tool.try_get_object("inputSchema") { fnobj.put_object("parameters", schema); }
            f.put_object("function", fnobj);
            fns.push_object(f);
        }
        out.put_string("tools_from", "current MCP list (row predates tools capture)");
        out.put_array("tools", fns);
    } else {
        out.put_string("tools_from", "not captured on this row");
        out.put_array("tools", DataArray::new());
    }
}

out