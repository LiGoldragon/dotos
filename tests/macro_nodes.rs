use nota::{
    AtomShape, BlockShape, CaptureName, DelimitedShape, Document, MacroCandidate, MacroDelimiter,
    MacroMatch, MacroNodeDefinition, MacroObjectCount, MacroRegistry, Pattern, PatternElement,
    PositionPredicate, StructuralMacroError, StructuralMacroNode, StructuralMacroNodeError,
    StructuralVariant, StructuralVariantSet,
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
fn structural_block_shape_order_controls_specific_head_shadowing() {
    let document = Document::parse("(Optional Integer)").expect("fixture parses");
    let block = document.root_object_at(0).expect("fixture has root");
    let position = PositionPredicate::named("TypeReference");

    let specific_first = StructuralVariantSet::new(
        position.clone(),
        vec![
            BlockShape::headed_parenthesis(
                "Optional",
                MacroObjectCount::Exact(2),
                Some(CaptureName::new("signature")),
            )
            .into_structural_variant("optional reference", "Optional head carrying one reference"),
            BlockShape::pascal_headed_parenthesis(
                MacroObjectCount::Exact(2),
                CaptureName::new("constructor"),
                Some(CaptureName::new("signature")),
            )
            .into_structural_variant(
                "application reference",
                "PascalCase head carrying one reference",
            ),
        ],
    )
    .expect("structural variants have no conflicts");
    let matched_specific = specific_first
        .dispatch(&MacroCandidate::from_block(position.clone(), block))
        .expect("specific-first variant set matches");
    assert_eq!(matched_specific.macro_name(), "optional reference");

    let general_first = StructuralVariantSet::new(
        position,
        vec![
            BlockShape::pascal_headed_parenthesis(
                MacroObjectCount::Exact(2),
                CaptureName::new("constructor"),
                Some(CaptureName::new("signature")),
            )
            .into_structural_variant(
                "application reference",
                "PascalCase head carrying one reference",
            ),
            BlockShape::headed_parenthesis(
                "Optional",
                MacroObjectCount::Exact(2),
                Some(CaptureName::new("signature")),
            )
            .into_structural_variant("optional reference", "Optional head carrying one reference"),
        ],
    )
    .expect_err("general Pascal head makes the later Optional head unreachable");
    assert!(general_first.to_string().contains("application reference"));
    assert!(general_first.to_string().contains("optional reference"));
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

#[test]
fn structural_macro_node_derive_uses_enum_variant_order() {
    let input = "(Optional Integer)";
    let decoded = DerivedReference::from_structural_nota(input).expect("derived node decodes");
    let misordered = MisorderedDerivedReference::from_structural_nota(input)
        .expect_err("misordered derived node has an unreachable Optional variant");

    assert_eq!(
        decoded,
        DerivedReference::Optional(Box::new(DerivedReference::Named(DerivedTypeName(
            "Integer".to_owned()
        ))))
    );
    assert_eq!(decoded.to_structural_nota(), input);
    assert!(misordered.to_string().contains("Application"));
    assert!(misordered.to_string().contains("Optional"));
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

    fn structural_variants() -> Vec<StructuralVariant> {
        vec![
            BlockShape::literal("Reserved")
                .into_structural_variant("reserved literal variant", "literal Reserved variant"),
            BlockShape::pascal_atom(Some(CaptureName::new("variant_name")))
                .into_structural_variant("unit variant", "PascalCase variant atom"),
            BlockShape::delimited(
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
            ]))
            .into_structural_variant(
                "data variant",
                "parenthesized variant name plus payload name",
            ),
        ]
    }

    fn from_structural_candidate(
        candidate: MacroCandidate<'_>,
    ) -> Result<Self, StructuralMacroError<Self::Error>> {
        let variants =
            StructuralVariantSet::new(Self::structural_position(), Self::structural_variants())
                .map_err(StructuralMacroError::Dispatch)?;
        let matched = variants
            .dispatch(&candidate)
            .map_err(StructuralMacroError::Dispatch)?;
        (|| -> Result<Self, Self::Error> {
            match matched.macro_name() {
                "reserved literal variant" => Ok(Self::Reserved),
                "unit variant" => {
                    let variant_name = ExampleStructuralVariantMatch::new(&matched)
                        .captured_text("variant_name")?;
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
        })()
        .map_err(StructuralMacroError::MatchedNode)
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct DerivedTypeName(String);

impl StructuralMacroNode for DerivedTypeName {
    type Error = StructuralMacroNodeError;

    fn structural_position() -> PositionPredicate {
        PositionPredicate::named("DerivedTypeName")
    }

    fn structural_variants() -> Vec<StructuralVariant> {
        vec![
            BlockShape::pascal_atom(Some(CaptureName::new("field_0")))
                .into_structural_variant("DerivedTypeName", "PascalCase type name"),
        ]
    }

    fn from_structural_candidate(
        candidate: MacroCandidate<'_>,
    ) -> Result<Self, StructuralMacroError<Self::Error>> {
        let variants =
            StructuralVariantSet::new(Self::structural_position(), Self::structural_variants())
                .map_err(StructuralMacroError::Dispatch)?;
        let matched = variants
            .dispatch(&candidate)
            .map_err(StructuralMacroError::Dispatch)?;
        Self::from_match(matched).map_err(StructuralMacroError::MatchedNode)
    }

    fn to_structural_nota(&self) -> String {
        self.0.clone()
    }
}

impl DerivedTypeName {
    fn from_match(matched: MacroMatch<'_>) -> Result<Self, StructuralMacroNodeError> {
        let block = matched.block_capture(&CaptureName::new("field_0")).ok_or(
            StructuralMacroNodeError::MissingCapture {
                node: "DerivedTypeName",
                variant: "DerivedTypeName",
                capture: "field_0",
            },
        )?;
        let Some(text) = block.demote_to_string() else {
            return Err(StructuralMacroNodeError::MissingCapture {
                node: "DerivedTypeName",
                variant: "DerivedTypeName",
                capture: "field_0",
            });
        };
        Ok(Self(text.to_owned()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, StructuralMacroNode)]
enum DerivedReference {
    #[shape(pascal_atom)]
    Named(DerivedTypeName),
    #[shape(head = "Optional", arity = 2)]
    Optional(Box<DerivedReference>),
    #[shape(pascal_head, arity = 2)]
    Application(DerivedTypeName, Box<DerivedReference>),
}

#[derive(Clone, Debug, Eq, PartialEq, StructuralMacroNode)]
enum MisorderedDerivedReference {
    #[shape(pascal_atom)]
    Named(DerivedTypeName),
    #[shape(pascal_head, arity = 2)]
    Application(DerivedTypeName, Box<MisorderedDerivedReference>),
    #[shape(head = "Optional", arity = 2)]
    Optional(Box<MisorderedDerivedReference>),
}

#[test]
fn structural_macro_node_keyword_field_discriminates_inner_marker() {
    let opens = "(Watch WatchRequest opens RecordStream)";
    let decoded = DerivedVariantSignature::from_structural_nota(opens).expect("opens form decodes");
    assert_eq!(
        decoded,
        DerivedVariantSignature::Streaming(
            DerivedTypeName("Watch".to_owned()),
            DerivedTypeName("WatchRequest".to_owned()),
            DerivedRelationKeyword::Opens,
            DerivedTypeName("RecordStream".to_owned()),
        )
    );
    assert_eq!(decoded.to_structural_nota(), opens);

    let belongs = "(RecordChanged RecordChanged belongs RecordStream)";
    let decoded =
        DerivedVariantSignature::from_structural_nota(belongs).expect("belongs form decodes");
    assert_eq!(
        decoded,
        DerivedVariantSignature::Streaming(
            DerivedTypeName("RecordChanged".to_owned()),
            DerivedTypeName("RecordChanged".to_owned()),
            DerivedRelationKeyword::Belongs,
            DerivedTypeName("RecordStream".to_owned()),
        )
    );
    assert_eq!(decoded.to_structural_nota(), belongs);

    let unit =
        DerivedVariantSignature::from_structural_nota("Reserved").expect("unit form decodes");
    assert_eq!(
        unit,
        DerivedVariantSignature::Unit(DerivedTypeName("Reserved".to_owned()))
    );

    let data =
        DerivedVariantSignature::from_structural_nota("(Record Entry)").expect("data form decodes");
    assert_eq!(
        data,
        DerivedVariantSignature::Data(
            DerivedTypeName("Record".to_owned()),
            DerivedTypeName("Entry".to_owned()),
        )
    );
}

#[derive(Clone, Debug, Eq, PartialEq, StructuralMacroNode)]
enum DerivedRelationKeyword {
    #[shape(keyword = "opens")]
    Opens,
    #[shape(keyword = "belongs")]
    Belongs,
}

#[derive(Clone, Debug, Eq, PartialEq, StructuralMacroNode)]
enum DerivedVariantSignature {
    #[shape(pascal_atom)]
    Unit(DerivedTypeName),
    #[shape(pascal_head, arity = 2)]
    Data(DerivedTypeName, DerivedTypeName),
    #[shape(pascal_head, arity = 4)]
    Streaming(
        DerivedTypeName,
        DerivedTypeName,
        DerivedRelationKeyword,
        DerivedTypeName,
    ),
}

#[derive(Clone, Debug, Eq, PartialEq, StructuralMacroNode)]
enum DerivedTemplate {
    #[shape(head = "Variants", body)]
    Variants(Vec<DerivedTypeName>),
    #[shape(head = "Reference", arity = 2)]
    Reference(DerivedTypeName),
}

/// A heterogeneous body payload: the expected type reads the headed tail as
/// its own ordered object stream through `from_structural_candidate`.
#[derive(Clone, Debug, Eq, PartialEq)]
struct DerivedSignature {
    name: DerivedTypeName,
    payload: DerivedTypeName,
}

impl StructuralMacroNode for DerivedSignature {
    type Error = StructuralMacroNodeError;

    fn structural_position() -> PositionPredicate {
        PositionPredicate::named("DerivedSignature")
    }

    fn structural_variants() -> Vec<StructuralVariant> {
        Vec::new()
    }

    fn from_structural_candidate(
        candidate: MacroCandidate<'_>,
    ) -> Result<Self, StructuralMacroError<Self::Error>> {
        match candidate.blocks() {
            [name, payload] => Ok(Self {
                name: DerivedTypeName::from_structural_block(name)?,
                payload: DerivedTypeName::from_structural_block(payload)?,
            }),
            blocks => Err(StructuralMacroError::MatchedNode(
                StructuralMacroNodeError::MissingSlot {
                    node: "DerivedSignature",
                    variant: "DerivedSignature",
                    capture: "body",
                    slot: blocks.len(),
                },
            )),
        }
    }

    fn to_structural_nota(&self) -> String {
        format!(
            "{} {}",
            self.name.to_structural_nota(),
            self.payload.to_structural_nota()
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, StructuralMacroNode)]
enum DerivedDeclaration {
    #[shape(head = "Declare", body)]
    Declare(DerivedSignature),
}

#[test]
fn structural_macro_node_body_shape_reads_headed_tail_as_vector() {
    let decoded = DerivedTemplate::from_structural_nota("(Variants Alpha Beta Gamma)")
        .expect("variable-arity body decodes");
    assert_eq!(
        decoded,
        DerivedTemplate::Variants(vec![
            DerivedTypeName("Alpha".to_owned()),
            DerivedTypeName("Beta".to_owned()),
            DerivedTypeName("Gamma".to_owned()),
        ])
    );
    assert_eq!(decoded.to_structural_nota(), "(Variants Alpha Beta Gamma)");

    let sibling = DerivedTemplate::from_structural_nota("(Reference Entry)")
        .expect("fixed-arity sibling still decodes");
    assert_eq!(
        sibling,
        DerivedTemplate::Reference(DerivedTypeName("Entry".to_owned()))
    );
}

#[test]
fn structural_macro_node_body_shape_accepts_empty_tail() {
    let decoded = DerivedTemplate::from_structural_nota("(Variants)").expect("empty body decodes");
    assert_eq!(decoded, DerivedTemplate::Variants(Vec::new()));
    assert_eq!(decoded.to_structural_nota(), "(Variants)");
}

#[test]
fn structural_macro_node_body_shape_reads_heterogeneous_body_type() {
    let decoded = DerivedDeclaration::from_structural_nota("(Declare Watch WatchRequest)")
        .expect("struct-shaped body decodes through its own candidate");
    assert_eq!(
        decoded,
        DerivedDeclaration::Declare(DerivedSignature {
            name: DerivedTypeName("Watch".to_owned()),
            payload: DerivedTypeName("WatchRequest".to_owned()),
        })
    );
    assert_eq!(decoded.to_structural_nota(), "(Declare Watch WatchRequest)");

    let error = DerivedDeclaration::from_structural_nota("(Declare Watch)")
        .expect_err("short body is a typed field error");
    assert!(matches!(
        error,
        StructuralMacroError::MatchedNode(StructuralMacroNodeError::Field { .. })
    ));
}

#[test]
fn structural_macro_node_body_shape_rejects_unknown_head() {
    let error = DerivedTemplate::from_structural_nota("(Bogus Alpha)")
        .expect_err("unknown head does not match any variant");
    assert!(matches!(error, StructuralMacroError::Dispatch(_)));
}
/// The `pascal_head` + `body` keystone form: a captured PascalCase head plus a
/// variable-arity tail. The fixed-arity `Pair` variant precedes the broad
/// `Apply` so it is reached first for its exact shape.
#[derive(Clone, Debug, Eq, PartialEq, StructuralMacroNode)]
enum DerivedApplication {
    #[shape(pascal_head, arity = 2)]
    Pair(DerivedTypeName, DerivedTypeName),
    #[shape(pascal_head, body)]
    Apply(DerivedTypeName, Vec<DerivedTypeName>),
}

#[test]
fn structural_macro_node_pascal_head_body_reads_captured_head_with_tail() {
    let decoded = DerivedApplication::from_structural_nota("(Foo A B)")
        .expect("captured head with variable body decodes");
    assert_eq!(
        decoded,
        DerivedApplication::Apply(
            DerivedTypeName("Foo".to_owned()),
            vec![
                DerivedTypeName("A".to_owned()),
                DerivedTypeName("B".to_owned()),
            ],
        )
    );
    assert_eq!(decoded.to_structural_nota(), "(Foo A B)");
}

#[test]
fn structural_macro_node_pascal_head_body_accepts_empty_tail() {
    let decoded = DerivedApplication::from_structural_nota("(Foo)")
        .expect("captured head with empty body decodes");
    assert_eq!(
        decoded,
        DerivedApplication::Apply(DerivedTypeName("Foo".to_owned()), Vec::new())
    );
    assert_eq!(decoded.to_structural_nota(), "(Foo)");
}

#[test]
fn structural_macro_node_pascal_head_body_rejects_non_pascal_head() {
    let error = DerivedApplication::from_structural_nota("(lower A B)")
        .expect_err("a non-PascalCase head matches no variant");
    assert!(matches!(error, StructuralMacroError::Dispatch(_)));
}

#[test]
fn structural_macro_node_pascal_head_body_does_not_shadow_fixed_arity_sibling() {
    // The arity-2 `Pair` shape is two root objects total: head plus one slot.
    // It must win for that exact shape rather than fall to the broad `Apply`.
    let pair = DerivedApplication::from_structural_nota("(Bar X)")
        .expect("exact-arity head decodes through the specific Pair variant");
    assert_eq!(
        pair,
        DerivedApplication::Pair(
            DerivedTypeName("Bar".to_owned()),
            DerivedTypeName("X".to_owned()),
        )
    );
    assert_eq!(pair.to_structural_nota(), "(Bar X)");

    // A longer head has no fixed-arity match, so it falls to the broad variant.
    let apply = DerivedApplication::from_structural_nota("(Baz X Y Z)")
        .expect("longer head falls to the broad PascalHeadBody variant");
    assert_eq!(
        apply,
        DerivedApplication::Apply(
            DerivedTypeName("Baz".to_owned()),
            vec![
                DerivedTypeName("X".to_owned()),
                DerivedTypeName("Y".to_owned()),
                DerivedTypeName("Z".to_owned()),
            ],
        )
    );
    assert_eq!(apply.to_structural_nota(), "(Baz X Y Z)");
}

/// Mirrors schema-next `TypeReference`: a headed atom-leaf variant
/// (`FixedBytes`) reads a numeric width directly from the atom text, a
/// keyword variant (`Bytes`) is the bare-head sibling, and a two-child
/// `Headed` variant (`Map`) proves the flat map form needs no named payload —
/// a head with two typed sub-node children already decodes.
#[derive(Clone, Debug, Eq, PartialEq, StructuralMacroNode)]
enum DerivedTypeReference {
    #[shape(keyword = "Bytes")]
    Bytes,
    #[shape(head = "Bytes", atom)]
    FixedBytes(u64),
    #[shape(head = "Vector", arity = 2)]
    Vector(Box<DerivedTypeReference>),
    #[shape(head = "Optional", arity = 2)]
    Optional(Box<DerivedTypeReference>),
    #[shape(head = "ScopeOf", arity = 2)]
    ScopeOf(Box<DerivedTypeReference>),
    #[shape(head = "Map", arity = 3)]
    Map(Box<DerivedTypeReference>, Box<DerivedTypeReference>),
    #[shape(pascal_atom)]
    Plain(DerivedTypeName),
}

#[test]
fn structural_macro_node_atom_shape_decodes_numeric_width() {
    let decoded = DerivedTypeReference::from_structural_nota("(Bytes 32)")
        .expect("fixed-bytes width decodes");
    assert_eq!(decoded, DerivedTypeReference::FixedBytes(32));
    assert_eq!(decoded.to_structural_nota(), "(Bytes 32)");

    let reparsed = DerivedTypeReference::from_structural_nota(&decoded.to_structural_nota())
        .expect("re-encoded fixed-bytes decodes identically");
    assert_eq!(reparsed, decoded);
}

#[test]
fn structural_macro_node_atom_shape_round_trips_distinct_widths() {
    for width in [0u64, 1, 8, 64, 4096] {
        let source = format!("(Bytes {width})");
        let decoded = DerivedTypeReference::from_structural_nota(&source)
            .expect("each width decodes through the atom leaf");
        assert_eq!(decoded, DerivedTypeReference::FixedBytes(width));
        assert_eq!(decoded.to_structural_nota(), source);
    }
}

#[test]
fn structural_macro_node_atom_shape_rejects_non_numeric_atom() {
    let error = DerivedTypeReference::from_structural_nota("(Bytes wide)")
        .expect_err("a non-numeric width is a typed field error, not a silent miss");
    assert!(matches!(
        error,
        StructuralMacroError::MatchedNode(StructuralMacroNodeError::Field { field: 0, .. })
    ));
}

#[test]
fn structural_macro_node_atom_keyword_sibling_stays_distinct() {
    let bare =
        DerivedTypeReference::from_structural_nota("Bytes").expect("bare Bytes keyword decodes");
    assert_eq!(bare, DerivedTypeReference::Bytes);
    assert_eq!(bare.to_structural_nota(), "Bytes");
}

#[test]
fn structural_macro_node_map_head_decodes_two_typed_children() {
    let source = "(Map Integer (Vector Boolean))";
    let decoded = DerivedTypeReference::from_structural_nota(source)
        .expect("map head with two sub-node children decodes without a named payload");
    assert_eq!(
        decoded,
        DerivedTypeReference::Map(
            Box::new(DerivedTypeReference::Plain(DerivedTypeName(
                "Integer".to_owned()
            ))),
            Box::new(DerivedTypeReference::Vector(Box::new(
                DerivedTypeReference::Plain(DerivedTypeName("Boolean".to_owned()))
            ))),
        )
    );
    assert_eq!(decoded.to_structural_nota(), source);
}

#[test]
fn structural_macro_node_type_reference_round_trips_each_form() {
    for source in [
        "Bytes",
        "(Bytes 32)",
        "(Vector Integer)",
        "(Optional Boolean)",
        "(ScopeOf Path)",
        "(Map String Integer)",
        "Entry",
    ] {
        let decoded = DerivedTypeReference::from_structural_nota(source)
            .unwrap_or_else(|error| panic!("{source} decodes: {error}"));
        assert_eq!(
            decoded.to_structural_nota(),
            source,
            "{source} re-encodes identically"
        );
        let reparsed = DerivedTypeReference::from_structural_nota(&decoded.to_structural_nota())
            .unwrap_or_else(|error| panic!("{source} re-decodes: {error}"));
        assert_eq!(reparsed, decoded, "{source} round-trips through the node");
    }
}
