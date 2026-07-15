use std::collections::BTreeMap;

use nota::{
    Block, Delimiter, NotaBlock, NotaBody, NotaBodyDecode, NotaBodyEncode, NotaDecode,
    NotaDecodeError, NotaDocumentBody, NotaDocumentDecode, NotaDocumentEncode,
    NotaDocumentEncoding, NotaEncode, NotaSource,
};

#[test]
fn codec_decodes_and_encodes_scalars() {
    assert_eq!(
        NotaSource::new("(schema owns strings)")
            .parse::<String>()
            .expect("string decodes"),
        "schema owns strings"
    );
    assert_eq!(
        NotaSource::new("42")
            .parse::<u64>()
            .expect("integer decodes"),
        42
    );
    assert_eq!(
        NotaSource::new("65535")
            .parse::<u16>()
            .expect("u16 decodes"),
        65_535
    );
    assert_eq!(
        NotaSource::new("255").parse::<u8>().expect("u8 decodes"),
        255
    );
    assert_eq!(
        NotaSource::new("4294967295")
            .parse::<u32>()
            .expect("u32 decodes"),
        4_294_967_295
    );
    assert_eq!(
        NotaSource::new("-2147483648")
            .parse::<i32>()
            .expect("i32 decodes"),
        -2_147_483_648
    );
    assert_eq!(
        NotaSource::new("-128")
            .parse::<i64>()
            .expect("signed integer decodes"),
        -128
    );
    assert_eq!(
        NotaSource::new("-122.3")
            .parse::<f64>()
            .expect("float decodes"),
        -122.3
    );
    assert!(
        NotaSource::new("True")
            .parse::<bool>()
            .expect("boolean decodes")
    );

    assert_eq!(
        "schema owns strings".to_owned().to_nota(),
        "(schema owns strings)"
    );
    assert_eq!(
        "schema@next;required*;a&b^2%>x<y:path/to"
            .to_owned()
            .to_nota(),
        "schema@next;required*;a&b^2%>x<y:path/to"
    );
    assert_eq!(
        NotaSource::new("schema@next;required*;a&b^2%>x<y:path/to")
            .parse::<String>()
            .expect("broad bare string decodes"),
        "schema@next;required*;a&b^2%>x<y:path/to"
    );
    assert_eq!("100%".to_owned().to_nota(), "100%");
    assert_eq!("alpha; beta".to_owned().to_nota(), "(alpha; beta)");
    assert_eq!("alpha;;beta".to_owned().to_nota(), "(|alpha;;beta|)");
    let bracket_safe = "text containing [brackets] and a closing pipe marker |)".to_owned();
    let encoded = bracket_safe.to_nota();
    assert_eq!(
        encoded,
        "(|text containing [brackets] and a closing pipe marker \\|)|)"
    );
    assert_eq!(
        NotaSource::new(&encoded)
            .parse::<String>()
            .expect("bracket-safe string decodes"),
        bracket_safe
    );
    let slash_safe = String::from("text containing [brackets] and a backslash \\");
    let encoded = slash_safe.to_nota();
    assert_eq!(
        encoded,
        "(|text containing [brackets] and a backslash \\\\|)"
    );
    assert_eq!(
        NotaSource::new(&encoded)
            .parse::<String>()
            .expect("escaped backslash decodes"),
        slash_safe
    );
    assert_eq!(42_u64.to_nota(), "42");
    assert_eq!(255_u8.to_nota(), "255");
    assert_eq!(65_535_u16.to_nota(), "65535");
    assert_eq!(4_294_967_295_u32.to_nota(), "4294967295");
    assert_eq!((-2_147_483_648_i32).to_nota(), "-2147483648");
    assert_eq!((-128_i64).to_nota(), "-128");
    assert_eq!((-122.3_f64).to_nota(), "-122.3");
    assert_eq!(false.to_nota(), "False");
}

/// A period-bearing string is reclaimed by the expected `String` boundary just
/// as a float is reclaimed by an expected `Float`: a dotted raw application
/// rejoins into the bare string content, case-blind and through any depth of
/// dots. Bare is the canonical form for such content, so encode emits it bare
/// and a redundant pipe wrapper is rejected. Spaces still take `( … )` and
/// genuinely structural content still takes `(| … |)`.
#[test]
fn codec_rejoins_dotted_strings_under_expected_string_type() {
    // Decode: a dotted bare application rejoins into flat string content.
    for (source, expected) in [
        ("file.txt", "file.txt"),
        ("Foo.bar", "Foo.bar"),
        (
            "nix.prometheus.goldragon.criome",
            "nix.prometheus.goldragon.criome",
        ),
    ] {
        assert_eq!(
            NotaSource::new(source)
                .parse::<String>()
                .unwrap_or_else(|error| panic!("{source:?} decodes: {error}")),
            expected,
            "dotted string decode for {source:?}"
        );
    }

    // Encode: period-joined bare-atom content emits bare, with no pipe escape.
    assert_eq!("file.txt".to_owned().to_nota(), "file.txt");
    assert_eq!("Foo.bar".to_owned().to_nota(), "Foo.bar");
    assert_eq!(
        "nix.prometheus.goldragon.criome".to_owned().to_nota(),
        "nix.prometheus.goldragon.criome"
    );

    // Round trip (decode ∘ encode) for every canonical class: bare-dotted,
    // space-separated parenthesis, and structural pipe text.
    for original in [
        "file.txt",
        "Foo.bar",
        "nix.prometheus.goldragon.criome",
        "words with spaces",
        "version 1.2",
        "line one\nline two",
        "has (paren) and .dot",
    ] {
        let encoded = original.to_owned().to_nota();
        assert_eq!(
            NotaSource::new(&encoded)
                .parse::<String>()
                .unwrap_or_else(|error| panic!("{original:?} → {encoded:?} decodes: {error}")),
            original,
            "round trip for {original:?}"
        );
    }

    // A string with spaces still takes the parenthesis form.
    assert_eq!(
        "words with spaces".to_owned().to_nota(),
        "(words with spaces)"
    );
    // A multi-line string still takes the literal-preserving pipe form.
    let multiline = "line one\nline two".to_owned().to_nota();
    assert!(
        multiline.starts_with("(|") && multiline.ends_with("|)"),
        "multiline string takes pipe form, was {multiline}"
    );

    // Round trip (encode ∘ decode) on canonical text: canonical source decodes
    // and re-encodes to the identical bytes.
    for canonical in [
        "file.txt",
        "Foo.bar",
        "nix.prometheus.goldragon.criome",
        "(words with spaces)",
        "(version 1.2)",
    ] {
        let value = NotaSource::new(canonical)
            .parse::<String>()
            .unwrap_or_else(|error| panic!("{canonical:?} decodes: {error}"));
        assert_eq!(
            value.to_nota(),
            canonical,
            "canonical text is an encode fixpoint for {canonical:?}"
        );
    }

    // A redundant pipe wrapper around bare-dotted content is non-canonical.
    let error = NotaSource::new("(|file.txt|)")
        .parse::<String>()
        .expect_err("pipe wrapper around dotted-bare content rejects");
    assert!(
        error.to_string().contains("use file.txt"),
        "error was {error}"
    );
}

#[test]
fn codec_rejects_brackets_around_bare_eligible_strings() {
    let error = NotaSource::new("(schema)")
        .parse::<String>()
        .expect_err("redundant inline parentheses reject");

    assert!(
        error.to_string().contains("use schema"),
        "error was {error}"
    );

    let error = NotaSource::new("(|schema|)")
        .parse::<String>()
        .expect_err("redundant pipe parentheses reject");

    assert!(
        error.to_string().contains("use schema"),
        "error was {error}"
    );
}

#[test]
fn codec_rejects_out_of_range_integer_widths() {
    let error = NotaSource::new("65536")
        .parse::<u16>()
        .expect_err("u16 range rejects");

    assert!(
        error.to_string().contains("invalid integer 65536"),
        "error was {error}"
    );

    let error = NotaSource::new("256")
        .parse::<u8>()
        .expect_err("u8 range rejects");

    assert!(
        error.to_string().contains("invalid integer 256"),
        "error was {error}"
    );

    let error = NotaSource::new("2147483648")
        .parse::<i32>()
        .expect_err("i32 range rejects");

    assert!(
        error.to_string().contains("invalid integer 2147483648"),
        "error was {error}"
    );
}

#[test]
fn codec_rejects_invalid_float_text() {
    let error = NotaSource::new("not-a-float")
        .parse::<f64>()
        .expect_err("float grammar rejects");

    assert!(
        error.to_string().contains("invalid Float"),
        "error was {error}"
    );
}

#[test]
fn codec_renders_domain_value_validation_errors() {
    let error = NotaDecodeError::InvalidValue {
        type_name: "Keygrip",
        value: "abc".to_owned(),
        reason: "expected 40 hex chars".to_owned(),
    };

    assert_eq!(
        error.to_string(),
        "invalid Keygrip \"abc\": expected 40 hex chars"
    );
}

#[test]
fn codec_decodes_and_encodes_collection_values() {
    let vector = NotaSource::new("[alpha beta gamma]")
        .parse::<Vec<String>>()
        .expect("vector decodes");
    assert_eq!(vector, vec!["alpha", "beta", "gamma"]);
    assert_eq!(vector.to_nota(), "[alpha beta gamma]");

    let option = NotaSource::new("Some.(cache entry)")
        .parse::<Option<String>>()
        .expect("option decodes");
    assert_eq!(option, Some("cache entry".to_owned()));
    assert_eq!(option.to_nota(), "Some.(cache entry)");

    let none = NotaSource::new("None")
        .parse::<Option<String>>()
        .expect("none decodes");
    assert_eq!(none, None);
    assert_eq!(none.to_nota(), "None");
}

#[test]
fn codec_decodes_and_encodes_byte_sequences_as_hex_text() {
    let bytes = NotaSource::new("deadbeef")
        .parse::<nota::ByteSequence>()
        .expect("byte sequence decodes");
    assert_eq!(bytes.payload(), &[0xde, 0xad, 0xbe, 0xef]);
    assert_eq!(bytes.to_nota(), "deadbeef");

    let fixed = NotaSource::new("01020304")
        .parse::<nota::FixedByteSequence<4>>()
        .expect("fixed byte sequence decodes");
    assert_eq!(fixed.payload(), &[0x01, 0x02, 0x03, 0x04]);
    assert_eq!(fixed.to_nota(), "01020304");
}

#[test]
fn codec_rejects_noncanonical_byte_sequence_hex() {
    let odd = NotaSource::new("abc")
        .parse::<nota::ByteSequence>()
        .expect_err("odd hex length rejects");
    assert!(odd.to_string().contains("odd length"));

    let wrong_width = NotaSource::new("deadbeef")
        .parse::<nota::FixedByteSequence<2>>()
        .expect_err("wrong fixed width rejects");
    assert!(wrong_width.to_string().contains("expected 4 hex digits"));
}

#[test]
fn codec_decodes_and_encodes_ordered_map_values() {
    let map = NotaSource::new("Map.(alpha.1 beta.2)")
        .parse::<BTreeMap<String, u64>>()
        .expect("map decodes");

    assert_eq!(map.get("alpha"), Some(&1));
    assert_eq!(map.get("beta"), Some(&2));
    assert_eq!(map.to_nota(), "Map.(alpha.1 beta.2)");
}

#[test]
fn codec_decodes_and_encodes_boxed_values_without_shape_noise() {
    let boxed = NotaSource::new("(recursive reference)")
        .parse::<Box<String>>()
        .expect("boxed value decodes");

    assert_eq!(*boxed, "recursive reference");
    assert_eq!(boxed.to_nota(), "(recursive reference)");
}

#[test]
fn codec_rejects_multi_root_source_for_typed_parse() {
    let error = NotaSource::new("alpha beta")
        .parse::<String>()
        .expect_err("multi-root source rejects");

    assert!(
        error
            .to_string()
            .contains("expected exactly one NOTA root object")
    );
}

#[derive(Debug, Eq, PartialEq)]
struct KnownRootExample {
    name: String,
    imports: Vec<String>,
    output_variants: Vec<String>,
}

impl NotaBodyDecode for KnownRootExample {
    fn from_nota_body(body: &NotaBody<'_>) -> Result<Self, NotaDecodeError> {
        let fields = body.expect_fields("KnownRootExample", 3)?;
        Ok(Self {
            name: String::from_nota_block(&fields[0])?,
            imports: Vec::<String>::from_nota_block(&fields[1])?,
            output_variants: Vec::<String>::from_nota_block(&fields[2])?,
        })
    }
}

impl NotaBodyEncode for KnownRootExample {
    fn to_nota_body(&self) -> nota::NotaBodyEncoding {
        NotaDocumentEncoding::new(vec![
            self.name.to_nota(),
            self.imports.to_nota(),
            self.output_variants.to_nota(),
        ])
    }
}

impl NotaDocumentDecode for KnownRootExample {
    fn from_nota_document_body(body: &NotaDocumentBody<'_>) -> Result<Self, NotaDecodeError> {
        Self::from_nota_body(body.as_body())
    }
}

impl NotaDocumentEncode for KnownRootExample {
    fn to_nota_document_body(&self) -> NotaDocumentEncoding {
        self.to_nota_body()
    }
}

impl KnownRootExample {
    fn from_nota_source(source: &str) -> Result<Self, NotaDecodeError> {
        NotaSource::new(source).parse_document_body()
    }

    fn to_nota(&self) -> String {
        self.to_nota_document_body().to_nota()
    }
}

#[test]
fn codec_decodes_known_root_and_parenthesized_object_from_the_same_body_shape() {
    let document_body = NotaSource::new("schema:core\n[alpha beta]\n[Recorded Rejected]")
        .parse_document_body::<KnownRootExample>()
        .expect("document body decodes");
    let block = NotaSource::new("(schema:core [alpha beta] [Recorded Rejected])")
        .parse_root()
        .expect("parenthesized object parses");
    let object_body = NotaBlock::new(&block)
        .expect_body(Delimiter::Parenthesis, "KnownRootExample")
        .expect("object body opens");
    let object_value = KnownRootExample::from_nota_body(&object_body).expect("object body decodes");

    assert_eq!(document_body, object_value);
}

#[test]
fn codec_decodes_and_encodes_known_root_document_body() {
    let source = r#"schema:core
[alpha beta]
[Recorded Rejected]"#;
    let value = KnownRootExample::from_nota_source(source).expect("known root body decodes");

    assert_eq!(
        value,
        KnownRootExample {
            name: "schema:core".to_owned(),
            imports: vec!["alpha".to_owned(), "beta".to_owned()],
            output_variants: vec!["Recorded".to_owned(), "Rejected".to_owned()],
        }
    );
    assert_eq!(
        value.to_nota(),
        "schema:core\n[alpha beta]\n[Recorded Rejected]"
    );
}

#[test]
fn codec_known_root_body_preserves_raw_root_structure_for_callers() {
    let value =
        KnownRootExample::from_nota_source("core\n[]\n[]").expect("empty body vectors decode");
    let encoding = value.to_nota_document_body();
    let reparsed =
        nota::Document::parse(encoding.to_nota()).expect("known-root body emits legal NOTA");

    assert_eq!(encoding.fields().len(), 3);
    assert_eq!(reparsed.root_objects().len(), 3);
    assert!(matches!(
        reparsed.root_objects().get(1),
        Some(Block::Delimited {
            delimiter: Delimiter::SquareBracket,
            ..
        })
    ));
}
