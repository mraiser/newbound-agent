fn entry(claim: &str, parent: &str, lifecycle: &str, steps: usize) -> DataObject {
    let mut e = DataObject::new();
    e.put_string("claim", claim);
    e.put_string("parent", parent);
    e.put_string("tags", &format!("plan,{}", lifecycle));
    if steps > 0 {
        let mut log = DataArray::new();
        for _ in 0..steps { log.push_object(DataObject::new()); }
        e.put_array("step_log", log);
    }
    e
}
let wait = crate::agent::plan::worker_start::parent_waits_for_active_child;
let coordinating = crate::agent::plan::worker_start::parent_is_coordination_task;
let mut classification_pass = true;
classification_pass &= coordinating("p", &vec![entry("c", "p", "completed", 3)]);
classification_pass &= coordinating("p", &vec![entry("c", "p", "done", 3)]);
classification_pass &= !coordinating("p", &vec![entry("c", "p", "abandoned", 0)]);
classification_pass &= !coordinating("childless", &vec![entry("c", "p", "proposed", 0)]);
let cases = vec![
    ("proposed child leaves parent eligible", vec![entry("c", "p", "proposed", 0)], false),
    ("accepted child defers parent", vec![entry("c", "p", "accepted", 0)], true),
    ("in-progress child defers parent", vec![entry("c", "p", "in-progress", 2)], true),
    ("blocked child returns parent to coordination", vec![entry("c", "p", "blocked", 1)], false),
    ("mixed proposed and blocked leaves parent eligible", vec![entry("a", "p", "proposed", 0), entry("b", "p", "blocked", 0)], false),
    ("newly completed child returns parent to coordination", vec![entry("c", "p", "completed", 3)], false),
    ("terminal children leave closing parent eligible", vec![entry("a", "p", "done", 2), entry("b", "p", "abandoned", 0)], false),
    ("unrelated active item does not defer parent", vec![entry("other", "", "accepted", 0), entry("c", "p", "proposed", 0)], false),
    ("budget-exhausted active child still requires reapproval", vec![entry("c", "p", "accepted", 20)], true),
];
let mut results = DataArray::new();
let mut passed = 0i64;
for (name, entries, expected) in cases {
    let actual = wait("p", &entries);
    let ok = actual == expected;
    if ok { passed += 1; }
    let mut r = DataObject::new();
    r.put_string("case", name);
    r.put_boolean("expected", expected);
    r.put_boolean("actual", actual);
    r.put_boolean("pass", ok);
    results.push_object(r);
}
let mut out = DataObject::new();
out.put_string("status", if passed == results.len() as i64 { "ok" } else { "err" });
out.put_int("passed", passed);
out.put_int("total", results.len() as i64 + 4);
if classification_pass { passed += 4; } else {
    out.put_string("status", "err");
    out.put_string("msg", "parent coordination classification fixture failed");
}
out.put_int("passed", passed);
out.put_boolean("closing_and_abandoned_classification_pass", classification_pass);
out.put_array("results", results);
out