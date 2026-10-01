// The Anthropic arm: native /v1/messages. x-api-key + anthropic-version, system top-level,
// REQUIRED max_tokens, input_schema tools, tool_use/tool_result blocks, thinking blocks
// replayed verbatim, no temperature, optional ANTHROPIC_EFFORT/ANTHROPIC_THINKING, prompt
// caching on by default. Helpers from llm_common. Internal: entry point agent.llm.chat_llm.
pub fn llm_anthropic() -> DataObject {
    err_out("llm_anthropic is the Anthropic arm, called by chat_llm - call agent.llm.chat_llm")
}

pub fn run(messages:&DataArray, tools:&DataArray, meta:&DataObject, arm:&str,
           url:&str, model:&str, headers:Vec<(String,String)>) -> DataObject {
    let max_tokens = opt(meta, "LLM_MAX_TOKENS", "8192").parse::<i64>().unwrap_or(8192);
    let payload = build_anthropic_payload(messages, tools, model, max_tokens,
                                           &opt(meta, "ANTHROPIC_EFFORT", ""),
                                           &opt(meta, "ANTHROPIC_THINKING", ""),
                                           opt(meta, "ANTHROPIC_CACHE", "on") != "off");
    dispatch(messages, tools, arm, "anthropic", url, payload, headers, parse_anthropic)
}

pub fn build_anthropic_payload(messages: &DataArray, tools: &DataArray, model: &str,
                           max_tokens: i64, effort: &str, thinking: &str,
                           cache: bool) -> DataObject {
    // Tool results are their own USER turn here, and every tool_use in the
    // preceding assistant turn must be answered inside ONE of them. tool_loop
    // emits one role:"tool" message per call, so consecutive ones are merged
    // instead of sent as separate turns - splitting them is what teaches a
    // model to stop calling tools in parallel.
    fn flush(pending: &mut Vec<DataObject>, out: &mut DataArray) {
        if pending.is_empty() { return; }
        let mut arr = DataArray::new();
        for b in pending.drain(..) { arr.push_object(b); }
        let mut m = DataObject::new();
        m.put_string("role", "user");
        m.put_array("content", arr);
        out.push_object(m);
    }

    let mut payload = DataObject::new();
    payload.put_string("model", model);
    // REQUIRED - the Messages API has no server-side default. It caps THINKING
    // plus visible text together, so on a thinking-by-default model a tight
    // LLM_MAX_TOKENS truncates the answer rather than the reasoning.
    payload.put_int("max_tokens", max_tokens);

    let mut system = String::new();
    let mut out = DataArray::new();
    let mut pending: Vec<DataObject> = Vec::new();

    for i in 0..messages.len() {
        let m = messages.get_object(i);
        let role = m.try_get_string("role").unwrap_or_default();
        let content = m.try_get_string("content").unwrap_or_default();

        // Not a message at all on this wire: a top-level parameter.
        if role == "system" {
            if !system.is_empty() { system.push_str("\n\n"); }
            system.push_str(&content);
            continue;
        }
        if role == "tool" {
            let body = if content.is_empty() { "(no output)".to_string() } else { content.clone() };
            let mut tr = DataObject::new();
            tr.put_string("type", "tool_result");
            tr.put_string("tool_use_id", &m.try_get_string("tool_call_id").unwrap_or_default());
            tr.put_string("content", &body);
            pending.push(tr);
            continue;
        }
        flush(&mut pending, &mut out);

        let mut blocks = DataArray::new();
        if role == "assistant" {
            // Replayed VERBATIM and FIRST. With thinking on - the default on
            // Opus 5 - an assistant turn that called a tool is rejected on the
            // next request unless its thinking block comes back untouched.
            // They ride the assistant_message the caller already keeps,
            // exactly as Gemini's thought_signature does.
            if let Ok(tb) = m.try_get_array("thinking_blocks") {
                for b in tb.objects() { blocks.push_object(b.object()); }
            }
        }
        if !content.is_empty() {
            let mut t = DataObject::new();
            t.put_string("type", "text");
            t.put_string("text", &content);
            blocks.push_object(t);
        }
        for (mime, b64) in message_images(&m) {
            let mut src = DataObject::new();
            src.put_string("type", "base64");
            src.put_string("media_type", &mime);
            src.put_string("data", &b64);
            let mut ib = DataObject::new();
            ib.put_string("type", "image");
            ib.put_object("source", src);
            blocks.push_object(ib);
        }
        if role == "assistant" {
            if let Ok(calls) = m.try_get_array("tool_calls") {
                for c in calls.objects() {
                    let c = c.object();
                    let f = c.get_object("function");
                    let mut tu = DataObject::new();
                    tu.put_string("type", "tool_use");
                    tu.put_string("id", &c.get_string("id"));
                    tu.put_string("name", &f.get_string("name"));
                    // input is an OBJECT here, not OpenAI's JSON string.
                    tu.put_object("input", args_to_object(&args_to_string(&f.get_property("arguments"))));
                    blocks.push_object(tu);
                }
            }
        }
        // An empty content array is rejected outright, so a contentless turn
        // is dropped rather than sent.
        if blocks.len() == 0 { continue; }
        let mut n = DataObject::new();
        n.put_string("role", if role == "assistant" { "assistant" } else { "user" });
        n.put_array("content", blocks);
        out.push_object(n);
    }
    flush(&mut pending, &mut out);
    payload.put_array("messages", out);
    if !system.is_empty() { payload.put_string("system", &system); }

    if tools.len() > 0 {
        let mut ts = DataArray::new();
        for t in tools.objects() {
            let t = t.object();
            let f = if t.has("function") { t.get_object("function") } else { t.clone() };
            if !f.has("name") { continue; }
            let mut d = DataObject::new();
            d.put_string("name", &f.get_string("name"));
            if f.has("description") { d.put_string("description", &f.get_string("description")); }
            // `input_schema`, not `parameters`, and it must be an object
            // schema. Copied rather than edited in place: the caller's tools
            // array is shared, and a builder has no business mutating it.
            let mut schema = DataObject::new();
            if let Ok(p) = f.try_get_object("parameters") {
                for (k, v) in p.objects() { schema.set_property(&k, v.clone()); }
            }
            if !schema.has("type") { schema.put_string("type", "object"); }
            d.put_object("input_schema", schema);
            ts.push_object(d);
        }
        if ts.len() > 0 { payload.put_array("tools", ts); }
    }

    // NO temperature: Opus 5 and the 4.7/4.8 family reject temperature, top_p
    // and top_k outright (400). Depth is output_config.effort instead, which
    // is why LLM_TEMPERATURE is not threaded into this builder at all.
    if !effort.is_empty() {
        let mut oc = DataObject::new();
        oc.put_string("effort", effort);
        payload.put_object("output_config", oc);
    }
    // Left unset by default: omitting it runs adaptive thinking on Opus 5 but
    // means NO thinking on 4.8/4.7, and `disabled` is a 400 above effort
    // `high`. Set ANTHROPIC_THINKING=adaptive to force it on an older model.
    if !thinking.is_empty() {
        let mut th = DataObject::new();
        th.put_string("type", thinking);
        payload.put_object("thinking", th);
    }
    // Top-level cache_control auto-places the breakpoint on the last cacheable
    // block, which for a growing conversation is the newest turn - so every
    // request after the first reads the whole prior prefix at ~0.1x input
    // price. The agent loop resends its entire history every call; this is the
    // difference between that being cheap and being the bill.
    if cache {
        let mut cc = DataObject::new();
        cc.put_string("type", "ephemeral");
        payload.put_object("cache_control", cc);
    }
    payload
}

pub fn parse_anthropic(root: &DataObject, arm: &str) -> Result<DataObject, DataObject> {
    if let Ok(e) = root.try_get_object("error") {
        return Err(err_out(&format!("{} error: {}", arm,
            e.try_get_string("message").unwrap_or_else(|_| e.to_string()))));
    }
    let stop = root.try_get_string("stop_reason").unwrap_or_default();
    // A safety decline is a 200 with EMPTY content, not an HTTP error. Read it
    // before walking content or it surfaces as "unexpected response shape" -
    // the exact misdiagnosis the native dialects exist to prevent.
    if stop == "refusal" {
        let detail = match root.try_get_object("stop_details") {
            Ok(d) => format!(" ({}{})",
                d.try_get_string("category").unwrap_or_else(|_| "unspecified".to_string()),
                match d.try_get_string("explanation") {
                    Ok(x) if !x.is_empty() => format!(": {}", x),
                    _ => String::new(),
                }),
            _ => String::new(),
        };
        return Err(err_out(&format!("{} declined the request{}", arm, detail)));
    }
    if let Ok(content) = root.try_get_array("content") {
        let mut text = String::new();
        let mut raw = Vec::new();
        let mut tb = DataArray::new();
        for item in content.objects() {
            let b = item.object();
            match b.try_get_string("type").unwrap_or_default().as_str() {
                "text" => text.push_str(&b.try_get_string("text").unwrap_or_default()),
                "tool_use" => {
                    let args = match b.try_get_object("input") {
                        Ok(a) => a.to_string(), _ => "{}".to_string() };
                    raw.push((b.try_get_string("id").unwrap_or_default(),
                              b.try_get_string("name").unwrap_or_default(),
                              args, String::new()));
                },
                // Reasoning, not the answer: never concatenated into it, but
                // kept whole so the next turn can replay it.
                "thinking" | "redacted_thinking" => tb.push_object(b),
                _ => {}
            }
        }
        if !raw.is_empty() {
            let (norm, replay) = pack_calls(raw);
            let mut out = DataObject::new();
            out.put_string("kind", "tool_calls");
            out.put_array("tool_calls", norm);
            out.put_object("assistant_message", assistant_msg(&text, Some(replay)));
            if tb.len() > 0 {
                let mut am = out.get_object("assistant_message");
                am.put_array("thinking_blocks", tb);
            }
            return Ok(out);
        }
        if !text.is_empty() {
            let out = text_result(&text);
            if tb.len() > 0 {
                let mut am = out.get_object("assistant_message");
                am.put_array("thinking_blocks", tb);
            }
            return Ok(out);
        }
        if stop == "max_tokens" {
            return Err(err_out(&format!("{} returned no content (stop_reason: max_tokens). max_tokens caps THINKING plus text together, so a thinking model can spend the whole budget before answering - raise LLM_MAX_TOKENS.", arm)));
        }
        if !stop.is_empty() {
            return Err(err_out(&format!("{} returned no content (stop_reason: {})", arm, stop)));
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

// Shared send/retry engine. The arm's run() builds a payload and passes its parser; this
// posts with the arm's headers, retries only 408/429/5xx/transport (a 4xx is a
// configuration answer - retrying only delays the report), normalizes via the parser.
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
    let (retryable, retry_after) = match req.send_json(payload.to_json()) {
        Ok(resp) => {
            match resp.into_string() {
                Ok(body) => {
                    match obj_from_str(&body) {
                        Some(root) => match match dialect.as_str() {
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