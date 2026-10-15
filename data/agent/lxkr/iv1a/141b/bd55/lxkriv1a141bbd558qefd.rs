let mut out = DataObject::new();
let id = flowlang::flowlang::system::unique_session_id::unique_session_id();
let mut args = DataObject::new();
args.put_string("id", &id);
args.put_string("js", &js);
args.put_int("timeout_ms", timeout_ms);
let r = match Command::lookup("agent", "stage", "eval_pump").execute(args) {
  Ok(o) => o,
  Err(e) => { out.put_string("status", "err"); out.put_string("value", &format!("eval_pump: {:?}", e)); return out; }
};
if !r.has("status") { out.put_string("status", "err"); out.put_string("value", "no response from stage bridge"); return out; }
match r.get_string("status").as_str() {
  "accepted" => { out.put_string("status", "err"); out.put_string("value", "request was accepted but its result had no poll waiting — the page delivered late; retry"); }
  "timeout" => { out.put_string("status", "timeout"); out.put_string("value", "stage page did not answer in time (is the agent chat open?)"); }
  _ => {
    out.put_string("status", "ok");
    out.put_string("value", &r.get_string("value"));
  }
}
out