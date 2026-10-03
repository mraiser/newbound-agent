// Agent-committer observability: which repos are enabled for agent-commit, each
// one's dev-side autocommit state (to surface the shadowing conflict), its
// current branch, and whether the tree is dirty. Read-only.
fn sargs(v:&[&str])->DataArray{let mut a=DataArray::new();for s in v{a.push_string(s);}a}
fn okr(r:&DataObject)->bool{ r.get_string("status")=="ok" }
fn outs(r:&DataObject)->String{ r.try_get_string("out").unwrap_or_default() }
let mut o=DataObject::new(); o.put_string("status","ok");
let devregpath = DataStore::new().root.parent().unwrap()
    .join("runtime").join("dev").join("repos.json");
let aregpath = DataStore::new().root.parent().unwrap()
    .join("runtime").join("agent").join("commit-repos.json");
let devreg = if devregpath.exists(){ DataObject::try_from_string(&std::fs::read_to_string(&devregpath).unwrap_or_default()).unwrap_or_else(|_|DataObject::new()) } else { DataObject::new() };
let areg = if aregpath.exists(){ DataObject::try_from_string(&std::fs::read_to_string(&aregpath).unwrap_or_default()).unwrap_or_else(|_|DataObject::new()) } else { DataObject::new() };
let mut rows=DataArray::new();
for repo in areg.get_keys(){
    let mut r=DataObject::new(); r.put_string("repo",&repo);
    if !devreg.has(&repo){ r.put_string("state","not_in_dev_registry"); rows.push_object(r); continue; }
    let e=devreg.get_object(&repo);
    let dev_ac = matches!(e.try_get_boolean("autocommit"),Ok(true));
    r.put_boolean("dev_autocommit",dev_ac);
    r.put_string("role",&e.try_get_string("role").unwrap_or_default());
    let st = crate::api::new().dev.git.read(repo.clone(),"status".to_string(),sargs(&["--porcelain=v2","--branch"]));
    let mut branch=String::new(); let mut dirty=false;
    if okr(&st){ for line in outs(&st).lines(){
        if let Some(rest)=line.strip_prefix("# branch.head "){ branch=rest.trim().to_string(); }
        else if !line.starts_with('#'){ dirty=true; } } }
    r.put_string("branch",&branch); r.put_boolean("dirty",dirty);
    r.put_string("state", if dev_ac {"shadowed_by_dev_autocommit"} else if branch=="master"||branch=="main" {"refused_default_branch"} else {"active"});
    rows.push_object(r);
}
o.put_array("repos",rows);
o.put_int("enabled", areg.get_keys().len() as i64);
o