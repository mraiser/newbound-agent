let mut o = DataObject::new();
o.put_string("status", "ok");
o.put_boolean("imported", store::has_index(LIB, NS));
o.put_string("hash", &format!("r{}", store::read_index(LIB, NS).try_get_int("rev").unwrap_or(0)));
o.put_int("items", store::read_index(LIB, NS).get_array("order").len() as i64);
o