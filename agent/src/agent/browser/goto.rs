use ndata::dataobject::DataObject;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["url"] {
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
        let arg_0: String = o.get_string("url");
        goto(arg_0)
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

pub fn goto(url: String) -> DataObject {
// Navigate the bound Noobscape session to `url`.
//
// In-tab script navigation is not possible in this browser build, and the
// reason is structural rather than a bug to patch around:
//  - Injected scripts run with the SYSTEM principal (that is what gives eval
//    its chrome powers), so DOM navigation (location.href / assign / pushState)
//    fails its same-origin / subject-principal check and is silently vetoed.
//  - A chrome-context docshell load from the CONTENT process (where the tab
//    lives under Fission) returns NS_OK but never commits: a top-level load
//    must originate in the owning flow, which an injected watcher is not part
//    of. Verified empirically (parent=0 content=1 isTop=1, rv=NS_OK, no load).
//
// The one primitive that reliably drives this browser is the command-line
// launch open() already uses. goto() therefore REBINDS: it closes the current
// session and relaunches at `url`, and the bind latch attaches to the fresh
// tab. The default profile persists, so cookies and localStorage survive; only
// in-page JS state resets — exactly what a real navigation does. Returns
// open()'s {status, pid, url}.
let _ = crate::agent::browser::close::close();
// close() TERMs the recorded pid; let the profile's single-instance lock
// release before relaunch so firefox does not refuse it as already running.
std::thread::sleep(std::time::Duration::from_millis(800));
crate::agent::browser::open::open(url)
}
