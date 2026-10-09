use ndata::dataobject::DataObject;
use ndata::dataarray::DataArray;
use ndata::data::Data;
use flowlang::datastore::DataStore;
use flowlang::command::Command;
use std::collections::HashMap;

use crate::agent::llm::llm_common::*;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["messages", "tools", "meta"] {
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
        let arg_2: DataObject = o.get_object("meta");
        llm_local(arg_0, arg_1, arg_2)
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

pub fn llm_local(messages: DataArray, tools: DataArray, meta: DataObject) -> DataObject {
// The LOCAL arm: the resident model service answers the chat (POST /chat). Callers use chat_llm.
//pub fn run(messages:&DataArray, tools:&DataArray, meta:&DataObject) -> DataObject {
    // Phase 8a: the resident service's USER-FACING pointer answers the
    // chat (POST /chat). Text-only - the local model carries no tool
    // protocol, so the agent loop degrades to plain answers. Routing
    // here is the owner's deliberate flip (LLM=LOCAL in botd), and the
    // service refuses (503) until its user gate has promoted a pointer.
    let _ = &tools;
    let url = format!("http://127.0.0.1:{}/chat", opt(&meta, "MODEL_SERVICE_PORT", "8077"));
    let mut msgs = DataArray::new();
    for i in 0..messages.len() {
        let m = messages.get_object(i);
        let role = m.try_get_string("role").unwrap_or_default();
        let content = m.try_get_string("content").unwrap_or_default();
        if content.is_empty() { continue; }
        let role = if role == "system" || role == "assistant" { role } else { "user".to_string() };
        let mut o2 = DataObject::new();
        o2.put_string("role", &role);
        o2.put_string("content", &content);
        msgs.push_object(o2);
    }
    let mut body = DataObject::new();
    body.put_array("messages", msgs);
    body.put_int("max_tokens",
        opt(&meta, "LLM_MAX_TOKENS", "256").parse::<i64>().unwrap_or(256).min(1024));
    body.put_float("temperature",
        opt(&meta, "LLM_TEMPERATURE", "0.7").parse::<f64>().unwrap_or(0.7));
    let resp = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_millis(180000))
        .build()
        .post(&url)
        .set("Content-Type", "application/json")
        .send_string(&body.to_string());
    let text = match resp {
        Ok(r) => r.into_string().unwrap_or_default(),
        Err(ureq::Error::Status(_c, r)) => r.into_string().unwrap_or_default(),
        Err(e) => return err_out(&format!(
            "LOCAL arm: model service unreachable at {}: {} - is the \
             resident service up (agent-model-bootstrap)?", url, e)),
    };
    return match DataObject::try_from_string(&text) {
        Ok(d) => {
            if d.try_get_string("status").ok().as_deref() == Some("ok") {
                text_result(&d.try_get_string("text").unwrap_or_default())
            } else {
                err_out(&format!("LOCAL arm: {}",
                    d.try_get_string("msg").unwrap_or_else(|_| "service refused".to_string())))
            }
        },
        Err(_) => err_out(&format!("LOCAL arm: service answered non-JSON at {}", url)),
    };
//}

//err_out("llm_local is the LOCAL arm, called by chat_llm - call agent.llm.chat_llm")
}
