// No-LLM, no-board-write regression for the durable continuity contract.
let mut checks = DataArray::new();
let mut passed = 0i64;
let mut total = 0i64;
let mut check = |name: &str, ok: bool| {
    total += 1; if ok { passed += 1; }
    let mut r = DataObject::new(); r.put_string("case", name); r.put_boolean("pass", ok);
    checks.push_object(r);
};
let mut ci = DataObject::new(); ci.put_string("response", "PROGRESS: implemented the reader\nNEXT: execute the integration check\nBLOCKERS: none\nVERIFIED: reader fixture passed\nSTATUS: IN_PROGRESS");
ci.put_string("status", "IN_PROGRESS"); ci.put_string("run_id", "run-b");
ci.put_string("previous_run_id", "run-a"); ci.put_string("last_action", "read agent.plan.checkpoint");
ci.put_string("last_result", "ok"); ci.put_string("verification_pending", "integration");
let mut work = DataArray::new();
let mut w = DataObject::new(); w.put_string("tool", "dev-code-patch_command_body");
w.put_string("target", "agent.plan.checkpoint"); w.put_boolean("ok", true); w.put_string("result", "compiled"); work.push_object(w);
let mut v = DataObject::new(); v.put_string("tool", "dev-code-invoke_command");
v.put_string("target", "agent.plan.checkpoint"); v.put_boolean("ok", true); v.put_string("result", "fixture ok"); work.push_object(v);
ci.put_array("work", work);
let cp = crate::agent::plan::checkpoint::checkpoint(ci);
check("checkpoint version and run chain", cp.get_int("version") == 2 && cp.get_string("run_id") == "run-b" && cp.get_string("previous_run_id") == "run-a");
check("next and evidence survive", cp.get_string("next") == "execute the integration check" && cp.get_string("verified").contains("reader fixture passed"));
check("writes are changes not completion", cp.get_array("changes").len() == 1 && cp.get_array("evidence").len() == 1 && !cp.get_boolean("complete"));
check("pending verification explicit", cp.get_string("verification_pending") == "integration");
let read = crate::agent::plan::checkpoint_read::checkpoint_read(cp.clone(), String::new());
check("structured reader retains text and fields", read.get_boolean("structured") && read.get_string("text").contains("execute the integration check") && read.get_string("next") == cp.get_string("next"));
let legacy = crate::agent::plan::checkpoint_read::checkpoint_read(DataObject::new(), "Legacy full response and evidence".to_string());
check("legacy checkpoint remains useful", !legacy.get_boolean("structured") && legacy.get_string("text").contains("Legacy full response"));
let mut entry = DataObject::new(); let mut logs = DataArray::new();
let mut le = DataObject::new(); le.put_string("result", "legacy response longer than a one-line hint"); le.put_string("run_id", "run-old"); logs.push_object(le); entry.put_array("step_log", logs);
let assembled = crate::agent::plan::continuity_context::continuity_context(entry);
check("legacy continuity does not amputate response", assembled.get_string("handoff_received").contains("longer than a one-line hint") && assembled.get_string("previous_run_id") == "run-old");
let mut out = DataObject::new(); out.put_string("status", if passed == total { "ok" } else { "err" });
out.put_int("passed", passed); out.put_int("total", total); out.put_array("checks", checks); out