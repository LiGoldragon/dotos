//! Per-instance schema derive tests.
//!
//! These exercise `NotaDecodeTraced` on local types that model the shapes the
//! spirit contract uses — a newtype over an enum (`Certainty`/`Magnitude`), a
//! struct of newtypes (`Entry`), a recursive enum taxonomy
//! (`Domain`/`Technology`/`Software`), and a root enum whose chosen variant
//! wraps a transparent struct (`Input::Record`). Every assertion checks both
//! the decoded value and the `expected` reference the decoder captured at each
//! position — proving the trace rides the same type-directed traversal that
//! validates the value.

use nota_next::{
    InstanceSchema, InstanceSchemaBody, NotaDecode, NotaDecodeTraced, NotaSource, TypeReference,
};

#[derive(Debug, PartialEq, Eq, NotaDecode, NotaDecodeTraced)]
enum Magnitude {
    Zero,
    Low,
    High,
}

#[derive(Debug, PartialEq, Eq, NotaDecode, NotaDecodeTraced)]
struct Certainty(Magnitude);

#[derive(Debug, PartialEq, Eq, NotaDecode, NotaDecodeTraced)]
struct Importance(Magnitude);

#[derive(Debug, PartialEq, Eq, NotaDecode, NotaDecodeTraced)]
struct Privacy(Magnitude);

#[derive(Debug, PartialEq, Eq, NotaDecode, NotaDecodeTraced)]
enum Kind {
    Decision,
    Principle,
    Constraint,
}

#[derive(Debug, PartialEq, Eq, NotaDecode, NotaDecodeTraced)]
struct Description(String);

#[derive(Debug, PartialEq, Eq, NotaDecode, NotaDecodeTraced)]
struct Referent(String);

#[derive(Debug, PartialEq, Eq, NotaDecode, NotaDecodeTraced)]
struct Referents(Vec<Referent>);

#[derive(Debug, PartialEq, Eq, NotaDecode, NotaDecodeTraced)]
enum Programming {
    CodeGeneration,
    Parsing,
}

#[derive(Debug, PartialEq, Eq, NotaDecode, NotaDecodeTraced)]
enum Software {
    Programming(Programming),
    Theory,
}

#[derive(Debug, PartialEq, Eq, NotaDecode, NotaDecodeTraced)]
enum Technology {
    Software(Software),
}

#[derive(Debug, PartialEq, Eq, NotaDecode, NotaDecodeTraced)]
enum Domain {
    Technology(Technology),
}

#[derive(Debug, PartialEq, Eq, NotaDecode, NotaDecodeTraced)]
struct Domains(Vec<Domain>);

#[derive(Debug, PartialEq, Eq, NotaDecode, NotaDecodeTraced)]
struct Entry {
    domains: Domains,
    kind: Kind,
    description: Description,
    certainty: Certainty,
    importance: Importance,
    privacy: Privacy,
    referents: Referents,
}

#[derive(Debug, PartialEq, Eq, NotaDecode, NotaDecodeTraced)]
struct RecordRequest {
    entry: Entry,
}

#[derive(Debug, PartialEq, Eq, NotaDecode, NotaDecodeTraced)]
struct Record(RecordRequest);

#[derive(Debug, PartialEq, Eq, NotaDecode, NotaDecodeTraced)]
enum Input {
    Record(Record),
    Version,
}

fn trace<Value>(source: &str) -> (Value, InstanceSchema)
where
    Value: NotaDecodeTraced,
{
    let block = NotaSource::new(source)
        .parse_root()
        .expect("parse a single root object");
    Value::from_nota_block_traced(&block)
        .expect("decode value and capture its instance schema")
        .into_parts()
}

fn named(reference: &TypeReference) -> &str {
    match reference {
        TypeReference::Named(name) => name,
        other => panic!("expected a named reference, found {other:?}"),
    }
}

#[test]
fn enum_position_records_the_enum_name_not_the_variant() {
    // `Decision` is a Kind value; the per-instance schema names `Kind`.
    let (value, schema) = trace::<Kind>("Decision");
    assert_eq!(value, Kind::Decision);
    assert_eq!(named(schema.expected()), "Kind");
    assert!(matches!(
        schema.body(),
        InstanceSchemaBody::EnumPayload(None)
    ));
}

#[test]
fn newtype_preserves_the_wrapper_then_the_inner_enum() {
    // Certainty(High): parent node is Certainty; one level in is Magnitude.
    let (value, schema) = trace::<Certainty>("High");
    assert_eq!(value, Certainty(Magnitude::High));
    assert_eq!(named(schema.expected()), "Certainty");
    let InstanceSchemaBody::Newtype(inner) = schema.body() else {
        panic!("certainty newtype must carry a Newtype body");
    };
    assert_eq!(named(inner.expected()), "Magnitude");
}

#[test]
fn empty_vector_still_names_its_element_type() {
    // `[]` at a Domains position still yields Domains, (Vec Domain) inside.
    let (value, schema) = trace::<Domains>("[]");
    assert_eq!(value, Domains(vec![]));
    assert_eq!(named(schema.expected()), "Domains");
    let InstanceSchemaBody::Newtype(inner) = schema.body() else {
        panic!("Domains newtype must carry a Newtype body");
    };
    match inner.expected() {
        TypeReference::Vector(element) => assert_eq!(named(element), "Domain"),
        other => panic!("expected (Vec Domain), found {other:?}"),
    }
    assert!(matches!(inner.body(), InstanceSchemaBody::Vector(nodes) if nodes.is_empty()));
}

#[test]
fn domain_path_traces_expected_types_down_the_taxonomy() {
    // value: (Technology (Software (Programming CodeGeneration)))
    // expected trace: Domain -> Technology -> Software -> Programming
    let (value, schema) = trace::<Domain>("(Technology (Software (Programming CodeGeneration)))");
    assert_eq!(
        value,
        Domain::Technology(Technology::Software(Software::Programming(
            Programming::CodeGeneration
        )))
    );

    assert_eq!(named(schema.expected()), "Domain");
    let technology = enum_payload(&schema);
    assert_eq!(named(technology.expected()), "Technology");
    let software = enum_payload(technology);
    assert_eq!(named(software.expected()), "Software");
    let programming = enum_payload(software);
    assert_eq!(named(programming.expected()), "Programming");
    // The chosen unit variant terminates with no further payload.
    assert!(matches!(
        programming.body(),
        InstanceSchemaBody::EnumPayload(None)
    ));
}

fn enum_payload(schema: &InstanceSchema) -> &InstanceSchema {
    match schema.body() {
        InstanceSchemaBody::EnumPayload(Some(inner)) => inner,
        other => panic!("expected an enum payload, found {other:?}"),
    }
}

#[test]
fn entry_struct_records_each_field_type_in_declared_order() {
    let source = "([(Technology (Software (Programming CodeGeneration)))] Decision [a description] High Low Zero [spirit])";
    let (value, schema) = trace::<Entry>(source);
    assert_eq!(value.kind, Kind::Decision);
    assert_eq!(value.certainty, Certainty(Magnitude::High));
    assert_eq!(value.privacy, Privacy(Magnitude::Zero));

    assert_eq!(named(schema.expected()), "Entry");
    let InstanceSchemaBody::Struct(fields) = schema.body() else {
        panic!("Entry must carry a Struct body");
    };
    let field_names: Vec<&str> = fields.iter().map(|node| named(node.expected())).collect();
    assert_eq!(
        field_names,
        vec![
            "Domains",
            "Kind",
            "Description",
            "Certainty",
            "Importance",
            "Privacy",
            "Referents",
        ]
    );

    // The Kind field is an enum-name position; the chosen variant is in the value.
    let kind_field = &fields[1];
    assert_eq!(named(kind_field.expected()), "Kind");
    // The Certainty field preserves the newtype, with Magnitude one level in.
    let certainty_field = &fields[3];
    let InstanceSchemaBody::Newtype(magnitude) = certainty_field.body() else {
        panic!("certainty field must carry a Newtype body");
    };
    assert_eq!(named(magnitude.expected()), "Magnitude");
}

#[test]
fn root_input_records_enum_name_and_descends_through_transparent_wrappers() {
    // value: (Record ( (entry-fields) )) — Input::Record(Record(RecordRequest { entry })).
    let source = "(Record (([(Technology (Software (Programming CodeGeneration)))] Decision [a description] High Low Zero [spirit])))";
    let (value, schema) = trace::<Input>(source);
    assert!(matches!(value, Input::Record(_)));

    // Root schema node is the enum name Input — not the variant Record.
    assert_eq!(named(schema.expected()), "Input");
    // The Record variant carries a payload: the transparent Record newtype.
    let record = enum_payload(&schema);
    assert_eq!(named(record.expected()), "Record");
    let InstanceSchemaBody::Newtype(record_request) = record.body() else {
        panic!("Record newtype must carry a Newtype body");
    };
    // Inside is the RecordRequest struct (a provenance-only wrapper at render time).
    assert_eq!(named(record_request.expected()), "RecordRequest");
    let InstanceSchemaBody::Struct(request_fields) = record_request.body() else {
        panic!("RecordRequest must carry a Struct body");
    };
    assert_eq!(named(request_fields[0].expected()), "Entry");
}

#[test]
fn unit_root_variant_is_a_scalar_terminal() {
    let (value, schema) = trace::<Input>("Version");
    assert_eq!(value, Input::Version);
    assert_eq!(named(schema.expected()), "Input");
    assert!(matches!(
        schema.body(),
        InstanceSchemaBody::EnumPayload(None)
    ));
}

#[test]
fn traced_value_matches_ordinary_decode() {
    // The traced decoder must agree with the ordinary NotaDecode on the value.
    let source = "(Technology (Software Theory))";
    let traced = {
        let block = NotaSource::new(source).parse_root().expect("parse root");
        Domain::from_nota_block_traced(&block)
            .expect("traced decode")
            .into_value()
    };
    let ordinary = NotaSource::new(source)
        .parse::<Domain>()
        .expect("ordinary decode");
    assert_eq!(traced, ordinary);
}
