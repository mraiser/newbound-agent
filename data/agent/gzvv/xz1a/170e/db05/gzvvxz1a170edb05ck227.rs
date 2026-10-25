// Structured, model-independent continuity checkpoint. The model supplies its
// response; this command derives a bounded handoff from that response and real
// tool observations. It never claims verification merely because work exists.
fn bounded(s: &str, n: usize) -> String { s.chars().take(n).collect() }
fn label(response: &str, name: &str) -> String {
    let marker = format!("{}:", name);
    for (i, line) in response.lines().enumerate() {
        let t = line.trim().trim_start_matches('#').trim();
        if let Some(rest) = t.strip_prefix(&marker) {
            let mut out = rest.trim().to_string();
            for next in response.lines().skip(i + 1) {
                let nt = next.trim().trim_start_matches('#').trim();
                let boundary = ["PROGRESS:", "NEXT:", "BLOCKERS:", "VERIFIED:", "STATUS:",
                    "LAST ACTION:", "LAST RESULT:", "PREVIOUS RUN ID:", "SOURCE CONFIRMATION:"]
                    .iter().any(|m| nt.starts_with(m));
                if boundary { break; }
                if !out.is_empty() { out.push('\n'); }
                out.push_str(next.trim_end());
            }
            return bounded(out.trim(), 4000);
        }
    }
    String::new()
}
let response = input.try_get_string("response").unwrap_or_default();
let status = input.try_get_string("status").unwrap_or_else(|_| "IN_PROGRESS".to_string());
let kind = input.try_get_string("kind").unwrap_or_else(|_| "response".to_string());
let mut work = input.try_get_array("work").unwrap_or_else(|_| DataArray::new());
while work.len() > 16 { work.remove_property(0); }
let mut observations = DataArray::new();
let mut changes = DataArray::new();
let mut evidence = DataArray::new();
for w in work.objects() {
    let w = w.object();
    let tool = w.try_get_string("tool").unwrap_or_default();
    let target = w.try_get_string("target").unwrap_or_default();
    let ok = w.try_get_boolean("ok").unwrap_or(false);
    let result = bounded(&w.try_get_string("result").unwrap_or_default(), 1800);
    let mut observation = DataObject::new();
    observation.put_string("tool", &tool); observation.put_string("target", &target);
    observation.put_boolean("ok", ok); observation.put_string("result", &result);
    observations.push_object(observation);
    let mut changed = DataObject::new(); changed.put_string("tool", &tool);
    changed.put_string("target", &target); changed.put_boolean("ok", ok);
    changed.put_string("result", &result);
    if ["patch", "upsert", "write", "delete", "move", "set_"].iter().any(|x| tool.contains(x)) {
        changes.push_object(changed.clone());
    }
    if ok && ["invoke", "evaluate", "test", "verify", "snapshot"].iter().any(|x| tool.contains(x)) {
        evidence.push_object(changed);
    }
}
let summary = {
    let s = label(&response, "PROGRESS");
    if !s.is_empty() { s } else if !response.trim().is_empty() { bounded(response.trim(), 4000) }
    else { input.try_get_string("summary").unwrap_or_else(|_| "Observed work; no model summary was reached.".to_string()) }
};
let mut next = label(&response, "NEXT");
if next.is_empty() { next = input.try_get_string("next").unwrap_or_default(); }
if next.is_empty() { next = "Inspect the saved activity and choose a new, verifiable increment; do not repeat an identical failed call without new information.".to_string(); }
let mut blockers = label(&response, "BLOCKERS");
if blockers.is_empty() && status == "BLOCKED" { blockers = summary.clone(); }
let verification_pending = input.try_get_string("verification_pending").unwrap_or_default();
let verified = label(&response, "VERIFIED");
let last_action = bounded(&input.try_get_string("last_action").unwrap_or_default(), 1000);
let last_result = bounded(&input.try_get_string("last_result").unwrap_or_default(), 3000);
let mut out = DataObject::new();
out.put_int("version", 2); out.put_int("time", time());
out.put_string("kind", &kind); out.put_string("status", &status);
out.put_boolean("complete", status == "DONE");
out.put_string("reason", &bounded(&input.try_get_string("reason").unwrap_or_default(), 1000));
out.put_string("summary", &summary); out.put_string("next", &bounded(&next, 2000));
out.put_string("blockers", &blockers); out.put_string("verified", &verified);
out.put_string("verification_pending", &verification_pending);
out.put_string("last_action", &last_action); out.put_string("last_result", &last_result);
out.put_array("observations", observations); out.put_array("changes", changes); out.put_array("evidence", evidence);
out.put_string("run_id", &input.try_get_string("run_id").unwrap_or_default());
out.put_string("previous_run_id", &input.try_get_string("previous_run_id").unwrap_or_default());
out.put_int("turn", input.try_get_int("turn").unwrap_or(0));
out.put_int("round", input.try_get_int("round").unwrap_or(0));
out.put_int("max_rounds", input.try_get_int("max_rounds").unwrap_or(0));
out.put_int("tool_calls", input.try_get_int("tool_calls").unwrap_or(0));
out.put_int("errors", input.try_get_int("errors").unwrap_or(0));
let mut text = format!("CONTINUITY v2 — {}\nSTATUS: {}\nPROGRESS: {}\nNEXT: {}", kind, status, summary, next);
if !blockers.is_empty() { text.push_str(&format!("\nBLOCKERS: {}", blockers)); }
if !verified.is_empty() { text.push_str(&format!("\nVERIFIED: {}", verified)); }
if !verification_pending.is_empty() { text.push_str(&format!("\nVERIFICATION PENDING: {}", verification_pending)); }
if !last_action.is_empty() { text.push_str(&format!("\nLAST ACTION: {}\nLAST RESULT:\n{}", last_action, last_result)); }
if out.get_array("changes").len() > 0 {
    text.push_str("\nOBSERVED CHANGE TARGETS (verify before treating as complete):");
    for c in out.get_array("changes").objects() {
        let c = c.object();
        text.push_str(&format!("\n- {} {} [{}]", c.get_string("tool"), c.get_string("target"), if c.get_boolean("ok") { "ok" } else { "error" }));
    }
}
out.put_string("text", &bounded(&text, 8000));
out