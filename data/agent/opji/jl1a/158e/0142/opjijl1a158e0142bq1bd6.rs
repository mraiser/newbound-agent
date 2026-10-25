// step_item (kb.plan GATED plan-item autonomy, P-a): advance ONE accepted plan
// item by REASONING about it. The human move to `accepted` is the
// authorization - exactly like the remember request being its own
// authorization. Invariants enforced HERE, in one place:
//   (1) GATE: only entries tagged `accepted` are eligible. proposed stays
//       human-only - the agent never executes its own ideas, and this command
//       refuses in-progress/done/abandoned too (those are moved, not stepped,
//       by the executive's lifecycle logic or are already settled).
//   (2) THINK: the item's claim + detail is the GOAL. The executive reasons
//       about the MEANS via agent.llm.tool_loop (ONE bounded autonomous turn,
//       its own TOOL_BUDGET + failure-abort). The human owns the WHAT (the
//       accepted tag); the agent owns the HOW. v1 demanded a pre-authored
//       `step` field and refused prose - but a pre-authored step only captures
//       work already so well-understood it's done, so the ladder was empty for
//       every novel item. The ban on reasoning was a lobotomy, not a guardrail
//       (owner, 2026-08): the real guardrails are the substrate - the per-call
//       mutating gate, TOOL_BUDGET, MAX_PLAN_STEPS, and this journaled
//       step_log - not a refusal to think.
//   (5) JOURNALED: the reasoning's answer is appended to the entry's
//       `step_log` (home agent.llm.tool_loop) and the whole facet is written
//       through patch_control_facet with the base hash - a bad step reverts
//       like any human edit. On stale_base we abort and report current_hash;
//       we NEVER force.
//   (6) STALE LOOP: if the entry's source pointer's hash drifted since the
//       entry was written, we defer to the stale path (evidence decay
//       re-opens planning) instead of acting on possibly-outdated intent.
// This command performs ONE reasoning turn and returns; the executive's drive
// budget (it spends a drive unit to call us) is the throttle - we never loop.
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
// (1) GATE: `accepted` OR `in-progress`. Both are human-authorized - an item
// must have been `accepted` (a human act) before it could ever reach
// `in-progress`, so the agent never self-promotes into either. `accepted` =
// signed but not yet started; `in-progress` = the agent is actively working it.
// proposed stays human-only; completed/done/abandoned are settled (terminal for
// the loop) and are refused.
let tags = e.try_get_string("tags").unwrap_or_default().to_lowercase();
let toks: Vec<&str> = tags.split(',').map(|t| t.trim()).collect();
let has_tag = |s: &str| toks.iter().any(|t| *t == s);
let was_accepted = has_tag("accepted");
if !was_accepted && !has_tag("in-progress") {
    out.put_string("status", "err");
    out.put_string("msg", "gate: only `accepted` or `in-progress` plan items are steppable - proposed stays human-only and the agent never self-promotes; a `blocked` item waits for a human to move it back to accepted");
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
// (A) PARENT COORDINATION IS THE DEFAULT (owner, 2026-11): a PARENT (an entry
// other items point to via `parent`) is itself an executable COORDINATION
// task even while its children are proposed/accepted/in-progress/blocked -
// its own claim+detail is the coordination goal, no opt-in tag. The prior
// behavior (refuse with gated while any child is unfinished) is removed: it
// made an accepted parent unsteppable exactly when it had work to direct.
// Abandoned children are out of the dependency graph (they carry an explicit
// human disposition); every OTHER non-terminal child (proposed/accepted/
// in-progress/blocked) keeps the parent open and rides the turn below.
let mut children: Vec<DataObject> = Vec::new();
for i in 0..arr.len() {
    if i as i64 == idx { continue; }
    let c = arr.get_object(i);
    if c.try_get_string("parent").unwrap_or_default() != claim { continue; }
    let ct = c.try_get_string("tags").unwrap_or_default().to_lowercase();
    if ct.split(',').map(|t| t.trim()).any(|t| t == "abandoned") { continue; }
    children.push(c);
}
let is_parent = !children.is_empty();
// (2) THE GOAL: the item's claim + detail is the objective. v1 demanded a
// pre-authored structured `step` {lib,ctl,cmd,args} and refused pure prose
// (surfaced, never executed) - but a pre-authored step only captures work
// already so well-understood it's basically done, so the ladder was empty for
// every novel item. The owner (2026-... ) ruled the ban on executive reasoning
// was a lobotomy, not a guardrail: the EXECUTIVE reasons about the MEANS via
// tool_loop; the human's `accepted` tag owns the WHAT (intent). The guardrails
// live in the substrate (per-call mutating gate, TOOL_BUDGET, MAX_PLAN_STEPS,
// this journaled step_log), NOT in refusing to think.
let goal = claim.clone();
let detail = e.try_get_string("detail").unwrap_or_default();
let continuity = crate::agent::plan::continuity_context::continuity_context(e.deep_copy());
let inherited_structured = if e.has("checkpoint") { e.get_object("checkpoint") } else { DataObject::new() };
let inherited_checkpoint_time = inherited_structured.try_get_int("time").unwrap_or(0);
let last_journal_time = e.try_get_array("step_log").ok()
    .and_then(|a| if a.len() == 0 { None } else { a.get_object(a.len()-1).try_get_int("time").ok() }).unwrap_or(0);
let prefer_structured = inherited_structured.has("summary") && inherited_checkpoint_time > 0 && inherited_checkpoint_time >= last_journal_time;
let mut prior_steps = continuity.get_string("prior_steps");
let mut previous_run_id = if prefer_structured { inherited_structured.try_get_string("run_id").unwrap_or_default() } else { continuity.get_string("previous_run_id") };
let inherited_checkpoint = if prefer_structured {
    crate::agent::plan::checkpoint_read::checkpoint_read(inherited_structured.clone(), String::new())
} else { DataObject::new() };
let mut handoff_received = if prefer_structured { inherited_checkpoint.get_string("text") } else { continuity.get_string("handoff_received") };
let mut recovered_run = false;
let mut continuity_recovery_kind = if prefer_structured && e.has("active_run") { "durable_checkpoint" } else { "" };
let journaled_run = previous_run_id.clone();
// An interrupted run or a stale post-turn board write must not erase working
// state. The runtime run is a recovery facet, never an authority to override
// human lifecycle: this code is reached only after the normal step gate.
if handoff_received.is_empty() || !journaled_run.is_empty() {
    let store = flowlang::datastore::DataStore::new();
    if store.exists("runtime", "agent_plan_runs") {
        let runs = store.get_data("runtime", "agent_plan_runs").get_object("data").get_array("runs");
        let mut newest = 0i64;
        for i in 0..runs.len() {
            let r = runs.get_object(i);
            if r.try_get_string("claim").unwrap_or_default() != claim { continue; }
            let rid = r.try_get_string("run_id").unwrap_or_default();
            if rid == journaled_run { continue; }
            // Already-journaled history is older context, not a newer recovery
            // source. A different run may represent an interrupted write.
            if e.try_get_array("step_log").ok().map(|a| {
                (0..a.len()).any(|j| a.get_object(j).try_get_string("run_id").unwrap_or_default() == rid)
            }).unwrap_or(false) { continue; }
            let updated = r.try_get_int("updated").unwrap_or(0);
            // A stale older run must not replace a newer durable handoff.
            if updated <= last_journal_time || updated <= inherited_checkpoint_time || updated <= newest { continue; }
            let text = r.try_get_object("handoff").ok()
                .and_then(|h| h.try_get_string("checkpoint").ok()).unwrap_or_default();
            if text.trim().is_empty() { continue; }
            newest = updated;
            previous_run_id = r.try_get_string("run_id").unwrap_or_default();
            handoff_received = text;
            recovered_run = true;
            continuity_recovery_kind = "runtime_observation";
        }
    }
    if recovered_run {
        prior_steps = format!("- [recovered run not yet journaled to this item] [run_id: {}] -> {}\n{}", previous_run_id, handoff_received, prior_steps);
    }
}
let mut prompt = format!(
    "You are the executive's plan-step engine. Advance ONE accepted plan item toward completion. \
Think, then act via the available tools. Prefer a single concrete, verifiable increment over a sweep. \
When you finish, END your reply with a status line in EXACTLY this form (last line, no tool call after):\n\
STATUS: DONE  - the item is complete AND you verified it against the goal with a concrete check you actually ran (build/invoke/parity/test). Name that check on the line above, e.g. 'VERIFIED: <the exact command you ran and its result>'. A DONE with no VERIFIED evidence is treated as IN_PROGRESS.\n\
STATUS: IN_PROGRESS  - you made progress but the item is not done yet.\n\
STATUS: BLOCKED  - you cannot proceed without a human (a decision, access, credentials, hardware, an external dependency). Say exactly what you need above. This MOVES the item to the board's BLOCKED column: the plan loop stops stepping it and goes on to the next task until a human unblocks it, so use BLOCKED instead of describing a blocker under IN_PROGRESS - and do not use it for problems you could still work around yourself.\n\
Only claim DONE when you actually verified completion against the goal, not when you merely believe it.\n\n\
GOAL: {}\n", goal);
// (B) STOP TRUNCATING DETAIL (owner, 2026-10): the floweditor3d postmortem -
// a phased plan whose `detail` was cut at 3000 chars dropped the LATER phases
// from the step prompt, so the deciding step could not see the remaining work
// and the goal-scope gate alone can't fix what it can't read. Carry the FULL
// detail; the bound lives at the source (the human-authored item), not a blind
// cut here. A phased plan should be DECOMPOSED into child items (A), whose
// per-phase detail rides each child's own step prompt in full.
if !detail.trim().is_empty() {
    prompt.push_str(&format!("\nDETAIL:\n{}\n", detail));
}
if prefer_structured || recovered_run {
    prompt.push_str(&format!("\nCONTINUITY v2 — ACTUAL HANDOFF FROM RUN {} (recovery: {}):\n{}\nThis is durable working state, not a hidden transcript. Preserve verified evidence separately from unverified changes, keep the unresolved next action and blockers, and do not restart the goal from scratch. Inspect activity_history if you need more concrete outcomes.\n", previous_run_id, if continuity_recovery_kind.is_empty() { "checkpoint" } else { continuity_recovery_kind }, handoff_received));
}
if !prefer_structured && !prior_steps.is_empty() {
    prompt.push_str(&format!("\nCONTINUITY ACROSS TURNS (this is your actual handoff, not a hidden replay):\n{}\n", prior_steps));
    if !handoff_received.is_empty() {
        prompt.push_str("\nTreat the latest handoff above as your working state. Preserve its verified facts, changed targets, unresolved error, and next action. Do not restart the goal from scratch or repeat an identical failed call without new information.\n");
    }
    if !previous_run_id.is_empty() {
        prompt.push_str(&format!("\nPREVIOUS RUN: {}. Inspect it with dev-code-invoke_command lib=agent ctl=plan cmd=activity_history args={{run_id,after:0,limit:400}} when you need concrete tool outcomes not present above. Read-only activity_status shows the live/recent run. Runtime history is bounded and instance-local; kb.plan.step_log is the durable handoff.\n", previous_run_id));
    }
}
// (C) HUMAN NOTES (agent.plan.add_note, from the kanban card): the owner's
// running guidance on an item, appended between steps. EVERY note rides the
// prompt in full (they are human-authored and bounded at the source, 8000
// chars each); the ones this loop has not consumed yet (no `seen_at`) are
// flagged NEW so a mid-flight correction is noticed on the very next trip to
// the LLM instead of being buried under the original detail. The post-turn
// write stamps seen_at on exactly the notes presented here (matched by time),
// so a note added DURING the turn stays NEW for the following step.
let mut notes_presented: Vec<i64> = Vec::new();
let mut notes_new_count: i64 = 0;
if let Ok(na) = e.try_get_array("notes") {
    let mut lines_new: Vec<String> = Vec::new();
    let mut lines_old: Vec<String> = Vec::new();
    for i in 0..na.len() {
        if let Ok(n) = na.try_get_object(i) {
            let txt = n.try_get_string("text").unwrap_or_default();
            if txt.trim().is_empty() { continue; }
            let who = n.try_get_string("author").unwrap_or_default();
            let t = n.try_get_int("time").unwrap_or(0);
            notes_presented.push(t);
            let body = txt.trim().replace('\n', "\n  ");
            let line = format!("- ({}) {}", if who.is_empty() { "owner".to_string() } else { who }, body);
            if n.has("seen_at") { lines_old.push(line); } else { lines_new.push(line); }
        }
    }
    notes_new_count = lines_new.len() as i64;
    if !lines_new.is_empty() || !lines_old.is_empty() {
        prompt.push_str("\nHUMAN NOTES on this item (added by the owner from the plan board between steps; they refine the GOAL/DETAIL and OUTRANK them, and any prior step, where they conflict):\n");
        if !lines_new.is_empty() {
            prompt.push_str(&format!("NEW since your last step - read these first and act on them this step:\n{}\n", lines_new.join("\n")));
        }
        if !lines_old.is_empty() {
            prompt.push_str(&format!("Earlier notes (seen on a prior step, still in force):\n{}\n", lines_old.join("\n")));
        }
    }
}
out.put_int("notes_new", notes_new_count);
// (A2) THE COORDINATION TURN: a parent sees the full LIVE state of its
// children and a modified mandate. Board order = authored dependency order;
// the parent's own detail may narrow or override it. One active child at a
// time is the default; explicitly independent siblings may be accepted
// together when the detail says so. A blocked child may be re-accepted ONLY
// with a written remedy. The turn is IN_PROGRESS until every child is
// completed/done; a premature DONE is REJECTED below (premature_done) and the
// item keeps its lifecycle - the final turn, once all children are done, is
// the normal verified close.
if is_parent {
    let mut open_kids: i64 = 0;
    let mut lines: Vec<String> = Vec::new();
    for c in children.iter() {
        let cc = c.try_get_string("claim").unwrap_or_default();
        let ct = c.try_get_string("tags").unwrap_or_default().to_lowercase();
        let cts: Vec<&str> = ct.split(',').map(|t| t.trim()).collect();
        let lc = ["proposed","accepted","in-progress","blocked","completed","done","abandoned"]
            .iter().find(|l| cts.iter().any(|t| *t == **l)).map(|s| s.to_string())
            .unwrap_or_else(|| "none".to_string());
        let csteps = c.try_get_array("step_log").map(|a| a.len() as i64).unwrap_or(0);
        let latest = c.try_get_object("latest").ok()
            .and_then(|l| l.try_get_string("note").ok()).unwrap_or_default()
            .chars().take(240).collect::<String>();
        lines.push(format!("- [{}] ({} steps) {}{}", lc, csteps, cc,
            if latest.is_empty() { String::new() } else { format!(" | latest: {}", latest) }));
        if lc != "completed" && lc != "done" { open_kids += 1; }
    }
    out.put_int("children_open", open_kids);
    prompt.push_str("\nTHIS IS A PARENT COORDINATION TICKET. Your job THIS TURN is to coordinate its children, not to implement the whole goal yourself:\n");
    prompt.push_str("1. Re-read the CHILDREN board below and your PRIOR STEPS; evaluate progress and dependencies.\n");
    prompt.push_str("2. Choose the next child to advance (board order is the authored dependency order unless the DETAIL says otherwise; ONE active child at a time is the default - explicitly independent siblings only as the DETAIL permits).\n");
    prompt.push_str("3. Act through agent.plan.move_item with a FRESH base from agent.plan.board (never force a stale write): move the chosen child proposed -> accepted (a coordinator acceptance of its OWN child; the item's full detail is the authorization). Corrections or addenda to a child's text belong in a note (agent.plan.add_note) - do NOT rewrite its claim or detail.\n");
    prompt.push_str("4. A BLOCKED child may be moved back to accepted ONLY with a concrete, written remedy - your final answer must state the blocker and exactly how the remedy resolves it, and an add_note carrying that remedy is required so the next step sees it. Without a remedy, leave it blocked.\n");
    prompt.push_str("5. Then YIELD: end with STATUS: IN_PROGRESS and a concise status update (which child you promoted and why). The worker now runs an accepted/in-progress child BEFORE another parent turn; do not spend a turn merely confirming that an active child still needs implementation. DONE is only for a later turn, after every child is completed/done AND you have verified their evidence and the integrated outcome - a DONE while any child is unfinished is REJECTED.\n");
    prompt.push_str("6. If every unfinished child is blocked and you have no reasonable remedy for any of them, end with STATUS: BLOCKED naming the actual blocker(s) - this moves the parent itself to the blocked column (no retry loop).\n");
    prompt.push_str("\nCHILDREN (live board state, in board order):\n");
    prompt.push_str(&lines.join("\n"));
    prompt.push_str("\n");
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
// START TRANSITION (owner, 2026-10): accepted -> in-progress at the START of
// the turn, not the end. A tool_loop turn can run for minutes and the human
// has no window into the executive - an item that stayed `accepted` for the
// whole time it was being worked told the kanban board a lie ("approved but
// untouched"). Flip the lifecycle FIRST, in its own journaled, base-hashed
// write, so the card moves to in-progress the moment the agent picks it up.
// On a failed write (stale_base) we abort BEFORE spending the turn - never
// force. The post-turn write re-reads the board (FRESH BASE, below), so this
// extra write never strands the step_log on a stale hash.
fn with_lifecycle(tags: &str, to: &str) -> String {
    let lc = ["proposed","accepted","in-progress","blocked","completed","done","abandoned"];
    let mut kept: Vec<String> = tags.split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty() && !lc.contains(&t.as_str()))
        .collect();
    kept.push(to.to_string());
    kept.join(",")
}
if was_accepted || e.has("active_run") {
    let nt = with_lifecycle(&e.try_get_string("tags").unwrap_or_default(), "in-progress");
    e.put_string("tags", &nt);
    let short0: String = claim.chars().take(50).collect();
    let sw = api.dev.code.patch_control_facet(
        "kb".to_string(), "plan".to_string(), "memory".to_string(),
        "".to_string(), arr.to_string(), hash.clone(),
        format!("plan started/resumed (in-progress): '{}'", short0),
        "executive".to_string(), nn_sessionid.clone());
    let sok = sw.try_get_boolean("ok").unwrap_or(false)
        || sw.try_get_string("status").map(|s| s == "ok").unwrap_or(false);
    if !sok {
        out.put_string("status", "err");
        out.put_string("action", "start_failed");
        out.put_string("msg", "could not move the item to in-progress before the turn (likely stale_base) - re-read the board and retry; the turn was NOT run");
        if sw.has("msg") { out.put_string("write_msg", &sw.get_string("msg")); }
        if sw.has("current_hash") { out.put_string("current_hash", &sw.get_string("current_hash")); }
        return out;
    }
    out.put_boolean("moved_to_in_progress", true);
}
// Reason + act, panic-contained: ONE bounded autonomous turn via tool_loop.
// Dispatch is a short-lived handoff, not a second plan authority. The loop
// claims it by exact prompt; no tool_loop signature or other caller changes.
let now = flowlang::flowlang::system::time::time();
let run_id = format!("plan-{}-{}", now, flowlang::flowlang::system::unique_session_id::unique_session_id());
// Durable active pointer: if this turn dies before its final journal write,
// the item still identifies the saved run and the checkpoint it inherited.
let mut active_pointer_saved = false;
{
    let r0 = api.dev.code.read_control_facet("kb".to_string(), "plan".to_string(), "memory".to_string());
    let h0 = r0.get_string("hash");
    if let Ok(p0) = DataObject::try_from_string(&format!("{{\"a\":{}}}", r0.get_string("source"))) {
        let a0 = p0.get_array("a");
        for i in 0..a0.len() {
            let mut item = a0.get_object(i);
            if item.try_get_string("claim").unwrap_or_default() != claim { continue; }
            let mut active = DataObject::new(); active.put_string("run_id", &run_id);
            active.put_string("claim", &claim); active.put_int("turn", prior + 1);
            active.put_int("started", now); active.put_string("state", "starting");
            active.put_string("previous_run_id", &previous_run_id);
            active.put_boolean("recovered", recovered_run);
            if prefer_structured { active.put_object("inherited", inherited_structured.clone()); }
            else if !handoff_received.is_empty() { active.put_string("inherited_text", &handoff_received); }
            item.put_object("active_run", active);
            let aw = api.dev.code.patch_control_facet("kb".to_string(), "plan".to_string(), "memory".to_string(),
                "".to_string(), a0.to_string(), h0, "plan run started; durable active pointer".to_string(), "executive".to_string(), nn_sessionid.clone());
            active_pointer_saved = aw.try_get_boolean("ok").unwrap_or(false)
                || aw.try_get_string("status").map(|s| s == "ok").unwrap_or(false);
            break;
        }
    }
}
if !active_pointer_saved {
    out.put_string("status", "err"); out.put_string("action", "start_failed");
    out.put_string("msg", "could not journal the active run pointer before the turn (likely stale_base); no model turn was run");
    return out;
}
let mut context_manifest = DataObject::new();
context_manifest.put_int("turn", prior + 1);
context_manifest.put_int("max_turns", cap);
context_manifest.put_int("prompt_chars", prompt.chars().count() as i64);
context_manifest.put_int("detail_chars", detail.chars().count() as i64);
context_manifest.put_int("prior_chars", prior_steps.chars().count() as i64);
context_manifest.put_int("notes_presented", notes_presented.len() as i64);
context_manifest.put_int("notes_new", notes_new_count);
context_manifest.put_string("detail_policy", "full");
context_manifest.put_string("checkpoint_policy", "latest continuation in full, bounded at 8000 characters");
context_manifest.put_string("history_policy", "recent non-checkpoint steps bounded at 1200 characters; inspect run activity for details");
context_manifest.put_string("previous_run_id", &previous_run_id);
context_manifest.put_string("handoff_received", &handoff_received);
context_manifest.put_int("included_steps", continuity.get_int("included_steps"));
context_manifest.put_int("available_steps", continuity.get_int("available_steps"));
context_manifest.put_boolean("recovered_from_unjournaled_run", recovered_run);
context_manifest.put_boolean("structured_checkpoint", prefer_structured);
if prefer_structured { context_manifest.put_object("inherited_checkpoint", inherited_checkpoint.clone()); }
context_manifest.put_string("source", if prefer_structured { "kb.plan.checkpoint" } else if recovered_run { "runtime recovery" } else { "kb.plan.step_log" });
context_manifest.put_string("recovery_kind", continuity_recovery_kind);
{
    let mut globals = flowlang::datastore::DataStore::globals();
    let mut d = DataObject::new(); d.put_string("run_id", &run_id);
    d.put_string("prompt", &prompt); d.put_string("claim", &claim); d.put_int("turn", prior + 1);
    d.put_string("previous_run_id", &previous_run_id);
    d.put_boolean("recovered", recovered_run);
    if prefer_structured { d.put_object("inherited_checkpoint", inherited_checkpoint.clone()); }
    d.put_object("context", context_manifest); globals.put_object("AGENT_PLAN_DISPATCH", d);
}
let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    let cmd = flowlang::command::Command::lookup("agent", "llm", "tool_loop");
    let mut a = DataObject::new();
    a.put_string("prompt", &prompt);
    cmd.execute(a)
}));
{
    let mut globals = flowlang::datastore::DataStore::globals();
    if let Ok(d) = globals.try_get_object("AGENT_PLAN_DISPATCH") {
        if d.try_get_string("run_id").unwrap_or_default() == run_id { globals.remove_property("AGENT_PLAN_DISPATCH"); }
    }
}
out.put_string("run_id", &run_id);
// A panic inside tool_loop bypasses its own finish event. Close the activity
// run here even though the service process remains alive; otherwise the UI
// would confuse a dead turn with a live one. Recovery uses the last saved
// observation below, never an assumption that unfinished work succeeded.
let loop_returned = matches!(&ran, Ok(Ok(_)));
if !loop_returned {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let cmd = flowlang::command::Command::lookup("agent", "plan", "activity_emit");
        let mut event = DataObject::new(); event.put_string("kind", "finish");
        event.put_string("phase", "interrupted"); event.put_string("state", "interrupted");
        event.put_string("outcome", "ERROR");
        event.put_string("detail", "The plan tool loop returned an execution error or panicked before its finish boundary; recover from the last saved observation.");
        let mut a = DataObject::new(); a.put_string("run_id", &run_id);
        a.put_object("event", event); let _ = cmd.execute(a);
    }));
}
let read_run_structured = |rid: &str| -> DataObject {
    if rid.is_empty() { return DataObject::new(); }
    let store = flowlang::datastore::DataStore::new();
    if !store.exists("runtime", "agent_plan_runs") { return DataObject::new(); }
    let runs = store.get_data("runtime", "agent_plan_runs").get_object("data").get_array("runs");
    for i in 0..runs.len() {
        let r = runs.get_object(i);
        if r.try_get_string("run_id").unwrap_or_default() == rid {
            if let Ok(h) = r.try_get_object("handoff") {
                if h.has("summary") { return h; }
            }
        }
    }
    DataObject::new()
};
let read_run_checkpoint = |rid: &str| -> String {
    if rid.is_empty() { return String::new(); }
    let store = flowlang::datastore::DataStore::new();
    if !store.exists("runtime", "agent_plan_runs") { return String::new(); }
    let runs = store.get_data("runtime", "agent_plan_runs").get_object("data").get_array("runs");
    for i in 0..runs.len() {
        let r = runs.get_object(i);
        if r.try_get_string("run_id").unwrap_or_default() == rid {
            if let Ok(h) = r.try_get_object("handoff") { return h.try_get_string("checkpoint").unwrap_or_default(); }
        }
    }
    String::new()
};
let (ok_step, result_json, tl_status, handoff_checkpoint, actual_run_id, structured_checkpoint) = match ran {
    Ok(Ok(rr)) => {
        // Log the FINAL ANSWER in FULL (owner, 2026-10) - never the messages
        // transcript (a 143KB tool_loop dump per step would bloat kb.plan and
        // poison every later read; that guard stands). The old 1500-char cap
        // dropped exactly the context the next step needed: the phase ledger
        // and the verification detail. Older entries are re-truncated to 200
        // chars when re-fed as PRIOR STEPS, and the kanban `latest` pulse
        // stays at 240, so prompt + card size remain bounded. FLAT may nest
        // under "a"; pull msg + status from either.
        let mut answer = rr.try_get_string("msg").unwrap_or_default();
        let mut st = rr.try_get_string("status").unwrap_or_default();
        if answer.is_empty() || st.is_empty() {
            if let Ok(a) = rr.try_get_object("a") {
                if answer.is_empty() { answer = a.try_get_string("msg").unwrap_or_default(); }
                if st.is_empty() { st = a.try_get_string("status").unwrap_or_default(); }
            }
        }
        if answer.is_empty() { answer = rr.to_string(); }
        if st.is_empty() { st = "IN_PROGRESS".to_string(); }
        let actual_run = rr.try_get_string("run_id").ok()
            .or_else(|| rr.try_get_object("a").ok().and_then(|a| a.try_get_string("run_id").ok()))
            .unwrap_or_else(|| run_id.clone());
        let checkpoint = read_run_checkpoint(&actual_run);
        let checkpoint = if !checkpoint.is_empty() { checkpoint } else {
            format!("CONTINUATION FROM PREVIOUS PLAN TURN\nSTATUS: {}\nRESPONSE / EVIDENCE:\n{}\nNEXT: Continue from the stated response and verify a new concrete increment. The preceding run did not supply a checkpoint; inspect its run activity and do not repeat a failed call without new information.", st, answer.chars().take(7000).collect::<String>())
        };
        let mut structured = read_run_structured(&actual_run);
        if !structured.has("summary") {
            let mut ci = DataObject::new(); ci.put_string("response", &answer);
            ci.put_string("status", &st); ci.put_string("run_id", &actual_run);
            ci.put_string("previous_run_id", &previous_run_id);
            ci.put_string("reason", "Checkpoint synthesized at the planner boundary because the loop returned no structured handoff.");
            structured = crate::agent::plan::checkpoint::checkpoint(ci);
        }
        (true, answer, st, checkpoint, actual_run, structured)
    }
    Ok(Err(pe)) => {
        let fallback = read_run_checkpoint(&run_id);
        let text = if fallback.is_empty() { "The previous run returned an execution error. Inspect its activity, diagnose the cause, and take a different verified approach.".to_string() } else { fallback };
        let structured = read_run_structured(&run_id);
        (false, format!("{{\"error\":\"{}\"}}", esc(&format!("{:?}", pe))), "ERROR".to_string(), text, run_id.clone(), structured)
    },
    Err(_) => {
        let fallback = read_run_checkpoint(&run_id);
        let text = if fallback.is_empty() { "The previous run panicked before a model checkpoint. Inspect its last recorded activity and resume with a different approach; do not assume unfinished writes were verified.".to_string() } else { fallback };
        let structured = read_run_structured(&run_id);
        (false, "{\"error\":\"panic during executive reasoning\"}".to_string(), "ERROR".to_string(), text, run_id.clone(), structured)
    },
};
// FRESH BASE after the turn: the turn can run for minutes, the start
// transition above already changed the board hash, and the turn itself (or a
// human dragging a card) may write kb.plan meanwhile. Re-read the board,
// re-locate this entry by claim, and journal the step onto the CURRENT state
// under the CURRENT hash. The final write is still base-hashed, so a write
// racing THIS read still aborts - we never force.
let r2 = api.dev.code.read_control_facet("kb".to_string(), "plan".to_string(), "memory".to_string());
let hash = r2.get_string("hash");
let parsed2 = match DataObject::try_from_string(&format!("{{\"a\": {} }}", r2.get_string("source"))) {
    Ok(o) => o,
    Err(_) => {
        out.put_string("status", "err");
        out.put_string("msg", "step ran but kb.plan no longer parses as a JSON array - step_log not written");
        out.put_string("result", &result_json);
        return out;
    }
};
let arr = parsed2.get_array("a");
let mut fidx: i64 = -1;
for i in 0..arr.len() {
    if arr.get_object(i).try_get_string("claim").unwrap_or_default() == claim { fidx = i as i64; break; }
}
if fidx < 0 {
    out.put_string("status", "err");
    out.put_string("msg", "step ran but the plan entry vanished from kb.plan during the turn - step_log not written");
    out.put_string("result", &result_json);
    return out;
}
let mut e = arr.get_object(fidx as usize);
// Is the item still live in the agent's lane (a human may have moved it
// mid-turn)? Only a live item may be auto-completed below.
let live_now = {
    let t = e.try_get_string("tags").unwrap_or_default().to_lowercase();
    t.split(',').map(|x| x.trim()).any(|x| x == "accepted" || x == "in-progress")
};
// (5) Append to the entry's step_log and write the facet back journaled.
let mut log = if e.has("step_log") {
    match e.try_get_array("step_log") { Ok(a) => a, Err(_) => DataArray::new() }
} else { DataArray::new() };
let mut entry_log = DataObject::new();
entry_log.put_int("time", now);
entry_log.put_string("home", "agent.llm.tool_loop");
entry_log.put_boolean("ok", ok_step);
entry_log.put_string("result", &result_json);
entry_log.put_string("checkpoint", &handoff_checkpoint);
entry_log.put_object("checkpoint_v2", structured_checkpoint.clone());
entry_log.put_string("run_id", &actual_run_id);
entry_log.put_string("previous_run_id", &previous_run_id);
entry_log.put_boolean("recovered", recovered_run);
entry_log.put_int("prompt_chars", prompt.chars().count() as i64);
entry_log.put_string("author", "executive");
entry_log.put_int("notes_presented", notes_presented.len() as i64);
entry_log.put_int("notes_new", notes_new_count);
log.push_object(entry_log);
e.put_array("step_log", log);
// Durable compact continuity facet. Runtime activity may roll off; the
// checkpoint and its provenance remain with the item and its journal.
e.put_object("checkpoint", structured_checkpoint.clone());
let mut last_run = DataObject::new(); last_run.put_string("run_id", &actual_run_id);
last_run.put_string("previous_run_id", &previous_run_id); last_run.put_int("time", now);
last_run.put_string("outcome", &tl_status); last_run.put_boolean("recovered", recovered_run);
last_run.put_boolean("ok", ok_step); last_run.put_int("turn", prior + 1);
e.put_object("last_run", last_run);
e.remove_property("active_run");
let mut continuity_state = DataObject::new();
continuity_state.put_int("version", 2); continuity_state.put_string("source", "kb.plan.checkpoint");
continuity_state.put_string("run_id", &actual_run_id);
continuity_state.put_string("previous_run_id", &previous_run_id);
continuity_state.put_string("handoff_received", &handoff_received);
continuity_state.put_int("prompt_chars", prompt.chars().count() as i64);
continuity_state.put_boolean("recovered_from_unjournaled_run", recovered_run);
continuity_state.put_string("recovery_kind", continuity_recovery_kind);
continuity_state.put_boolean("structured", structured_checkpoint.has("summary"));
continuity_state.put_string("delivery", if ok_step { "provider responded to the context prompt" } else { "no completed provider exchange" });
e.put_object("continuity", continuity_state);
// (C) mark the notes this turn was SHOWN as consumed - only on a turn that
// actually reached the model (ok_step); a failed/panicked turn leaves them NEW
// so the retry still flags them. Matched by note time, so a note appended
// mid-turn (re-read into this fresh entry) stays unseen for the next step.
if ok_step && !notes_presented.is_empty() {
    if let Ok(na) = e.try_get_array("notes") {
        for i in 0..na.len() {
            if let Ok(mut n) = na.try_get_object(i) {
                if !n.has("seen_at") && notes_presented.contains(&n.try_get_int("time").unwrap_or(-1)) {
                    n.put_int("seen_at", now);
                }
            }
        }
    }
}
// ROLLING-LATEST (owner, 2026-10-21): the human had NO window into the loop
// except asking the agent - the card showed claim+detail+tags, none of which
// change as work progresses. Write a `latest` field each step: what just
// happened, OVERWRITTEN (never grows; step_log remains the full append-only
// journal). The kanban card renders it, so progress is visible at a glance.
// FULL ENTRY (owner, 2026-10-22): keep the WHOLE body, multi-line, no 240-char
// cap, no line-flattening - the flat one-line pulse was useless; the human
// wants to read the entire step. Prefer the model's own "## Summary" section;
// fall back to the whole final answer. Only the STATUS/VERIFIED trailer lines
// are stripped (they are markers, not prose).
{
    let full = result_json.trim();
    let body = if let Some(pos) = full.find("## Summary") {
        full[pos + "## Summary".len()..].trim()
    } else {
        full
    };
    // keep every line, multi-line; drop only the STATUS:/VERIFIED: trailer lines
    let mut note = String::new();
    for ln in body.lines() {
        let t = ln.trim_end();
        let tt = t.trim_start();
        if tt.starts_with("STATUS:") || tt.starts_with("VERIFIED:") { continue; }
        note.push_str(t);
        note.push('\n');
    }
    let note = note.trim().to_string();
    let mut latest = DataObject::new();
    latest.put_int("time", now);
    latest.put_string("status", &tl_status);
    latest.put_string("note", &note);
    e.put_object("latest", latest);
}
// LIFECYCLE TRANSITIONS on the post-turn write (onto the FRESH board state).
// Both lifecycle moves are within the agent's authority; `done`
// (human-confirmed) stays human-only:
//   accepted -> in-progress : now made at the START of the turn (its own
//     journaled write, START TRANSITION above) - the board tells the truth
//     "the agent is on it" WHILE it works, not only after the turn ends.
//   * -> completed : a VERIFIED DONE (VERIFIED: evidence required) - the agent
//     finished, but a HUMAN hasn't seen it yet. Terminal for the loop (no
//     re-run), visibly awaiting confirmation. Crossing completed -> done is a
//     human kanban act, the same trust boundary as `accepted` itself.
let short: String = claim.chars().take(50).collect();
let mut label = format!("plan step: '{}' -> {}", short, if ok_step { "ok" } else { "err" });
let lifecycles = ["proposed","accepted","in-progress","blocked","completed","done","abandoned"];
let mut set_lifecycle = |e: &mut DataObject, to: &str| {
    let tags = e.try_get_string("tags").unwrap_or_default();
    let mut kept: Vec<String> = tags.split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty() && !lifecycles.contains(&t.as_str()))
        .collect();
    kept.push(to.to_string());
    e.put_string("tags", &kept.join(","));
};
let mut moved_to_blocked = false;
let mut premature_done = false;
if premature_done {
    // A parent reported DONE with unfinished children: REJECTED - the step is
    // journaled above but the lifecycle was not moved and the worker treats
    // this as an ordinary in-progress step (no park, no extra reasoning turn
    // from the coordinator's own board edits).
    out.put_string("action", "premature_done");
    out.put_string("msg", "DONE rejected: this parent still has unfinished children - coordinate them (promote the next child) or verify the finished whole before closing");
} else if tl_status == "DONE" {
    // PARENT ROLLUP GUARD: a coordinating parent may NOT close while any
    // non-abandoned child is unfinished - its DONE is REJECTED: the step is
    // journaled (evidence preserved), the lifecycle stays where it is, and
    // the action is stepped (in-progress) rather than completed. The final
    // coordination turn - with every child completed/done - goes through the
    // normal verified close below. This replaces the old auto-rollup (the
    // last child completing used to complete the parent sight unseen; now the
    // parent always gets a closing turn to verify the integrated outcome).
    let mut premature = false;
    if live_now {
        let ptags = e.try_get_string("tags").unwrap_or_default().to_lowercase();
        if ptags.split(',').any(|x| x.trim() == "in-progress") {
            for i in 0..arr.len() {
                if i as i64 == fidx { continue; }
                let c = arr.get_object(i);
                if c.try_get_string("parent").unwrap_or_default() != claim { continue; }
                let ct = c.try_get_string("tags").unwrap_or_default().to_lowercase();
                let ctoks: Vec<&str> = ct.split(',').map(|t| t.trim()).collect();
                if ctoks.iter().any(|t| *t == "abandoned") { continue; }
                if !ctoks.iter().any(|t| *t == "completed" || *t == "done") { premature = true; break; }
            }
        }
    }
    if premature {
        premature_done = true;
        label = format!("plan step: '{}' -> DONE REJECTED: unfinished children remain - coordinate them, then close with verified evidence", short);
    } else if live_now {
        set_lifecycle(&mut e, "completed");
        label = format!("plan completed (verified, awaiting review): '{}'", short);
    } else {
        // A human moved the card out of the agent's lane mid-turn - their
        // lifecycle wins; journal the step but do not override it.
        label = format!("plan step: '{}' -> DONE claimed, but the item left accepted/in-progress mid-turn; lifecycle left as the human set it", short);
    }
} else if tl_status == "BLOCKED" {
    //   * -> blocked (2026-10): the agent cannot proceed without a human. The
    //     card MOVES to the kanban's `blocked` column in this same atomic
    //     write, instead of only saying so in the log while the item sat
    //     in-progress - a blocked item is not steppable (gate) and not
    //     eligible for the plan worker, so the loop stops spending turns on it
    //     and goes on to the next unblocked item. The blocker text rides
    //     `latest` (visible on the card) and step_log; a human unblocks by
    //     moving the card back to accepted, and the next step reads it.
    if live_now {
        set_lifecycle(&mut e, "blocked");
        moved_to_blocked = true;
        label = format!("plan blocked (needs a human): '{}'", short);
    } else {
        label = format!("plan step: '{}' -> BLOCKED claimed, but the item left accepted/in-progress mid-turn; lifecycle left as the human set it", short);
    }
}
// accepted -> in-progress is no longer done here: it happens at the START of
// the turn (START TRANSITION above) so the board is truthful while we work.
// (A) PARENT ROLLUP: REMOVED (owner, 2026-11). The old code auto-completed a
// parent in this write when its last child completed - the parent then closed
// without ever re-reading the children's evidence or the integrated outcome.
// Now the parent always gets a subsequent coordination turn: it stays
// eligible after its last child completes, verifies the whole, and closes
// itself with a VERIFIED DONE (the premature-DONE rejection above guards the
// path). Child completion here only journals the child itself.
let pw = api.dev.code.patch_control_facet(
    "kb".to_string(), "plan".to_string(), "memory".to_string(),
    "".to_string(), arr.to_string(), hash.clone(), label, "executive".to_string(), nn_sessionid.clone());
let write_ok = pw.try_get_boolean("ok").unwrap_or(false)
    || pw.try_get_string("status").map(|s| s == "ok").unwrap_or(false);
if !write_ok {
    out.put_string("status", "err");
    out.put_string("msg", "step ran but the step_log write failed (likely stale_base) - re-read the board and retry");
    if pw.has("msg") { out.put_string("write_msg", &pw.get_string("msg")); }
    return out;
}
// COMPLETION TRANSITION: the step_log (and, on DONE, the done tag) is journaled
// in the single atomic write above; now report the model's declared status.
// This is what ends the loop - without it a finished item stays `accepted` and
// gets re-proved at full budget every 15 min forever. Trust boundary (owner):
// `accepted` is human-only, but `done` is a VERIFIED claim - the model must
// have produced a VERIFIED: evidence line (a bare DONE is parsed as
// IN_PROGRESS by tool_loop), and the transition rides the same journaled,
// base-hashed write a human kanban drag uses (revertible). DONE -> done tag ->
// epistemic_work drops it, the loop ENDS. BLOCKED -> the item MOVES to the
// `blocked` lifecycle (kanban's blocked column) in the write above; the gate
// refuses it and the plan worker skips it, so no further turns are spent until
// a human moves it back to accepted (the next step then reads the blocker and
// any notes). IN_PROGRESS -> normal continuation under MAX_PLAN_STEPS.
out.put_string("loop_status", &tl_status);
out.put_string("status", "ok");
if premature_done {
    // A parent reported DONE with unfinished children: REJECTED - the step is
    // journaled above but the lifecycle was not moved and the worker treats
    // this as an ordinary in-progress step (no park, no extra reasoning turn
    // from the coordinator's own board edits).
    out.put_string("action", "premature_done");
    out.put_string("msg", "DONE rejected: this parent still has unfinished children - coordinate them (promote the next child) or verify the finished whole before closing");
} else if tl_status == "DONE" {
    // The `completed` tag was applied atomically in the write above - the agent
    // finished with verified evidence, epistemic_work drops it, the loop ENDS,
    // and the item now awaits HUMAN confirmation (completed -> done).
    out.put_string("action", "completed");
    out.put_boolean("moved_to_completed", write_ok);
} else if tl_status == "BLOCKED" {
    // The `blocked` tag was applied atomically in the write above (when the
    // item was still in the agent's lane): the card sits in the kanban's
    // blocked column, the gate refuses it, and the worker moves on to the
    // next unblocked item until a human moves it back to accepted.
    out.put_string("action", "blocked");
    out.put_boolean("moved_to_blocked", moved_to_blocked);
} else {
    out.put_string("action", if ok_step { "stepped" } else { "step_failed" });
}
out.put_boolean("ok", ok_step);
out.put_string("home", "agent.llm.tool_loop");
out.put_string("result", &result_json);
out.put_int("time", now);
out