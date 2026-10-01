// export_tick: the timer-fired auto-export. Reads CURRICULUM_EXPORT_MIN from
// globals (system.apps.agent.runtime - the same live botd.properties map every
// LLM setting rides) to learn its own cadence, then delegates to
// curriculum_export, landing a batch in the trainer's ingest dir. The interval
// the timer uses is read from the same key by the service-boot wiring, so the
// two never disagree. on/off: absent/0/off disables (returns skipped); the
// manual curriculum_export path is untouched and always available.
fn prop(key: &str, dflt: &str) -> String {
    (|| -> Option<String> {
        let s = DataStore::globals().try_get_object("system").ok()?;
        let a = s.try_get_object("apps").ok()?;
        let g = a.try_get_object("agent").ok()?;
        let r = g.try_get_object("runtime").ok()?;
        match r.try_get_string(key) {
            Ok(v) if !v.trim().is_empty() => Some(v.trim().to_string()),
            _ => None,
        }
    })().unwrap_or_else(|| dflt.to_string())
}
fn flat_status(kv: &[(&str, &str)]) -> DataObject {
    let mut o = DataObject::new();
    o.put_string("status", "ok");
    for (k, v) in kv { o.put_string(k, v); }
    o
}

// gate: on/off like every other LLM setting. Truthy minutes > 0 enables.
let raw = prop("CURRICULUM_EXPORT_MIN", "off");
let minutes: i64 = match raw.trim().to_lowercase().as_str() {
    "off" | "" | "0" | "no" | "false" => 0,
    s => s.parse::<i64>().unwrap_or(0),
};
if minutes <= 0 {
    return flat_status(&[("skipped", "CURRICULUM_EXPORT_MIN disabled or unset"),
                          ("cadence_min", "0")]);
}

// land the batch where the trainer's drain loop reads it.
let store = DataStore::new();
let root = match store.root.canonicalize().ok()
        .and_then(|r| r.parent().map(|p| p.to_path_buf())) {
    Some(r) => r,
    None => {
        let mut o = DataObject::new();
        o.put_string("status", "err");
        o.put_string("msg", "cannot resolve runtime folder");
        return o;
    }
};
let ingest = root.join("runtime").join("agent").join("model").join("ingest");
let _ = std::fs::create_dir_all(&ingest);
let path = ingest.join(format!("batch-auto-{}.jsonl", time()));

let res = curriculum_export(path.display().to_string());
// surface the cadence alongside the export's own counts
let mut o = res.clone();
o.put_int("cadence_min", minutes);
o.put_string("auto", "true");
o