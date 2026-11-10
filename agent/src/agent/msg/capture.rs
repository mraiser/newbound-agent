use ndata::dataobject::DataObject;
use ndata::dataarray::DataArray;
use ndata::data::Data;
use crate::agent::msg::put::put;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["venue", "messages", "entity", "provenance"] {
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
        let arg_0: String = o.get_string("venue");
        let arg_1: DataArray = o.get_array("messages");
        let arg_2: String = o.get_string("entity");
        let arg_3: String = o.get_string("provenance");
        capture(arg_0, arg_1, arg_2, arg_3)
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

pub fn capture(venue: String, messages: DataArray, entity: String, provenance: String) -> DataObject {
// agent-msg-capture — records one REALIZED conversational turn into the
// message store under a labeled venue (chat | agent-app | ...). This is the
// boundary the venue calls after the assistant reply is final, NOT an
// ambient hook in chat_llm: every arm (archivist, executive, plugin, model)
// also drives chat_llm, and capturing there would flood the conversation
// stream with background traffic. Opt-in by venue keeps "chat" meaning
// "what a human actually saw and said."
//
// Two rules make this the non-duplicating layer:
//   - CONTENT is king, occurrence ids are derived (fnv over venue|role|
//     content), so a re-sent or replayed turn records once. Full-fidelity
//     history (tool payloads, images, the exact request envelope) stays in
//     the session store; msg holds pointers to meaning, never copies of
//     substance.
//   - TOOL rows become narrative glue only. Each tool_call + result pair
//     collapses to one line naming the tool and its args — never the result
//     text — so the record reads as reasoned ("let me check" -> a visible
//     step -> a confident answer) instead of conjured, without turning msg
//     into a queryable tool-result cache.
fn err(msg: &str) -> DataObject {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", msg);
    o
}
fn fnv128(s: &str) -> String {
    let mut h: u128 = 0x6c62272e07bb014262b821756295c58d;
    let prime: u128 = 0x0000000001000000000000000000013b;
    for b in s.as_bytes() { h ^= *b as u128; h = h.wrapping_mul(prime); }
    format!("{:032x}", h)
}
fn clip(s: &str, chars: usize) -> String {
    if s.chars().count() <= chars { return s.to_string(); }
    let cut: String = s.chars().take(chars.saturating_sub(1)).collect();
    format!("{}\u{2026}", cut)
}

let venue_t = venue.trim().to_lowercase();
if venue_t.is_empty() { return err("venue is required (the conversation label: chat | agent-app | ...)"); }

// Walk the venue's own message array — system skipped, user/assistant kept,
// tool exchanges folded to glue. newest-last is preserved.
let mut written: i64 = 0;
let mut skipped: i64 = 0;
// map tool_call_id -> tool name, so each result row can fold into its call's
// glue line rather than free-stand (a result without its call is dropped:
// substance lives in the session store, and a dangling payload is exactly
// the duplication this layer refuses).
let mut call_name: std::collections::HashMap<String, String> = std::collections::HashMap::new();
// ids of tool-call assistant rows already folded, so a stray repeat is dropped
let mut i: usize = 0;
while i < messages.len() {
    let m = match messages.try_get_object(i) { Ok(m) => m, Err(_) => { i += 1; continue; } };
    let role = if m.has("role") { m.get_string("role") } else { String::new() };
    let content = if m.has("content") && !m.get_property("content").is_null() { m.get_string("content") } else { String::new() };

    if role == "system" { i += 1; continue; }

    // the venue's hidden [CONTEXT] preamble rides as a user row ahead of the
    // real ask (session titling hit this too); it is not conversation, skip it
    if role == "user" && content.trim_start().starts_with("[CONTEXT]") { i += 1; continue; }

    if role == "user" || role == "assistant" {
        // an assistant row that only carries tool_calls (no text) folds into glue below
        let has_calls = m.has("tool_calls") && m.get_property("tool_calls").is_array();
        if !content.trim().is_empty() {
            let oid = format!("mo{}", fnv128(&format!("{}\u{1f}{}\u{1f}{}", venue_t, role, content)));
            let r = put(role.clone(), venue_t.clone(), content, entity.clone(), provenance.clone(), oid);
            if r.try_get_string("status").ok().as_deref() == Some("ok") { written += 1; } else { skipped += 1; }
        }
        if has_calls {
            for tc in m.get_array("tool_calls").objects() {
                let tc = tc.object();
                let name = tc.get_object("function").get_string("name");
                let args = tc.get_object("function").get_string("arguments");
                let cid = if tc.has("id") { tc.get_string("id") } else { String::new() };
                call_name.insert(cid, name.clone());
                let glue = format!("[tool] {}({})", name, clip(&args, 160));
                let oid = format!("mo{}", fnv128(&format!("{}\u{1f}tool\u{1f}{}", venue_t, glue)));
                let r = put("tool".to_string(), venue_t.clone(), glue, entity.clone(), provenance.clone(), oid);
                if r.try_get_string("status").ok().as_deref() == Some("ok") { written += 1; } else { skipped += 1; }
            }
        }
        i += 1; continue;
    }

    if role == "tool" {
        // the result row: its call already emitted glue. Substance stays in the
        // session store; here we drop the payload (that is the whole design).
        skipped += 1;
        i += 1; continue;
    }

    i += 1;
}

let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_string("venue", &venue_t);
o.put_int("written", written);
o.put_int("skipped", skipped);
o
}
