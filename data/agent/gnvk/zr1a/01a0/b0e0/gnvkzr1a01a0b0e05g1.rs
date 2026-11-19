// agent-model-why_harvest - the procedural why-harvester (harvest H3a).
// The store journals every mutation with a label and an author; our
// commits carry rationale. This command pairs each CHANGE with its
// STATED why - no model, free, retroactive - and feeds the pairs into
// the code-why stream dataset (kind cpt) through the one feeder.
// Re-runs are idempotent: dataset_feed dedups line-wise.
//
// Two sources:
//   patches - every control's patch journal (dev.code.patch_journal): one row per patch
//     entry {home, facet|cmd, label (the why), author, t, excerpt}.
//   git - commit subjects+bodies (the why) paired with --stat file
//     lists (the change), from repo_path (empty = this checkout).
//     Owner call 5's proposal is full history; depth caps a run.
fn err(msg: String) -> DataObject {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", &msg);
    o
}
fn esc(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}
fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n { return s.to_string(); }
    s.chars().take(n).collect()
}
// Patches newer than `since`, oldest first, through dev.code.patch_journal
// (which answers from the instance-local runtime journal, or the legacy
// record when a control has none). read is newest-first, so the window
// grows until it reaches back past `since`.
fn patches_since(lib: &str, ctlid: &str, since: i64) -> Vec<DataObject> {
    let mut limit: i64 = if since <= 0 { 0 } else { 32 };
    loop {
        let mut a = DataObject::new();
        a.put_string("op", "read");
        a.put_string("lib", lib);
        a.put_string("ctl", ctlid);
        a.put_string("to_lib", "");
        a.put_object("entry", DataObject::new());
        a.put_int("limit", limit);
        a.put_boolean("dry_run", false);
        a.put_string("nn_sessionid", "");
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            flowlang::command::Command::lookup("dev", "code", "patch_journal").execute(a)
        }));
        let r = match r { Ok(Ok(r)) => if r.has("a") { r.get_object("a") } else { r }, _ => return Vec::new() };
        let list = match r.try_get_array("patches") { Ok(l) => l, Err(_) => return Vec::new() };
        let total = r.try_get_int("total").unwrap_or(0);
        let mut out: Vec<DataObject> = Vec::new();
        let mut reached = false;
        for i in 0..list.len() {
            if let Ok(e) = list.try_get_object(i) {
                if e.try_get_int("time").unwrap_or(0) > since { out.push(e); } else { reached = true; }
            }
        }
        if reached || limit <= 0 || limit >= total { out.reverse(); return out; }
        limit *= 4;
    }
}

let source_t = source.trim().to_lowercase();
if !["patches", "git", "both"].contains(&source_t.as_str()) {
    return err(format!("source must be patches | git | both (got '{}')", source));
}
if limit < 0 { return err("limit must be >= 0 (0 = no cap)".to_string()); }
let store = DataStore::new();
let root = match store.root.canonicalize().ok()
        .and_then(|r| r.parent().map(|p| p.to_path_buf())) {
    Some(r) => r,
    None => { return err("cannot resolve the checkout root".to_string()); }
};

let mut lines: Vec<String> = Vec::new();
let mut patch_rows = 0i64;
let mut git_rows = 0i64;

if source_t == "patches" || source_t == "both" {
    // id -> control name, per library, from the controls records
    let mut names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut libs: Vec<String> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(store.root.clone()) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                if let Some(lib) = e.file_name().to_str() { libs.push(lib.to_string()); }
            }
        }
    }
    for lib in &libs {
        if !store.exists(lib, "controls") { continue; }
        let d = store.get_data(lib, "controls").get_object("data");
        if !d.has("list") { continue; }
        for c in d.get_array("list").objects() {
            let c = c.object();
            if c.has("id") && c.has("name") {
                names.insert(format!("{}/{}", lib, c.get_string("id")),
                             c.get_string("name"));
            }
        }
    }
    // every control's patch journal, through dev.code.patch_journal (the
    // instance-local runtime journal, or the legacy record) - no file walk
    let mut homes: Vec<(&String, &String)> = names.iter().collect();
    homes.sort();
    'outer: for (key, name) in homes {
        let (lib, id) = match key.split_once('/') { Some(x) => x, None => continue };
        let home = format!("{}.{}", lib, name);
        for p in patches_since(lib, id, -1) {
            {
                let label = if p.has("label") { p.get_string("label") } else { String::new() };
                if label.trim().is_empty() { continue; }
                // A command edit (facet "command" with an op: create | upsert |
                // patch | imports | meta) names the command and the op. Entries
                // without an op - delete_command's included - keep the older
                // wording, so rows already harvested dedup instead of doubling.
                let op = p.try_get_string("op").unwrap_or_default();
                let is_cmd_edit = !op.is_empty() && p.has("cmd")
                    && p.try_get_string("facet").unwrap_or_default() == "command";
                let what = if is_cmd_edit { format!("command {} ({})", p.get_string("cmd"), op) }
                    else if p.has("facet") { format!("facet {}", p.get_string("facet")) }
                    else if p.has("cmd") { format!("command {}", p.get_string("cmd")) }
                    else { "record".to_string() };
                let author = if p.has("author") { p.get_string("author") } else { String::new() };
                let t = if p.has("time") { p.get_int("time") } else { 0 };
                let mut excerpt = if p.has("new") { clip(&p.get_string("new"), 240) } else { String::new() };
                // create/upsert journal the whole {command, impl} record as
                // JSON: excerpt the code body, not the start of the record.
                if is_cmd_edit && (op == "create" || op == "upsert") {
                    if let Ok(n) = DataObject::try_from_string(&format!("{{\"a\": {}}}", p.try_get_string("new").unwrap_or_default())) {
                        if let Ok(rec) = n.try_get_object("a") {
                            if let (Ok(c), Ok(im)) = (rec.try_get_object("command"), rec.try_get_object("impl")) {
                                let t2 = c.try_get_string("type").unwrap_or_else(|_| "rust".to_string());
                                let ext = if t2 == "rust" { "rs".to_string() } else { t2 };
                                if let Ok(body) = im.try_get_string(&ext) { excerpt = clip(&body, 240); }
                            }
                        }
                    }
                }
                lines.push(format!(
                    "{{\"kind\": \"code_why\", \"source\": \"patch\", \"home\": \"{}\", \"what\": \"{}\", \"why\": \"{}\", \"author\": \"{}\", \"t\": {}, \"excerpt\": \"{}\"}}",
                    esc(&home), esc(&what), esc(&label), esc(&author), t, esc(&excerpt)));
                patch_rows += 1;
                if limit > 0 && patch_rows >= limit { break 'outer; }
            }
        }
    }
}

if source_t == "git" || source_t == "both" {
    let repo = if repo_path.trim().is_empty() { root.display().to_string() }
               else { repo_path.trim().to_string() };
    if !std::path::Path::new(&repo).join(".git").exists() {
        return err(format!("'{}' is not a git checkout", repo));
    }
    let mut cmdv = DataArray::new();
    for a in ["git", "-C", &repo, "log",
              "--pretty=format:%x1e%H%x1f%an%x1f%at%x1f%B%x1f", "--stat",
              "--no-color"] {
        cmdv.push_string(a);
    }
    if limit > 0 {
        cmdv.push_string("-n");
        cmdv.push_string(&limit.to_string());
    }
    let r = system_call(cmdv);
    if r.try_get_string("status").ok().as_deref() != Some("ok") {
        return err("git log failed to execute".to_string());
    }
    let out_s = r.get_string("out");
    for chunk in out_s.split('\u{1e}') {
        let chunk = chunk.trim();
        if chunk.is_empty() { continue; }
        let parts: Vec<&str> = chunk.split('\u{1f}').collect();
        if parts.len() < 4 { continue; }
        let (sha, author, at, msg) = (parts[0], parts[1], parts[2], parts[3]);
        let stat = parts.get(4).map(|s| s.trim()).unwrap_or("");
        // trailers are provenance, not rationale - they would teach the
        // model to emit attribution footers as "why"
        let why: String = msg.lines()
            .filter(|l| {
                let lt = l.trim();
                !lt.starts_with("Co-Authored-By:") && !lt.starts_with("Claude-Session:")
                    && !lt.starts_with("Signed-off-by:")
            })
            .collect::<Vec<_>>().join("\n");
        let why = why.trim();
        if why.is_empty() { continue; }
        // the stat is the change: file list + churn, one compact line each
        let files: Vec<String> = stat.lines().map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty()).take(24).collect();
        lines.push(format!(
            "{{\"kind\": \"code_why\", \"source\": \"commit\", \"commit\": \"{}\", \"author\": \"{}\", \"t\": {}, \"why\": \"{}\", \"change\": \"{}\"}}",
            esc(&clip(sha, 12)), esc(author),
            at.trim().parse::<i64>().unwrap_or(0) * 1000,
            esc(&clip(why, 2000)), esc(&clip(&files.join("; "), 1200))));
        git_rows += 1;
        if limit > 0 && git_rows >= limit { break; }
    }
}

if lines.is_empty() {
    let mut o = DataObject::new();
    o.put_string("status", "ok");
    o.put_int("patch_rows", 0);
    o.put_int("git_rows", 0);
    o.put_int("appended", 0);
    o.put_string("note", "nothing to harvest (no labeled patches / no commits matched)");
    return o;
}
let fed = dataset_feed("code-why".to_string(), "cpt".to_string(),
    lines.join("\n"), "harvested:patches+git".to_string(),
    "why_harvest".to_string(), 10);
let appended = if fed.has("appended") { fed.get_int("appended") } else { 0 };
let total = if fed.has("rows") { fed.get_int("rows") } else { -1 };

let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_int("patch_rows", patch_rows);
o.put_int("git_rows", git_rows);
o.put_int("appended", appended);
o.put_int("dataset_rows", total);
o.put_string("dataset", "code-why");
o
