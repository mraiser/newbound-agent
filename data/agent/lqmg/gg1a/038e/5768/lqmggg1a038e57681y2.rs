let to = if timeout_secs <= 0 { 10 } else { timeout_secs };
let result = Command::new("ssh")
  .arg("-o").arg("BatchMode=yes")
  .arg("-o").arg("StrictHostKeyChecking=accept-new")
  .arg("-o").arg(format!("ConnectTimeout={}", to))
  .arg(&host)
  .arg(&cmd)
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
    out.put_string("stderr", &format!("failed to spawn ssh: {}", e));
  }
}
out