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