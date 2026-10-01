pub fn llm_common() -> DataObject {
    let mut o = DataObject::new();
    o.put_string("kind","error");
    o
}