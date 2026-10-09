use ndata::dataobject::DataObject;
use ndata::dataarray::DataArray;
use ndata::data::Data;
use flowlang::datastore::DataStore;
use flowlang::command::Command;
use std::collections::HashMap;

use crate::agent::llm::llm_common::*;
use crate::agent::llm::llm_openai::parse_openai;
use crate::agent::llm::llm_anthropic::parse_anthropic;
use crate::agent::llm::llm_gemini::parse_gemini;

pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["messages", "tools", "model", "temperature", "max_tokens", "keep_alive"] {
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
        let arg_5: String = o.get_string("keep_alive");
        llm_ollama(arg_0, arg_1, arg_2, arg_3, arg_4, arg_5)
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

pub fn llm_ollama(messages: DataArray, tools: DataArray, model: String, temperature: f64, max_tokens: i64, keep_alive: String) -> DataObject {
panic!("Someone just couldn't use correct ndata types so we have this fucked up hack.");
}

pub fn build_ollama_payload(messages: &DataArray, tools: &DataArray, model: &str,
                        temperature: f64, max_tokens: i64, keep_alive: &str) -> DataObject {
    let mut payload = DataObject::new();
    payload.put_string("model", model);

    let mut out = DataArray::new();
    for i in 0..messages.len() {
        let m = messages.get_object(i);
        let role = m.try_get_string("role").unwrap_or_default();
        let mut n = DataObject::new();
        n.put_string("role", &role);
        n.put_string("content", &m.try_get_string("content").unwrap_or_default());
        let imgs = message_images(&m);
        if !imgs.is_empty() {
            // Ollama's native form: bare base64 strings, no data-URL wrapper.
            let mut ia = DataArray::new();
            for (_mime, b64) in imgs { ia.push_string(&b64); }
            n.put_array("images", ia);
        }
        if role == "assistant" {
            if let Ok(calls) = m.try_get_array("tool_calls") {
                let mut oc = DataArray::new();
                for c in calls.objects() {
                    let c = c.object();
                    let f = c.get_object("function");
                    let mut nf = DataObject::new();
                    nf.put_string("name", &f.get_string("name"));
                    // Ollama wants arguments as an OBJECT, not a JSON string.
                    nf.put_object("arguments", args_to_object(&args_to_string(&f.get_property("arguments"))));
                    let mut w = DataObject::new();
                    w.put_object("function", nf);
                    oc.push_object(w);
                }
                if oc.len() > 0 { n.put_array("tool_calls", oc); }
            }
        }
        out.push_object(n);
    }
    payload.put_array("messages", out);
    payload.put_boolean("stream", false);
    // 0 unloads the model as soon as the call finishes — your original
    // /api/generate code chose that deliberately, and it matters on a shared
    // GPU. Set OLLAMA_KEEP_ALIVE=5m (or any duration Ollama accepts) to keep
    // it resident and pay the load cost only once.
    match keep_alive.parse::<i64>() {
        Ok(n) => payload.put_int("keep_alive", n),
        Err(_) => payload.put_string("keep_alive", keep_alive),
    }
    if tools.len() > 0 { payload.put_array("tools", tools.clone()); }

    let mut options = DataObject::new();
    options.put_float("temperature", temperature);
    options.put_int("num_predict", max_tokens);
    payload.put_object("options", options);
    payload
}

pub fn parse_ollama(root: &DataObject, arm: &str) -> Result<DataObject, DataObject> {
    if let Ok(e) = root.try_get_string("error") {
        return Err(err_out(&format!("{} error: {}", arm, e)));
    }
    if let Ok(message) = root.try_get_object("message") {
        let content = message.try_get_string("content").unwrap_or_default();
        if let Ok(calls) = message.try_get_array("tool_calls") {
            if calls.len() > 0 {
                let mut raw = Vec::new();
                for (i, c) in calls.objects().iter().enumerate() {
                    let c = c.object();
                    let f = c.get_object("function");
                    // Ollama emits no ids either.
                    raw.push((format!("call_{}", i), f.get_string("name"),
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
    Err(err_out(&format!("{}: unexpected response shape: {}", arm,
        root.to_string().chars().take(1200).collect::<String>())))
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

pub fn run(messages:&DataArray, tools:&DataArray, meta:&DataObject, arm:&str,
           url:&str, model:&str, headers:Vec<(String,String)>) -> DataObject {
    let temperature = opt(meta, "LLM_TEMPERATURE", "0.2").parse::<f64>().unwrap_or(0.2);
    let max_tokens = opt(meta, "LLM_MAX_TOKENS", "8192").parse::<i64>().unwrap_or(8192);
    let payload = build_ollama_payload(messages, tools, model, temperature, max_tokens,
                                     &opt(meta, "OLLAMA_KEEP_ALIVE", "0"));
    dispatch(messages, tools, arm, "ollama", url, payload, headers, parse_ollama)

}
