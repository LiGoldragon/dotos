//! Witnesses for the second-generation DOTOS grammar (wave one).
//!
//! Each test proves one load-bearing rule of the revised grammar: the
//! raw-layer dot-application binding, its right-associative chaining, the
//! glued-period constraint, the reconstruction of flat literals a structural
//! period split apart, and the delimiter reshuffle (`{}` structs, `[]`
//! vectors, `()` payloads, curly-quoted strings, and bare-angle quality
//! applications) as it lands in the
//! codec. These are the acceptance witnesses cited by the wave-one report.

use std::collections::BTreeMap;

use dotos::{Document, DotosDecode, DotosEncode, DotosSource};

/// A glued period binds a head to the following payload as one application
/// block; the head and payload are recovered structurally, not by splitting
/// atom text.
#[test]
fn dot_binds_head_to_payload_as_one_application() {
    let document = Document::parse("Variant.Data").expect("valid dotos");
    assert_eq!(document.holds_root_objects(), 1, "one application block");
    let block = document.root_object_at(0).expect("root");
    let (head, payload) = block.as_application().expect("dot-application");
    assert_eq!(head.demote_to_string(), Some("Variant"));
    assert_eq!(payload.demote_to_string(), Some("Data"));
}

/// A delimited payload — parenthesis, square bracket, or brace — binds to its
/// head as one application, matching `Variant.( … )` / `.[ … ]` / `.{ … }`.
#[test]
fn dot_binds_each_delimited_payload_kind() {
    for (source, is_expected) in [
        ("Variant.(a b)", "parenthesis"),
        ("Variant.[a b]", "square bracket"),
        ("Variant.{a b}", "brace"),
    ] {
        let document = Document::parse(source).expect("valid dotos");
        let block = document.root_object_at(0).expect("root");
        let (head, payload) = block.as_application().expect("dot-application");
        assert_eq!(head.demote_to_string(), Some("Variant"), "{source}");
        assert_eq!(
            payload.holds_root_objects(),
            2,
            "{source} payload holds its two objects ({is_expected})"
        );
    }
}

/// A dotted chain binds right-associatively: the head is the leftmost single
/// segment and the payload is the remainder, so `Visibility.name.Type` reads
/// as visibility, then the (name, type) remainder.
#[test]
fn dotted_chain_binds_right_associatively() {
    let document = Document::parse("Private.secretDigest.StateDigest").expect("valid dotos");
    let block = document.root_object_at(0).expect("root");
    let (visibility, remainder) = block.as_application().expect("outer application");
    assert_eq!(visibility.demote_to_string(), Some("Private"));

    let (name, kind) = remainder.as_application().expect("inner application");
    assert_eq!(name.demote_to_string(), Some("secretDigest"));
    assert_eq!(kind.demote_to_string(), Some("StateDigest"));
}

/// The period binds only when glued to both sides. A space before it leaves a
/// leading period with no head; a space or end of input after it leaves the
/// application dangling; a leading period at object position has no head.
#[test]
fn a_period_binds_only_when_glued_on_both_sides() {
    assert!(
        Document::parse("Head .Payload").is_err(),
        "a space before the period is not a binding"
    );
    assert!(
        Document::parse("Head. Payload").is_err(),
        "a space after the period leaves the application dangling"
    );
    assert!(
        Document::parse("Head.").is_err(),
        "a trailing period has no payload"
    );
    assert!(
        Document::parse(".Payload").is_err(),
        "a leading period has no head"
    );
}

/// A period is a structural operator, so an atom never contains one: a flat
/// literal that carried a period — a dotted Rust path — is reconstructed from
/// the application's segments rather than read as a single atom.
#[test]
fn a_dotted_path_reconstructs_its_flat_text() {
    let document = Document::parse("rustfmt.skip").expect("valid dotos");
    let block = document.root_object_at(0).expect("root");
    assert!(block.is_application(), "rustfmt.skip is an application");
    assert_eq!(
        block.demote_to_string(),
        None,
        "an application is not a string"
    );
    assert_eq!(block.dotted_text(), Some("rustfmt.skip".to_owned()));
}

/// A float literal's fractional period is a structural dot, so the float
/// parses as an application and the Float codec reconstructs the literal from
/// the dotted text. The scalar still round-trips to its canonical form.
#[test]
fn float_literal_round_trips_through_its_structural_period() {
    let value = DotosSource::new("-122.3")
        .parse::<f64>()
        .expect("float decodes");
    assert_eq!(value, -122.3);
    assert_eq!((-122.3_f64).to_dotos(), "-122.3");
    assert_eq!(
        DotosSource::new(&(-122.3_f64).to_dotos())
            .parse::<f64>()
            .expect("re-decodes"),
        -122.3
    );
}

/// Under the reshuffle a canonical string is a bare atom — a period-joined chain
/// of bare atoms included, since an expected `String` reclaims the dotted text —
/// and every non-bare value uses the literal-preserving curly-text form.
#[test]
fn strings_reshuffle_to_bare_and_curly_forms() {
    assert_eq!("schema".to_owned().to_dotos(), "schema");
    assert_eq!("alpha beta".to_owned().to_dotos(), "“alpha beta”");
    assert_eq!("a.b".to_owned().to_dotos(), "a.b");
    assert_eq!("has (paren)".to_owned().to_dotos(), "“has (paren)”");
    assert_eq!("alpha;;beta".to_owned().to_dotos(), "“alpha;;beta”");

    for original in ["schema", "alpha beta", "a.b", "has (paren)", "alpha;;beta"] {
        let encoded = original.to_owned().to_dotos();
        assert_eq!(
            DotosSource::new(&encoded)
                .parse::<String>()
                .unwrap_or_else(|error| panic!("{original:?} → {encoded:?} decodes: {error}")),
            original,
            "round trip for {original:?}"
        );
    }
}

/// Parentheses and pipes are not string syntax: `(schema)` must be written
/// bare, while a multi-word value uses curly quotes.
#[test]
fn parenthesized_and_pipe_strings_are_rejected() {
    let error = DotosSource::new("(schema)")
        .parse::<String>()
        .expect_err("redundant parentheses reject");
    assert!(
        error.to_string().contains("curly quote"),
        "error was {error}"
    );
    assert!(
        DotosSource::new("(|schema|)").parse::<String>().is_err(),
        "pipe text is retired"
    );
}

/// A vector keeps the square bracket; an option is a dotted variant — `None`
/// bare, `Some.payload` applied — and both round-trip.
#[test]
fn vectors_keep_brackets_and_options_are_dotted_variants() {
    let vector = DotosSource::new("[alpha beta gamma]")
        .parse::<Vec<String>>()
        .expect("vector decodes");
    assert_eq!(vector, vec!["alpha", "beta", "gamma"]);
    assert_eq!(vector.to_dotos(), "[alpha beta gamma]");

    let some = DotosSource::new("Some.“cache entry”")
        .parse::<Option<String>>()
        .expect("some decodes");
    assert_eq!(some, Some("cache entry".to_owned()));
    assert_eq!(some.to_dotos(), "Some.“cache entry”");

    let some_bare = DotosSource::new("Some.42")
        .parse::<Option<u64>>()
        .expect("some integer decodes");
    assert_eq!(some_bare, Some(42));
    assert_eq!(some_bare.to_dotos(), "Some.42");

    let none = DotosSource::new("None")
        .parse::<Option<String>>()
        .expect("none decodes");
    assert_eq!(none, None);
    assert_eq!(none.to_dotos(), "None");
}

/// A map has no delimiter of its own: it is a `Map`-headed application over a
/// parenthesis payload of `key.Value` dotted entries, and it round-trips.
#[test]
fn maps_are_map_headed_applications_over_dotted_entries() {
    let map = DotosSource::new("Map.(alpha.1 beta.2)")
        .parse::<BTreeMap<String, u64>>()
        .expect("map decodes");
    assert_eq!(map.get("alpha"), Some(&1));
    assert_eq!(map.get("beta"), Some(&2));
    assert_eq!(map.to_dotos(), "Map.(alpha.1 beta.2)");
}

/// Curly strings preserve multiline text and nest without turning inner quotes
/// into syntax errors.
#[test]
fn curly_multiline_string_round_trips_with_nesting() {
    let original = "line one\nline two with “nested” marker".to_owned();
    let encoded = original.to_dotos();
    assert!(
        encoded.starts_with('“') && encoded.ends_with('”'),
        "{encoded}"
    );
    assert_eq!(
        DotosSource::new(&encoded)
            .parse::<String>()
            .expect("curly string decodes"),
        original
    );
}

#[test]
fn bare_angles_form_nested_quality_applications() {
    let document = Document::parse("Result<Vector<Ordered> Error>").expect("quality syntax");
    let root = document.root_object_at(0).expect("single root");
    assert_eq!(root.application_form(), Some(dotos::ApplicationForm::Angle));
    let (head, payload) = root.as_application().expect("outer angle application");
    assert_eq!(head.demote_to_string(), Some("Result"));
    let arguments = payload
        .as_delimited(dotos::Delimiter::Angle)
        .expect("angle payload");
    assert_eq!(arguments.len(), 2);
    assert_eq!(
        arguments[0].application_form(),
        Some(dotos::ApplicationForm::Angle)
    );
    assert_eq!(arguments[1].demote_to_string(), Some("Error"));
    assert!(
        Document::parse("Vector.<Ordered>").is_err(),
        "angle is never dot-prefixed"
    );
}

#[test]
fn transformer_applications_keep_dot_and_parenthesis_as_distinct_shapes() {
    let standalone = Document::parse("Name.Transformer.(Input Output)")
        .expect("standalone transformer application");
    let (name, transformer) = standalone
        .root_object_at(0)
        .expect("standalone root")
        .as_application()
        .expect("name application");
    assert_eq!(name.demote_to_string(), Some("Name"));
    let (transformer_name, arguments) = transformer
        .as_application()
        .expect("transformer application");
    assert_eq!(transformer_name.demote_to_string(), Some("Transformer"));
    assert!(arguments.is_parenthesis());
    assert_eq!(arguments.holds_root_objects(), 2);

    let sectioned =
        Document::parse("Name.(Input Output)").expect("sectioned transformer application");
    let (section_name, section_arguments) = sectioned
        .root_object_at(0)
        .expect("sectioned root")
        .as_application()
        .expect("section application");
    assert_eq!(section_name.demote_to_string(), Some("Name"));
    assert!(section_arguments.is_parenthesis());
    assert_eq!(section_arguments.holds_root_objects(), 2);
}

/// The psyche-authored base sample parses under the new grammar with its
/// intended nested application shape: `Public.Newtype.( … )` is visibility,
/// then kind, then the parenthesised body.
#[test]
fn psyche_authored_newtype_sample_parses_with_intended_shape() {
    let source = r#"Public.Newtype.(
  CommitSequence
  [ Literal.[rustfmt.skip]
    Derive.[rkyv.[Archive Serialize Deserialize] Clone Debug PartialEq Eq] ]
  Integer
)"#;
    let document = Document::parse(source).expect("psyche sample parses");
    let block = document.root_object_at(0).expect("root");

    let (visibility, remainder) = block.as_application().expect("outer application");
    assert_eq!(visibility.demote_to_string(), Some("Public"));
    let (kind, body) = remainder.as_application().expect("kind application");
    assert_eq!(kind.demote_to_string(), Some("Newtype"));
    assert!(
        body.is_parenthesis(),
        "the body is the parenthesised payload"
    );
    assert_eq!(
        body.holds_root_objects(),
        3,
        "name, attributes vector, type"
    );
}

#[derive(Debug, PartialEq, Eq, dotos::DotosEncode, dotos::DotosDecode)]
enum Signal {
    Idle,
    Tick(u64),
    Range(u64, u64),
}

#[derive(Debug, PartialEq, Eq, dotos::DotosEncode, dotos::DotosDecode)]
struct Marker {
    label: String,
    count: u64,
}

/// A derived enum encodes unit variants as bare atoms, single-field variants
/// as `Variant.payload`, and multi-field variants as `Variant.{f0 f1}`, and
/// each round-trips (item b — dotted data variants replace `(Variant Data)`).
#[test]
fn derived_enum_uses_dotted_variants() {
    for (value, text) in [
        (Signal::Idle, "Idle"),
        (Signal::Tick(7), "Tick.7"),
        (Signal::Range(3, 9), "Range.{3 9}"),
    ] {
        assert_eq!(value.to_dotos(), text, "encode {value:?}");
        assert_eq!(
            Signal::from_dotos_block(
                Document::parse(text)
                    .expect("parses")
                    .root_object_at(0)
                    .expect("root")
            )
            .expect("decodes"),
            value,
            "round trip {text}"
        );
    }
}

/// A derived named struct encodes its body as a brace record under the
/// reshuffle (`{}` = structs) and round-trips.
#[test]
fn derived_struct_body_is_a_brace_record() {
    let marker = Marker {
        label: "commit sequence".to_owned(),
        count: 4,
    };
    assert_eq!(marker.to_dotos(), "{“commit sequence” 4}");
    let decoded = Marker::from_dotos_block(
        Document::parse("{“commit sequence” 4}")
            .expect("parses")
            .root_object_at(0)
            .expect("root"),
    )
    .expect("decodes");
    assert_eq!(decoded, marker);
}
