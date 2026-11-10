// Per-repo agent-commit toggle. The agent committer keeps its OWN registry at
// runtime/agent/commit-repos.json (agent-side; dev's repos.json stays owned by
// dev's writers). A repo listed here is agent-committed on the next sweep; a
// repo absent is left to dev.git's mechanical autocommit (or nothing). The two
// never both drive a repo: the sweep skips any repo still flagged autocommit in
// dev's registry and reports shadowed_by_dev_autocommit.
fn fail(msg:&str)->DataObject{let mut o=DataObject::new();o.put_string("status","err");o.put_string("msg",msg);o}
let repo = repo.trim().to_string();
if repo.is_empty(){ return fail("repo is required"); }
let path = DataStore::new().root.parent().unwrap()
    .join("runtime").join("agent").join("commit-repos.json");
let mut reg = if path.exists(){
    match DataObject::try_from_string(&std::fs::read_to_string(&path).unwrap_or_default()){
        Ok(o)=>o, Err(_)=>return fail("commit-repos.json is not valid JSON")
    }
} else { DataObject::new() };
if enabled { reg.put_boolean(&repo, true); }
else if reg.has(&repo) { let _ = reg.remove_property(&repo); }
if let Some(p)=path.parent(){ let _ = std::fs::create_dir_all(p); }
// deterministic, sorted, one repo per line so hand edits and writes diff cleanly
let mut names = reg.get_keys(); names.sort();
let mut s = String::from("{");
let mut first=true;
for n in &names { if !first { s.push(','); } first=false;
    s.push_str(&format!("\n  \"{}\": true", n.replace('\\',"\\\\").replace('"',"\\\""))); }
s.push_str(if names.is_empty(){"}\n"} else {"\n}\n"});
if let Err(e)=std::fs::write(&path,s){ return fail(&format!("write failed: {}",e)); }
let mut o=DataObject::new();
o.put_string("status","ok");
o.put_string("repo",&repo);
o.put_boolean("agent_commit",enabled);
o.put_string("msg",&format!("{} agent-commit {}", repo, if enabled{"on"}else{"off"}));
o