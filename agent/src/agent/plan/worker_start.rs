use ndata::dataobject::DataObject;
use ndata::dataarray::DataArray;
use flowlang::datastore::DataStore;
use flowlang::command::Command;
use flowlang::flowlang::system::time::time;
use std::collections::HashMap;
pub fn execute(_: DataObject) -> DataObject {
    use std::panic;
    let ax = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        worker_start()
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

pub fn worker_start() -> DataObject {
// agent.plan.worker_start - THE PLAN WORKER: kb.plan autonomy on its own
// loop, decoupled from the executive's drive (owner, 2026-10: plan autonomy
// should run full speed on plan items and stop when it runs out of tasks).
//
// PLAN_AUTONOMY (runtime/agent/botd.properties - persists across restarts)
// is the ONE switch, read live every pass. Armed, the worker steps the next
// eligible item through agent.plan.step_item and goes straight on to the
// next: the LLM's own latency is the only pacing. When nothing is eligible it
// makes NO LLM calls - it re-reads only the kb.plan facet hash every 5s and
// re-evaluates the board when that hash moves or a parked item comes due.
//
// Eligible = accepted | in-progress, not a parent with open children, under
// MAX_PLAN_STEPS (else listed as awaiting re-approval), not parked.
// In-progress before accepted (finish what was started), then board order.
//
// Full speed needs a failure guard, or a stuck item retries at full speed:
//   stepped / completed  -> next pass immediately
//   step_failed          -> retry after 5m, doubling to 1h (transient: API)
//   start_failed         -> retry after 30s (board write race)
//   anything else (blocked, defer_stale, needs_reapproval, gated, error)
//                        -> parked until a human changes that ENTRY
// The park fingerprint is the entry's tags/detail/parent/step-count as of
// AFTER the step, so the worker's own step_log write never unparks it and
// other items' steps don't either; a human edit, move, or re-accept does.
// Toggling PLAN_AUTONOMY off and on clears all parking.
//
// PLAN_STEPS_PER_HOUR (unset or 0 = unlimited, the default): a sliding-hour
// cap on reasoning turns (stepped | completed | blocked | step_failed).
//
// One step at a time: step_item's journal writes are base-hashed against the
// whole board, so parallel steps would abort each other.
//
// Restartable: each call bumps a generation; an older thread exits at its
// next check, and a newer one won't begin a step while an older one is still
// mid-step. A run-once boot timer (worker_boot) calls this at startup.
// Observability: globals AGENT_PLAN_WORKER, read by agent.plan.worker_status.
const KEY: &str = "AGENT_PLAN_WORKER";
struct Park { fp: String, until: i64, reason: String }
fn rprop(key: &str) -> Option<String> {
    let s = DataStore::globals().try_get_object("system").ok()?;
    let a = s.try_get_object("apps").ok()?;
    let g = a.try_get_object("agent").ok()?;
    let r = g.try_get_object("runtime").ok()?;
    match r.try_get_string(key) {
        Ok(v) if !v.trim().is_empty() => Some(v.trim().to_string()),
        _ => None,
    }
}
fn autonomy_on() -> bool {
    rprop("PLAN_AUTONOMY").map(|v| {
        let v = v.to_lowercase();
        v == "on" || v == "true" || v == "1" || v == "yes"
    }).unwrap_or(false)
}
fn per_hour() -> i64 {
    rprop("PLAN_STEPS_PER_HOUR").and_then(|v| v.parse::<i64>().ok()).filter(|n| *n > 0).unwrap_or(0)
}
fn max_steps() -> i64 {
    rprop("MAX_PLAN_STEPS").and_then(|v| v.parse::<i64>().ok()).filter(|n| *n > 0)
        .map(|n| n.min(1000)).unwrap_or(20)
}
fn state() -> DataObject {
    let mut g = DataStore::globals();
    if !g.has(KEY) {
        let mut w = DataObject::new();
        w.put_int("gen", 0);
        w.put_string("phase", "stopped");
        w.put_boolean("in_step", false);
        w.put_int("in_step_gen", 0);
        w.put_int("turns_total", 0);
        w.put_int("completed_total", 0);
        w.put_int("turns_last_hour", 0);
        w.put_int("eligible", 0);
        w.put_int("idle_since", 0);
        w.put_int("resume_at", 0);
        w.put_string("current", "");
        w.put_string("next", "");
        w.put_array("parked", DataArray::new());
        w.put_array("awaiting_reapproval", DataArray::new());
        g.put_object(KEY, w);
    }
    g.get_object(KEY)
}
fn nap(gen: i64, secs: u64) {
    for _ in 0..secs {
        std::thread::sleep(std::time::Duration::from_secs(1));
        if state().try_get_int("gen").unwrap_or(0) != gen { return; }
    }
}
fn tagset(e: &DataObject) -> Vec<String> {
    e.try_get_string("tags").unwrap_or_default().to_lowercase()
        .split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect()
}
// Deterministic (DataObject key order is not): the fields a human changes.
fn fingerprint(e: &DataObject) -> String {
    // notes count included: a human note (agent.plan.add_note) is a change to
    // the entry, so it un-parks a blocked/parked item and the next step reads it
    format!("{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
        e.try_get_string("tags").unwrap_or_default(),
        e.try_get_string("detail").unwrap_or_default(),
        e.try_get_string("parent").unwrap_or_default(),
        e.try_get_array("step_log").map(|a| a.len()).unwrap_or(0),
        e.try_get_array("notes").map(|a| a.len()).unwrap_or(0))
}
// A human note the agent has NOT been shown yet: unseen (no seen_at) and newer
// than the start of the last step (step_log entries carry the turn-start time).
// The park fingerprint is taken AFTER a step, so a note typed DURING the turn
// is already inside it - this catches that case and un-parks the item so the
// note rides the very next prompt. A note that WAS presented to a failed turn
// predates that step's start, so it never bypasses step_failed backoff.
fn fresh_note(e: &DataObject) -> bool {
    let notes = match e.try_get_array("notes") { Ok(a) => a, Err(_) => return false };
    let last_step = e.try_get_array("step_log").ok()
        .and_then(|sl| if sl.len() > 0 { sl.try_get_object(sl.len() - 1).ok() } else { None })
        .and_then(|le| le.try_get_int("time").ok())
        .unwrap_or(0);
    for i in 0..notes.len() {
        if let Ok(n) = notes.try_get_object(i) {
            if n.has("seen_at") { continue; }
            if n.try_get_int("time").unwrap_or(0) > last_step { return true; }
        }
    }
    false
}
// The wrapper stays alive until the counted array handle is taken.
fn parse_board(src: &str) -> Vec<DataObject> {
    let wr = match DataObject::try_from_string(&format!("{{\"a\":{}}}", src)) { Ok(w) => w, Err(_) => return Vec::new() };
    let arr = match wr.try_get_array("a") { Ok(a) => a, Err(_) => return Vec::new() };
    let mut v = Vec::new();
    for i in 0..arr.len() { if let Ok(e) = arr.try_get_object(i) { v.push(e); } }
    v
}
fn read_facet() -> (String, String) {
    let api = crate::api::new();
    let f = api.dev.code.read_control_facet("kb".to_string(), "plan".to_string(), "memory".to_string());
    (f.try_get_string("hash").unwrap_or_default(), f.try_get_string("source").unwrap_or_default())
}
fn fp_now(claim: &str) -> String {
    let (_, src) = read_facet();
    for e in parse_board(&src) {
        if e.try_get_string("claim").unwrap_or_default() == claim { return fingerprint(&e); }
    }
    String::new()
}
fn run(gen: i64) {
    let mut parked: HashMap<String, Park> = HashMap::new();
    let mut fails: HashMap<String, u32> = HashMap::new();
    let mut turns: Vec<i64> = Vec::new();
    let mut quiet_hash = String::new();   // board hash at which nothing was eligible
    let mut recheck_at: i64 = i64::MAX;   // earliest parked retry while quiet
    let mut was_on = false;
    loop {
        let mut w = state();
        if w.try_get_int("gen").unwrap_or(0) != gen { return; }
        w.put_int("beat", time());
        let pass = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> u64 {
            let mut w = state();
            let now = time();
            if !autonomy_on() {
                if was_on { parked.clear(); fails.clear(); }
                was_on = false;
                w.put_string("phase", "off");
                w.put_string("current", "");
                w.put_string("next", "");
                return 5;
            }
            if !was_on { was_on = true; quiet_hash.clear(); }
            turns.retain(|t| now - *t < 3_600_000);
            let lim = per_hour();
            w.put_int("turns_last_hour", turns.len() as i64);
            if lim > 0 && (turns.len() as i64) >= lim {
                w.put_string("phase", "rate_limited");
                w.put_int("resume_at", turns[0] + 3_600_000);
                quiet_hash.clear();
                return 5;
            }
            w.put_int("resume_at", 0);
            let (hash, src) = read_facet();
            if !quiet_hash.is_empty() && hash == quiet_hash && now < recheck_at {
                return 5;   // nothing was eligible at this exact board; no change since
            }
            let entries = parse_board(&src);
            let cap = max_steps();
            let mut cands: Vec<(u8, usize, String)> = Vec::new();
            let mut awaiting = DataArray::new();
            for (i, e) in entries.iter().enumerate() {
                let claim = e.try_get_string("claim").unwrap_or_default();
                if claim.trim().is_empty() { continue; }
                let t = tagset(e);
                let has = |s: &str| t.iter().any(|x| x == s);
                if has("done") || has("completed") || has("abandoned") { continue; }
                let prog = has("in-progress");
                if !prog && !has("accepted") { continue; }
                let open_children = entries.iter().filter(|c| {
                    c.try_get_string("parent").unwrap_or_default() == claim
                        && !tagset(c).iter().any(|x| x == "completed" || x == "done")
                }).count();
                if open_children > 0 { continue; }
                let steps = e.try_get_array("step_log").map(|a| a.len() as i64).unwrap_or(0);
                if steps >= cap { awaiting.push_string(&claim); continue; }
                let verdict = parked.get(&claim).map(|p| (p.fp != fingerprint(e) || fresh_note(e), p.until > 0 && now >= p.until));
                match verdict {
                    Some((true, _)) => { parked.remove(&claim); fails.remove(&claim); }
                    Some((false, true)) => { parked.remove(&claim); }
                    Some((false, false)) => continue,
                    None => {}
                }
                cands.push((if prog { 0 } else { 1 }, i, claim));
            }
            parked.retain(|k, _| entries.iter().any(|e| e.try_get_string("claim").unwrap_or_default() == *k));
            cands.sort();
            let mut pk = DataArray::new();
            let mut next_due = i64::MAX;
            for (c, p) in parked.iter() {
                let mut o = DataObject::new();
                o.put_string("claim", c);
                o.put_string("reason", &p.reason);
                o.put_int("until", p.until);
                pk.push_object(o);
                if p.until > 0 && p.until < next_due { next_due = p.until; }
            }
            w.put_array("parked", pk);
            w.put_array("awaiting_reapproval", awaiting);
            w.put_int("eligible", cands.len() as i64);
            if cands.is_empty() {
                quiet_hash = hash;
                recheck_at = next_due;
                if w.try_get_string("phase").unwrap_or_default() != "idle" { w.put_int("idle_since", now); }
                w.put_string("phase", "idle");
                w.put_string("current", "");
                w.put_string("next", "");
                return 5;
            }
            quiet_hash.clear();
            let claim = cands[0].2.clone();
            w.put_string("next", &claim);
            if w.try_get_boolean("in_step").unwrap_or(false) && w.try_get_int("in_step_gen").unwrap_or(0) != gen {
                w.put_string("phase", "waiting for the previous worker's step");
                return 2;
            }
            w.put_boolean("in_step", true);
            w.put_int("in_step_gen", gen);
            w.put_string("phase", "stepping");
            w.put_string("current", &claim);
            w.put_int("current_since", now);
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut a = DataObject::new();
                a.put_string("claim", &claim);
                a.put_string("base", "");
                a.put_string("nn_sessionid", "");
                Command::lookup("agent", "plan", "step_item").execute(a)
            }));
            w.put_boolean("in_step", false);
            let t1 = time();
            let (action, msg, steps_taken) = match res {
                Ok(Ok(rr)) => {
                    let r = match rr.try_get_object("a") { Ok(a) => a, Err(_) => rr.clone() };
                    (r.try_get_string("action").unwrap_or_else(|_| "error".to_string()),
                     r.try_get_string("msg").unwrap_or_default(),
                     r.try_get_int("steps_taken").unwrap_or(-1))
                }
                Ok(Err(_)) => ("step_failed".to_string(), "step_item raised an exception".to_string(), -1),
                Err(_) => ("step_failed".to_string(), "step_item panicked".to_string(), -1),
            };
            let turn = matches!(action.as_str(), "stepped" | "completed" | "blocked" | "step_failed");
            if turn {
                turns.push(now);
                w.put_int("turns_total", w.try_get_int("turns_total").unwrap_or(0) + 1);
            }
            if action == "completed" {
                w.put_int("completed_total", w.try_get_int("completed_total").unwrap_or(0) + 1);
            }
            let mut last = DataObject::new();
            last.put_string("claim", &claim);
            last.put_string("action", &action);
            last.put_string("msg", &msg.chars().take(300).collect::<String>());
            last.put_int("steps_taken", steps_taken);
            last.put_int("time", t1);
            last.put_int("ms", t1 - now);
            w.put_object("last", last);
            match action.as_str() {
                "stepped" | "completed" => { fails.remove(&claim); }
                "step_failed" => {
                    let n = { let c = fails.entry(claim.clone()).or_insert(0); *c += 1; *c };
                    let backoff = (300_000i64 << (n - 1).min(4)).min(3_600_000);
                    parked.insert(claim.clone(), Park { fp: fp_now(&claim), until: t1 + backoff,
                        reason: format!("step_failed x{} - auto-retry", n) });
                }
                "start_failed" => {
                    parked.insert(claim.clone(), Park { fp: fp_now(&claim), until: t1 + 30_000,
                        reason: "start_failed (board write race) - auto-retry".to_string() });
                }
                other => {
                    parked.insert(claim.clone(), Park { fp: fp_now(&claim), until: 0,
                        reason: format!("{} - waits for a change to this item", other) });
                }
            }
            w.put_string("current", "");
            0
        }));
        let secs = match pass {
            Ok(s) => s,
            Err(_) => { state().put_string("last_error", "a pass panicked - retrying in 30s"); 30 }
        };
        if secs > 0 { nap(gen, secs); }
    }
}

let mut w = state();
let gen = w.try_get_int("gen").unwrap_or(0) + 1;
w.put_int("gen", gen);
w.put_int("started", time());
w.put_int("beat", time());
w.put_string("phase", "starting");
std::thread::spawn(move || run(gen));
let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_int("gen", gen);
o.put_boolean("plan_autonomy", autonomy_on());
o
}
