// Compose ONE commit message for ONE store unit (lib.ctl) in a repo. READ-ONLY:
// stages nothing, commits nothing - it only reads git state and asks the LLM.
// The unit's own diff is the source of truth, so the message is correct no
// matter WHO made the edit (this agent, a peer, an MCP harness, or a hand edit).
// Prompt = house-style subjects (log) + sibling units changed in the same batch
// (so multi-control changes read as parts of a whole) + the unit's diff.
// Output contract enforced here: subject <=72 chars, imperative, blank line,
// 1-4 why-bullets, an Agent-Message trailer naming the arm. Falls back to a
// deterministic mechanical message when the LLM fails or misbehaves, so the
// caller can always commit. Returns {message, source:"llm"|"fallback", unit}.
fn fail(msg:&str)->DataObject{let mut o=DataObject::new();o.put_string("status","err");o.put_string("msg",msg);o}
fn sargs(v:&[&str])->DataArray{let mut a=DataArray::new();for s in v{a.push_string(s);}a}
fn okr(r:&DataObject)->bool{ r.get_string("status")=="ok" }
fn outs(r:&DataObject)->String{ r.try_get_string("out").unwrap_or_default() }
let unit = unit.trim().to_string();
let parts:Vec<&str> = unit.splitn(2,'.').collect();
if parts.len()!=2 { return fail("unit must be 'lib.ctl'"); }
let (lib, ctl) = (parts[0], parts[1]);

// --- gather the unit's diff (working tree + already-staged, capped) ----------
let pathsib = format!("data/{}", lib);
let mut dargs = DataArray::new();
dargs.push_string("HEAD"); dargs.push_string("--"); dargs.push_string(&pathsib);
let d = crate::api::new().dev.git.read(repo.clone(), "diff".to_string(), dargs);
let mut diff = if okr(&d){ outs(&d) } else { String::new() };
// also the generated rust for this control, so a message can mention behavior
let gen = format!("newbound_core/src/{}/{}", lib, ctl);
let mut gargs = DataArray::new();
gargs.push_string("HEAD"); gargs.push_string("--"); gargs.push_string(&gen);
let g = crate::api::new().dev.git.read(repo.clone(), "diff".to_string(), gargs);
if okr(&g){ let gd = outs(&g); if !gd.trim().is_empty(){ diff.push_str("\n# generated sources\n"); diff.push_str(&gd); } }
let diff_truncated = diff.len() > 12000;
if diff_truncated { diff = diff.chars().take(12000).collect(); diff.push_str("\n...[diff truncated]..."); }
if diff.trim().is_empty(){ return fail("no diff for this unit - nothing to describe"); }

// --- house style: recent subjects --------------------------------------------
let lg = crate::api::new().dev.git.read(repo.clone(), "log".to_string(), sargs(&["-8","--format=%s"]));
let style = if okr(&lg){ outs(&lg) } else { String::new() };

// --- sibling context ----------------------------------------------------------
let sibs = if siblings.trim().is_empty(){ "none".to_string() } else { siblings.trim().to_string() };

// --- the prompt ---------------------------------------------------------------
let sys = "You write ONE git commit message for ONE store unit of the Newbound platform. \
A unit is one control (lib.ctl): a record plus its facets, its command impls, and generated rust. \
Rules: first line is the subject - imperative mood, <=72 chars, no trailing period, match the repo's existing subject style. \
Then a blank line, then 1-4 bullets explaining WHY (not restating the diff). \
If sibling units changed in the same batch, you may end with a line like 'Part of: <siblings>'. \
Output ONLY the commit message - no preamble, no code fences, no quotes around it.";
let user = format!(
"REPO: {repo}\nUNIT: {unit}\nSIBLING UNITS CHANGED IN THIS BATCH: {sibs}\n\nRECENT SUBJECTS (house style):\n{style}\n\nDIFF (truncated={diff_truncated}):\n{diff}",
repo=repo, unit=unit, sibs=sibs, style=style, diff_truncated=diff_truncated, diff=diff);

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
    message = format!("{}: update {}", lib, unit);
}
// trailer: mark provenance of the message
message = format!("{}\n\nAgent-Message: {}", message.trim_end(), if source=="llm"{"llm"}else{"fallback"});
let mut o=DataObject::new();
o.put_string("status","ok");
o.put_string("unit",&unit);
o.put_string("source",source);
o.put_string("message",&message);
o