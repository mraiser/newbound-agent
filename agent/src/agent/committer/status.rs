use flowlang::datastore::DataStore;
use ndata::dataobject::DataObject;
use ndata::dataarray::DataArray;
pub fn execute(_: DataObject) -> DataObject {
    use std::panic;
    let ax = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        status()
    }));
    match ax {
        Ok(ax) => {
            let mut result_obj = DataObject::new();
    result_obj.put_object("a", ax);
            result_obj
        }
        Err(err) => {
            let mut err_obj = DataObject::new();
            err_obj.put_string("status", "err");

            let msg = if let Some(s) = err.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = err.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic occurred".to_string()
            };

            err_obj.put_string("msg", &msg);
            // Wrapped in the same `a` envelope a successful return uses.
            // Unwrapped, callers that unpack the envelope (newbound's
            // format_result, for one) report an opaque 500 — "Not an object:
            // DString(\"err\")" — instead of this message.
            let mut result_obj = DataObject::new();
            result_obj.put_object("a", err_obj);
            result_obj
        }
    }
}

pub fn status() -> DataObject {
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
// every dev-registered repo, for a UI picker (toggle targets, not just the enabled set)
let mut all=DataArray::new();
let mut names = devreg.get_keys(); names.sort();
for repo in names {
    let e=devreg.get_object(&repo);
    let mut r=DataObject::new();
    r.put_string("repo",&repo);
    r.put_string("role",&e.try_get_string("role").unwrap_or_default());
    r.put_boolean("dev_autocommit", matches!(e.try_get_boolean("autocommit"),Ok(true)));
    r.put_boolean("agent_commit", areg.has(&repo));
    all.push_object(r);
}
o.put_array("all",all);
o
}
