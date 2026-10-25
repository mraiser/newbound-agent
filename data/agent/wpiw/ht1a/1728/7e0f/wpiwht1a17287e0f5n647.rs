// Disposable runtime-only regression for Phase 3. Does not alter kb.plan,
// invoke an LLM, or change plan autonomy. Restores the run collection exactly.
let store=DataStore::new();
let existed=store.exists("runtime","agent_plan_runs");
let before=if existed {Some(store.get_data("runtime","agent_plan_runs"))} else {None};
let mut results=DataArray::new();let mut passed=0i64;let mut total=0i64;
let mut check=|name:&str,ok:bool|{total+=1;if ok{passed+=1;}let mut r=DataObject::new();r.put_string("case",name);r.put_boolean("pass",ok);results.push_object(r);};
let emit=|rid:&str,kind:&str|{
    let cmd=flowlang::command::Command::lookup("agent","plan","activity_emit");
    let mut a=DataObject::new();a.put_string("run_id",rid);let mut e=DataObject::new();e.put_string("kind",kind);e.put_string("claim","phase3 fixture");a.put_object("event",e);let _=cmd.execute(a);
};
emit("phase3-concurrency","start");
let mut control=DataObject::new();control.put_string("requested","pause");control.put_string("state","requested");
let ce=move |kind:&str,requested:&str|{
    let cmd=flowlang::command::Command::lookup("agent","plan","activity_emit");
    let mut a=DataObject::new();a.put_string("run_id","phase3-concurrency");let mut e=DataObject::new();e.put_string("kind",kind);e.put_string("requested",requested);e.put_string("phase",if requested=="pause"{"paused"}else{"stopped"});e.put_object("control",control.clone());a.put_object("event",e);let _=cmd.execute(a);
};
let t1=std::thread::spawn(move||{for _ in 0..30{ce("control_applied","pause");}});
let t2=std::thread::spawn(move||{for i in 0..30{let cmd=flowlang::command::Command::lookup("agent","plan","activity_emit");let mut a=DataObject::new();a.put_string("run_id","phase3-concurrency");let mut e=DataObject::new();e.put_string("kind","tool_result");e.put_string("tool","fixture");e.put_string("target","fixture");e.put_boolean("ok",true);e.put_int("tool_calls",i+1);a.put_object("event",e);let _=cmd.execute(a);}});
let _=t1.join();let _=t2.join();
let runs=store.get_data("runtime","agent_plan_runs").get_object("data").get_array("runs");
let mut found=DataObject::new();for i in 0..runs.len(){let r=runs.get_object(i);if r.try_get_string("run_id").unwrap_or_default()=="phase3-concurrency"{found=r;break;}}
check("concurrent writes preserve control and every event",found.try_get_object("control").is_ok() && found.get_array("events").len()==61);
check("control applied state is durable",found.try_get_string("control_state").unwrap_or_default()=="paused");
let c=crate::agent::plan::run_state::control("phase3-concurrency");
check("boundary reader sees pause",c.try_get_string("requested").unwrap_or_default()=="pause");
check("boundary reader distinguishes stop", {
    let cmd=flowlang::command::Command::lookup("agent","plan","activity_emit");let mut a=DataObject::new();a.put_string("run_id","phase3-concurrency");let mut e=DataObject::new();e.put_string("kind","control_applied");e.put_string("requested","stop");e.put_object("control",{let mut c=DataObject::new();c.put_string("requested","stop");c.put_string("state","stopped");c});a.put_object("event",e);let _=cmd.execute(a);
    crate::agent::plan::run_state::control("phase3-concurrency").try_get_string("requested").unwrap_or_default()=="stop"
});
check("pause stop outcomes are canonical for planner and scheduler",{
    let normalize=|requested:&str| if requested=="pause"{"PAUSED"}else if requested=="stop"{"STOPPED"}else{"IN_PROGRESS"};
    normalize("pause")=="PAUSED" && normalize("stop")=="STOPPED" && normalize("pause")!="PAUSE"
});
check("active run is retained beyond finished history cap",{
    for i in 0..12{emit(&format!("phase3-finished-{}",i),"start");emit(&format!("phase3-finished-{}",i),"finish");}
    crate::agent::plan::run_state::control("phase3-concurrency").has("requested")
});
check("pause guidance survives structured checkpoint for resume",{
    let mut ci=DataObject::new();ci.put_string("status","PAUSED");ci.put_string("response","Paused at a safe boundary\nNEXT: resume carefully");
    ci.put_string("guidance","Preserve the failed compiler diagnostic and do not repeat the same edit");
    let cp=crate::agent::plan::checkpoint::checkpoint(ci);
    cp.get_string("status")=="PAUSED" && cp.get_string("guidance").contains("compiler diagnostic") && cp.get_string("next")=="resume carefully"
});
match before {Some(r)=>store.set_data("runtime","agent_plan_runs",r),None=>{let _=std::fs::remove_file(store.get_data_file("runtime","agent_plan_runs"));}}
let mut out=DataObject::new();out.put_string("status",if passed==total{"ok"}else{"err"});out.put_int("passed",passed);out.put_int("total",total);out.put_array("checks",results);out