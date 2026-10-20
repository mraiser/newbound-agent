// step_item (kb.plan GATED plan-item autonomy, P-a): run ONE bounded step of
// an ACCEPTED plan item. The human move to `accepted` is the authorization -
// exactly like the remember request being its own authorization. Every
// invariant from the plan item's design is enforced HERE, in one place:
//   (1) GATE: only entries tagged `accepted` are eligible. proposed stays
//       human-only - the agent never executes its own ideas, and this command
//       refuses in-progress/done/abandoned too (those are moved, not stepped,
//       by the executive's lifecycle logic or are already settled).
//   (2) ELIGIBILITY: the entry must carry a STRUCTURED `step` field
//       {lib,ctl,cmd,args?} - a concrete, machine-checkable act. Pure-prose
//       intent surfaces as a no-op (not an error): no LLM guessing in v1.
//   (5) JOURNALED: the step's result is appended to the entry's `step_log`
//       and the whole facet is written through patch_control_facet with the
//       base hash - a bad step reverts like any human edit. On stale_base we
//       abort and report current_hash; we NEVER force.
//   (6) STALE LOOP: if the entry's source pointer's hash drifted since the
//       entry was written, we defer to the stale path (evidence decay
//       re-opens planning) instead of acting on possibly-outdated intent.
// This command performs ONE step and returns; the executive's drive budget
// (it spends a drive unit to call us) is the throttle - we never loop.
fn esc(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}
// MAX_PLAN_STEPS (convergence guard): the most autonomous steps the agent may
// take on ONE accepted plan item before it must hand back to a human for
// re-approval. This is deliberately SEPARATE from TOOL_BUDGET: that is a
// per-turn chatTurn round limit (conversational tool calls); this is a
// per-item bound on background initiative. Read from agent runtime settings;
// absent, non-numeric, or <=0 falls back to 20, capped at 1000 as a guard
// (same shape as get_tool_budget). Read live so the owner can tune it without
// a restart.
fn max_plan_steps() -> i64 {
    let mut n: i64 = 20;
    let g = flowlang::datastore::DataStore::globals();
    if let Ok(sys) = g.try_get_object("system") {
        if let Ok(apps) = sys.try_get_object("apps") {
            if let Ok(agent) = apps.try_get_object("agent") {
                if let Ok(rt) = agent.try_get_object("runtime") {
                    if let Ok(v) = rt.try_get_string("MAX_PLAN_STEPS") {
                        if let Ok(p) = v.trim().parse::<i64>() {
                            if p > 0 { n = p.min(1000); }
                        }
                    }
                }
            }
        }
    }
    n
}
let mut out = DataObject::new();
let api = crate::api::new();
let r = api.dev.code.read_control_facet("kb".to_string(), "plan".to_string(), "memory".to_string());
if !r.try_get_boolean("exists").unwrap_or(false) {
    out.put_string("status", "err");
    out.put_string("msg", "kb.plan has no memory facet");
    return out;
}
let src = r.get_string("source");
let hash = r.get_string("hash");
if !base.is_empty() && base != hash {
    out.put_string("status", "err");
    out.put_string("msg", "stale_base");
    out.put_string("current_hash", &hash);
    return out;
}
let wrapped = format!("{{\"a\": {} }}", src);
let parsed = match DataObject::try_from_string(&wrapped) {
    Ok(o) => o,
    Err(_) => {
        out.put_string("status", "err");
        out.put_string("msg", "kb.plan memory facet did not parse as a JSON array");
        return out;
    }
};
let arr = parsed.get_array("a");
// Locate the entry by exact claim.
let mut idx: i64 = -1;
for i in 0..arr.len() {
    let e = arr.get_object(i);
    if e.try_get_string("claim").unwrap_or_default() == claim { idx = i as i64; break; }
}
if idx < 0 {
    out.put_string("status", "err");
    out.put_string("msg", "no kb.plan entry with that claim - the board may be stale, refresh it");
    return out;
}
let mut e = arr.get_object(idx as usize);
// (1) GATE on the accepted lifecycle token.
let tags = e.try_get_string("tags").unwrap_or_default().to_lowercase();
let toks: Vec<&str> = tags.split(',').map(|t| t.trim()).collect();
let has_tag = |s: &str| toks.iter().any(|t| *t == s);
if !has_tag("accepted") {
    out.put_string("status", "err");
    out.put_string("msg", "gate: only `accepted` plan items are steppable - proposed stays human-only and the agent never self-promotes");
    out.put_string("action", "gated");
    return out;
}
// (1b) CONVERGENCE GUARD: bound the autonomous steps on this one item. The
// count of prior steps is the entry's step_log length; at/over MAX_PLAN_STEPS
// the agent stops and hands the item back to a human (re-accept via
// move_item, which resets an exhausted log). This runs BEFORE eligibility so
// an exhausted item reports needs_reapproval regardless of its step field -
// the guard is about the item's autonomy budget, not its actionability.
let prior = if e.has("step_log") {
    match e.try_get_array("step_log") { Ok(a) => a.len() as i64, Err(_) => 0 }
} else { 0 };
let cap = max_plan_steps();
if prior >= cap {
    out.put_string("status", "ok");
    out.put_string("action", "needs_reapproval");
    out.put_int("steps_taken", prior);
    out.put_int("max_steps", cap);
    out.put_string("msg", "convergence guard: this item has reached MAX_PLAN_STEPS - a human must re-approve (re-accept) it before the agent steps it again");
    return out;
}
out.put_int("steps_taken", prior);
out.put_int("max_steps", cap);
// (2) ELIGIBILITY: a structured step field, not prose.
if !e.has("step") {
    out.put_string("status", "ok");
    out.put_string("action", "surfaced");
    out.put_string("msg", "no structured `step` field ({lib,ctl,cmd,args}) - pure-prose intent is not machine-actionable in v1");
    return out;
}
let step = match e.try_get_object("step") {
    Ok(s) => s,
    Err(_) => {
        out.put_string("status", "err");
        out.put_string("msg", "`step` field is present but not an object {lib,ctl,cmd,args}");
        return out;
    }
};
if !step.has("lib") || !step.has("ctl") || !step.has("cmd") {
    out.put_string("status", "err");
    out.put_string("msg", "`step` must name lib, ctl, and cmd");
    return out;
}
// (6) STALE LOOP: re-check the source pointer's hash before acting.
if e.has("source") {
    if let Ok(srcptr) = e.try_get_object("source") {
        if srcptr.has("lib") && srcptr.has("ctl") && srcptr.has("facet") && srcptr.has("hash") {
            let sl = srcptr.get_string("lib");
            let sc = srcptr.get_string("ctl");
            let sf = srcptr.get_string("facet");
            let sh = srcptr.get_string("hash");
            let cur = api.dev.code.read_control_facet(sl.clone(), sc.clone(), sf.clone());
            let now_h = cur.get_string("hash");
            if !now_h.is_empty() && now_h != sh {
                out.put_string("status", "ok");
                out.put_string("action", "defer_stale");
                out.put_string("msg", "source hash drifted - deferring to the stale path (evidence decay re-opens planning)");
                out.put_string("expected_hash", &sh);
                out.put_string("current_hash", &now_h);
                return out;
            }
        }
    }
}
// Execute the ONE bounded step, panic-contained.
let slib = step.get_string("lib");
let sctl = step.get_string("ctl");
let scmd = step.get_string("cmd");
let sargs = if step.has("args") {
    match step.try_get_object("args") { Ok(a) => a, Err(_) => DataObject::new() }
} else { DataObject::new() };
let now = flowlang::flowlang::system::time::time();
let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    let cmd = flowlang::command::Command::lookup(&slib, &sctl, &scmd);
    cmd.execute(sargs.deep_copy())
}));
let (ok_step, result_json) = match ran {
    Ok(Ok(rr)) => (true, rr.to_string()),
    Ok(Err(pe)) => (false, format!("{{\"error\":\"{}\"}}", esc(&format!("{:?}", pe)))),
    Err(_) => (false, "{\"error\":\"panic during step execution\"}".to_string()),
};
// (5) Append to the entry's step_log and write the facet back journaled.
let mut log = if e.has("step_log") {
    match e.try_get_array("step_log") { Ok(a) => a, Err(_) => DataArray::new() }
} else { DataArray::new() };
let mut entry_log = DataObject::new();
entry_log.put_int("time", now);
entry_log.put_string("home", &format!("{}.{}.{}", slib, sctl, scmd));
entry_log.put_boolean("ok", ok_step);
entry_log.put_string("result", &result_json);
entry_log.put_string("author", "executive");
log.push_object(entry_log);
e.put_array("step_log", log);
let short: String = claim.chars().take(50).collect();
let label = format!("plan step: '{}' -> {}", short, if ok_step { "ok" } else { "err" });
let pw = api.dev.code.patch_control_facet(
    "kb".to_string(), "plan".to_string(), "memory".to_string(),
    "".to_string(), arr.to_string(), hash.clone(), label, "executive".to_string(), nn_sessionid);
let write_ok = pw.try_get_boolean("ok").unwrap_or(false)
    || pw.try_get_string("status").map(|s| s == "ok").unwrap_or(false);
if !write_ok {
    out.put_string("status", "err");
    out.put_string("msg", "step ran but the step_log write failed (likely stale_base) - re-read the board and retry");
    if pw.has("msg") { out.put_string("write_msg", &pw.get_string("msg")); }
    return out;
}
out.put_string("status", "ok");
out.put_string("action", if ok_step { "stepped" } else { "step_failed" });
out.put_boolean("ok", ok_step);
out.put_string("home", &format!("{}.{}.{}", slib, sctl, scmd));
out.put_string("result", &result_json);
out.put_int("time", now);
out