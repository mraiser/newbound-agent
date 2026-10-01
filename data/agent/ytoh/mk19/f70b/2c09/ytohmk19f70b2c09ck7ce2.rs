// ── THE provider engine, now a thin GATE. LLM= picks an arm; resolve it, apply <ARM>_HEADERS,
// delegate to the provider command under agent.llm. The capture seam wraps the whole dispatch.
let __result: DataObject = (|| -> DataObject {
// ── resolve ──────────────────────────────────────────────────────────────
let meta = (|| -> Option<DataObject> {
    let s = DataStore::globals().try_get_object("system").ok()?;
    let a = s.try_get_object("apps").ok()?;
    let g = a.try_get_object("agent").ok()?;
    g.try_get_object("runtime").ok()
})();
let meta = match meta {
    Some(m) => m,
    None => return err_out("the agent app is not configured: add `agent` to config.properties apps= and restart; runtime/agent/botd.properties holds the LLM settings"),
};
let arm = match meta.try_get_string("LLM") {
    Ok(v) if !v.trim().is_empty() => v.trim().to_uppercase(),
    _ => "VLLM".to_string(),
};

if arm == "LOCAL" {
    return crate::agent::llm::llm_local::run(&messages, &tools, &meta);
}

// (dialect, url, model, headers)
let resolved: Option<(String, String, String, Vec<(String, String)>)> = match arm.as_str() {
    "VLLM" => {
        let url = match need(&meta, "VLLM_URL", &arm) { Ok(v) => v, Err(e) => return err_out(&e) };
        let model = match need(&meta, "VLLM_MODEL", &arm) { Ok(v) => v, Err(e) => return err_out(&e) };
        let mut h = vec![("Content-Type".to_string(), "application/json".to_string())];
        // vLLM served with --api-key needs this; a bare local server does not,
        // which is why it is optional rather than required.
        let k = opt_key(&meta, "VLLM");
        if !k.is_empty() { h.push(("Authorization".to_string(), format!("Bearer {}", k))); }
        Some(("openai".to_string(), url, model, h))
    },
    "OPENAI" => {
        let custom_url = opt(&meta, "OPENAI_URL", "");
        let url = if custom_url.is_empty() { "https://api.openai.com/v1/chat/completions".to_string() } else { custom_url.clone() };
        let model = match need(&meta, "OPENAI_MODEL", &arm) { Ok(v) => v, Err(e) => return err_out(&e) };
        let mut h = vec![("Content-Type".to_string(), "application/json".to_string())];
        // api.openai.com always needs a key; a self-hosted compatible server
        // reached through OPENAI_URL may be keyless.
        let k = if custom_url.is_empty() {
            match need_key(&meta, "OPENAI") { Ok(v) => v, Err(e) => return err_out(&e) }
        } else { opt_key(&meta, "OPENAI") };
        if !k.is_empty() { h.push(("Authorization".to_string(), format!("Bearer {}", k))); }
        Some(("openai".to_string(), url, model, h))
    },
    "ANTHROPIC" => {
        let model = match need(&meta, "ANTHROPIC_MODEL", &arm) { Ok(v) => v, Err(e) => return err_out(&e) };
        let key = match need_key(&meta, "ANTHROPIC") { Ok(v) => v, Err(e) => return err_out(&e) };
        let url = opt(&meta, "ANTHROPIC_URL", "https://api.anthropic.com/v1/messages");
        // The key rides x-api-key, NOT Authorization: Bearer - and
        // anthropic-version is required on every request. Neither is
        // expressible by pointing the OPENAI arm at this URL, and the body
        // differs anyway (see the dialect note at the top).
        let h = vec![("Content-Type".to_string(), "application/json".to_string()),
                     ("x-api-key".to_string(), key),
                     ("anthropic-version".to_string(),
                      opt(&meta, "ANTHROPIC_VERSION", "2023-06-01"))];
        Some(("anthropic".to_string(), url, model, h))
    },
    "GEMINI" => {
        let model = match need(&meta, "GEMINI_MODEL", &arm) { Ok(v) => v, Err(e) => return err_out(&e) };
        let key = match need_key(&meta, "GEMINI") { Ok(v) => v, Err(e) => return err_out(&e) };
        // The key rides a HEADER, not `?key=` — a URL-embedded secret ends up
        // in every log line that prints the endpoint.
        let url = opt(&meta, "GEMINI_URL",
            &format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent", model));
        let h = vec![("Content-Type".to_string(), "application/json".to_string()),
                     ("x-goog-api-key".to_string(), key)];
        Some(("gemini".to_string(), url, model, h))
    },
    "OLLAMA" => {
        // /api/chat, NOT /api/generate: generate is single-prompt with no
        // conversation and no tools, so it cannot carry the agent loop.
        let url = opt(&meta, "OLLAMA_URL", "http://127.0.0.1:11434/api/chat");
        let model = match need(&meta, "OLLAMA_MODEL", &arm) { Ok(v) => v, Err(e) => return err_out(&e) };
        let mut h = vec![("Content-Type".to_string(), "application/json".to_string())];
        let k = opt_key(&meta, "OLLAMA");       // hosted Ollama needs one
        if !k.is_empty() { h.push(("Authorization".to_string(), format!("Bearer {}", k))); }
        Some(("ollama".to_string(), url, model, h))
    },
    _ => None,
};


if arm == "REMOTE" {
    return crate::agent::llm::llm_remote::run(&messages, &tools, &meta);
}

if resolved.is_none() {
    // LLM_CTL dispatch (the CUSTOM arm). TWO command shapes are honored,
    // selected by the command's DECLARED params:
    //   (messages, tools) — tool-capable: the command gets this call's
    //     messages/tools verbatim and answers in chat_llm's own result shape,
    //     passed through, so a custom provider can drive the full agent loop.
    //   (prompt, system_prompt) — legacy text: conversation flattened, no
    //     tools forwarded, text back.
    let whole = match need(&meta, "LLM_CTL", &arm) { Ok(v) => v, Err(e) => return err_out(&e) };
    let parts: Vec<&str> = whole.split(':').collect();
    if parts.len() < 3 { return err_out("LLM_CTL is not in the format 'lib:ctl:cmd'"); }
    let custom_cmd = Command::lookup(parts[0], parts[1], parts[2]);
    if custom_cmd.params.iter().any(|(n, _)| n == "messages") {
        let mut params = DataObject::new();
        params.put_array("messages", messages.clone());
        params.put_array("tools", tools.clone());
        match custom_cmd.execute(params) {
            Ok(res) => {
                let res = match res.try_get_object("a") { Ok(inner) => inner, _ => res };
                if res.try_get_string("kind").is_ok() { return res; }
                let msg = if let Ok(m) = res.try_get_string("msg") { m }
                    else if let Ok(m) = res.try_get_string("content") { m }
                    else { res.to_string() };
                return text_result(&msg);
            },
            Err(e) => return err_out(&format!("LLM_CTL {} failed: {:?}", whole, e)),
        }
    }
    let mut system = String::new();
    let mut turns: Vec<(String, String)> = Vec::new();
    for i in 0..messages.len() {
        let m = messages.get_object(i);
        let role = m.try_get_string("role").unwrap_or_default();
        let content = m.try_get_string("content").unwrap_or_default();
        if content.is_empty() { continue; }
        if role == "system" && system.is_empty() { system = content; }
        else { turns.push((role, content)); }
    }
    // A single user turn is a plain ASK, so it goes through verbatim — that
    // is exactly what ask_llm used to hand a legacy LLM_CTL command, and
    // prefixing it with "USER: " would have silently changed the contract
    // for every custom text provider. Only a real multi-turn conversation
    // gets role labels, because otherwise it is unreadable.
    let convo = if turns.len() == 1 && turns[0].0 == "user" {
        turns[0].1.clone()
    } else {
        turns.iter().map(|(r, c)| format!("{}: {}\n\n", r.to_uppercase(), c))
             .collect::<String>()
    };
    let mut params = DataObject::new();
    params.put_string("prompt", convo.trim());
    params.put_string("system_prompt", &system);
    match custom_cmd.execute(params) {
        Ok(res) => {
            let msg = if let Ok(m) = res.try_get_string("msg") { m }
                else if let Ok(m) = res.try_get_string("a") { m }
                else if let Ok(o) = res.try_get_object("a") {
                    o.try_get_string("msg").unwrap_or_else(|_| o.to_string()) }
                else { res.to_string() };
            return text_result(&msg);
        },
        Err(e) => return err_out(&format!("LLM_CTL {} failed: {:?}", whole, e)),
    }
}

let (dialect, url, model, mut headers) = resolved.unwrap();
for kv in extra_headers(&meta, &arm) {
    headers.retain(|(k, _)| !k.eq_ignore_ascii_case(&kv.0));
    headers.push(kv);
}
match dialect.as_str() {
    "anthropic" => return crate::agent::llm::llm_anthropic::run(&messages, &tools, &meta, &arm, &url, &model, headers),
    "gemini"    => return crate::agent::llm::llm_gemini::run(&messages, &tools, &meta, &arm, &url, &model, headers),
    "ollama"    => return crate::agent::llm::llm_ollama::run(&messages, &tools, &meta, &arm, &url, &model, headers),
    _           => return crate::agent::llm::llm_openai::run(&messages, &tools, &meta, &arm, &url, &model, headers),
}

})();

// what follows must never break the answer: capture failures are the
// capture channel's problem, not the caller's.
let __cap = (|| -> Option<DataObject> {
    let s = DataStore::globals().try_get_object("system").ok()?;
    let a = s.try_get_object("apps").ok()?;
    let g = a.try_get_object("agent").ok()?;
    let r = g.try_get_object("runtime").ok()?;
    if r.try_get_string("LLM_CAPTURE").ok()?.trim().to_lowercase() != "on" { return None; }
    Some(r)
})();
if let Some(meta2) = __cap {
    let kind2 = __result.try_get_string("kind").unwrap_or_default();
    if kind2 == "text" || kind2 == "tool_calls" {
        fn fnv_cap(s: &str) -> u128 {
            let mut h: u128 = 0x6c62272e07bb014262b821756295c58d;
            for b in s.as_bytes() { h ^= *b as u128; h = h.wrapping_mul(0x0000000001000000000000000000013b); }
            h
        }
        let arm2 = meta2.try_get_string("LLM").ok()
            .map(|v| v.trim().to_uppercase()).filter(|v| !v.is_empty())
            .unwrap_or_else(|| "VLLM".to_string());
        let model2 = meta2.try_get_string(&format!("{}_MODEL", arm2)).ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| meta2.try_get_string("LLM_CTL").ok())
            .unwrap_or_default();
        // occurrence ids chain over the conversation prefix, so the
        // same history re-sent next turn dedupes to the same ids
        let mut chain: u128 = 0;
        let mut msg_ids = DataArray::new();
        for i in 0..messages.len() {
            let m = messages.get_object(i);
            let role2 = m.try_get_string("role").unwrap_or_default();
            let content2 = m.try_get_string("content").unwrap_or_default();
            chain = fnv_cap(&format!("{:032x}\u{1f}{}\u{1f}{}", chain, role2, content2));
            if content2.is_empty() { continue; }
            let oid = format!("mo{:032x}", chain);
            let r2 = put(role2, "llm".to_string(), content2, String::new(), arm2.clone(), oid.clone());
            if r2.try_get_string("status").ok().as_deref() == Some("ok") { msg_ids.push_string(&oid); }
        }
        let reply_text = if kind2 == "text" {
            __result.try_get_string("content").unwrap_or_default()
        } else {
            __result.try_get_object("assistant_message").map(|m| m.to_string()).unwrap_or_default()
        };
        let mut reply_id = String::new();
        if !reply_text.is_empty() {
            chain = fnv_cap(&format!("{:032x}\u{1f}assistant\u{1f}{}", chain, reply_text));
            reply_id = format!("mo{:032x}", chain);
            let _ = put("assistant".to_string(), "llm".to_string(), reply_text, String::new(), arm2.clone(), reply_id.clone());
        }
        let store2 = DataStore::new();
        if let Some(root2) = store2.root.canonicalize().ok().and_then(|r| r.parent().map(|p| p.to_path_buf())) {
            let dir2 = root2.join("runtime").join("agent").join("model").join("capture");
            if std::fs::create_dir_all(&dir2).is_ok() {
                use std::io::Write;
                let now2 = flowlang::flowlang::system::time::time();
                let day = now2 / 86_400_000;
                let mut row = DataObject::new();
                row.put_int("t", now2);
                row.put_string("venue", "llm");
                row.put_string("arm", &arm2);
                row.put_string("model", &model2);
                row.put_string("kind", &kind2);
                row.put_array("msg_ids", msg_ids);
                row.put_string("reply_id", &reply_id);
                row.put_int("tools", tools.len() as i64);
                if let Ok(c) = __result.try_get_float("cost_usd") { row.put_float("cost_usd", c); }
                if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true)
                        .open(dir2.join(format!("d{}.jsonl", day))) {
                    let _ = writeln!(f, "{}", row.to_string().replace('\n', " "));
                }
            }
        }
    }
}
__result