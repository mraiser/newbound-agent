use ndata::dataarray::DataArray;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use ndata::dataobject::DataObject;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["argv", "timeout_secs"] {
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
        let arg_0: DataArray = o.get_array("argv");
        let arg_1: i64 = o.get_int("timeout_secs");
        run_local(arg_0, arg_1)
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

pub fn run_local(argv: DataArray, timeout_secs: i64) -> DataObject {
let to = if timeout_secs <= 0 { 60 } else { timeout_secs } as u64;
let mut out = DataObject::new();
out.put_boolean("ok", false);
out.put_int("exit_code", -1);
out.put_boolean("timed_out", false);
out.put_string("stdout", "");
if argv.len() == 0 {
  out.put_string("stderr", "argv is empty - pass the program and its arguments, e.g. [\"ls\", \"-la\"]");
  return out;
}
let mut args = Vec::<String>::new();
for a in argv.objects() { args.push(ndata::data::Data::as_string(a)); }
let mut command = Command::new(&args[0]);
command.args(&args[1..]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
// Own process group, so a timeout kills what `sh -c` spawned as well.
#[cfg(unix)]
{ use std::os::unix::process::CommandExt; command.process_group(0); }
let spawned = command.spawn();
let mut child = match spawned {
  Ok(c) => c,
  Err(e) => {
    out.put_string("stderr", &format!("failed to spawn {}: {}", args[0], e));
    return out;
  }
};
// Drain both pipes on their own threads so a chatty child can't fill a
// pipe buffer and deadlock against the wait below.
let mut so = child.stdout.take().unwrap();
let mut se = child.stderr.take().unwrap();
let (tx_out, rx_out) = mpsc::channel::<Vec<u8>>();
let (tx_err, rx_err) = mpsc::channel::<Vec<u8>>();
std::thread::spawn(move || { let mut b = Vec::new(); let _ = so.read_to_end(&mut b); let _ = tx_out.send(b); });
std::thread::spawn(move || { let mut b = Vec::new(); let _ = se.read_to_end(&mut b); let _ = tx_err.send(b); });
let deadline = Instant::now() + Duration::from_secs(to);
let mut timed_out = false;
let status = loop {
  match child.try_wait() {
    Ok(Some(s)) => break Some(s),
    Ok(None) => {
      if Instant::now() >= deadline {
        // Signal the whole group through libc (std links it already); the
        // external `kill -KILL -<pgid>` silently no-ops on procps-ng 4.x.
        #[cfg(unix)]
        {
          extern "C" { fn kill(pid: i32, sig: i32) -> i32; }
          unsafe { kill(-(child.id() as i32), 9); }
        }
        let _ = child.kill();
        timed_out = true;
        break child.wait().ok();
      }
      std::thread::sleep(Duration::from_millis(20));
    }
    Err(_) => break None,
  }
};
// Bounded: a descendant that escaped the group kill can hold a pipe open.
let stdout = rx_out.recv_timeout(Duration::from_secs(5)).unwrap_or_default();
let mut stderr = String::from_utf8_lossy(&rx_err.recv_timeout(Duration::from_secs(5)).unwrap_or_default()).to_string();
if timed_out { stderr.push_str(&format!("\n[run_local: killed after {}s timeout]", to)); }
let code = status.and_then(|s| s.code()).unwrap_or(-1) as i64;
out.put_boolean("ok", !timed_out && code == 0);
out.put_int("exit_code", code);
out.put_boolean("timed_out", timed_out);
out.put_string("stdout", &String::from_utf8_lossy(&stdout));
out.put_string("stderr", &stderr);
out

}
