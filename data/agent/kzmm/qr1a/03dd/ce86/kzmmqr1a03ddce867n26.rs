// Apply the Noobscape v2 mechanism to the extracted Firefox source by
// anchored insertion (robust to point-release line drift; no header edit
// needed). Idempotent: a tree already carrying the mechanism is a no-op.
fn prop(key: &str, dflt: &str) -> String {
    (|| -> Option<String> {
        let r = DataStore::globals().try_get_object("system").ok()?
            .try_get_object("apps").ok()?.try_get_object("agent").ok()?
            .try_get_object("runtime").ok()?;
        match r.try_get_string(key) { Ok(v) if !v.trim().is_empty() => Some(v.trim().to_string()), _ => None }
    })().unwrap_or_else(|| dflt.to_string())
}
fn err(m: String) -> DataObject { let mut o = DataObject::new(); o.put_string("status","err"); o.put_string("msg", &m); o }

let workspace = prop("NOOBSCAPE_WORKSPACE", "/newbound/runtime/agent/noobscape-build");
let version   = prop("NOOBSCAPE_VERSION", "128.0esr");
let src_dir = format!("{}/work/firefox-{}", workspace, version);
let cpp_path = format!("{}/docshell/base/nsDocShell.cpp", src_dir);

let src = match std::fs::read_to_string(&cpp_path) {
    Ok(s) => s,
    Err(e) => return err(format!("cannot read {} (extract the source first): {}", cpp_path, e)),
};
if src.contains("NoobscapeStartWatcher") {
    let mut o = DataObject::new();
    o.put_string("status", "ok"); o.put_string("action", "already");
    o.put_string("file", &cpp_path); o.put_boolean("changed", false);
    return o;
}

let block = match std::fs::read_to_string(DataStore::new().root.join("agent").join("_ASSETS").join("browser_builder").join("nsDocShell_v2_block.cpp")) {
    Ok(b) => b,
    Err(e) => return err(format!("cannot read v2 block asset: {}", e)),
};

let anchor = "nsresult nsDocShell::EndPageLoad(nsIWebProgress* aProgress,\n                                 nsIChannel* aChannel, nsresult aStatus) {\n";
let idx = match src.find(anchor) {
    Some(i) => i,
    None => return err(format!("EndPageLoad anchor not found in {} — Firefox may have changed this signature; patch by hand or update the anchor", cpp_path)),
};
let after_brace = idx + anchor.len();
let mut out = String::with_capacity(src.len() + block.len() + 64);
out.push_str(&src[..idx]);
out.push('\n');
out.push_str(&block);
out.push('\n');
out.push_str(&src[idx..after_brace]);
out.push_str("  NoobscapeStartWatcher(this);\n\n");
out.push_str(&src[after_brace..]);

// Back up the pristine file once, then write.
let orig = format!("{}.noobscape.orig", cpp_path);
if !std::path::Path::new(&orig).exists() { let _ = std::fs::write(&orig, &src); }
if let Err(e) = std::fs::write(&cpp_path, &out) { return err(format!("write {}: {}", cpp_path, e)); }

let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_string("action", "patched");
o.put_string("file", &cpp_path);
o.put_string("backup", &orig);
o.put_boolean("changed", true);
o.put_boolean("header_patch_needed", false);
o