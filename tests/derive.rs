use std::collections::BTreeMap;

use nota::{
    Block, Delimiter, Document, NotaDecode, NotaDecodeError, NotaDocumentEncode, NotaEncode,
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
    let source = NotaSource::new("{schema (derive works) 7}");
    let entry = source.parse::<Entry>().expect("entry decodes");

    assert_eq!(
        entry,
        Entry {
            topic: Topic(String::from("schema")),
            description: String::from("derive works"),
            magnitude: 7,
        }
    );
    assert_eq!(entry.to_nota(), "{schema (derive works) 7}");
}

#[test]
fn derive_reads_and_writes_enum_shapes() {
    let record = NotaSource::new("Record.{schema (derive works) 7}")
        .parse::<Request>()
        .expect("record request decodes");
    let ping = NotaSource::new("Ping")
        .parse::<Request>()
        .expect("unit request decodes");

    assert!(matches!(record, Request::Record(_)));
    assert_eq!(record.to_nota(), "Record.{schema (derive works) 7}");
    assert_eq!(ping, Request::Ping);
    assert_eq!(ping.to_nota(), "Ping");
}

#[test]
fn derive_reads_and_writes_multi_field_enum_payloads() {
    let reference = NotaSource::new("Map.{String Optional.Plain.Entry}")
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
    assert_eq!(reference.to_nota(), "Map.{String Optional.Plain.Entry}");
}

#[test]
fn derive_rejects_multi_field_enum_payloads_with_wrong_tuple_size() {
    let error = NotaSource::new("Map.{String}")
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
    let source = NotaSource::new("{Map.(alpha.{alpha first 1} beta.{beta second 2})}");
    let entries = source
        .parse::<TopicMap>()
        .expect("map-backed record decodes");

    assert_eq!(entries.entries.len(), 2);
    assert_eq!(
        entries.to_nota(),
        "{Map.(alpha.{alpha first 1} beta.{beta second 2})}"
    );
}

#[test]
fn derive_reads_and_writes_known_root_document_bodies() {
    let source = NotaSource::new("schema\n[]\n[Record Observe]");
    let document = source
        .parse_document_body::<KnownRootDocument>()
        .expect("known-root body decodes");
    let body_document = source
        .parse_body::<KnownRootDocument>()
        .expect("known-root body decodes through semantic body API");
    let object = NotaSource::new("{schema [] [Record Observe]}")
        .parse::<KnownRootDocument>()
        .expect("brace object body decodes");

    assert_eq!(document.name, "schema");
    assert_eq!(document.input.name, "Input");
    assert_eq!(document.input.variants, ["Record", "Observe"]);
    assert_eq!(document, body_document);
    assert_eq!(document, object);
    assert_eq!(
        document.to_nota_document_body().to_nota(),
        "schema\n[]\n[Record Observe]"
    );
}

#[test]
fn derive_body_parser_is_wrapper_agnostic_for_struct_types() {
    let wrapped = Document::parse("(schema (derive works) 7)").expect("wrapped form parses");
    let wrapper_root = wrapped.root_object_at(0).expect("one root object");
    let wrapper_body = wrapper_root
        .as_delimited(Delimiter::Parenthesis)
        .expect("wrapper is parenthesized");
    let from_wrapper = Entry::from_body_objects(wrapper_body).expect("body decodes");

    let unwrapped = Document::parse("schema (derive works) 7").expect("body-only form parses");
    let from_file_root = Entry::from_body_objects(unwrapped.root_objects()).expect("body decodes");

    assert_eq!(from_wrapper, from_file_root);
    assert_eq!(
        from_file_root,
        Entry {
            topic: Topic(String::from("schema")),
            description: String::from("derive works"),
            magnitude: 7,
        }
    );
}

#[test]
fn derive_body_parser_validates_field_count() {
    let too_few = Document::parse("schema [derive works]").expect("source parses");
    let error = Entry::from_body_objects(too_few.root_objects())
        .expect_err("two-field body cannot fill three-field struct");
    let message = error.to_string();
    assert!(
        message.contains("Entry") && message.contains("3 root objects"),
        "error was {message}"
    );

    let too_many = Document::parse("schema [derive works] 7 stray").expect("source parses");
    let error = Entry::from_body_objects(too_many.root_objects())
        .expect_err("four-object body cannot fill three-field struct");
    assert!(error.to_string().contains("Entry"), "error was {error}");
}

mod local_result_alias {
    use nota::{NotaDecode, NotaEncode, NotaSource};

    type Result<Value> = std::result::Result<Value, String>;

    fn local_alias_is_in_scope() -> Result<()> {
        Ok(())
    }

    #[derive(Clone, Debug, Eq, NotaDecode, NotaEncode, PartialEq)]
    struct AliasedRecord {
        name: String,
        count: u16,
    }

    #[derive(Clone, Debug, Eq, NotaDecode, NotaEncode, PartialEq)]
    enum AliasedRequest {
        Ping,
        Record(AliasedRecord),
    }

    #[test]
    fn derive_generated_code_ignores_local_result_alias() {
        local_alias_is_in_scope().expect("local alias helper returns");
        let record = NotaSource::new("{schema 7}")
            .parse::<AliasedRecord>()
            .expect("record decodes despite local Result alias");
        let request = NotaSource::new("Record.{schema 7}")
            .parse::<AliasedRequest>()
            .expect("enum decodes despite local Result alias");

        assert_eq!(
            record,
            AliasedRecord {
                name: "schema".to_owned(),
                count: 7,
            }
        );
        assert_eq!(request, AliasedRequest::Record(record));
        assert_eq!(AliasedRequest::Ping.to_nota(), "Ping");
    }
}
