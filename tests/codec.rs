use std::collections::BTreeMap;

use nota_next::{
    Block, Delimiter, NotaBlock, NotaBody, NotaBodyDecode, NotaBodyEncode, NotaDecode,
    NotaDecodeError, NotaDocumentBody, NotaDocumentDecode, NotaDocumentEncode,
    NotaDocumentEncoding, NotaEncode, NotaSource,
};

#[test]
fn codec_decodes_and_encodes_scalars() {
    assert_eq!(
        NotaSource::new("[schema owns strings]")
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
        NotaSource::new("4294967295")
            .parse::<u32>()
            .expect("u32 decodes"),
        4_294_967_295
    );
    assert_eq!(
        NotaSource::new("-128")
            .parse::<i64>()
            .expect("signed integer decodes"),
        -128
    );
    assert!(
        NotaSource::new("True")
            .parse::<bool>()
            .expect("boolean decodes")
    );

    assert_eq!(
        "schema owns strings".to_owned().to_nota(),
        "[schema owns strings]"
    );
    assert_eq!(42_u64.to_nota(), "42");
    assert_eq!(65_535_u16.to_nota(), "65535");
    assert_eq!(4_294_967_295_u32.to_nota(), "4294967295");
    assert_eq!((-128_i64).to_nota(), "-128");
    assert_eq!(false.to_nota(), "False");
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
    assert_eq!(vector.to_nota(), "[[alpha] [beta] [gamma]]");

    let option = NotaSource::new("(Some [cache entry])")
        .parse::<Option<String>>()
        .expect("option decodes");
    assert_eq!(option, Some("cache entry".to_owned()));
    assert_eq!(option.to_nota(), "(Some [cache entry])");

    let none = NotaSource::new("None")
        .parse::<Option<String>>()
        .expect("none decodes");
    assert_eq!(none, None);
    assert_eq!(none.to_nota(), "None");
}

#[test]
fn codec_decodes_and_encodes_ordered_map_values() {
    let map = NotaSource::new("{alpha 1 beta 2}")
        .parse::<BTreeMap<String, u64>>()
        .expect("map decodes");

    assert_eq!(map.get("alpha"), Some(&1));
    assert_eq!(map.get("beta"), Some(&2));
    assert_eq!(map.to_nota(), "{[alpha] 1 [beta] 2}");
}

#[test]
fn codec_decodes_and_encodes_boxed_values_without_shape_noise() {
    let boxed = NotaSource::new("[recursive reference]")
        .parse::<Box<String>>()
        .expect("boxed value decodes");

    assert_eq!(*boxed, "recursive reference");
    assert_eq!(boxed.to_nota(), "[recursive reference]");
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
    fn to_nota_body(&self) -> nota_next::NotaBodyEncoding {
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
    let document_body = NotaSource::new("[schema-next:core]\n[alpha beta]\n[Recorded Rejected]")
        .parse_document_body::<KnownRootExample>()
        .expect("document body decodes");
    let block = NotaSource::new("([schema-next:core] [alpha beta] [Recorded Rejected])")
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
    let source = "[schema-next:core]\n[alpha beta]\n[Recorded Rejected]";
    let value = KnownRootExample::from_nota_source(source).expect("known root body decodes");

    assert_eq!(
        value,
        KnownRootExample {
            name: "schema-next:core".to_owned(),
            imports: vec!["alpha".to_owned(), "beta".to_owned()],
            output_variants: vec!["Recorded".to_owned(), "Rejected".to_owned()],
        }
    );
    assert_eq!(
        value.to_nota(),
        "[schema-next:core]\n[[alpha] [beta]]\n[[Recorded] [Rejected]]"
    );
}

#[test]
fn codec_known_root_body_preserves_raw_root_structure_for_callers() {
    let value =
        KnownRootExample::from_nota_source("[core]\n[]\n[]").expect("empty body vectors decode");
    let encoding = value.to_nota_document_body();
    let reparsed =
        nota_next::Document::parse(encoding.to_nota()).expect("known-root body emits legal NOTA");

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
