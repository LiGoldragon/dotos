use std::collections::BTreeMap;

use nota_next::{NotaDecode, NotaEncode, NotaSource};

#[derive(Clone, Debug, Eq, NotaDecode, NotaEncode, Ord, PartialEq, PartialOrd)]
struct Topic(String);

#[derive(Clone, Debug, Eq, NotaDecode, NotaEncode, PartialEq)]
struct Entry {
    topic: Topic,
    description: String,
    magnitude: u64,
}

#[derive(Clone, Debug, Eq, NotaDecode, NotaEncode, PartialEq)]
enum Request {
    Record(Entry),
    Observe(Topic),
    Ping,
}

#[derive(Clone, Debug, Eq, NotaDecode, NotaEncode, PartialEq)]
struct TopicMap {
    entries: BTreeMap<Topic, Entry>,
}

#[test]
fn derive_reads_and_writes_record_shapes() {
    let source = NotaSource::new("([schema] [derive works] 7)");
    let entry = source.parse::<Entry>().expect("entry decodes");

    assert_eq!(
        entry,
        Entry {
            topic: Topic(String::from("schema")),
            description: String::from("derive works"),
            magnitude: 7,
        }
    );
    assert_eq!(entry.to_nota(), "([schema] [derive works] 7)");
}

#[test]
fn derive_reads_and_writes_enum_shapes() {
    let record = NotaSource::new("(Record ([schema] [derive works] 7))")
        .parse::<Request>()
        .expect("record request decodes");
    let ping = NotaSource::new("Ping")
        .parse::<Request>()
        .expect("unit request decodes");

    assert!(matches!(record, Request::Record(_)));
    assert_eq!(record.to_nota(), "(Record ([schema] [derive works] 7))");
    assert_eq!(ping, Request::Ping);
    assert_eq!(ping.to_nota(), "Ping");
}

#[test]
fn derive_uses_shared_collection_codec() {
    let source = NotaSource::new("({alpha ([alpha] [first] 1) beta ([beta] [second] 2)})");
    let entries = source
        .parse::<TopicMap>()
        .expect("map-backed record decodes");

    assert_eq!(entries.entries.len(), 2);
    assert_eq!(
        entries.to_nota(),
        "({[alpha] ([alpha] [first] 1) [beta] ([beta] [second] 2)})"
    );
}
