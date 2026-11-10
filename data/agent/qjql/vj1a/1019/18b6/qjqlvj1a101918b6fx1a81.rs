// The agent-committer's sweep (its 5-minute timer; runnable by hand): runs
// sweep_repo for every repo enabled in the agent registry
// (runtime/agent/commit-repos.json) and totals the results. One sweep at a
// time: a sweep that finds another still running (LLM latency can outlast the
// timer) returns busy without touching git or the LLM. A clean tree / git-less
// box / absent registry is a free no-op. Takes NO params - timers fire with
// empty args.
fn now_ms()->i64{ std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d|d.as_millis() as i64).unwrap_or(0) }
let mut o=DataObject::new(); o.put_string("status","ok");
let mut results=DataArray::new();

let aregpath = DataStore::new().root.parent().unwrap().join("runtime").join("agent").join("commit-repos.json");
if !aregpath.exists(){ o.put_string("msg","no agent-commit repos enabled (agent.committer.set_enabled)"); o.put_array("results",results); return o; }
let areg = match DataObject::try_from_string(&std::fs::read_to_string(&aregpath).unwrap_or_default()){
    Ok(x)=>x, Err(_)=>{ o.put_string("msg","commit-repos.json invalid"); o.put_array("results",results); return o; } };

let mut g = DataStore::globals();
let now = now_ms();
if g.has("AGENT_COMMIT_SWEEPING") {
    let since = g.get_int("AGENT_COMMIT_SWEEPING");
    if since>0 && now-since < 30*60*1000 {
        o.put_string("msg","busy: another sweep is still running"); o.put_int("busy_since",since);
        o.put_array("results",results); return o;
    }
}
g.put_int("AGENT_COMMIT_SWEEPING", now);

let (mut swept,mut commits,mut pushed,mut llm_calls)=(0i64,0i64,0i64,0i64);
let mut repos: Vec<String> = areg.clone().keys();
repos.sort();
for repo in repos{
    if !matches!(areg.try_get_boolean(&repo),Ok(true)){ continue; }
    let r = sweep_repo(repo.clone());
    let res = r.try_get_string("result").unwrap_or_default();
    if res!="not_in_dev_registry" && res!="shadowed_by_dev_autocommit" && res!="no_dev_registry" { swept+=1; }
    commits += r.try_get_int("commits").unwrap_or(0);
    llm_calls += r.try_get_int("llm_calls").unwrap_or(0);
    if r.try_get_boolean("pushed").unwrap_or(false){ pushed+=1; }
    results.push_object(r);
}
g.put_int("AGENT_COMMIT_SWEEPING", 0);
o.put_int("swept",swept); o.put_int("units_committed",commits);
o.put_int("commits",commits); o.put_int("pushed",pushed); o.put_int("llm_calls",llm_calls);
o.put_array("results",results);
o