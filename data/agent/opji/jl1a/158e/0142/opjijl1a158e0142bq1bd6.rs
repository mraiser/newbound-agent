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
    out.put_string("msg", "gate: only `accepted` or `in-progress` plan items are steppable - proposed stays human-only and the agent never self-promotes");
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
// (A) SUB-ITEM DECOMPOSITION: a PARENT (an entry other items point to via
// `parent`) is a container, not a unit of work - its phases are its CHILDREN,
// each accepted/stepped on its own. Refuse to step a parent that still has
// unfinished children so the loop drives the children instead; when every
// child is terminal (completed/done) the parent itself becomes steppable so it
// can be verified whole and closed (the rollup also fires when the LAST child
// completes, below). Terminal-for-rollup = completed | done (both mean the
// work is finished; done is only the human's confirm on top of completed).
{
    let mut unfinished: i64 = 0;
    for i in 0..arr.len() {
        if i as i64 == idx { continue; }
        let c = arr.get_object(i);
        if c.try_get_string("parent").unwrap_or_default() != claim { continue; }
        let ct = c.try_get_string("tags").unwrap_or_default().to_lowercase();
        let ctoks: Vec<&str> = ct.split(',').map(|t| t.trim()).collect();
        let child_done = ctoks.iter().any(|t| *t == "completed" || *t == "done");
        if !child_done { unfinished += 1; }
    }
    if unfinished > 0 {
        out.put_string("status", "ok");
        out.put_string("action", "gated");
        out.put_int("children_open", unfinished);
        out.put_string("msg", "this is a PARENT with unfinished child phases - the children are the unit of work; accept and step each child, and the parent closes once they are all done");
        return out;
    }
}
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
let mut prior_steps = String::new();
if e.has("step_log") {
    if let Ok(sl) = e.try_get_array("step_log") {
        let n = sl.len();
        if n > 0 {
            // Feed the LATEST [budget checkpoint] IN FULL - it is the rolling
            // thread the next turn resumes from, and truncating it reintroduces
            // the amnesia we just fixed (the checkpoint is length-bounded at
            // generation, ~1500 chars, by tool_loop's summary prompt - so the
            // bound lives at the source, not a blind cut here). Ordinary steps
            // stay terse one-liners (200 chars); older checkpoints are
            // superseded by the newest and drop out.
            let mut latest_ckpt: i64 = -1;
            for i in 0..n {
                if let Ok(le) = sl.try_get_object(i) {
                    if le.try_get_string("result").unwrap_or_default().starts_with("[budget checkpoint") {
                        latest_ckpt = i as i64;
                    }
                }
            }
            let mut parts = Vec::new();
            for i in 0..n {
                if let Ok(le) = sl.try_get_object(i) {
                    let home = le.try_get_string("home").unwrap_or_default();
                    let ok = le.try_get_boolean("ok").unwrap_or(false);
                    let res = le.try_get_string("result").unwrap_or_default();
                    let is_ckpt = res.starts_with("[budget checkpoint");
                    let keep = (i as i64 == latest_ckpt) || (!is_ckpt && (i as i64) >= n as i64 - 3);
                    if !keep { continue; }
                    let res: String = if i as i64 == latest_ckpt { res } else { res.chars().take(200).collect() };
                    parts.push(format!("- [{}] {} -> {}", if ok { "ok" } else { "err" }, home, res));
                }
            }
            prior_steps = parts.join("\n");
        }
    }
}
let mut prompt = format!(
    "You are the executive's plan-step engine. Advance ONE accepted plan item toward completion. \
Think, then act via the available tools. Prefer a single concrete, verifiable increment over a sweep. \
When you finish, END your reply with a status line in EXACTLY this form (last line, no tool call after):\n\
STATUS: DONE  - the item is complete AND you verified it against the goal with a concrete check you actually ran (build/invoke/parity/test). Name that check on the line above, e.g. 'VERIFIED: <the exact command you ran and its result>'. A DONE with no VERIFIED evidence is treated as IN_PROGRESS.\n\
STATUS: IN_PROGRESS  - you made progress but the item is not done yet.\n\
STATUS: BLOCKED  - you cannot proceed; say what you need above.\n\
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
if !prior_steps.is_empty() {
    prompt.push_str(&format!("\nPRIOR STEPS (do not repeat without new information):\n{}\n", prior_steps));
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
    let lc = ["proposed","accepted","in-progress","completed","done","abandoned"];
    let mut kept: Vec<String> = tags.split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty() && !lc.contains(&t.as_str()))
        .collect();
    kept.push(to.to_string());
    kept.join(",")
}
if was_accepted {
    let nt = with_lifecycle(&e.try_get_string("tags").unwrap_or_default(), "in-progress");
    e.put_string("tags", &nt);
    let short0: String = claim.chars().take(50).collect();
    let sw = api.dev.code.patch_control_facet(
        "kb".to_string(), "plan".to_string(), "memory".to_string(),
        "".to_string(), arr.to_string(), hash.clone(),
        format!("plan started (in-progress): '{}'", short0),
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
let now = flowlang::flowlang::system::time::time();
let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    let cmd = flowlang::command::Command::lookup("agent", "llm", "tool_loop");
    let mut a = DataObject::new();
    a.put_string("prompt", &prompt);
    cmd.execute(a)
}));
let (ok_step, result_json, tl_status) = match ran {
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
        (true, answer, st)
    }
    Ok(Err(pe)) => (false, format!("{{\"error\":\"{}\"}}", esc(&format!("{:?}", pe))), "ERROR".to_string()),
    Err(_) => (false, "{\"error\":\"panic during executive reasoning\"}".to_string(), "ERROR".to_string()),
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
entry_log.put_string("author", "executive");
log.push_object(entry_log);
e.put_array("step_log", log);
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
let lifecycles = ["proposed","accepted","in-progress","completed","done","abandoned"];
let mut set_lifecycle = |e: &mut DataObject, to: &str| {
    let tags = e.try_get_string("tags").unwrap_or_default();
    let mut kept: Vec<String> = tags.split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty() && !lifecycles.contains(&t.as_str()))
        .collect();
    kept.push(to.to_string());
    e.put_string("tags", &kept.join(","));
};
if tl_status == "DONE" {
    if live_now {
        set_lifecycle(&mut e, "completed");
        label = format!("plan completed (verified, awaiting review): '{}'", short);
    } else {
        // A human moved the card out of the agent's lane mid-turn - their
        // lifecycle wins; journal the step but do not override it.
        label = format!("plan step: '{}' -> DONE claimed, but the item left accepted/in-progress mid-turn; lifecycle left as the human set it", short);
    }
}
// accepted -> in-progress is no longer done here: it happens at the START of
// the turn (START TRANSITION above) so the board is truthful while we work.
// (A) PARENT ROLLUP, ATOMIC with this write: when the item that just completed
// is a CHILD (it carries `parent`) and that was its LAST unfinished sibling,
// the parent has no remaining work - mark the parent `completed` in the SAME
// journaled patch so a parent is done only when all its children are done.
// The parent tags the array index (entry ordering is stable within a single
// in-memory rewrite), so we recompute over the array AFTER this child's tag
// change is already reflected in `arr`.
let mut rolled_parent = String::new();
if tl_status == "DONE" && e.has("parent") {
    let p = e.get_string("parent");
    if !p.is_empty() {
        let mut pidx: i64 = -1;
        let mut all_done = true;
        for i in 0..arr.len() {
            let c = arr.get_object(i);
            if c.try_get_string("claim").unwrap_or_default() == p { pidx = i as i64; continue; }
            if c.try_get_string("parent").unwrap_or_default() != p { continue; }
            let ct = c.try_get_string("tags").unwrap_or_default().to_lowercase();
            let ctoks: Vec<&str> = ct.split(',').map(|t| t.trim()).collect();
            let cd = ctoks.iter().any(|t| *t == "completed" || *t == "done");
            if !cd { all_done = false; }
        }
        if pidx >= 0 && all_done {
            let mut pe = arr.get_object(pidx as usize);
            let ptags = pe.try_get_string("tags").unwrap_or_default().to_lowercase();
            let ptoks: Vec<&str> = ptags.split(',').map(|t| t.trim()).collect();
            if !ptoks.iter().any(|t| *t == "completed" || *t == "done") {
                set_lifecycle(&mut pe, "completed");
                rolled_parent = p.clone();
                label = format!("plan completed (verified): '{}' -> parent '{}' completed (all children done)", short, p);
            }
        }
    }
}
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
// epistemic_work drops it, the loop ENDS. BLOCKED -> surface once; the item
// stays accepted but the blocker rides the next prompt so the agent addresses
// it instead of re-proving. IN_PROGRESS -> normal continuation under
// MAX_PLAN_STEPS.
out.put_string("loop_status", &tl_status);
out.put_string("status", "ok");
if tl_status == "DONE" {
    // The `completed` tag was applied atomically in the write above - the agent
    // finished with verified evidence, epistemic_work drops it, the loop ENDS,
    // and the item now awaits HUMAN confirmation (completed -> done).
    out.put_string("action", "completed");
    out.put_boolean("moved_to_completed", write_ok);
} else if tl_status == "BLOCKED" {
    // Surface once; the blocker rides the next prompt so the agent addresses it
    // rather than re-proving. Item stays in-progress.
    out.put_string("action", "blocked");
} else {
    out.put_string("action", if ok_step { "stepped" } else { "step_failed" });
}
out.put_boolean("ok", ok_step);
if !rolled_parent.is_empty() { out.put_string("rolled_up_parent", &rolled_parent); }
out.put_string("home", "agent.llm.tool_loop");
out.put_string("result", &result_json);
out.put_int("time", now);
out