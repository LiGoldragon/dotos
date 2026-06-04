use nota_next::{
    AtomShape, CaptureName, DelimitedShape, Document, MacroCandidate, MacroDelimiter, MacroMatch,
    MacroNodeDefinition, MacroObjectCount, MacroRegistry, Pattern, PatternElement,
    PositionPredicate, StructuralMacroNode,
};
use std::fmt;

#[test]
fn dispatches_structural_namespace_pair_with_named_captures() {
    let document = Document::parse(include_str!("fixtures/macro-node/strict-namespace.nota"))
        .expect("fixture parses");
    let namespace = document.root_object_at(0).expect("fixture has root");
    let key = namespace.root_object_at(0).expect("entry key");
    let value = namespace.root_object_at(1).expect("entry value");
    let registry = MacroRegistry::new(vec![MacroNodeDefinition::new(
        "StructDeclaration",
        PositionPredicate::named("NamespaceDeclaration"),
        Pattern::new(vec![
            PatternElement::atom(AtomShape::symbol(Some(CaptureName::new("type_name")))),
            PatternElement::delimited(DelimitedShape::new(
                MacroDelimiter::Brace,
                MacroObjectCount::Even,
                Some(CaptureName::new("body")),
            )),
        ]),
        "symbol key followed by brace value",
    )])
    .expect("registry has no conflicts");

    let matched = registry
        .dispatch(&MacroCandidate::from_pair(
            PositionPredicate::named("NamespaceDeclaration"),
            key,
            value,
        ))
        .expect("entry matches");

    assert_eq!(matched.macro_name(), "StructDeclaration");
    assert!(
        matched
            .captures()
            .get(&CaptureName::new("type_name"))
            .is_some()
    );
    let body = matched
        .captures()
        .get(&CaptureName::new("body"))
        .and_then(|capture| capture.body())
        .expect("delimited capture exposes body content");
    assert_eq!(body.root_objects().len(), 4);
    assert!(
        matched
            .captures()
            .get(&CaptureName::new("body"))
            .and_then(|capture| capture.block())
            .is_none(),
        "delimited captures do not hand the wrapper delimiter to the next semantic parser"
    );
}

#[test]
fn dispatches_nested_structural_constraints_inside_delimited_values() {
    let document = Document::parse("{Entry {topic Topic}}").expect("fixture parses");
    let namespace = document.root_object_at(0).expect("fixture has root");
    let key = namespace.root_object_at(0).expect("entry key");
    let value = namespace.root_object_at(1).expect("entry value");
    let registry = MacroRegistry::new(vec![MacroNodeDefinition::new(
        "SingleTopicStruct",
        PositionPredicate::named("NamespaceDeclaration"),
        Pattern::new(vec![
            PatternElement::atom(AtomShape::pascal_case(Some(CaptureName::new("type_name")))),
            PatternElement::delimited(
                DelimitedShape::new(
                    MacroDelimiter::Brace,
                    MacroObjectCount::Exact(2),
                    Some(CaptureName::new("body")),
                )
                .with_children(Pattern::new(vec![
                    PatternElement::literal("topic"),
                    PatternElement::atom(AtomShape::pascal_case(Some(CaptureName::new(
                        "field_type",
                    )))),
                ])),
            ),
        ]),
        "Pascal type key followed by a brace whose body is topic plus Pascal type",
    )])
    .expect("registry has no conflicts");

    let matched = registry
        .dispatch(&MacroCandidate::from_pair(
            PositionPredicate::named("NamespaceDeclaration"),
            key,
            value,
        ))
        .expect("entry matches nested child constraints");

    assert_eq!(matched.macro_name(), "SingleTopicStruct");
    assert!(
        matched
            .captures()
            .get(&CaptureName::new("field_type"))
            .is_some(),
        "nested child pattern captures the type inside the brace value"
    );
}

#[test]
fn rejects_delimited_values_when_nested_constraints_do_not_match() {
    let document = Document::parse("{Entry {topic [Topic]}}").expect("fixture parses");
    let namespace = document.root_object_at(0).expect("fixture has root");
    let key = namespace.root_object_at(0).expect("entry key");
    let value = namespace.root_object_at(1).expect("entry value");
    let registry = MacroRegistry::new(vec![MacroNodeDefinition::new(
        "SingleTopicStruct",
        PositionPredicate::named("NamespaceDeclaration"),
        Pattern::new(vec![
            PatternElement::atom(AtomShape::pascal_case(Some(CaptureName::new("type_name")))),
            PatternElement::delimited(
                DelimitedShape::new(MacroDelimiter::Brace, MacroObjectCount::Exact(2), None)
                    .with_children(Pattern::new(vec![
                        PatternElement::literal("topic"),
                        PatternElement::atom(AtomShape::pascal_case(Some(CaptureName::new(
                            "field_type",
                        )))),
                    ])),
            ),
        ]),
        "Pascal type key followed by topic plus Pascal type",
    )])
    .expect("registry has no conflicts");

    let error = registry
        .dispatch(&MacroCandidate::from_pair(
            PositionPredicate::named("NamespaceDeclaration"),
            key,
            value,
        ))
        .expect_err("bracket child is not a Pascal type atom");

    assert!(error.to_string().contains("SingleTopicStruct"));
}

#[test]
fn dispatches_recursively_nested_structural_constraints() {
    let document = Document::parse("{Entry {topic (Vec [Topic])}}").expect("fixture parses");
    let namespace = document.root_object_at(0).expect("fixture has root");
    let key = namespace.root_object_at(0).expect("entry key");
    let value = namespace.root_object_at(1).expect("entry value");
    let registry = MacroRegistry::new(vec![MacroNodeDefinition::new(
        "VectorTopicStruct",
        PositionPredicate::named("NamespaceDeclaration"),
        Pattern::new(vec![
            PatternElement::atom(AtomShape::pascal_case(Some(CaptureName::new("type_name")))),
            PatternElement::delimited(
                DelimitedShape::new(MacroDelimiter::Brace, MacroObjectCount::Exact(2), None)
                    .with_children(Pattern::new(vec![
                        PatternElement::literal("topic"),
                        PatternElement::delimited(
                            DelimitedShape::new(
                                MacroDelimiter::Parenthesis,
                                MacroObjectCount::Exact(2),
                                Some(CaptureName::new("reference")),
                            )
                            .with_children(Pattern::new(vec![
                                PatternElement::literal("Vec"),
                                PatternElement::delimited(
                                    DelimitedShape::new(
                                        MacroDelimiter::SquareBracket,
                                        MacroObjectCount::Exact(1),
                                        None,
                                    )
                                    .with_children(
                                        Pattern::new(vec![PatternElement::atom(
                                            AtomShape::pascal_case(Some(CaptureName::new(
                                                "element_type",
                                            ))),
                                        )]),
                                    ),
                                ),
                            ])),
                        ),
                    ])),
            ),
        ]),
        "Pascal type key followed by a deeply constrained vector topic field",
    )])
    .expect("registry has no conflicts");

    let matched = registry
        .dispatch(&MacroCandidate::from_pair(
            PositionPredicate::named("NamespaceDeclaration"),
            key,
            value,
        ))
        .expect("entry matches recursive child constraints");

    assert_eq!(matched.macro_name(), "VectorTopicStruct");
    assert!(
        matched
            .captures()
            .get(&CaptureName::new("element_type"))
            .is_some(),
        "recursive structural pattern captures through more than one nested delimiter"
    );
}

#[test]
fn reports_expected_shapes_when_no_macro_matches() {
    let document = Document::parse(include_str!("fixtures/macro-node/strict-namespace.nota"))
        .expect("fixture parses");
    let namespace = document.root_object_at(0).expect("fixture has root");
    let key = namespace.root_object_at(2).expect("kind key");
    let value = namespace.root_object_at(3).expect("kind value");
    let registry = MacroRegistry::new(vec![MacroNodeDefinition::new(
        "StructDeclaration",
        PositionPredicate::named("NamespaceDeclaration"),
        Pattern::new(vec![
            PatternElement::atom(AtomShape::symbol(Some(CaptureName::new("type_name")))),
            PatternElement::delimited(DelimitedShape::new(
                MacroDelimiter::Brace,
                MacroObjectCount::Even,
                Some(CaptureName::new("body")),
            )),
        ]),
        "symbol key followed by brace value",
    )])
    .expect("registry has no conflicts");

    let error = registry
        .dispatch(&MacroCandidate::from_pair(
            PositionPredicate::named("NamespaceDeclaration"),
            key,
            value,
        ))
        .expect_err("bracket value does not match struct declaration");

    let rendered = error.to_string();
    assert!(rendered.contains("NamespaceDeclaration"));
    assert!(rendered.contains("StructDeclaration"));
    assert!(rendered.contains("symbol key followed by brace value"));
}

#[test]
fn structural_macro_node_selects_first_matching_variant_in_order() {
    let document = Document::parse("Reserved").expect("fixture parses");
    let block = document.root_object_at(0).expect("fixture has root");

    let decoded = ExampleStructuralVariant::from_structural_block(block)
        .expect("reserved literal wins before generic PascalCase");

    assert_eq!(decoded, ExampleStructuralVariant::Reserved);
    assert_eq!(decoded.to_structural_nota(), "Reserved");
}

#[test]
fn structural_macro_node_decodes_and_encodes_data_variant() {
    let document = Document::parse("(Record Entry)").expect("fixture parses");
    let block = document.root_object_at(0).expect("fixture has root");

    let decoded =
        ExampleStructuralVariant::from_structural_block(block).expect("data variant decodes");

    assert_eq!(
        decoded,
        ExampleStructuralVariant::Data {
            variant_name: "Record".to_owned(),
            payload_name: "Entry".to_owned(),
        }
    );
    assert_eq!(decoded.to_structural_nota(), "(Record Entry)");

    let encoded_document = Document::parse(decoded.to_structural_nota()).expect("encoded parses");
    let encoded_block = encoded_document
        .root_object_at(0)
        .expect("encoded fixture has root");
    assert_eq!(
        ExampleStructuralVariant::from_structural_block(encoded_block)
            .expect("encoded structural node decodes"),
        decoded
    );
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ExampleStructuralVariant {
    Reserved,
    Unit {
        variant_name: String,
    },
    Data {
        variant_name: String,
        payload_name: String,
    },
}

impl StructuralMacroNode for ExampleStructuralVariant {
    type Error = ExampleStructuralVariantError;

    fn structural_position() -> PositionPredicate {
        PositionPredicate::named("ExampleVariant")
    }

    fn structural_variants() -> Vec<MacroNodeDefinition> {
        vec![
            MacroNodeDefinition::new(
                "reserved literal variant",
                Self::structural_position(),
                Pattern::new(vec![PatternElement::literal("Reserved")]),
                "literal Reserved variant",
            ),
            MacroNodeDefinition::new(
                "unit variant",
                Self::structural_position(),
                Pattern::new(vec![PatternElement::atom(AtomShape::pascal_case(Some(
                    CaptureName::new("variant_name"),
                )))]),
                "PascalCase variant atom",
            ),
            MacroNodeDefinition::new(
                "data variant",
                Self::structural_position(),
                Pattern::new(vec![PatternElement::delimited(
                    DelimitedShape::new(
                        MacroDelimiter::Parenthesis,
                        MacroObjectCount::Exact(2),
                        Some(CaptureName::new("variant_signature")),
                    )
                    .with_children(Pattern::new(vec![
                        PatternElement::atom(AtomShape::pascal_case(Some(CaptureName::new(
                            "variant_name",
                        )))),
                        PatternElement::atom(AtomShape::pascal_case(Some(CaptureName::new(
                            "payload_name",
                        )))),
                    ])),
                )]),
                "parenthesized variant name plus payload name",
            ),
        ]
    }

    fn from_structural_match(matched: MacroMatch<'_>) -> Result<Self, Self::Error> {
        match matched.macro_name() {
            "reserved literal variant" => Ok(Self::Reserved),
            "unit variant" => {
                let variant_name =
                    ExampleStructuralVariantMatch::new(&matched).captured_text("variant_name")?;
                Ok(Self::Unit { variant_name })
            }
            "data variant" => {
                let structural_match = ExampleStructuralVariantMatch::new(&matched);
                Ok(Self::Data {
                    variant_name: structural_match.captured_text("variant_name")?,
                    payload_name: structural_match.captured_text("payload_name")?,
                })
            }
            other => Err(ExampleStructuralVariantError::UnexpectedVariant(
                other.to_owned(),
            )),
        }
    }

    fn to_structural_nota(&self) -> String {
        match self {
            Self::Reserved => "Reserved".to_owned(),
            Self::Unit { variant_name } => variant_name.clone(),
            Self::Data {
                variant_name,
                payload_name,
            } => format!("({variant_name} {payload_name})"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ExampleStructuralVariantMatch<'match_value, 'block> {
    matched: &'match_value MacroMatch<'block>,
}

impl<'match_value, 'block> ExampleStructuralVariantMatch<'match_value, 'block> {
    fn new(matched: &'match_value MacroMatch<'block>) -> Self {
        Self { matched }
    }

    fn captured_text(
        &self,
        capture_name: &'static str,
    ) -> Result<String, ExampleStructuralVariantError> {
        let name = CaptureName::new(capture_name);
        self.matched
            .block_capture(&name)
            .and_then(|block| block.demote_to_string())
            .map(str::to_owned)
            .ok_or(ExampleStructuralVariantError::MissingCapture(capture_name))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ExampleStructuralVariantError {
    MissingCapture(&'static str),
    UnexpectedVariant(String),
}

impl fmt::Display for ExampleStructuralVariantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCapture(name) => write!(formatter, "missing capture {name}"),
            Self::UnexpectedVariant(name) => {
                write!(formatter, "unexpected structural variant {name}")
            }
        }
    }
}

impl std::error::Error for ExampleStructuralVariantError {}
