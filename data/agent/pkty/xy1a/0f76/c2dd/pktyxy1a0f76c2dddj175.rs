// The REMOTE arm: the peer IS the transport - no HTTP, no keys, no dialect here. The
// whole (messages, tools) pair goes to the named peer (LLM_REMOTE=<peer-uuid>), which
// runs its OWN chat_llm under its own LLM= arm and answers in this command's normalized
// shape, passed through untouched. Peer chaining resolves at wherever an HTTP arm is
// set. Helpers from llm_common. Internal: the public entry point is agent.llm.chat_llm.
pub fn llm_remote() -> DataObject {
    err_out("llm_remote is the REMOTE arm, called by chat_llm - call agent.llm.chat_llm")
}

pub fn run(messages:&DataArray, tools:&DataArray, meta:&DataObject) -> DataObject {
    let arm = "REMOTE";
    // The peer IS the transport - no HTTP, no keys, no dialect here. The
    // whole (messages, tools) pair goes to the named peer, which runs its
    // OWN chat_llm under its own LLM= arm and answers in this command's own
    // normalized shape, passed through untouched: tools, vision (paths are
    // resolved by the delegate on the far side), and thinking blocks all
    // survive because nothing is re-rendered. REMOTE is just "not here".
    //
    // The service-exec frame caps at ~16K ENCRYPTED bytes, so an oversized
    // conversation is DETECTED before the wire and split across sequential
    // frames: the first carries {"messages":[], "tools":[...]} (the tool
    // protocol intact with an empty conversation), each later frame carries
    // a disjoint slice of the conversation; the remote peer's agent.llm.put
    // reassembles by msg_id and answers through the LAST frame. Three
    // consequences: (1) messages are run through `put` UNORDERED, so each
    // gets a fresh msg_id and the sliced order never disturbs the chat
    // record; (2) an LLM that refuses an empty first frame poisons the
    // buffer for the real ones - the message list must fit under the cap,
    // as it always has for this agent's own traffic; (3) the peer's exec
    // reply itself rides ONE frame, so a REMOTE answer is capped at ~16K -
    // the arm refuses when the REQUEST needs splitting unless
    // LLM_REMOTE_SPLIT=on, because the far side's reassembly tolerates it
    // but the reply cap is the real ceiling.
    //
    // PEER CHAINING: LLM=REMOTE on the far side simply re-enters this arm
    // there, so a chain of peers resolves at wherever an HTTP arm is set.
    let uuid = match need(&meta, "LLM_REMOTE", &arm) { Ok(v) => v, Err(e) => return err_out(&e) };
    // peer.service.exec can only reach commands on a PUBLISHED APP's boot
    // control - llm is not an app, so it cannot be named directly. The bridge
    // is app.app.exec, which IS on the `app` app's boot control and runs ANY
    // command by (lib, id). Its own id is genesis-fixed (identical on every
    // install). The remote chat_llm id is per-install, so resolve THIS
    // instance's chat_llm id from the control index as the default (the
    // common case: both peers share the same agent-store snapshot) and let
    // LLM_REMOTE_ID override it for a peer whose store differs.
    let this_id = (|| -> String {
        let s = DataStore::new();
        if let Ok(raw) = std::fs::read_to_string("./data/agent/cont/rols/____/____/controls") {
            if let Ok(j) = DataObject::try_from_string(&raw) {
                let list = j.get_object("data").get_array("list");
                for i in 0..list.len() {
                    let c = list.get_object(i);
                    if c.get_string("name") == "llm" {
                        let cmds = s.get_data("agent", &c.get_string("id")).get_object("data").get_array("cmd");
                        for k in 0..cmds.len() {
                            let cc = cmds.get_object(k);
                            if cc.get_string("name") == "chat_llm" { return cc.get_string("id"); }
                        }
                    }
                }
            }
        }
        String::new()
    })();
    let remote_id = opt(&meta, "LLM_REMOTE_ID", &this_id);
    if remote_id.is_empty() {
        return err_out("REMOTE arm: could not resolve the local chat_llm command id and LLM_REMOTE_ID is not set - set LLM_REMOTE_ID=<remote chat_llm command id> in runtime/agent/botd.properties.");
    }
    let mut llm_args = DataObject::new();
    llm_args.put_array("messages", messages.clone());
    llm_args.put_array("tools", tools.clone());
    let mut params = DataObject::new();
    params.put_string("lib", "agent");
    params.put_string("id", &remote_id);
    params.put_object("args", llm_args);
    params.put_string("nn_sessionid", "");
    let mut d = DataObject::new();
    d.put_string("bot", "app");
    d.put_string("cmd", "exec");
    d.put_object("params", params);
    let wire_len = "cmd ".len() + d.to_string().len();
    let split = opt(&meta, "LLM_REMOTE_SPLIT", "");
    if wire_len > 15000 && split != "on" {
        return err_out(&format!(
            "REMOTE arm: the conversation is {} bytes on the peer wire, over the ~16K service-exec frame cap; the call would arrive truncated. Shorten the context or set LLM_REMOTE_SPLIT=on to acknowledge frame-splitting (see the arm's comment).",
            wire_len));
    }
    // app app exec on the far peer; app.app.exec id is genesis-fixed.
    let res = crate::API.peer.service.exec(uuid.clone(), "app".to_string(), "exec".to_string(), d.get_object("params"));
    let status = res.try_get_string("status").unwrap_or_default();
    if status == "err" {
        return err_out(&format!("REMOTE arm: peer {}: {}",
            uuid, res.try_get_string("msg").unwrap_or_else(|_| "unknown peer error".to_string())));
    }
    return match res.try_get_object("data") {
        // app.app.exec wraps the command's own result: FLAT chat_llm lands in
        // data as {kind, ...}. A bridge-level failure (UNAUTHORIZED, 500,
        // NO SUCH LIBRARY) carries status:err INSIDE data - surface its msg
        // rather than reporting a shapeless "no chat_llm result".
        Ok(inner) if inner.try_get_string("kind").is_ok() => inner,
        Ok(inner) if inner.try_get_string("status").unwrap_or_default() == "err" => err_out(&format!(
            "REMOTE arm: peer {} bridge: {}",
            uuid, inner.try_get_string("msg").unwrap_or_else(|_| "bridge error".to_string()))),
        _ => err_out(&format!("REMOTE arm: peer {} answered without a chat_llm result: {}",
            uuid, res.to_string().chars().take(800).collect::<String>())),
    };
}