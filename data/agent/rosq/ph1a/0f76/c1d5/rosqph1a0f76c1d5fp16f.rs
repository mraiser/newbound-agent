panic!("Someone just couldn't use correct ndata types so we have this fucked up hack.");
}

// defines, recursively.
pub fn gemini_schema(o: DataObject) -> DataObject {
    let mut out = DataObject::new();
    for (k, v) in o.objects() {
        match k.as_str() {
            "type" | "format" | "description" | "nullable" | "enum" => out.set_property(&k, v.clone()),
            "required" => out.set_property(&k, v.clone()),
            "items" => { if v.is_object() { out.put_object("items", gemini_schema(v.object())); } },
            "properties" => {
                if v.is_object() {
                    let mut props = DataObject::new();
                    for (pk, pv) in v.object().objects() {
                        if pv.is_object() { props.put_object(&pk, gemini_schema(pv.object())); }
                    }
                    out.put_object("properties", props);
                }
            },
            _ => {}   // $schema, additionalProperties, examples, ... dropped
        }
    }
    if !out.has("type") { out.put_string("type", "object"); }
    // Gemini REJECTS an array with no `items` ("...properties[params].items:
    // missing field", INVALID_ARGUMENT) where every OpenAI-compatible server
    // tolerates it. There is nothing to derive the element type FROM:
    // newbound's declared param types are just `JSONArray`, which
    // flowlang's describe.rs maps to a bare {"type":"array"}.
    // `object` is right for the arrays the agent actually uses
    // (upsert_command.params, chat_llm messages/tools) and wrong for the
    // string lists on app.app.newlib / security.setuser
    // (readers/writers/groups) - the per-tool description still documents
    // those, and they are not on the agent's hot path. The real fix is
    // element types in the platform's param declarations; that is a schema
    // change to every command record and the owner's call.
    let is_array = matches!(out.try_get_string("type"), Ok(ref t) if t == "array");
    if is_array && !out.has("items") {
        let mut it = DataObject::new();
        it.put_string("type", "object");
        out.put_object("items", it);
    }
    out
}

pub fn build_gemini_payload(messages: &DataArray, tools: &DataArray,
                        temperature: f64, max_tokens: i64) -> DataObject {
    let mut payload = DataObject::new();
    let mut contents = DataArray::new();
    let mut system = String::new();
    // Gemini's functionResponse is keyed by the function NAME, not by a call
    // id (it has no call ids at all — we synthesize them on the way out), so
    // walking forward we remember which id belonged to which name.
    let mut id_names: HashMap<String, String> = HashMap::new();

    for i in 0..messages.len() {
        let m = messages.get_object(i);
        let role = m.try_get_string("role").unwrap_or_default();
        let content = m.try_get_string("content").unwrap_or_default();

        if role == "system" {
            if !system.is_empty() { system.push_str("\n\n"); }
            system.push_str(&content);
            continue;
        }
        if role == "tool" {
            let id = m.try_get_string("tool_call_id").unwrap_or_default();
            let name = id_names.get(&id).cloned().unwrap_or_else(|| "tool".to_string());
            // response must be an OBJECT. A tool answering with an ARRAY
            // (search_commands) or with plain text gets WRAPPED rather than
            // dropped — and an array keeps its STRUCTURE instead of being
            // flattened to a string, so the model can still read the rows.
            let inner = match parse_json(&content) {
                Some(d) if d.is_object() => d.object(),
                Some(d) => { let mut w = DataObject::new(); w.set_property("result", d); w },
                None => { let mut w = DataObject::new(); w.put_string("result", &content); w }
            };
            let mut fr = DataObject::new();
            fr.put_string("name", &name);
            fr.put_object("response", inner);
            let mut part = DataObject::new();
            part.put_object("functionResponse", fr);
            let mut parts = DataArray::new();
            parts.push_object(part);
            let mut c = DataObject::new();
            c.put_string("role", "user");
            c.put_array("parts", parts);
            contents.push_object(c);
            continue;
        }

        let mut parts = DataArray::new();
        if !content.is_empty() {
            let mut p = DataObject::new();
            p.put_string("text", &content);
            if role == "assistant" {
                if let Ok(s) = m.try_get_string("thought_signature") {
                    if !s.is_empty() { p.put_string("thoughtSignature", &s); }
                }
            }
            parts.push_object(p);
        }
        for (mime, b64) in message_images(&m) {
            let mut blob = DataObject::new();
            blob.put_string("mime_type", &mime);
            blob.put_string("data", &b64);
            let mut part = DataObject::new();
            part.put_object("inline_data", blob);
            parts.push_object(part);
        }
        if role == "assistant" {
            if let Ok(calls) = m.try_get_array("tool_calls") {
                for c in calls.objects() {
                    let c = c.object();
                    let f = c.get_object("function");
                    let name = f.get_string("name");
                    if c.has("id") { id_names.insert(c.get_string("id"), name.clone()); }
                    let mut fc = DataObject::new();
                    fc.put_string("name", &name);
                    fc.put_object("args", args_to_object(&args_to_string(&f.get_property("arguments"))));
                    let mut p = DataObject::new();
                    p.put_object("functionCall", fc);
                    // Echo the signature Gemini gave us for THIS call, or it
                    // rejects the replayed turn outright.
                    if let Ok(s) = c.try_get_string("thought_signature") {
                        if !s.is_empty() { p.put_string("thoughtSignature", &s); }
                    }
                    parts.push_object(p);
                }
            }
        }
        if parts.len() == 0 { continue; }
        let mut c = DataObject::new();
        c.put_string("role", if role == "assistant" { "model" } else { "user" });
        c.put_array("parts", parts);
        contents.push_object(c);
    }

    if !system.is_empty() {
        let mut t = DataObject::new();
        t.put_string("text", &system);
        let mut parts = DataArray::new();
        parts.push_object(t);
        let mut si = DataObject::new();
        si.put_array("parts", parts);
        payload.put_object("system_instruction", si);
    }
    payload.put_array("contents", contents);

    if tools.len() > 0 {
        let mut decls = DataArray::new();
        for t in tools.objects() {
            let t = t.object();
            let f = if t.has("function") { t.get_object("function") } else { t.clone() };
            if !f.has("name") { continue; }
            let mut d = DataObject::new();
            d.put_string("name", &f.get_string("name"));
            if f.has("description") { d.put_string("description", &f.get_string("description")); }
            if let Ok(p) = f.try_get_object("parameters") { d.put_object("parameters", gemini_schema(p)); }
            decls.push_object(d);
        }
        let mut tool = DataObject::new();
        tool.put_array("functionDeclarations", decls);
        let mut ta = DataArray::new();
        ta.push_object(tool);
        payload.put_array("tools", ta);
    }

    let mut gen = DataObject::new();
    gen.put_float("temperature", temperature);
    gen.put_int("maxOutputTokens", max_tokens);
    payload.put_object("generationConfig", gen);

    // The reason this dialect exists. Without it Gemini's defaults block a
    // working agent's ordinary traffic and the compat layer gives no way to
    // say otherwise.
    let mut safety = DataArray::new();
    for cat in ["HARM_CATEGORY_HARASSMENT", "HARM_CATEGORY_HATE_SPEECH",
                "HARM_CATEGORY_SEXUALLY_EXPLICIT", "HARM_CATEGORY_DANGEROUS_CONTENT"] {
        let mut s = DataObject::new();
        s.put_string("category", cat);
        s.put_string("threshold", "BLOCK_NONE");
        safety.push_object(s);
    }
    payload.put_array("safetySettings", safety);
    payload
}

pub fn parse_gemini(root: &DataObject, arm: &str) -> Result<DataObject, DataObject> {
    // Prompt-level block: no candidates at all.
    if let Ok(fb) = root.try_get_object("promptFeedback") {
        if let Ok(reason) = fb.try_get_string("blockReason") {
            return Err(err_out(&format!("{} blocked the PROMPT ({}). safetySettings are already BLOCK_NONE, so this is a non-overridable category.", arm, reason)));
        }
    }
    if let Ok(e) = root.try_get_object("error") {
        return Err(err_out(&format!("{} error: {}", arm,
            e.try_get_string("message").unwrap_or_else(|_| e.to_string()))));
    }
    if let Ok(candidates) = root.try_get_array("candidates") {
        if candidates.len() > 0 {
            let cand = candidates.get_object(0);
            let mut text = String::new();
            let mut text_sig = String::new();
            let mut raw = Vec::new();
            if let Ok(content) = cand.try_get_object("content") {
                if let Ok(parts) = content.try_get_array("parts") {
                    for (i, p) in parts.objects().iter().enumerate() {
                        let p = p.object();
                        // A thinking part is not the answer; it carries
                        // thought:true and must not be concatenated into it.
                        let is_thought = matches!(p.try_get_boolean("thought"), Ok(true));
                        // JSON is camelCase (thoughtSignature); the error text
                        // uses the proto spelling. Accept both.
                        let sig = p.try_get_string("thoughtSignature")
                            .or_else(|_| p.try_get_string("thought_signature"))
                            .unwrap_or_default();
                        if let Ok(t) = p.try_get_string("text") {
                            if !is_thought { text.push_str(&t); }
                            if !sig.is_empty() && text_sig.is_empty() { text_sig = sig.clone(); }
                        }
                        if let Ok(fc) = p.try_get_object("functionCall") {
                            let args = match fc.try_get_object("args") {
                                Ok(a) => a.to_string(), _ => "{}".to_string() };
                            // Gemini has no call ids; synthesize stable ones so
                            // the tool_call_id round trip works.
                            raw.push((format!("call_{}", i), fc.get_string("name"), args, sig));
                        }
                    }
                }
            }
            if !raw.is_empty() {
                let (norm, replay) = pack_calls(raw);
                let mut out = DataObject::new();
                out.put_string("kind", "tool_calls");
                out.put_array("tool_calls", norm);
                out.put_object("assistant_message", assistant_msg(&text, Some(replay)));
                return Ok(out);
            }
            if !text.is_empty() {
                // not `mut`: the signature is written through the handle
                // get_object returns, not through `out` itself.
                let out = text_result(&text);
                if !text_sig.is_empty() {
                    let mut am = out.get_object("assistant_message");
                    am.put_string("thought_signature", &text_sig);
                }
                return Ok(out);
            }
            // Empty candidate: say WHY instead of "unexpected JSON".
            if let Ok(reason) = cand.try_get_string("finishReason") {
                return Err(err_out(&format!("{} returned no content (finishReason: {}). SAFETY here means a category that BLOCK_NONE does not cover; MAX_TOKENS means raise LLM_MAX_TOKENS.", arm, reason)));
            }
        }
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
    let payload = build_gemini_payload(messages, tools, temperature, max_tokens);
    dispatch(messages, tools, arm, "gemini", url, payload, headers, parse_gemini)
