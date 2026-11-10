// The agent-committer's sweep (its 5-minute timer; runnable by hand). For each
// repo in the agent registry (runtime/agent/commit-repos.json): honor the
// branches-always doctrine (never commit on master/main), yield to dev.git's
// mechanical autocommit (a repo still flagged there is shadowed - never two
// drivers), group the dirty tree into store units via dev.git.store_status,
// compose ONE message per unit via compose_message (LLM, with fallback), and
// commit each unit with dev.git.commit_unit. Push once per repo, only when the
// branch tracks an upstream and holds unpushed work (an unpublished branch is
// reported, never published). A clean tree / git-less box / absent registry is
// a free no-op. Takes NO params - timers fire with empty args.
fn sargs(v:&[&str])->DataArray{let mut a=DataArray::new();for s in v{a.push_string(s);}a}
fn okr(r:&DataObject)->bool{ r.get_string("status")=="ok" }
fn outs(r:&DataObject)->String{ r.try_get_string("out").unwrap_or_default() }
let mut o=DataObject::new(); o.put_string("status","ok");
let mut results=DataArray::new();
let (mut swept,mut committed,mut pushed,mut units_total)=(0i64,0i64,0i64,0i64);

// dev's repo registry: the source of repo path/origin/autocommit-branch truth
let devregpath = DataStore::new().root.parent().unwrap()
    .join("runtime").join("dev").join("repos.json");
if !devregpath.exists(){ o.put_string("msg","no dev repo registry"); o.put_array("results",results); return o; }
let devreg = match DataObject::try_from_string(&std::fs::read_to_string(&devregpath).unwrap_or_default()){
    Ok(x)=>x, Err(_)=>{ o.put_string("msg","repos.json invalid"); o.put_array("results",results); return o; } };

// the agent committer's own registry
let aregpath = DataStore::new().root.parent().unwrap()
    .join("runtime").join("agent").join("commit-repos.json");
if !aregpath.exists(){ o.put_string("msg","no agent-commit repos enabled (agent.committer.set_enabled)"); o.put_array("results",results); return o; }
let areg = match DataObject::try_from_string(&std::fs::read_to_string(&aregpath).unwrap_or_default()){
    Ok(x)=>x, Err(_)=>{ o.put_string("msg","commit-repos.json invalid"); o.put_array("results",results); return o; } };

for repo in areg.get_keys(){
    let mut r=DataObject::new(); r.put_string("repo",&repo);
    if !devreg.has(&repo){ r.put_string("result","not_in_dev_registry"); results.push_object(r); continue; }
    let e = devreg.get_object(&repo);
    // yield: dev's mechanical autocommit still owns this repo
    if matches!(e.try_get_boolean("autocommit"),Ok(true)){
        r.put_string("result","shadowed_by_dev_autocommit");
        r.put_string("hint","turn off dev.git.set_autocommit for this repo to let the agent committer drive it");
        results.push_object(r); continue;
    }
    swept+=1;

    // branches-always: never commit on the default branch
    let st = crate::api::new().dev.git.read(repo.clone(),"status".to_string(),sargs(&["--porcelain=v2","--branch"]));
    if !okr(&st){ r.put_string("result","status_failed"); r.put_string("err",&outs(&st)); results.push_object(r); continue; }
    let mut branch=String::new(); let mut upstream=String::new(); let mut ahead=0i64;
    for line in outs(&st).lines(){
        if let Some(rest)=line.strip_prefix("# branch.head "){ branch=rest.trim().to_string(); }
        else if let Some(rest)=line.strip_prefix("# branch.upstream "){ upstream=rest.trim().to_string(); }
        else if let Some(rest)=line.strip_prefix("# branch.ab "){
            if let Some(sp)=rest.find(' '){ if let Ok(a)=rest[..sp].trim_start_matches('+').parse::<i64>(){ ahead=a; } }
        }
    }
    r.put_string("branch",&branch);
    if branch=="master"||branch=="main"||branch=="(detached)"{
        r.put_string("result","refused_default_branch"); results.push_object(r); continue;
    }

    // group the dirty tree into store units
    let ss = crate::api::new().dev.git.store_status(repo.clone());
    if !okr(&ss){ r.put_string("result","store_status_failed"); r.put_string("err",&ss.get_string("msg")); results.push_object(r); continue; }
    let groups = ss.get_object("groups");
    let units: Vec<String> = groups.clone().keys();
    r.put_int("units", units.len() as i64);
    if units.is_empty(){
        r.put_string("result", if ss.get_string("text").contains("clean"){"clean"}else{"no_store_units"});
        // even with no store units, there may be unpushed commits
        if !upstream.is_empty() && ahead>0 {
            let p = crate::api::new().dev.git.remote_op(repo.clone(),"push".to_string(),sargs(&[]));
            if okr(&p){ pushed+=1; r.put_boolean("pushed",true); }
        }
        results.push_object(r); continue;
    }

    // compose + commit each unit; siblings = the other units in this batch
    let all = units.join(", ");
    let mut committed_units=DataArray::new();
    let mut any_err=String::new();
    for unit in &units{
        let parts:Vec<&str>=unit.splitn(2,'.').collect();
        if parts.len()!=2 { continue; }
        let sibs: Vec<String> = units.iter().filter(|u|*u!=unit).cloned().collect();
        let cm = compose_message(repo.clone(), unit.clone(), sibs.join(", "));
        let (msg, msrc) = if okr(&cm){ (cm.get_string("message"), cm.get_string("source")) }
                          else { (format!("{}: update {}\n\nAgent-Message: fallback", parts[0], unit), "fallback".to_string()) };
        let cu = crate::api::new().dev.git.commit_unit(repo.clone(), parts[0].to_string(), parts[1].to_string(), msg, "agent".to_string(), String::new());
        if okr(&cu){
            if cu.try_get_boolean("committed").unwrap_or(false){
                committed+=1; units_total+=1;
                let mut cuo=DataObject::new();
                cuo.put_string("unit",unit);
                cuo.put_string("msg_source",&msrc);
                committed_units.push_object(cuo);
            }
        } else { any_err = cu.get_string("msg"); }
    }
    r.put_array("committed",committed_units);
    if !any_err.is_empty(){ r.put_string("commit_err",&any_err); }

    // one push per repo, only when tracking an upstream with unpushed work
    if !upstream.is_empty(){
        let st2 = crate::api::new().dev.git.read(repo.clone(),"status".to_string(),sargs(&["--porcelain=v2","--branch"]));
        let mut ahead2=0i64;
        if okr(&st2){ for line in outs(&st2).lines(){
            if let Some(rest)=line.strip_prefix("# branch.ab "){
                if let Some(sp)=rest.find(' '){ if let Ok(a)=rest[..sp].trim_start_matches('+').parse::<i64>(){ ahead2=a; } } } } }
        if ahead2>0 {
            let p = crate::api::new().dev.git.remote_op(repo.clone(),"push".to_string(),sargs(&[]));
            if okr(&p){ pushed+=1; r.put_boolean("pushed",true); }
            else { r.put_string("push_err",&outs(&p)); }
        }
    } else { r.put_string("push","unpublished_branch"); }
    r.put_string("result","ok");
    results.push_object(r);
}
o.put_int("swept",swept); o.put_int("units_committed",units_total);
o.put_int("commits",committed); o.put_int("pushed",pushed);
o.put_array("results",results);
o