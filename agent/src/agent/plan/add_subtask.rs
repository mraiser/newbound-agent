use ndata::dataobject::DataObject;

pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["parent", "claim", "detail", "lifecycle", "nn_sessionid"] {
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
        let arg_0: String = o.get_string("parent");
        let arg_1: String = o.get_string("claim");
        let arg_2: String = o.get_string("detail");
        let arg_3: String = o.get_string("lifecycle");
        let arg_4: String = o.get_string("nn_sessionid");
        add_subtask(arg_0, arg_1, arg_2, arg_3, arg_4)
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

pub fn add_subtask(parent: String, claim: String, detail: String, lifecycle: String, nn_sessionid: String) -> DataObject {
// agent.plan.add_subtask (owner, 2026-11): "a task in progress should be able
// to create sub-tasks and set them in-progress without waiting for the human."
// The decomposition door for a RUNNING plan item. The parent's human
// acceptance is the authorization: a subtask of an accepted/in-progress
// parent may be born `accepted` (runnable at once) or `in-progress`, or
// `proposed` (queued behind an earlier sibling - the parent's coordination
// turn promotes it later via move_item). add_item stays the human/top-level
// front door (always proposed). Guardrails: the parent must exist by exact
// claim and be accepted/in-progress and not paused; bounded depth and fan-out
// so a runaway decomposition cannot flood the board.
const MAX_DEPTH: i64 = 4;       // levels below a top-level item
const MAX_CHILDREN: i64 = 12;   // non-abandoned children per parent
let fail = |m: &str| -> DataObject {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", m);
    o
};
let norm = |s: &str| -> String {
    let t = s.trim().to_string();
    let l = t.to_lowercase();
    if l == "null" || l == "undefined" || l == "none" { String::new() } else { t }
};
let parent = norm(&parent);
let claim = claim.trim().to_string();
if claim.is_empty() {
    return fail("claim is required - one self-contained sentence naming this subtask's outcome (it is matched exactly, keep it unique)");
}
if parent.is_empty() {
    return fail("parent is required: the EXACT claim of the accepted/in-progress plan item this subtask belongs to (use agent.plan.add_item for top-level items)");
}
if claim == parent {
    return fail("a subtask cannot be its own parent");
}
let lc_in = lifecycle.trim().to_lowercase();
let lc = if lc_in.is_empty() { "accepted".to_string() } else { lc_in };
if lc != "accepted" && lc != "in-progress" && lc != "proposed" {
    return fail("lifecycle must be accepted (runnable now), in-progress, or proposed (queued until the parent promotes it)");
}
let api = crate::api::new();
let r = api.dev.code.read_control_facet("kb".to_string(), "plan".to_string(), "memory".to_string());
if !r.try_get_boolean("exists").unwrap_or(false) {
    return fail("kb.plan has no memory facet");
}
let parsed = match DataObject::try_from_string(&format!("{{\"a\": {} }}", r.get_string("source"))) {
    Ok(o) => o,
    Err(_) => return fail("kb.plan memory facet did not parse as a JSON array"),
};
let arr = parsed.get_array("a");
let lifecycles = ["proposed", "accepted", "in-progress", "blocked", "completed", "done", "abandoned"];
let lc_of = |e: &DataObject| -> String {
    let t = e.try_get_string("tags").unwrap_or_default().to_lowercase();
    let toks: Vec<String> = t.split(',').map(|x| x.trim().to_string()).collect();
    lifecycles.iter().find(|l| toks.iter().any(|x| x == **l)).map(|s| s.to_string())
        .unwrap_or_else(|| "none".to_string())
};
let find = |c: &str| -> Option<DataObject> {
    for i in 0..arr.len() {
        if let Ok(e) = arr.try_get_object(i) {
            if e.try_get_string("claim").unwrap_or_default() == c { return Some(e); }
        }
    }
    None
};
let pe = match find(&parent) {
    Some(p) => p,
    None => return fail("no kb.plan entry has that exact parent claim - pass the parent's claim verbatim"),
};
let plc = lc_of(&pe);
if plc != "accepted" && plc != "in-progress" {
    return fail(&format!("parent is `{}` - only an accepted or in-progress item may file subtasks (its acceptance is what authorizes them); use agent.plan.add_item to propose work to a human", plc));
}
if pe.try_get_object("control").ok().and_then(|c| c.try_get_string("requested").ok())
    .map(|s| s == "pause" || s == "stop").unwrap_or(false) {
    return fail("parent is paused/stopped - resume it before filing subtasks");
}
if find(&claim).is_some() {
    return fail("a kb.plan entry with this exact claim already exists - choose a distinct claim");
}
// Depth: the new subtask sits one level below the parent; walk the parent's
// ancestor chain (cycle-guarded).
let mut depth: i64 = 1;
let mut seen: Vec<String> = vec![parent.clone()];
let mut cur = norm(&pe.try_get_string("parent").unwrap_or_default());
while !cur.is_empty() {
    if seen.contains(&cur) { return fail("the parent chain contains a cycle - fix the board before filing subtasks"); }
    seen.push(cur.clone());
    match find(&cur) {
        Some(a) => { depth += 1; cur = norm(&a.try_get_string("parent").unwrap_or_default()); }
        None => break,
    }
    if depth > MAX_DEPTH { break; }
}
if depth > MAX_DEPTH {
    return fail(&format!("decomposition depth limit reached ({} levels below a top-level item) - do this work inside the current item instead of splitting further", MAX_DEPTH));
}
let mut siblings: i64 = 0;
for i in 0..arr.len() {
    if let Ok(e) = arr.try_get_object(i) {
        if norm(&e.try_get_string("parent").unwrap_or_default()) == parent && lc_of(&e) != "abandoned" { siblings += 1; }
    }
}
if siblings >= MAX_CHILDREN {
    return fail(&format!("parent already has {} open subtasks (limit {}) - finish or abandon some, or group the work into fewer subtasks", siblings, MAX_CHILDREN));
}
// Author = calling session's user when there is one; the autonomous plan
// worker calls sessionless, so the fallback names the executive.
let mut author = String::new();
if !nn_sessionid.is_empty() {
    let system = flowlang::datastore::DataStore::globals().get_object("system");
    if system.has("sessions") {
        let sessions = system.get_object("sessions");
        if sessions.has(&nn_sessionid) {
            let session = sessions.get_object(&nn_sessionid);
            if session.has("user") {
                author = session.get_object("user").try_get_string("displayname").unwrap_or_default();
            }
            if author.trim().is_empty() { author = session.try_get_string("username").unwrap_or_default(); }
        }
    }
}
if author.trim().is_empty() { author = "executive".to_string(); }
let mut entry = DataObject::new();
entry.put_string("claim", &claim);
if !detail.trim().is_empty() { entry.put_string("detail", detail.trim()); }
entry.put_string("parent", &parent);
entry.put_string("tags", &format!("plan,{}", lc));
entry.put_string("confidence", "medium");
entry.put_string("origin", "subtask");
entry.put_boolean("work", true);
let mut res = api.agent.archivist.remember("kb".to_string(), "plan".to_string(), entry, author);
res.put_string("parent", &parent);
res.put_string("lifecycle", &lc);
res.put_int("depth", depth);
res.put_int("siblings_open", siblings + 1);
res
}
