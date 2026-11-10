// The agent's global approval kill-switch, read LIVE from
// runtime/agent/botd.properties (system.apps.agent.runtime — same map
// chat_llm reads LLM=/VLLM_URL from, re-read per call, so an edit to
// botd.properties takes effect without a restart). Truthy: 1/true/yes/on
// (case-insensitive). Absent or anything else = the normal per-call gate.
let mut out = DataObject::new();
let mut enabled = false;
let system = DataStore::globals().get_object("system");
if system.has("apps") {
    let apps = system.get_object("apps");
    if apps.has("agent") {
        let agent = apps.get_object("agent");
        if agent.has("runtime") {
            let rt = agent.get_object("runtime");
            if rt.has("AUTO_APPROVE_TOOLS") {
                let v = rt.get_string("AUTO_APPROVE_TOOLS").trim().to_lowercase();
                enabled = matches!(v.as_str(), "1" | "true" | "yes" | "on");
            }
        }
    }
}
out.put_boolean("enabled", enabled);
out