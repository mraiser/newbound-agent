use ndata::dataobject::DataObject;
use std::sync::{Mutex, MutexGuard, OnceLock};
use flowlang::datastore::DataStore;

pub fn lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|e| e.into_inner())
}

pub fn control(run_id: &str) -> DataObject {
    let store = DataStore::new();
    if !store.exists("runtime", "agent_plan_runs") { return DataObject::new(); }
    let runs = store.get_data("runtime", "agent_plan_runs").get_object("data").get_array("runs");
    for i in 0..runs.len() {
        let r = runs.get_object(i);
        if r.try_get_string("run_id").unwrap_or_default() == run_id {
            return r.try_get_object("control").unwrap_or_else(|_| DataObject::new());
        }
    }
    DataObject::new()
}
pub fn execute(_: DataObject) -> DataObject {
    use std::panic;
    let ax = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        run_state()
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

pub fn run_state() -> DataObject {
// Internal shared lock facet. The meaningful API is run_state::lock(), used
// by activity_emit and run_control around runtime read/modify/write cycles.
let mut out = DataObject::new();
out.put_string("status", "ok");
out.put_string("scope", "runtime.agent_plan_runs transaction lock");
out
}
