// Run-scoped control record. pause/stop take effect at a real model/tool
// boundary; resume is a normal accepted/in-progress step, never an
// authorization bypass. No thread/process is killed and no raw prompt is stored.
let action = action.trim().to_lowercase();
if !["pause", "stop", "resume", "clear"].contains(&action.as_str()) {
    let mut o=DataObject::new(); o.put_string("status","err"); o.put_string("msg","action must be pause, stop, resume, or clear"); return o;
}
if run_id.len()>160 || note.chars().count()>8000 {
    let mut o=DataObject::new(); o.put_string("status","err"); o.put_string("msg","run_id or note exceeds its bound"); return o;
}
let api=crate::api::new();
let mut claim = claim;
let mut item_checkpoint=DataObject::new();
let mut last_run=String::new();
let mut active_run=String::new();
let mut lifecycle=String::new();
let b=api.agent.plan.board();
for x in b.get_array("entries").objects() {
    let e=x.object();
    if e.try_get_string("claim").unwrap_or_default()==claim {
        lifecycle=e.try_get_string("tags").unwrap_or_default();
        item_checkpoint=e.try_get_object("checkpoint").unwrap_or_else(|_|DataObject::new());
        last_run=e.try_get_object("last_run").ok().and_then(|r|r.try_get_string("run_id").ok()).unwrap_or_default();
        active_run=e.try_get_object("active_run").ok().and_then(|r|r.try_get_string("run_id").ok()).unwrap_or_default();
    }
}
let store=DataStore::new();
let mut run=DataObject::new();
if store.exists("runtime","agent_plan_runs") {
    let runs=store.get_data("runtime","agent_plan_runs").get_object("data").get_array("runs");
    for i in 0..runs.len(){let r=runs.get_object(i);if r.try_get_string("run_id").unwrap_or_default()==run_id {run=r;break;}}
}
if claim.is_empty(){claim=run.try_get_string("claim").unwrap_or_default();}
if claim.is_empty(){
    let mut o=DataObject::new();o.put_string("status","err");o.put_string("msg","run has no plan-item claim");return o;
}
if lifecycle.is_empty(){
    // Resolve the claim after recovering it from the run.
    for x in b.get_array("entries").objects(){let e=x.object();if e.try_get_string("claim").unwrap_or_default()==claim{
        lifecycle=e.try_get_string("tags").unwrap_or_default();
        item_checkpoint=e.try_get_object("checkpoint").unwrap_or_else(|_|DataObject::new());
        last_run=e.try_get_object("last_run").ok().and_then(|r|r.try_get_string("run_id").ok()).unwrap_or_default();
        active_run=e.try_get_object("active_run").ok().and_then(|r|r.try_get_string("run_id").ok()).unwrap_or_default();
    }}
}
if lifecycle.is_empty(){let mut o=DataObject::new();o.put_string("status","err");o.put_string("msg","plan item not found");return o;}
let live = run.try_get_boolean("active").unwrap_or(false);
if (action=="pause"||action=="stop") && !live {
    let mut o=DataObject::new();o.put_string("status","err");o.put_string("msg","run is not active; its checkpoint is already available for resume");return o;
}
if action=="resume" && !lifecycle.split(',').any(|t|{let t=t.trim();t=="accepted"||t=="in-progress"}) {
    let mut o=DataObject::new();o.put_string("status","err");o.put_string("msg","item is not accepted/in-progress; re-accept it through the normal lifecycle gate before resuming");return o;
}
if action=="resume" && live {
    let mut o=DataObject::new();o.put_string("status","err");o.put_string("msg","run is still active; pause or stop it before starting a new turn");return o;
}
if action=="resume" && (run_id.is_empty() || (run_id!=last_run && run_id!=active_run && run_id!=item_checkpoint.try_get_string("run_id").unwrap_or_default())) {
    let mut o=DataObject::new();o.put_string("status","err");o.put_string("msg","resume requires the item's current or last run; choose its saved checkpoint rather than an unrelated history entry");return o;
}
if action=="resume" && !note.trim().is_empty() {
    let nr=api.agent.plan.add_note(claim.clone(),note.clone(),nn_sessionid.clone());
    if nr.try_get_string("status").unwrap_or_default()!="ok" {let mut o=DataObject::new();o.put_string("status","err");o.put_string("msg", &format!("could not journal resume guidance: {}",nr.try_get_string("msg").unwrap_or_default()));return o;}
}
if action=="resume" {
    let _guard=crate::agent::plan::run_state::lock();
    if store.exists("runtime","agent_plan_runs") {
        let mut rec=store.get_data("runtime","agent_plan_runs");let mut d=rec.get_object("data");let runs=d.get_array("runs");
        for i in 0..runs.len(){let mut r=runs.get_object(i);if r.try_get_string("run_id").unwrap_or_default()==run_id {
            r.put_boolean("paused",false);r.put_boolean("stop_requested",false);r.remove_property("control");r.put_string("control_state","resuming");
        }}
        d.put_array("runs",runs);rec.put_object("data",d);store.set_data("runtime","agent_plan_runs",rec);
    }
    drop(_guard);
    let h=api.agent.plan.board_hash().get_string("hash");
    let fresh=api.agent.plan.board();
    for x in fresh.get_array("entries").objects(){let e=x.object();if e.try_get_string("claim").unwrap_or_default()!=claim{continue;}
        if e.has("control") {
            let lifecycle=e.try_get_string("tags").unwrap_or_default();
            let to=if lifecycle.split(',').any(|t|t.trim()=="accepted"){"accepted"}else{"in-progress"};
            let mr=api.agent.plan.move_item(claim.clone(),to.to_string(),h.clone(),nn_sessionid.clone());
            if mr.try_get_string("status").unwrap_or_default()!="ok" {return mr;}
        }
    }
    let h=api.agent.plan.board_hash().get_string("hash");
    let r=api.agent.plan.step_item(claim,h,nn_sessionid);
    return r;
}
if action=="clear" {
    if live {let mut o=DataObject::new();o.put_string("status","err");o.put_string("msg","cannot clear control on a live run; use pause or stop at a safe boundary");return o;}
    let _=api.dev.code.patch_control_facet("kb".to_string(),"plan".to_string(),"memory".to_string(),"".to_string(),b.get_string("source"),b.get_string("hash"),"clear completed run control".to_string(),"plan-board".to_string(),nn_sessionid);
    let mut o=DataObject::new();o.put_string("status","ok");o.put_string("action","clear");return o;
}
// Pause and stop write the request under the runtime lock, so concurrent
// activity updates cannot erase it. tool_loop observes this at boundaries.
let control=DataObject::new();
{
    let _guard=crate::agent::plan::run_state::lock();
    let store=DataStore::new();
    let mut rec=if store.exists("runtime","agent_plan_runs"){store.get_data("runtime","agent_plan_runs")}else{
        let mut r=DataObject::new();r.put_string("id","agent_plan_runs");r.put_string("username","system");r.put_array("readers",DataArray::new());r.put_array("writers",DataArray::new());r.put_object("data",{let mut d=DataObject::new();d.put_array("runs",DataArray::new());d});r
    };
    let mut d=rec.get_object("data");let runs=d.get_array("runs");let mut found=false;
    for i in 0..runs.len(){let mut r=runs.get_object(i);if r.try_get_string("run_id").unwrap_or_default()!=run_id{continue;}found=true;
        let mut c=DataObject::new();c.put_string("requested", &action);c.put_int("time",time());c.put_string("note", &note);
        c.put_string("state","requested");r.put_object("control",c);r.put_string("control_state","requested");r.put_string("control_note",&note);
        r.put_boolean("paused", action=="pause"); r.put_boolean("stop_requested", action=="stop");
        if action=="pause"{r.put_string("state","pause requested");}else{r.put_string("state","stop requested");}
    }
    if !found {let mut o=DataObject::new();o.put_string("status","err");o.put_string("msg","active run record not found");return o;}
    d.put_array("runs",runs);rec.put_object("data",d);store.set_data("runtime","agent_plan_runs",rec);
}
let mut out=DataObject::new();out.put_string("status","ok");out.put_string("action",&action);out.put_string("run_id",&run_id);out.put_string("claim",&claim);
out.put_string("boundary","The current model/tool boundary will finish; no new model round will begin after the request is observed.");out