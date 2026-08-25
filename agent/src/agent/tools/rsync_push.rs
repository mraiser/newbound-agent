use ndata::dataobject::DataObject;
use std::process::Command;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["host", "src", "dst"] {
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
        let arg_0: String = o.get_string("host");
        let arg_1: String = o.get_string("src");
        let arg_2: String = o.get_string("dst");
        rsync_push(arg_0, arg_1, arg_2)
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

pub fn rsync_push(host: String, src: String, dst: String) -> DataObject {
let result = Command::new("rsync")
  .arg("-az")
  .arg("-e").arg("ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new")
  .arg(&src)
  .arg(&format!("{}:{}", host, dst))
  .output();
let mut out = DataObject::new();
match result {
  Ok(r) => {
    out.put_boolean("ok", r.status.success());
    out.put_int("exit_code", r.status.code().unwrap_or(-1) as i64);
    out.put_string("stdout", &String::from_utf8_lossy(&r.stdout));
    out.put_string("stderr", &String::from_utf8_lossy(&r.stderr));
  },
  Err(e) => {
    out.put_boolean("ok", false);
    out.put_int("exit_code", -1);
    out.put_string("stdout", "");
    out.put_string("stderr", &format!("failed to spawn rsync: {}", e));
  }
}
out
}
