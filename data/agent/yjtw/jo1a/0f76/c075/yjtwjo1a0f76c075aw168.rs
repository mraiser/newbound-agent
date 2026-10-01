// Shared helpers for every LLM provider arm. Extracted verbatim from agent.llm.chat_llm so
// each arm command does `use crate::agent::llm::llm_common::*;` instead of duplicating them.
// Internal support command - the public entry point for LLM work is agent.llm.chat_llm.
pub fn llm_common() -> DataObject {
    err_out("llm_common is a shared helper module for the LLM arms, not an entry point - call agent.llm.chat_llm")
}

pub fn err_out(msg: &str) -> DataObject {
    let mut o = DataObject::new();
    o.put_string("kind", "error");
    o.put_string("content", msg);
    o
}
pub fn assistant_msg(content: &str, replay: Option<DataArray>) -> DataObject {
    let mut a = DataObject::new();
    a.put_string("role", "assistant");
    a.put_string("content", content);
    if let Some(r) = replay { a.put_array("tool_calls", r); }
    a
}
pub fn text_result(msg: &str) -> DataObject {
    let mut o = DataObject::new();
    o.put_string("kind", "text");
    o.put_string("content", msg);
    o.put_object("assistant_message", assistant_msg(msg, None));
    o
}
pub fn need(meta: &DataObject, key: &str, arm: &str) -> Result<String, String> {
    match meta.try_get_string(key) {
        Ok(v) if !v.trim().is_empty() => Ok(v.trim().to_string()),
        _ => Err(format!("LLM arm {} needs {} - set it in runtime/agent/botd.properties and restart. (LLM= selects VLLM | OPENAI | ANTHROPIC | GEMINI | OLLAMA | LOCAL | REMOTE; REMOTE uses LLM_REMOTE=<peer-uuid>; any other value uses LLM_CTL=lib:ctl:cmd.)", arm, key)),
    }
}
pub fn opt(meta: &DataObject, key: &str, default: &str) -> String {
    match meta.try_get_string(key) {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => default.to_string(),
    }
}
// <ARM>_HEADERS: newline- or comma-separated `Name: value` pairs, applied
// AFTER the arm's own auth header so an explicit entry always wins. This is
// the escape hatch for every scheme not special-cased here — Azure OpenAI
// wants `api-key: <key>` rather than Bearer (plus ?api-version= on the URL),
// OpenRouter likes HTTP-Referer/X-Title. Config, not a code change.
// The key may be spelled <ARM>_KEY or <ARM>_API_KEY. Older configs used
// _API_KEY, and there is no reason to make anyone edit a working file.
pub fn need_key(meta: &DataObject, arm: &str) -> Result<String, String> {
    for k in [format!("{}_KEY", arm), format!("{}_API_KEY", arm)] {
        if let Ok(v) = meta.try_get_string(&k) {
            if !v.trim().is_empty() { return Ok(v.trim().to_string()); }
        }
    }
    Err(format!("LLM arm {} needs {}_KEY (or {}_API_KEY) - set it in runtime/agent/botd.properties and restart. (LLM= selects VLLM | OPENAI | ANTHROPIC | GEMINI | OLLAMA; any other value uses LLM_CTL=lib:ctl:cmd.)", arm, arm, arm))
}
pub fn opt_key(meta: &DataObject, arm: &str) -> String {
    need_key(meta, arm).unwrap_or_default()
}
pub fn extra_headers(meta: &DataObject, arm: &str) -> Vec<(String, String)> {
    let raw = opt(meta, &format!("{}_HEADERS", arm), "");
    let mut out: Vec<(String, String)> = Vec::new();
    for line in raw.split(|c| c == '\n' || c == ',') {
        let line = line.trim();
        if line.is_empty() { continue; }
        if let Some(i) = line.find(':') {
            let k = line[..i].trim().to_string();
            let v = line[i + 1..].trim().to_string();
            if !k.is_empty() { out.push((k, v)); }
        }
    }
    out
}
// OpenAI puts tool-call arguments in a JSON *string*; Gemini and Ollama use
// an object. Normalize to the string form the callers already parse.
pub fn args_to_string(d: &Data) -> String {
    if d.is_string() { d.string() }
    else if d.is_object() { d.object().to_string() }
    else { "{}".to_string() }
}
// ── vision: a message may carry an `images` array of image FILE PATHS (the
// chat's runtime/agent/uploads). content stays a plain string - each dialect
// builder below renders the same base64 into its provider's own envelope, so
// callers stay dialect-agnostic. Arms with no image form (LOCAL, custom
// LLM_CTL, CLAUDECODE) simply never read the key: the delegate gets the path.
pub fn b64_encode(bytes: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
        out.push(CHARS[(b0 >> 2) & 0x3f] as char);
        out.push(CHARS[((b0 << 4) | (b1 >> 4)) & 0x3f] as char);
        if chunk.len() > 1 { out.push(CHARS[((b1 << 2) | (b2 >> 6)) & 0x3f] as char); } else { out.push('='); }
        if chunk.len() > 2 { out.push(CHARS[b2 & 0x3f] as char); } else { out.push('='); }
    }
    out
}
// path -> (media_type, base64). None = unreadable, not a known image type,
// or over 20MB (the chat upload's own cap): skipped rather than sent broken.
pub fn image_b64(path: &str) -> Option<(String, String)> {
    let mime = match path.rsplit('.').next().map(|e| e.to_ascii_lowercase()).unwrap_or_default().as_str() {
        "png" => "image/png", "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif", "webp" => "image/webp",
        _ => return None,
    };
    let bytes = std::fs::read(path).ok()?;
    if bytes.is_empty() || bytes.len() > 20 * 1024 * 1024 { return None; }
    Some((mime.to_string(), b64_encode(&bytes)))
}
pub fn message_images(m: &DataObject) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Ok(imgs) = m.try_get_array("images") {
        for d in imgs.objects() {
            if d.is_string() {
                if let Some(pair) = image_b64(&d.string()) { out.push(pair); }
            }
        }
    }
    out
}
// ndata's try_from_string is NOT panic-safe. It returns Err only for
// MALFORMED json; for well-formed json that is not an OBJECT — an array, a
// string, a number, a bool, null — it reaches DataObject::from_json, whose
// `.expect("DataObject::from_json requires a JSON object Value")` PANICS.
// A tool answering with a JSON ARRAY (search_commands does) therefore took
// down the whole chat_llm call. Wrapping in {"a": ...} before parsing makes
// it total, and is already the house idiom (see dev.code.remember).
pub fn parse_json(s: &str) -> Option<Data> {
    DataObject::try_from_string(&format!("{{\"a\":{}}}", s))
        .ok().map(|w| w.get_property("a"))
}
pub fn obj_from_str(s: &str) -> Option<DataObject> {
    match parse_json(s) {
        Some(d) if d.is_object() => Some(d.object()),
        _ => None,
    }
}
pub fn args_to_object(s: &str) -> DataObject {
    obj_from_str(s).unwrap_or_else(DataObject::new)
}