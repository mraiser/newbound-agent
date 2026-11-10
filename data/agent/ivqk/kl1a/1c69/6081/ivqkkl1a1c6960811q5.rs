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
    _ => err("op must be status, board, import or verify"),
}