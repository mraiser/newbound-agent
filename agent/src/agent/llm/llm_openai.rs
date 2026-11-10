use ndata::dataobject::DataObject;
use ndata::dataarray::DataArray;
use ndata::data::Data;
use flowlang::datastore::DataStore;
use flowlang::command::Command;
use std::collections::HashMap;

use crate::agent::llm::llm_common::*;
use crate::agent::llm::llm_ollama::parse_ollama;
use crate::agent::llm::llm_anthropic::parse_anthropic;
use crate::agent::llm::llm_gemini::parse_gemini;

pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["messages", "tools", "model", "temperature", "max_tokens", "arm"] {
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
        let arg_0: DataArray = o.get_array("messages");
        let arg_1: DataArray = o.get_array("tools");
        let arg_2: String = o.get_string("model");
        let arg_3: f64 = o.get_float("temperature");
        let arg_4: i64 = o.get_int("max_tokens");
        let arg_5: String = o.get_string("arm");
        llm_openai(arg_0, arg_1, arg_2, arg_3, arg_4, arg_5)
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

pub fn llm_openai(messages: DataArray, tools: DataArray, model: String, temperature: f64, max_tokens: i64, arm: String) -> DataObject {
panic!("Someone just couldn't use correct ndata types so we have this fucked up hack.");
}

pub fn run(messages:&DataArray, tools:&DataArray, meta:&DataObject, arm:&str,
           url:&str, model:&str, headers:Vec<(String,String)>) -> DataObject {
    let temperature = opt(meta, "LLM_TEMPERATURE", "0.2").parse::<f64>().unwrap_or(0.2);
    let max_tokens = opt(meta, "LLM_MAX_TOKENS", "8192").parse::<i64>().unwrap_or(8192);
    let payload = build_openai_payload(messages, tools, model, temperature, max_tokens, arm);
    dispatch(messages, tools, arm, "openai", url, payload, headers, parse_openai)
}

pub fn build_openai_payload(messages: &DataArray, tools: &DataArray, model: &str, temperature: f64, max_tokens: i64, arm: &str) -> DataObject {
    let mut p = DataObject::new();
        p.put_string("model", &model);
        // Strict OpenAI-compatible servers (KIMI K3 above all) 400 with
        // "tool_call_id  is not found" on an id their validator dislikes: a
        // tool message with no id at all, an id that is empty or has
        // whitespace, or a tool message no live assistant tool_call claims.
        // The parse side already synthesizes ids for blank RETURNS; this is
        // the send side scrubbing what the conversation REPLAYS. The repair
        // keeps the answer: an id-less or orphan tool message becomes a plain
        // user message; a whitespace id is trimmed; a claimed-but-whitespace
        // id is renamed consistently on BOTH the tool message and its
        // assistant tool_call so the pair still matches. Runs before the
        // image pass so both rebuilt and verbatim messages get scrubbed.
        fn clean_tool_ids(msgs: &mut Vec<DataObject>) {
            let clean = |s: &str| s.split_whitespace().collect::<Vec<_>>().join("");
            // Pass 1: scrub whitespace out of ids on the assistant tool_calls
            // THEMSELVES, so `claimed` is already clean and the tool side
            // never needs a cross-message rename (a tool message carries no
            // tool_calls array - the id it echoes lives on the assistant).
            let mut claimed: Vec<String> = Vec::new();
            for m in msgs.iter_mut() {
                if m.try_get_string("role").unwrap_or_default() != "assistant" { continue; }
                if let Ok(tcs) = m.try_get_array("tool_calls") {
                    for tc in tcs.objects() {
                        let mut tc = tc.object();
                        if let Ok(id) = tc.try_get_string("id") {
                            let cid = clean(&id);
                            if !cid.is_empty() {
                                if cid != id { tc.put_string("id", &cid); }
                                claimed.push(cid);
                            }
                        }
                    }
                }
            }
            // Pass 2: every tool message must echo an id a live assistant
            // tool_call claims, exactly. A missing or blank id borrows the
            // nth claimed id (tool_loop emits answers in the order the calls
            // were made); a tool message no call claims becomes a plain user
            // message - the answer is KEPT and only its tool framing dropped,
            // rather than 400ing the turn.
            let mut n = 0usize;
            for m in msgs.iter_mut() {
                if m.try_get_string("role").unwrap_or_default() != "tool" { continue; }
                match m.try_get_string("tool_call_id") {
                    Ok(id) if !clean(&id).is_empty() => {
                        let cid = clean(&id);
                        if cid != id { m.put_string("tool_call_id", &cid); }
                        if !claimed.contains(&cid) { m.put_string("role", "user"); }
                    }
                    _ => {
                        match claimed.get(n) {
                            Some(cid) => { m.put_string("tool_call_id", cid); }
                            None => { m.put_string("role", "user"); }
                        }
                    }
                }
                n += 1;
            }
        }
        // Copied, not verbatim: a message carrying `images` becomes an OpenAI
        // content-parts array (text + data-URL image_url entries). The
        // caller's array is shared, and a builder has no business mutating it.
        let mut oai_msgs = DataArray::new();
        for i in 0..messages.len() {
            let m = messages.get_object(i);
            let imgs = message_images(&m);
            if imgs.is_empty() { oai_msgs.push_object(m); continue; }
            let mut n = DataObject::new();
            for (k, v) in m.objects() {
                if k != "images" && k != "content" { n.set_property(&k, v.clone()); }
            }
            let mut parts = DataArray::new();
            let text = m.try_get_string("content").unwrap_or_default();
            if !text.is_empty() {
                let mut t = DataObject::new();
                t.put_string("type", "text");
                t.put_string("text", &text);
                parts.push_object(t);
            }
            for (mime, b64) in imgs {
                let mut u = DataObject::new();
                u.put_string("url", &format!("data:{};base64,{}", mime, b64));
                let mut ip = DataObject::new();
                ip.put_string("type", "image_url");
                ip.put_object("image_url", u);
                parts.push_object(ip);
            }
            n.put_array("content", parts);
            oai_msgs.push_object(n);
        }
        let mut scrub: Vec<DataObject> = Vec::new();
        for m in oai_msgs.objects() { scrub.push(m.object()); }
        clean_tool_ids(&mut scrub);
        let mut cleaned = DataArray::new();
        for m in scrub { cleaned.push_object(m); }
        p.put_array("messages", cleaned);
        if tools.len() > 0 {
            p.put_array("tools", tools.clone());
            p.put_string("tool_choice", "auto");
        }
        // vLLM-only: no thinking for tool work. Other OpenAI-compatible
        // servers reject or ignore unknown fields, so this is gated on the arm.
        if arm == "VLLM" {
            let mut chat_kwargs = DataObject::new();
            chat_kwargs.put_boolean("enable_thinking", false);
            p.put_object("chat_template_kwargs", chat_kwargs);
        }
        p.put_float("temperature", temperature);
        p.put_int("max_tokens", max_tokens);
        p
}

// name + arguments-as-string pairs -> (normalized calls, replay tool_calls)
pub fn pack_calls(raw: Vec<(String, String, String, String)>) -> (DataArray, DataArray) {
    let mut norm = DataArray::new();
    let mut replay = DataArray::new();
    for (id, name, args, sig) in raw {
        let mut n = DataObject::new();
        n.put_string("id", &id);
        n.put_string("name", &name);
        n.put_string("arguments", &args);
        norm.push_object(n);

        let mut rf = DataObject::new();
        rf.put_string("name", &name);
        rf.put_string("arguments", &args);
        let mut rc = DataObject::new();
        rc.put_string("id", &id);
        rc.put_string("type", "function");
        rc.put_object("function", rf);
        // Gemini 3 returns a thoughtSignature on the part carrying a
        // functionCall and REQUIRES it back when that turn is replayed
        // ("Function call is missing a thought_signature in functionCall
        // parts", INVALID_ARGUMENT). It rides the replay object so the
        // conversation the caller keeps carries it; only the gemini payload
        // builder reads it back out, and only a gemini conversation ever
        // has one.
        if !sig.is_empty() { rc.put_string("thought_signature", &sig); }
        replay.push_object(rc);
    }
    (norm, replay)
}

pub fn parse_openai(root: &DataObject, arm: &str) -> Result<DataObject, DataObject> {
    if let Ok(choices) = root.try_get_array("choices") {
        if choices.len() > 0 {
            let choice = choices.get_object(0);
            if let Ok(message) = choice.try_get_object("message") {
                let content = message.try_get_string("content").unwrap_or_default();
                if let Ok(calls) = message.try_get_array("tool_calls") {
                    if calls.len() > 0 {
                        let mut raw = Vec::new();
                        for (i, c) in calls.objects().iter().enumerate() {
                            let c = c.object();
                            let f = c.get_object("function");
                            // Kimi intermittently returns "id": "" - blank is absent, or the
                            // echoed tool_call_id "" 400s on the next request. Millis seed keeps
                            // synthesized ids unique across turns for strict validators.
                            let id = match c.try_get_string("id") {
                                Ok(s) if !s.trim().is_empty() => s,
                                _ => format!("call_{}_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(), i),
                            };
                            raw.push((id, f.get_string("name"),
                                      args_to_string(&f.get_property("arguments")),
                                      String::new()));
                        }
                        let (norm, replay) = pack_calls(raw);
                        let mut out = DataObject::new();
                        out.put_string("kind", "tool_calls");
                        out.put_array("tool_calls", norm);
                        out.put_object("assistant_message", assistant_msg(&content, Some(replay)));
                        return Ok(out);
                    }
                }
                if message.has("content") { return Ok(text_result(&content)); }
            }
            // A length-capped or filtered choice can carry no message at all.
            if let Ok(fr) = choice.try_get_string("finish_reason") {
                return Err(err_out(&format!("{} returned no content (finish_reason: {})", arm, fr)));
            }
        }
    }
    if let Ok(e) = root.try_get_object("error") {
        return Err(err_out(&format!("{} error: {}", arm,
            e.try_get_string("message").unwrap_or_else(|_| e.to_string()))));
    }
    Err(err_out(&format!("{}: unexpected response shape: {}", arm,
        root.to_string().chars().take(1200).collect::<String>())))
}

fn dispatch(messages:&DataArray, tools:&DataArray, arm:&str, dialect:&str, url:&str,
            payload:DataObject, headers:Vec<(String,String)>,
            parse:fn(&DataObject,&str)->Result<DataObject,DataObject>) -> DataObject {
let http = ureq::AgentBuilder::new()
    .timeout_read(std::time::Duration::from_secs(600))
    .timeout_write(std::time::Duration::from_secs(600))
    .build();

println!("chat_llm[{}/{}]: {} messages, {} tools", arm, dialect, messages.len(), tools.len());

let mut out = err_out(&format!("{}: no response", arm));
let attempts = 4u32;
for attempt in 0..attempts {
    let mut req = http.post(&url);
    for (k, v) in &headers { req = req.set(k, v); }

    // RETRY ONLY WHAT IS RETRYABLE: transport, 408, 429, 5xx. A 400/401/403/404
    // is a configuration answer, and retrying it only delays the report — the
    // vLLM "System message must be at the beginning." 400 used to burn five
    // attempts and 31s of sleeps before surfacing.
    #[cfg(feature = "serde_support")]
    let rawres = req.send_json(payload.to_json());
    #[cfg(not(feature = "serde_support"))]
    let rawres = req.send_string(&payload.to_string());
    let (retryable, retry_after) = match rawres {
        Ok(resp) => {
            match resp.into_string() {
                Ok(body) => {
                    match obj_from_str(&body) {
                        Some(root) => match match dialect {
                            "anthropic" => parse_anthropic(&root, &arm),
                            "gemini" => parse_gemini(&root, &arm),
                            "ollama" => parse_ollama(&root, &arm),
                            _ => parse_openai(&root, &arm),
                        } {
                            Ok(good) => return good,
                            Err(e) => { out = e; (false, 0u64) }
                        },
                        None => {
                            out = err_out(&format!("{}: response was not a JSON object: {}", arm,
                                body.chars().take(600).collect::<String>()));
                            (false, 0u64)
                        }
                    }
                },
                Err(e) => {
                    out = err_out(&format!("{}: reading the response body failed: {}", arm, e));
                    (true, 0u64)
                }
            }
        },
        Err(ureq::Error::Status(code, resp)) => {
            let ra = resp.header("Retry-After")
                .and_then(|s| s.trim().parse::<u64>().ok()).unwrap_or(0).min(60);
            let body = resp.into_string().unwrap_or_default();
            out = err_out(&format!("{} API status {}: {}", arm, code,
                body.chars().take(1200).collect::<String>()));
            (code == 408 || code == 429 || code >= 500, ra)
        },
        Err(ureq::Error::Transport(e)) => {
            out = err_out(&format!("{}: network error to {}: {}", arm, url, e));
            (true, 0u64)
        }
    };

    if !retryable || attempt + 1 == attempts { break; }
    let backoff = if retry_after > 0 { retry_after } else { 2u64.pow(attempt) };
    println!("chat_llm[{}] attempt {} failed, retrying in {}s: {}",
             arm, attempt + 1, backoff, out.get_string("content"));
    std::thread::sleep(std::time::Duration::from_secs(backoff));
}
out

}
