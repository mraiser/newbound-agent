use ndata::dataobject::DataObject;
use flowlang::datastore::DataStore;
use flowlang::flowlang::system::time::time;
use std::sync::{Mutex, MutexGuard, OnceLock};

// THE PLAN STEP LOCK (2026-11): at most ONE plan step (agent.plan.step_item ->
// tool_loop turn) runs per process, whoever calls it - the plan worker,
// run_control resume, or a manual/agent call. Before this, the worker's
// in_step flag only serialized the worker's own threads, so a resume (which
// ran step_item on the request thread) overlapped worker turns: two runs
// alternated on one item, cross-adopted each other's handoff, and raced the
// single AGENT_PLAN_DISPATCH slot and whole-board writes.
// WHY GLOBALS, NOT A RUST STATIC: agent code hot-swaps as dylib generations
// and every generation has its own statics, while plan turns routinely
// recompile agent commands mid-turn - a static lock would silently reset
// under a running step. The holder lives in DataStore::globals() (shared by
// every generation; cleared by a process restart, so no stale lock survives
// one). ndata has no compare-and-swap, so: a per-generation mutex serializes
// same-generation callers, and a write-then-settle-then-verify handshake
// resolves the rare cross-generation race (the check->write gap is
// microseconds, the settle is SETTLE_MS; the last writer wins and the loser
// backs off). StepGuard releases on drop - normal return, early return, or
// panic unwind - and only ever removes its OWN token.
const STEP_KEY: &str = "AGENT_PLAN_STEP";
const SETTLE_MS: u64 = 60;

fn local_lock() -> MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|e| e.into_inner())
}

pub struct StepGuard { token: String }

impl StepGuard {
    /// Record the run id on the holder once step_item has minted it.
    pub fn set_run(&self, run_id: &str) {
        let _l = local_lock();
        let g = DataStore::globals();
        if let Ok(mut h) = g.try_get_object(STEP_KEY) {
            if h.try_get_string("token").unwrap_or_default() == self.token {
                h.put_string("run_id", run_id);
            }
        }
    }
}

impl Drop for StepGuard {
    fn drop(&mut self) {
        let _l = local_lock();
        let mut g = DataStore::globals();
        let mine = g.try_get_object(STEP_KEY).ok()
            .map(|h| h.try_get_string("token").unwrap_or_default() == self.token)
            .unwrap_or(false);
        if mine { g.remove_property(STEP_KEY); }
    }
}

/// The current holder (a detached copy), or an empty object when free.
pub fn holder() -> DataObject {
    match DataStore::globals().try_get_object(STEP_KEY) {
        Ok(h) if !h.try_get_string("token").unwrap_or_default().is_empty() => h.deep_copy(),
        _ => DataObject::new(),
    }
}

/// Take the lock for one plan step. Err(holder) when another step holds it.
pub fn acquire(claim: &str, caller: &str) -> Result<StepGuard, DataObject> {
    let token = format!("{}-{}", time(), flowlang::flowlang::system::unique_session_id::unique_session_id());
    {
        let _l = local_lock();
        let h = holder();
        if h.has("token") { return Err(h); }
        let mut n = DataObject::new();
        n.put_string("token", &token);
        n.put_string("claim", claim);
        n.put_string("caller", caller);
        n.put_int("since", time());
        let mut g = DataStore::globals();
        g.put_object(STEP_KEY, n);
    }
    std::thread::sleep(std::time::Duration::from_millis(SETTLE_MS));
    let h = holder();
    if h.try_get_string("token").unwrap_or_default() == token { Ok(StepGuard { token }) } else { Err(h) }
}
pub fn execute(_: DataObject) -> DataObject {
    use std::panic;
    let ax = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        step_lock()
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

pub fn step_lock() -> DataObject {
// Read-only probe of the plan step lock (helpers live in this module's
// imports: acquire / holder / StepGuard). The token is internal and omitted.
let h = holder();
let mut out = DataObject::new();
out.put_string("status", "ok");
out.put_boolean("held", h.has("token"));
if h.has("token") {
    let mut o = DataObject::new();
    for k in ["claim", "caller", "run_id"] {
        if let Ok(v) = h.try_get_string(k) { o.put_string(k, &v); }
    }
    let since = h.try_get_int("since").unwrap_or(0);
    o.put_int("since", since);
    o.put_int("age_ms", time() - since);
    out.put_object("holder", o);
}
out
}
