use std::collections::BTreeMap;

use nota_next::{
    Block, NotaDecode, NotaDecodeError, NotaDocumentEncode, NotaEncode,
    NotaNamedDocumentFieldDecode, NotaNamedDocumentFieldEncode, NotaSource,
};

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

#[derive(Clone, Debug, Eq, NotaDecode, NotaEncode, PartialEq)]
struct NamedVariants {
    name: String,
    variants: Vec<String>,
}

impl NotaNamedDocumentFieldDecode for NamedVariants {
    fn from_nota_named_document_field(
        name: &'static str,
        block: &Block,
    ) -> Result<Self, NotaDecodeError> {
        Ok(Self {
            name: name.to_owned(),
            variants: Vec::<String>::from_nota_block(block)?,
        })
    }
}

impl NotaNamedDocumentFieldEncode for NamedVariants {
    fn to_nota_named_document_field_body(&self) -> String {
        self.variants.to_nota()
    }
}

#[derive(Clone, Debug, Eq, NotaDecode, NotaEncode, PartialEq)]
#[nota(known_root)]
struct KnownRootDocument {
    name: String,
    imports: Vec<String>,
    #[nota(name = "Input")]
    input: NamedVariants,
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

#[test]
fn derive_reads_and_writes_known_root_document_bodies() {
    let source = NotaSource::new("[schema]\n[]\n[[Record] [Observe]]");
    let document = source
        .parse_document_body::<KnownRootDocument>()
        .expect("known-root body decodes");

    assert_eq!(document.name, "schema");
    assert_eq!(document.input.name, "Input");
    assert_eq!(document.input.variants, ["Record", "Observe"]);
    assert_eq!(
        document.to_nota_document_body().to_nota(),
        "[schema]\n[]\n[[Record] [Observe]]"
    );
}
