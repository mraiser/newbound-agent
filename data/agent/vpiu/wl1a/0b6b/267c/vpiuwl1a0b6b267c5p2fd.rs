// The agent's per-turn tool budget, read from runtime/agent/botd.properties
// (system.apps.agent.runtime — the same boot-time snapshot chat_llm reads
// LLM= from, so an edit takes effect on the next service restart, not
// live). TOOL_BUDGET=<positive integer>; absent, zero, negative, or
// non-numeric falls back to the default of 10. Capped at 1000 as a guard.
// Returns FLAT {budget: i64}. The agentloop js reads this once at ready and
// uses it as the chatTurn round limit in place of the hard-coded 10.
let mut out = DataObject::new();
let mut budget: i64 = 10;
let system = DataStore::globals().get_object("system");
if system.has("apps") {
    let apps = system.get_object("apps");
    if apps.has("agent") {
        let agent = apps.get_object("agent");
        if agent.has("runtime") {
            let rt = agent.get_object("runtime");
            if rt.has("TOOL_BUDGET") {
                if let Ok(n) = rt.get_string("TOOL_BUDGET").trim().parse::<i64>() {
                    if n > 0 { budget = n.min(1000); }
                }
            }
        }
    }
}
out.put_int("budget", budget);
out