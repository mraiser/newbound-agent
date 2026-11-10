let e = exists(path.clone());
let d = e && is_dir(path.clone());
let mut o = DataObject::new();
o.put_boolean("exists", e);
o.put_boolean("is_dir", d);
o.put_string("mime_type", &(if e && !d { mime_type(path) } else { String::new() }));
o