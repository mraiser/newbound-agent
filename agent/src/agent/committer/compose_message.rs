use ndata::dataobject::DataObject;
use ndata::dataarray::DataArray;
use ndata::data::Data;
use crate::agent::llm::ask_llm::ask_llm;
pub fn execute(o: DataObject) -> DataObject {
    use std::panic;
    for p in ["repo", "unit", "siblings", "changes"] {
        if !o.has(p) {
            let mut e = DataObject::new();
            e.put_string("status", "err");
            e.put_string("msg", &format!("missing required parameter: {}", p));
            let mut result_obj = DataObject::new();
            result_obj.put_object("a", e);
            return result_obj;
        }
    }
    let ax = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let arg_0: String = o.get_string("repo");
        let arg_1: String = o.get_string("unit");
        let arg_2: String = o.get_string("siblings");
        let arg_3: DataArray = o.get_array("changes");
        compose_message(arg_0, arg_1, arg_2, arg_3)
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

pub fn compose_message(repo: String, unit: String, siblings: String, changes: DataArray) -> DataObject {
// Compose ONE commit message for ONE change group in a repo. READ-ONLY:
// stages nothing, commits nothing - it only reads git state and asks the LLM.
// A group is what dev.git.store_status reports under one key: a store unit
// (lib.ctl), a library (lib), or - for the sweep - a set of non-store files.
// `changes` is that group's item list [{state, what?, path}]; empty means
// "look the unit up in store_status yourself". The diff is taken over exactly
// those paths, so the message describes this group and nothing else, and is
// correct no matter WHO made the edit. Output contract enforced here: subject
// <=72 chars, imperative, blank line, why-bullets, an Agent-Message trailer.
// Falls back to a deterministic mechanical message when the LLM fails or
// misbehaves, so the caller can always commit.
fn fail(msg:&str)->DataObject{let mut o=DataObject::new();o.put_string("status","err");o.put_string("msg",msg);o}
fn sargs(v:&[&str])->DataArray{let mut a=DataArray::new();for s in v{a.push_string(s);}a}
fn okr(r:&DataObject)->bool{ r.get_string("status")=="ok" }
fn outs(r:&DataObject)->String{ r.try_get_string("out").unwrap_or_default() }
let unit = unit.trim().to_string();
if unit.is_empty() { return fail("unit must name the change group (lib.ctl, lib, or a label)"); }

// --- the group's changes -------------------------------------------------------
let mut changes = changes;
if changes.len()==0 {
    let ss = crate::api::new().dev.git.store_status(repo.clone());
    if !okr(&ss){ return fail(&format!("store_status failed: {}", ss.try_get_string("msg").unwrap_or_default())); }
    if let Ok(a) = ss.get_object("groups").try_get_array(&unit){ changes = a; }
}
let mut listing = String::new();
let mut dargs = DataArray::new();
dargs.push_string("HEAD"); dargs.push_string("--");
let mut n = 0usize;
for it in changes.objects(){
    let io = match it { Data::DObject(_) => it.object(), _ => continue };
    let p = io.try_get_string("path").unwrap_or_default();
    if p.is_empty(){ continue; }
    n += 1;
    if n <= 200 {
        listing.push_str(&format!("{:9} {}  [{}]\n", io.try_get_string("state").unwrap_or_default(), io.try_get_string("what").unwrap_or_default(), p));
    }
    dargs.push_string(&format!(":(literal){}", p));
}
if n==0 { return fail("no changes for this unit - nothing to describe"); }
if n > 200 { listing.push_str(&format!("...and {} more\n", n-200)); }

// --- the diff over exactly those paths (tracked edits and deletions; a new
// file shows in the listing only), capped ---------------------------------------
let d = crate::api::new().dev.git.read(repo.clone(), "diff".to_string(), dargs);
let mut diff = if okr(&d){ outs(&d) } else { String::new() };
let diff_truncated = diff.chars().count() > 12000;
if diff_truncated { diff = diff.chars().take(12000).collect(); diff.push_str("\n...[diff truncated]..."); }

// --- house style: recent subjects ----------------------------------------------
let lg = crate::api::new().dev.git.read(repo.clone(), "log".to_string(), sargs(&["-8","--format=%s"]));
let style = if okr(&lg){ outs(&lg) } else { String::new() };
let sibs = if siblings.trim().is_empty(){ "none".to_string() } else { siblings.trim().to_string() };

// --- the prompt ----------------------------------------------------------------
let sys = "You write ONE git commit message for ONE group of changes in a Newbound platform repo. \
A group is usually a store unit - one control (lib.ctl): a record plus its facets, its command impls, and generated rust - \
or a library's own files, or a set of plain files outside the store. \
Rules: first line is the subject - imperative mood, <=72 chars, no trailing period, match the repo's existing subject style. \
Then a blank line, then 1-4 bullets explaining WHY (not restating the diff). \
If sibling groups changed in the same batch, you may end with a line like 'Part of: <siblings>'. \
Output ONLY the commit message - no preamble, no code fences, no quotes around it.";
let user = format!(
"REPO: {repo}\nGROUP: {unit}\nSIBLING GROUPS CHANGED IN THIS BATCH: {sibs}\n\nRECENT SUBJECTS (house style):\n{style}\n\nCHANGED PATHS ({n}):\n{listing}\nDIFF (truncated={diff_truncated}):\n{diff}",
repo=repo, unit=unit, sibs=sibs, style=style, n=n, listing=listing, diff_truncated=diff_truncated, diff=diff);

let mut message = String::new();
let mut source = "llm";
let resp = ask_llm(user, Data::DString(sys.to_string()));
if resp.starts_with("ERROR:") {
    source = "fallback";
} else {
    // sanitize: strip fences/quotes, drop empties, enforce subject<=72
    let mut t = resp.trim().trim_matches('`').trim().to_string();
    if t.starts_with("```"){ t = t.lines().skip(1).collect::<Vec<_>>().join("\n"); }
    if t.ends_with("```"){ t = t.lines().take(t.lines().count().saturating_sub(1)).collect::<Vec<_>>().join("\n"); }
    let t = t.trim().trim_matches('"').trim().to_string();
    let mut lines: Vec<String> = t.lines().map(|l| l.to_string()).collect();
    if lines.is_empty() || lines[0].trim().is_empty() { source="fallback"; }
    else {
        let mut subj = lines[0].trim().trim_end_matches('.').to_string();
        if subj.chars().count() > 72 { subj = subj.chars().take(72).collect(); }
        lines[0] = subj;
        message = lines.join("\n");
    }
}
if source=="fallback" {
    message = format!("{}: update {} path(s)", unit, n);
}
// trailer: mark provenance of the message
message = format!("{}\n\nAgent-Message: {}", message.trim_end(), if source=="llm"{"llm"}else{"fallback"});
let mut o=DataObject::new();
o.put_string("status","ok");
o.put_string("unit",&unit);
o.put_string("source",source);
o.put_int("paths", n as i64);
o.put_string("message",&message);
o
}
