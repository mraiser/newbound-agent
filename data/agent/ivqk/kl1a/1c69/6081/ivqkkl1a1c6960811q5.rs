let lib = if lib.trim().is_empty() { LIB.to_string() } else { lib.trim().to_string() };
let ns = if ns.trim().is_empty() { NS.to_string() } else { ns.trim().to_string() };
if lib != "kb" && lib != "runtime" { return err("lib must be kb or runtime"); }
if ns.is_empty() || ns.len() > 40 || !ns.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') { return err("ns must be [A-Za-z0-9_], max 40 chars"); }
match op.as_str() {
    "status" => {
        let idx = read_index(&lib, &ns);
        let mut o = DataObject::new(); o.put_string("status", "ok");
        o.put_string("lib", &lib); o.put_string("ns", &ns);
        o.put_boolean("imported", has_index(&lib, &ns));
        o.put_int("items", idx.get_array("order").len() as i64);
        o.put_string("hash", &format!("r{}", idx.try_get_int("rev").unwrap_or(0)));
        let mut lc = DataObject::new();
        for (_k, v) in idx.get_object("items").objects() {
            if !v.is_object() { continue; }
            let l = v.object().try_get_string("lifecycle").unwrap_or_default();
            let l = if l.is_empty() { "none".to_string() } else { l };
            let n = lc.try_get_int(&l).unwrap_or(0); lc.put_int(&l, n + 1);
        }
        o.put_object("lifecycles", lc);
        o
    },
    "board" => board(&lib, &ns),
    "import" => match legacy_entries() {
        Some(e) => import_entries(&lib, &ns, &e, dry_run),
        None => err("kb.plan memory facet did not parse as a JSON array"),
    },
    "verify" => match legacy_entries() {
        Some(e) => { let mut c = compare(&lib, &ns, &e); c.put_string("status", "ok"); c },
        None => err("kb.plan memory facet did not parse as a JSON array"),
    },
    // Retire the legacy monolith once the records are the board: back the
    // kb.plan memory facet up to runtime/agent/plan-legacy/, empty it to [],
    // and delete the control's patch journal (whole-facet snapshots of it).
    "retire" => {
        if lib != LIB || ns != NS { return err("retire applies to kb/plan only"); }
        if !has_index(&lib, &ns) { return err("the board has not been imported into records; nothing may be retired"); }
        let api = crate::api::new();
        let r = api.dev.code.read_control_facet("kb".to_string(), "plan".to_string(), "memory".to_string());
        let exists = r.try_get_boolean("exists").unwrap_or(false);
        let src = if exists { r.try_get_string("source").unwrap_or_default() } else { String::new() };
        let legacy = legacy_entries().map(|a| a.len() as i64).unwrap_or(-1);
        // Called by lookup: this crate's generated api may predate patch_journal.
        let journal = |op: &str, dry: bool| -> DataObject {
            let mut a = DataObject::new();
            a.put_string("op", op); a.put_string("lib", "kb"); a.put_string("ctl", "plan");
            a.put_string("to_lib", ""); a.put_object("entry", DataObject::new());
            a.put_int("limit", 0); a.put_boolean("dry_run", dry); a.put_string("nn_sessionid", "");
            let c = flowlang::command::Command::lookup("dev", "code", "patch_journal");
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c.execute(a))) {
                Ok(Ok(r)) => if r.has("a") && r.get_property("a").is_object() { r.get_object("a") } else { r },
                _ => err("dev.code.patch_journal failed"),
            }
        };
        let js = journal("status", true);
        let mut o = DataObject::new(); o.put_string("status", "ok");
        o.put_boolean("dry_run", dry_run);
        o.put_int("records", read_index(&lib, &ns).get_array("order").len() as i64);
        o.put_boolean("facet_exists", exists);
        o.put_int("facet_bytes", src.len() as i64);
        o.put_int("facet_entries", legacy);
        o.put_int("journal_patches", js.try_get_int("total").unwrap_or(0));
        o.put_string("journal_storage", &js.try_get_string("storage").unwrap_or_default());
        o.put_boolean("journal_legacy_present", js.try_get_boolean("legacy_present").unwrap_or(false));
        let already = !exists || src.trim() == "[]";
        o.put_boolean("facet_already_empty", already);
        if dry_run { return o; }
        if !already {
            let dir = "runtime/agent/plan-legacy";
            let path = format!("{}/kb-plan-memory-{}.json", dir, time());
            if std::fs::create_dir_all(dir).is_err() || std::fs::write(&path, src.as_bytes()).is_err() {
                return err("could not write the backup; nothing was changed");
            }
            let back = std::fs::read_to_string(&path).unwrap_or_default();
            if back != src { return err("the backup did not read back equal; nothing was changed"); }
            o.put_string("backup", &path);
            let w = api.dev.code.patch_control_facet("kb".to_string(), "plan".to_string(), "memory".to_string(), String::new(), "[]".to_string(), r.try_get_string("hash").unwrap_or_default(), "retire the legacy plan monolith (records are the board)".to_string(), "plan-store".to_string(), String::new());
            if w.try_get_string("status").unwrap_or_default() != "ok" {
                let mut e = err("emptying the facet failed; the backup was written, the journal was left alone");
                e.put_object("write", w); e.put_string("backup", &path);
                return e;
            }
            o.put_boolean("facet_emptied", true);
        }
        let d = journal("delete", false);
        o.put_boolean("journal_deleted", d.try_get_string("status").unwrap_or_default() == "ok");
        o.put_object("journal", d);
        o
    },
    _ => err("op must be status, board, import, verify or retire"),
}