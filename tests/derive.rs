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
enum TypeReference {
    String,
    Plain(String),
    Map(Box<Self>, Box<Self>),
    Optional(Box<Self>),
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
fn derive_reads_and_writes_multi_field_enum_payloads() {
    let reference = NotaSource::new("(Map (String (Optional (Plain [Entry]))))")
        .parse::<TypeReference>()
        .expect("multi-field enum variant decodes");

    assert_eq!(
        reference,
        TypeReference::Map(
            Box::new(TypeReference::String),
            Box::new(TypeReference::Optional(Box::new(TypeReference::Plain(
                "Entry".to_owned()
            )))),
        )
    );
    assert_eq!(
        reference.to_nota(),
        "(Map (String (Optional (Plain [Entry]))))"
    );
}

#[test]
fn derive_rejects_multi_field_enum_payloads_with_wrong_tuple_size() {
    let error = NotaSource::new("(Map (String))")
        .parse::<TypeReference>()
        .expect_err("multi-field enum variant requires its tuple payload");

    assert!(
        error
            .to_string()
            .contains("expected Map to hold 2 root objects"),
        "error was {error}"
    );
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
    let object = NotaSource::new("([schema] [] [[Record] [Observe]])")
        .parse::<KnownRootDocument>()
        .expect("parenthesized object body decodes");

    assert_eq!(document.name, "schema");
    assert_eq!(document.input.name, "Input");
    assert_eq!(document.input.variants, ["Record", "Observe"]);
    assert_eq!(document, object);
    assert_eq!(
        document.to_nota_document_body().to_nota(),
        "[schema]\n[]\n[[Record] [Observe]]"
    );
}
