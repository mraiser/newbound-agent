// Explicit kb.plan recovery. Raw evidence is preserved before any live-path
// replacement. No lifecycle, acceptance, step budget, or run id is rewritten.
fn fail(msg: &str) -> DataObject {
    let mut o = DataObject::new(); o.put_string("status", "err"); o.put_string("msg", msg); o
}
fn content_hash(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
    format!("{:016x}", h)
}
fn bounded(s: &str, n: usize) -> String { s.chars().take(n).collect() }
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| "path has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let tmp = parent.join(format!(".{}.tmp-{}-{}", path.file_name().unwrap().to_string_lossy(), std::process::id(), time()));
    {
        let mut f = fs::File::create(&tmp).map_err(|e| format!("create {}: {}", tmp.display(), e))?;
        f.write_all(bytes).map_err(|e| format!("write {}: {}", tmp.display(), e))?;
        f.sync_all().map_err(|e| format!("sync {}: {}", tmp.display(), e))?;
    }
    fs::rename(&tmp, path).map_err(|e| format!("rename {} -> {}: {}", tmp.display(), path.display(), e))?;
    if let Ok(dir) = fs::File::open(parent) { let _ = dir.sync_all(); }
    Ok(())
}
fn json_value(bytes: &[u8], start: usize) -> Result<(serde_json::Value, usize), String> {
    let mut stream = serde_json::Deserializer::from_slice(&bytes[start..]).into_iter::<serde_json::Value>();
    let value = stream.next().ok_or_else(|| "missing JSON value".to_string())
        .map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    Ok((value, start + stream.byte_offset()))
}
fn ws(bytes: &[u8], mut p: usize) -> usize {
    while p < bytes.len() && bytes[p].is_ascii_whitespace() { p += 1; } p
}
fn recover_patches(bytes: &[u8]) -> Result<(DataArray, bool, usize), String> {
    if let Ok((root, _)) = json_value(bytes, 0) {
        let values = root.get("data").and_then(|d| d.get("list")).and_then(|v| v.as_array())
            .ok_or_else(|| "valid journal has no data.list".to_string())?;
        let mut out = DataArray::new();
        for v in values { out.push_object(DataObject::try_from_string(&serde_json::to_string(v).map_err(|e| e.to_string())?).map_err(|e| format!("{:?}", e))?); }
        return Ok((out, false, bytes.len()));
    }
    let needle = b"\"list\":[";
    let at = bytes.windows(needle.len()).position(|w| w == needle).ok_or_else(|| "journal has no data.list array".to_string())?;
    let mut p = ws(bytes, at + needle.len());
    let mut out = DataArray::new();
    loop {
        p = ws(bytes, p);
        if p >= bytes.len() || bytes[p] == b']' { break; }
        let (value, end) = match json_value(bytes, p) { Ok(v) => v, Err(_) => break };
        out.push_object(DataObject::try_from_string(&serde_json::to_string(&value).map_err(|e| e.to_string())?).map_err(|e| format!("{:?}", e))?);
        p = ws(bytes, end);
        if p < bytes.len() && bytes[p] == b',' { p += 1; continue; }
        break;
    }
    if out.len() == 0 { return Err("no complete patch objects could be recovered".to_string()); }
    Ok((out, true, p))
}
fn compact_entry(entry: &DataObject) -> DataObject {
    let mut e = entry.deep_copy();
    if let Ok(cp) = e.try_get_object("checkpoint") {
        e.put_object("checkpoint", crate::agent::plan::compact_checkpoint::compact_checkpoint(cp));
    }
    if let Ok(active) = e.try_get_object("active_run") {
        let mut a = active.deep_copy();
        if let Ok(inherited) = active.try_get_object("inherited") {
            let mut r = DataObject::new();
            r.put_string("run_id", &inherited.try_get_string("run_id").unwrap_or_default());
            r.put_int("time", inherited.try_get_int("time").unwrap_or(0));
            r.put_string("source", "kb.plan.checkpoint");
            a.put_object("inherited_ref", r);
            a.remove_property("inherited");
        }
        if let Ok(text) = active.try_get_string("inherited_text") {
            a.put_string("inherited_text", &bounded(&text, 8000));
        }
        e.put_object("active_run", a);
    }
    if let Ok(continuity) = e.try_get_object("continuity") {
        let mut c = continuity.deep_copy();
        if let Ok(text) = continuity.try_get_string("handoff_received") {
            c.put_string("handoff_received", &bounded(&text, 8000));
        }
        e.put_object("continuity", c);
    }
    if let Ok(logs) = e.try_get_array("step_log") {
        let n = logs.len();
        let mut compacted = DataArray::new();
        for i in 0..n {
            let old = logs.get_object(i);
            let latest = i + 1 == n;
            let mut row = DataObject::new();
            // Preserve all scalar step-log metadata, but never arbitrary nested
            // runs/transcripts. Explicit legacy text remains enough for continuity.
            for (key, value) in old.objects() {
                match value {
                    ndata::data::Data::DString(s) => {
                        let cap = if key == "result" || key == "checkpoint" { if latest { 8000 } else { 1200 } }
                            else if key == "run_id" || key == "previous_run_id" { 180 }
                            else if key == "home" || key == "author" { 400 } else { 2000 };
                        row.put_string(&key, &bounded(&s, cap));
                    },
                    ndata::data::Data::DInt(v) => row.put_int(&key, v),
                    ndata::data::Data::DFloat(v) => row.put_float(&key, v),
                    ndata::data::Data::DBoolean(v) => row.put_boolean(&key, v),
                    _ => {}
                }
            }
            if let Ok(cv) = old.try_get_object("checkpoint_v2") {
                row.put_object("checkpoint_v2", crate::agent::plan::compact_checkpoint::compact_checkpoint(cv));
            }
            compacted.push_object(row);
        }
        e.put_array("step_log", compacted);
    }
    e
}
let store = DataStore::new();
let ctlid = crate::api::new().dev.editcontrol.lookup_id("kb".to_string(), "plan".to_string());
let jid = format!("{}_patches", ctlid);
let record_path = store.get_data_file("kb", &ctlid);
let memory_path = record_path.parent().unwrap().join(format!("{}.memory", ctlid));
let journal_path = store.get_data_file("kb", &jid);
let memory = match fs::read_to_string(&memory_path) { Ok(s) => s, Err(e) => return fail(&format!("read board attachment: {}", e)) };
let hash = content_hash(&memory.replace("\r", ""));
if base != hash { let mut o = fail("stale_base"); o.put_string("current_hash", &hash); return o; }
if apply && confirm != "RECOVER KB PLAN" { return fail("apply requires confirm='RECOVER KB PLAN'"); }
let parsed = match DataObject::try_from_string(&format!("{{\"a\":{}}}", memory)) { Ok(v) => v, Err(e) => return fail(&format!("live board malformed: {:?}", e)) };
let entries = parsed.get_array("a");
let mut after = DataArray::new();
let mut original_lifecycles = DataArray::new();
let mut original_steps = DataArray::new();
for ee in entries.objects() {
    let e = ee.object();
    original_lifecycles.push_string(&format!("{}|{}", e.try_get_string("claim").unwrap_or_default(), e.try_get_string("tags").unwrap_or_default()));
    original_steps.push_int(e.try_get_array("step_log").map(|a| a.len() as i64).unwrap_or(0));
    after.push_object(compact_entry(&e));
}
let after_text = after.to_string();
let check = match DataObject::try_from_string(&format!("{{\"a\":{}}}", after_text)) { Ok(v) => v, Err(_) => return fail("compacted board failed validation") };
let checked = check.get_array("a");
if checked.len() != entries.len() { return fail("entry count changed") }
for i in 0..checked.len() {
    let e = checked.get_object(i);
    if format!("{}|{}", e.try_get_string("claim").unwrap_or_default(), e.try_get_string("tags").unwrap_or_default()) != original_lifecycles.get_string(i) { return fail("claim/lifecycle changed during compaction"); }
    if e.try_get_array("step_log").map(|a| a.len() as i64).unwrap_or(0) != original_steps.get_int(i) { return fail("step count changed during compaction"); }
}
let journal_bytes = match fs::read(&journal_path) { Ok(b) => b, Err(e) => return fail(&format!("read journal: {}", e)) };
let (patches, was_malformed, consumed) = match recover_patches(&journal_bytes) { Ok(v) => v, Err(e) => return fail(&format!("journal recovery: {}", e)) };
let mut journal = DataObject::new(); journal.put_string("id", &jid); journal.put_string("username", "system");
journal.put_array("readers", DataArray::new()); journal.put_array("writers", DataArray::new());
journal.put_int("time", time());
let mut jd = DataObject::new(); jd.put_array("list", patches.clone()); journal.put_object("data", jd);
let journal_text = journal.to_string();
let mut out = DataObject::new(); out.put_string("status", "ok"); out.put_string("action", if apply { "apply" } else { "dry-run" });
out.put_string("base", &hash); out.put_int("entries", entries.len() as i64);
out.put_int("board_bytes_before", memory.len() as i64); out.put_int("board_bytes_after", after_text.len() as i64);
out.put_int("journal_bytes_before", journal_bytes.len() as i64); out.put_int("journal_bytes_after", journal_text.len() as i64);
out.put_int("recovered_patches", patches.len() as i64); out.put_boolean("journal_was_malformed", was_malformed);
out.put_int("journal_prefix_bytes", consumed as i64); out.put_int("discarded_incomplete_bytes", journal_bytes.len().saturating_sub(consumed) as i64);
if !apply { out.put_string("msg", "Dry run only. Apply preserves raw evidence and atomically replaces the live board attachment and journal."); return out; }
let worker = DataStore::globals().try_get_object("AGENT_PLAN_WORKER").ok();
if worker.as_ref().map(|w| w.try_get_boolean("in_step").unwrap_or(false)).unwrap_or(false) { return fail("a plan step is running; recovery refused"); }
let _repair_guard = match crate::agent::plan::step_lock::acquire("kb.plan storage recovery", "repair_storage") {
    Ok(g) => g,
    Err(_) => return fail("the plan-step lock is held; recovery refused"),
};
let recovery_dir = store.root.parent().unwrap().join("runtime").join("agent").join("plan-recovery").join(format!("{}", time()));
fs::create_dir_all(&recovery_dir).map_err(|e| e.to_string()).unwrap_or(());
let raw_journal = recovery_dir.join("plan.journal.corrupt.bin");
atomic_write(&raw_journal, &journal_bytes).unwrap_or_else(|e| eprintln!("plan recovery backup failed: {}", e));
if !raw_journal.exists() || fs::metadata(&raw_journal).map(|m| m.len()).unwrap_or(0) != journal_bytes.len() as u64 { return fail("raw journal backup failed; live files untouched"); }
let raw_board = recovery_dir.join("plan.memory.before.json");
if let Err(e) = atomic_write(&raw_board, memory.as_bytes()) { return fail(&format!("raw board backup failed: {}", e)); }
let after_board_backup = recovery_dir.join("plan.memory.after.json");
if let Err(e) = atomic_write(&after_board_backup, after_text.as_bytes()) { return fail(&format!("after-board archive failed: {}", e)); }
let after_journal = recovery_dir.join("plan.journal.after.json");
if let Err(e) = atomic_write(&after_journal, journal_text.as_bytes()) { return fail(&format!("after-journal archive failed: {}", e)); }
let mut manifest = DataObject::new(); manifest.put_string("operation", "kb.plan bounded recovery"); manifest.put_int("time", time());
manifest.put_string("author", "agent.plan.repair_storage"); manifest.put_string("base", &hash);
manifest.put_int("entries", entries.len() as i64); manifest.put_int("recovered_patches", patches.len() as i64);
manifest.put_boolean("journal_was_malformed", was_malformed); manifest.put_int("discarded_incomplete_bytes", journal_bytes.len().saturating_sub(consumed) as i64);
manifest.put_string("note", "Raw pre-repair bytes retained. Only the incomplete journal suffix is discarded; durable checkpoints are compacted. Lifecycle and step counts unchanged.");
if let Err(e) = atomic_write(&recovery_dir.join("manifest.json"), manifest.to_string().as_bytes()) { return fail(&format!("manifest archive failed: {}", e)); }
// Journal first: the old writer panics while reading a malformed journal, so
// this ordering restores its ability to record the board compaction itself.
if let Err(e) = atomic_write(&journal_path, journal_text.as_bytes()) { return fail(&format!("journal replacement failed; raw evidence retained at {}: {}", recovery_dir.display(), e)); }
let verified = store.get_data("kb", &jid).get_object("data").get_array("list");
if verified.len() != patches.len() { return fail("journal verification count mismatch; archived recovery is intact"); }
let label = "plan recovery: bounded checkpoints; valid patch prefix preserved";
let written = crate::api::new().dev.code.patch_control_facet("kb".to_string(), "plan".to_string(), "memory".to_string(), memory.clone(), after_text.clone(), hash.clone(), label.to_string(), "recovery".to_string(), nn_sessionid.clone());
if written.try_get_string("status").unwrap_or_default() != "ok" {
    out.put_string("status", "err"); out.put_string("msg", "journal recovered but board compaction failed"); out.put_object("write_result", written);
    out.put_string("recovery_dir", &recovery_dir.to_string_lossy()); return out;
}
out.put_string("recovery_dir", &recovery_dir.to_string_lossy());
out.put_string("new_hash", &written.try_get_string("hash").unwrap_or_default());
out.put_string("patch_id", &written.try_get_string("patch_id").unwrap_or_default());
out.put_string("msg", "Recovered the valid journal prefix, preserved raw evidence, compacted all durable checkpoints, and verified the journal through DataStore.");
out