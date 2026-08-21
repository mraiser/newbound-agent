// git_sense (brick 4; contract kind git_state): one sweep over the
// registered repos (runtime/dev/repos.json - dev.git's registry), one
// `status --porcelain=v2 --branch` each through dev.git.read (the one
// git engine), emission ONLY on edges: HEAD moved, branch switched,
// clean<->dirty crossed, ahead/behind changed, merge/rebase/cherry-pick
// state appeared or cleared. Deliberately NO .git mtime gate: worktree
// edits never touch .git, so such a gate would miss exactly the
// clean->dirty crossing; the porcelain subprocess is the cheapest
// honest probe and coalescing keeps volume at zero when nothing moves.
// State persists at runtime/agent/git_sense.json so a RESTART never
// re-seeds: an instance that boots into a dirty or mid-merge repo
// announces the standing condition once (the system sensor's
// silent-seed flaw, fixed here by design). Emitted envelopes bind the
// claims whose {repo,path} sources name the repo (brick 3), staleness
// by working-tree byte compare - the contract's promise, kept for git.
// Payloads are observations, never conclusions. The tailer loop calls
// this on the ~30s cadence; calling it deliberately is the test surface.
fn content_hash(s: &str) -> String {
    // FNV-1a over the \r-normalized source - MUST stay in sync with
    // remember/recall/epistemic_work/the store tailer.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", h)
}
// Claims whose {repo,path} source pointers name this repo, stale by
// comparing the stamped hash to the working tree's current bytes.
fn bind_repo_claims(repo_name: &str, base: &str) -> DataArray {
    let store = DataStore::new();
    let mut out = DataArray::new();
    let mut libs: Vec<String> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&store.root) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                if let Ok(n) = e.file_name().into_string() { libs.push(n); }
            }
        }
    }
    libs.sort();
    for lib in libs {
        if !store.exists(&lib, "controls") { continue; }
        let list = store.get_data(&lib, "controls").get_object("data").get_array("list");
        for i in 0..list.len() {
            let item = list.get_object(i);
            if !item.has("name") || !item.has("id") { continue; }
            let id = item.get_string("id");
            if !store.exists(&lib, &id) { continue; }
            let dd = store.get_data(&lib, &id).get_object("data");
            if !dd.has("memory") { continue; }
            let w = match DataObject::try_from_string(&format!("{{\"a\":{}}}", dd.get_string("memory"))) {
                Ok(w) => w,
                Err(_) => continue,
            };
            let a = match w.try_get_array("a") { Ok(a) => a, Err(_) => continue };
            for j in 0..a.len() {
                let e = match a.try_get_object(j) { Ok(e) => e, Err(_) => continue };
                if !e.has("claim") || !e.has("source") || e.has("superseded") { continue; }
                let src = match e.try_get_object("source") { Ok(s) => s, Err(_) => continue };
                if !src.has("repo") || !src.has("path") || !src.has("hash") { continue; }
                if src.get_string("repo") != repo_name { continue; }
                let mut stale = true;
                if let Ok(c) = std::fs::read_to_string(format!("{}/{}", base, src.get_string("path"))) {
                    stale = content_hash(&c.replace("\r", "")) != src.get_string("hash");
                }
                let mut c = DataObject::new();
                c.put_string("lib", &lib);
                c.put_string("ctl", &item.get_string("name"));
                c.put_string("claim", &e.get_string("claim"));
                c.put_boolean("stale", stale);
                out.push_object(c);
            }
        }
    }
    out
}

let mut g = DataStore::globals();
let mut st = if g.has("AGENT_SENSOR_GIT") { g.get_object("AGENT_SENSOR_GIT") }
    else {
        // cold start: prior state loads from disk, so a restart sees
        // EDGES against the persisted picture, never a fresh world
        let mut s = DataObject::new();
        let mut repos = DataObject::new();
        if let Ok(txt) = std::fs::read_to_string("runtime/agent/git_sense.json") {
            if let Ok(p) = DataObject::try_from_string(&txt) {
                if let Ok(r) = p.try_get_object("repos") { repos = r; }
            }
        }
        s.put_object("repos", repos.deep_copy());
        s.put_int("emitted_total", 0);
        s.put_int("last_sweep", 0);
        g.put_object("AGENT_SENSOR_GIT", s.deep_copy());
        s
    };
let prev_repos = st.get_object("repos");
let mut next_repos = DataObject::new();
let mut emitted = 0i64;
let now = time();

// the registry is the sensor's world: nothing outside it is watched
let mut names: Vec<(String, String)> = Vec::new();
if let Ok(txt) = std::fs::read_to_string("runtime/dev/repos.json") {
    if let Ok(rj) = DataObject::try_from_string(&txt) {
        let mut ks: Vec<String> = rj.clone().keys();
        ks.sort();
        for n in ks {
            if let Ok(r) = rj.try_get_object(&n) {
                if r.has("path") { names.push((n.clone(), r.get_string("path"))); }
            }
        }
    }
}

for (name, path) in &names {
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let cmd = Command::lookup("dev", "git", "read");
        let mut args = DataObject::new();
        args.put_string("repo", name);
        args.put_string("verb", "status");
        let mut va = DataArray::new();
        va.push_string("--porcelain=v2");
        va.push_string("--branch");
        args.put_array("args", va);
        cmd.execute(args)
    }));
    let res = match res { Ok(Ok(r)) => r, _ => continue };
    // execute wraps a FLAT command's return under "a" - unwrap either shape
    let res = if res.has("a") { res.get_object("a") } else { res };
    if !res.has("out") { continue; }
    if res.has("status") && res.get_string("status") == "err" { continue; }
    let out_txt = res.get_string("out");
    let mut oid = String::new();
    let mut branch = String::new();
    let mut ahead = -1i64;   // -1 = no upstream configured
    let mut behind = -1i64;
    let mut dirty = 0i64;
    for line in out_txt.lines() {
        if let Some(r) = line.strip_prefix("# branch.oid ") { oid = r.trim().to_string(); }
        else if let Some(r) = line.strip_prefix("# branch.head ") { branch = r.trim().to_string(); }
        else if let Some(r) = line.strip_prefix("# branch.ab ") {
            for tok in r.split_whitespace() {
                if let Some(n) = tok.strip_prefix('+') { if let Ok(v) = n.parse::<i64>() { ahead = v; } }
                else if let Some(n) = tok.strip_prefix('-') { if let Ok(v) = n.parse::<i64>() { behind = v; } }
            }
        } else if !line.is_empty() && !line.starts_with('#') { dirty += 1; }
    }
    if oid.is_empty() { continue; }
    // in-progress operations live as marker files under .git
    let gd = std::path::Path::new(path).join(".git");
    let op = if gd.join("rebase-merge").exists() || gd.join("rebase-apply").exists() { "rebase" }
        else if gd.join("MERGE_HEAD").exists() { "merge" }
        else if gd.join("CHERRY_PICK_HEAD").exists() { "cherry-pick" }
        else if gd.join("BISECT_LOG").exists() { "bisect" }
        else { "" };

    let mut cur = DataObject::new();
    cur.put_string("oid", &oid);
    cur.put_string("branch", &branch);
    cur.put_int("dirty_count", dirty);
    cur.put_int("ahead", ahead);
    cur.put_int("behind", behind);
    cur.put_string("op", op);

    let mut changed: Vec<String> = Vec::new();
    if prev_repos.has(name) {
        let p = prev_repos.get_object(name);
        if p.get_string("oid") != oid { changed.push("head".to_string()); }
        if p.get_string("branch") != branch { changed.push("branch".to_string()); }
        if (p.get_int("dirty_count") > 0) != (dirty > 0) { changed.push("dirty".to_string()); }
        if p.get_int("ahead") != ahead || p.get_int("behind") != behind { changed.push("ahead_behind".to_string()); }
        if p.get_string("op") != op { changed.push("op".to_string()); }
    } else if dirty > 0 || !op.is_empty() || ahead > 0 || behind > 0 {
        // first sight of a standing condition is announced, not seeded
        changed.push("initial".to_string());
    }

    if !changed.is_empty() {
        let mut pl = DataObject::new();
        pl.put_string("repo", name);
        pl.put_string("branch", &branch);
        pl.put_string("oid", &oid.chars().take(12).collect::<String>());
        pl.put_boolean("dirty", dirty > 0);
        pl.put_int("dirty_count", dirty);
        pl.put_int("ahead", ahead);
        pl.put_int("behind", behind);
        if !op.is_empty() { pl.put_string("op", op); }
        let mut ca = DataArray::new();
        for c in &changed { ca.push_string(c); }
        pl.put_array("changed", ca);
        if prev_repos.has(name) { pl.put_object("prev", prev_repos.get_object(name).deep_copy()); }
        let mut env = DataObject::new();
        env.put_int("v", 1);
        env.put_string("kind", "git_state");
        env.put_int("time", now);
        env.put_string("sensor", "git");
        env.put_object("payload", pl);
        env.put_array("claims", bind_repo_claims(name, path));
        // an interrupted merge/rebase outranks a moved HEAD outranks churn
        env.put_float("salience_hint",
            if !op.is_empty() { 0.8 }
            else if changed.iter().any(|c| c == "head" || c == "branch") { 0.5 }
            else { 0.35 });
        let sent = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let cmd = Command::lookup("agent", "executive", "perceive");
            let mut args = DataObject::new();
            args.put_object("perception", env.deep_copy());
            cmd.execute(args)
        }));
        if sent.is_ok() { emitted += 1; }
    }
    next_repos.put_object(name, cur);
}

// persist only when the picture moved: restarts see edges, disk stays quiet
if prev_repos.to_string() != next_repos.clone().to_string() {
    let mut f = DataObject::new();
    f.put_object("repos", next_repos.deep_copy());
    let _ = std::fs::create_dir_all("runtime/agent");
    let _ = std::fs::write("runtime/agent/git_sense.json", f.to_string());
}
st.put_object("repos", next_repos.deep_copy());
st.put_int("emitted_total", st.get_int("emitted_total") + emitted);
st.put_int("last_sweep", now);
g.put_object("AGENT_SENSOR_GIT", st.deep_copy());

let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_int("swept", names.len() as i64);
o.put_int("emitted", emitted);
o.put_object("repos", next_repos);
o