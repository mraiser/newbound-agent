// `blocked` (2026-10): the agent cannot proceed without a human. Not steppable
// (step_item's gate), not eligible for the plan worker - the loop moves on to
// the next unblocked item. A human unblocks by moving it back to accepted.
let lifecycles = ["proposed", "accepted", "in-progress", "blocked", "completed", "done", "abandoned"];
let mut out = DataObject::new();
if !lifecycles.contains(&lifecycle.as_str()) {
    out.put_string("status", "err");
    out.put_string("msg", "lifecycle must be one of proposed/accepted/in-progress/blocked/completed/done/abandoned");
    return out;
}
// PER-ITEM MOVE (2026-11): only this item is read, checked and written.
let board = crate::agent::plan::compat::board();
let hash = board.get_string("hash");
let entries = board.get_array("entries");
let mut found: Option<DataObject> = None;
for i in 0..entries.len() {
    let e = entries.get_object(i);
    if e.try_get_string("claim").unwrap_or_default() == claim { found = Some(e); break; }
}
let mut e = match found {
    Some(e) => e,
    None => {
        out.put_string("status", "err");
        out.put_string("msg", "no kb.plan entry with that claim - the board may be stale, refresh it");
        return out;
    }
};
// base = the board hash (r<N>) the caller last read. It is stale only when
// THIS item's record changed after board revision N (the index keeps that
// revision per item as `at`); writes to other items, and notes, do not
// invalidate it. The index is read after the board, so an `at` <= N means
// the entry in hand is the one the caller saw.
let base = base.trim().to_string();
if !base.is_empty() && base != hash {
    let cur_n = hash.trim_start_matches('r').parse::<i64>().unwrap_or(0);
    let base_n = if base.starts_with('r') { base[1..].parse::<i64>().unwrap_or(-1) } else { -1 };
    let idx = crate::agent::plan::store::read_index("kb", "plan");
    let at = idx.get_object("items").try_get_object(&e.try_get_string("item_id").unwrap_or_default())
        .map(|m| m.try_get_int("at").unwrap_or(0)).unwrap_or(i64::MAX);
    if base_n < 0 || base_n > cur_n || at > base_n {
        out.put_string("status", "err");
        out.put_string("msg", "stale_base");
        out.put_string("current_hash", &hash);
        out.put_string("scope", "item");
        return out;
    }
}
let tags = e.try_get_string("tags").unwrap_or_default();
let mut kept: Vec<String> = tags.split(',')
    .map(|t| t.trim().to_string())
    .filter(|t| !t.is_empty() && !lifecycles.contains(&t.as_str()))
    .collect();
kept.push(lifecycle.clone());
e.put_string("tags", &kept.join(","));
// Human lifecycle intent outranks a stale run hold. Re-accepting is
// the existing re-authorization; it clears pause, never verification.
if lifecycle == "accepted" || lifecycle == "in-progress" { e.remove_property("control"); }
// Re-approval hook (convergence guard): moving an item TO `accepted`
// is the human re-authorization. If its step_log has reached the cap
// (i.e. step_item refused it with needs_reapproval), reset the log so
// the agent may step it again. A normal accept leaves a non-exhausted
// log untouched (history preserved).
if lifecycle == "accepted" && e.has("step_log") {
    let prior = match e.try_get_array("step_log") { Ok(a) => a.len() as i64, Err(_) => 0 };
    let mut cap: i64 = 20;
    let g = flowlang::datastore::DataStore::globals();
    if let Ok(sys) = g.try_get_object("system") {
        if let Ok(apps) = sys.try_get_object("apps") {
            if let Ok(agent) = apps.try_get_object("agent") {
                if let Ok(rt) = agent.try_get_object("runtime") {
                    if let Ok(v) = rt.try_get_string("MAX_PLAN_STEPS") {
                        if let Ok(p) = v.trim().parse::<i64>() { if p > 0 { cap = p.min(1000); } }
                    }
                }
            }
        }
    }
    if prior >= cap {
        e.put_array("step_log", ndata::dataarray::DataArray::new());
        e.put_int("steps_reset", prior);
    }
}
// Write this one entry. facet_write leaves entries absent from the array
// alone and guards the write with the entry's own rev, so an item that
// changed between the read above and this write is refused, not clobbered.
let mut one = ndata::dataarray::DataArray::new();
one.push_object(e);
crate::agent::plan::compat::facet_write(
    "kb".to_string(), "plan".to_string(), "memory".to_string(),
    "".to_string(), one.to_string(), "".to_string(), "".to_string(), "".to_string(), nn_sessionid)