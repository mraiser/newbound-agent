// One repo's share of the agent-committer sweep. EVERY path git reports dirty
// is committed: dev.git.store_status names each one, grouped by store unit
// (lib.ctl), by library (lib - metadata, assets, publish metadata, records the
// store no longer knows), and the non-store rest (grouped here by top-level
// directory, key "files:<top>"). One composed message per group, committed
// with dev.git.commit_paths over exactly the reported paths - deletions
// included - so a swept repo is CLEAN and the next sweep costs nothing.
// A group whose commit fails is remembered (in-memory) by its path set: while
// it is unchanged, later sweeps retry the commit with the mechanical message
// and make NO LLM call. Does not consult the agent registry - sweep does.
fn sargs(v:&[&str])->DataArray{let mut a=DataArray::new();for s in v{a.push_string(s);}a}
fn okr(r:&DataObject)->bool{ r.get_string("status")=="ok" }
fn outs(r:&DataObject)->String{ r.try_get_string("out").unwrap_or_default() }
fn branch_state(repo:&str)->Option<(String,String,i64)>{
    let st = crate::api::new().dev.git.read(repo.to_string(),"status".to_string(),sargs(&["--porcelain=v2","--branch"]));
    if !okr(&st){ return None; }
    let (mut branch,mut upstream,mut ahead)=(String::new(),String::new(),0i64);
    for line in outs(&st).lines(){
        if let Some(rest)=line.strip_prefix("# branch.head "){ branch=rest.trim().to_string(); }
        else if let Some(rest)=line.strip_prefix("# branch.upstream "){ upstream=rest.trim().to_string(); }
        else if let Some(rest)=line.strip_prefix("# branch.ab "){
            if let Some(sp)=rest.find(' '){ if let Ok(a)=rest[..sp].trim_start_matches('+').parse::<i64>(){ ahead=a; } }
        }
    }
    Some((branch,upstream,ahead))
}
let repo = repo.trim().to_string();
let mut r=DataObject::new(); r.put_string("status","ok"); r.put_string("repo",&repo);
r.put_int("llm_calls",0); r.put_int("commits",0);

let devregpath = DataStore::new().root.parent().unwrap().join("runtime").join("dev").join("repos.json");
let devreg = match DataObject::try_from_string(&std::fs::read_to_string(&devregpath).unwrap_or_default()){
    Ok(x)=>x, Err(_)=>{ r.put_string("result","no_dev_registry"); return r; } };
if !devreg.has(&repo){ r.put_string("result","not_in_dev_registry"); return r; }
// yield: dev's mechanical autocommit still owns this repo - never two drivers
if matches!(devreg.get_object(&repo).try_get_boolean("autocommit"),Ok(true)){
    r.put_string("result","shadowed_by_dev_autocommit");
    r.put_string("hint","turn off dev.git.set_autocommit for this repo to let the agent committer drive it");
    return r;
}

// branches-always: never commit on the default branch
let (branch,upstream,ahead) = match branch_state(&repo){ Some(x)=>x, None=>{ r.put_string("result","status_failed"); return r; } };
r.put_string("branch",&branch);
if branch=="master"||branch=="main"||branch=="(detached)"{ r.put_string("result","refused_default_branch"); return r; }

// every dirty path, named and grouped
let ss = crate::api::new().dev.git.store_status(repo.clone());
if !okr(&ss){ r.put_string("result","store_status_failed"); r.put_string("err",&ss.try_get_string("msg").unwrap_or_default()); return r; }
if !ss.has("other_paths"){
    // the running core predates commit_paths / complete path reporting: do
    // nothing rather than compose messages for commits that cannot land
    r.put_string("result","core_not_activated");
    r.put_string("hint","dev.git (static newbound_core) is older than this committer - restart Newbound on the rebuilt binary");
    return r;
}
let mut keys: Vec<String> = Vec::new();
let mut groups = DataObject::new();
let sg = ss.get_object("groups");
for (k,v) in sg.objects(){ if let Data::DArray(_)=v { keys.push(k.clone()); groups.put_array(&k, v.array()); } }
keys.sort();
let mut fkeys: Vec<String> = Vec::new();
for it in ss.get_array("other_paths").objects(){
    let io = it.object();
    let p = io.get_string("path");
    let key = match p.find('/'){ Some(i)=>format!("files:{}", &p[..i]), None=>"files".to_string() };
    if !groups.has(&key){ groups.put_array(&key, DataArray::new()); fkeys.push(key.clone()); }
    let mut item = DataObject::new();
    item.put_string("state",&io.get_string("state")); item.put_string("what","file"); item.put_string("path",&p);
    groups.get_array(&key).push_object(item);
}
fkeys.sort(); keys.extend(fkeys);
r.put_int("groups", keys.len() as i64);

let mut pushed=false;
if keys.is_empty(){
    r.put_string("result","clean");
} else {
    let mut g = DataStore::globals();
    if !g.has("AGENT_COMMIT_FAILED"){ g.put_object("AGENT_COMMIT_FAILED", DataObject::new()); }
    let mut failed = g.get_object("AGENT_COMMIT_FAILED");
    let mut committed=DataArray::new();
    let mut errs=DataArray::new();
    let (mut llm_calls, mut commits) = (0i64, 0i64);
    for key in &keys{
        let items = groups.get_array(key);
        let mut paths=DataArray::new();
        let mut fp: Vec<String> = Vec::new();
        for it in items.objects(){ let io=it.object(); let p=io.get_string("path"); fp.push(format!("{}:{}", io.get_string("state"), p)); paths.push_string(&p); }
        fp.sort();
        let fp = fp.join("\n");
        let mkey = format!("{}|{}", repo, key);
        let n = paths.len();
        let retry = failed.has(&mkey) && failed.get_string(&mkey)==fp;
        let mechanical = format!("{}: update {} path(s)\n\nAgent-Message: fallback", key, n);
        let (msg, msrc) = if retry { (mechanical, "fallback".to_string()) } else {
            let sibs: Vec<String> = keys.iter().filter(|u|*u!=key).cloned().collect();
            llm_calls+=1;
            let cm = compose_message(repo.clone(), key.clone(), sibs.join(", "), items.clone());
            if okr(&cm){ (cm.get_string("message"), cm.get_string("source")) } else { (mechanical, "fallback".to_string()) }
        };
        // by name, not through this crate's generated api.rs: that file mirrors
        // the box it was last rebuilt on and may predate commit_paths
        let mut ca=DataObject::new();
        ca.put_string("repo",&repo); ca.put_array("paths",paths); ca.put_string("message",&msg);
        ca.put_string("author","agent"); ca.put_string("nn_sessionid","");
        let cu = match flowlang::command::Command::lookup("dev","git","commit_paths").execute(ca){
            Ok(x)=> if x.has("a") { x.get_object("a") } else { x },
            Err(_)=>{ let mut e=DataObject::new(); e.put_string("status","err"); e.put_string("msg","dev.git.commit_paths could not be executed"); e }
        };
        if okr(&cu){
            if failed.has(&mkey){ failed.remove_property(&mkey); }
            if cu.try_get_boolean("committed").unwrap_or(false){
                commits+=1;
                let mut cuo=DataObject::new();
                cuo.put_string("unit",key); cuo.put_string("msg_source",&msrc); cuo.put_int("paths", n as i64);
                committed.push_object(cuo);
            }
        } else {
            failed.put_string(&mkey,&fp);
            let mut eo=DataObject::new();
            eo.put_string("unit",key); eo.put_string("err",&cu.try_get_string("msg").unwrap_or_default());
            errs.push_object(eo);
        }
    }
    r.put_int("llm_calls",llm_calls); r.put_int("commits",commits);
    r.put_array("committed",committed);
    let clean = errs.len()==0;
    if !clean { r.put_array("commit_errs",errs); }
    r.put_string("result", if clean {"ok"} else {"commit_failed"});
}

// one push per repo, only when tracking an upstream with unpushed work
if upstream.is_empty(){ r.put_string("push","unpublished_branch"); }
else {
    let ahead2 = if keys.is_empty(){ ahead } else { branch_state(&repo).map(|x|x.2).unwrap_or(0) };
    if ahead2>0 {
        let p = crate::api::new().dev.git.remote_op(repo.clone(),"push".to_string(),sargs(&[]));
        if okr(&p){ pushed=true; } else { r.put_string("push_err",&p.try_get_string("msg").unwrap_or_default()); }
    }
}
r.put_boolean("pushed",pushed);
r