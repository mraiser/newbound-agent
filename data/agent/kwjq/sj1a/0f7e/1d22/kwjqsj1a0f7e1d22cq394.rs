// export_tick: the timer-fired auto-export. The timer is a fixed 1-minute
// heartbeat with NO cadence knowledge of its own; CURRICULUM_EXPORT_MIN in
// botd.properties (live globals system.apps.agent.runtime, the same map
// every LLM setting rides) is the single source of truth. Each fire
// compares wall-clock minutes elapsed since the last actual export
// (persisted in this command's own data record) against the setting and
// exports only when elapsed >= the setting. So the property changes pace
// live, with no timer re-registration and no dual knobs to disagree.
// on/off: absent/0/off disables (returns skipped); the manual
// curriculum_export path is untouched and always available.
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

// wall-clock gate: minutes since the last actual export vs the setting.
// last_run lives in globals (the AGENT_EXECUTIVE pattern for live runtime
// state) - the command's own meta record does not durably save ad-hoc puts.
let store = DataStore::new();
let mut g = DataStore::globals();
let mut state = match g.try_get_object("AGENT_CURRICULUM_EXPORT") {
    Ok(s) => s,
    Err(_) => {
        let s = DataObject::new();
        g.put_object("AGENT_CURRICULUM_EXPORT", s.clone());
        s
    }
};
let now = time();
let last_run = if state.has("last_run") { state.get_int("last_run") } else { 0 };
if last_run > 0 {
    let elapsed_min = (now - last_run) / 60000;
    if elapsed_min < minutes {
        return flat_status(&[
            ("skipped", "interval not elapsed"),
            ("cadence_min", &minutes.to_string()),
            ("elapsed_min", &elapsed_min.to_string()),
            ("next_in_min", &(minutes - elapsed_min).to_string()),
        ]);
    }
}

// land the batch where the trainer's drain loop reads it.
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
let path = ingest.join(format!("batch-auto-{}.jsonl", now));

let res = curriculum_export(path.display().to_string());

// stamp the export time: cadence changes behave, and the state is visible
// under globals AGENT_CURRICULUM_EXPORT like every other live executive state.
state.put_int("last_run", now);

// surface the cadence alongside the export's own counts
let mut o = res.clone();
o.put_int("cadence_min", minutes);
o.put_string("auto", "true");
o