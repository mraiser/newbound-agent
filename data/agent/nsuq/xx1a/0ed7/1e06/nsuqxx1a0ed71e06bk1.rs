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
