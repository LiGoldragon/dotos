use nota_next::{
    AtomShape, CaptureName, DelimitedShape, Document, MacroCandidate, MacroDelimiter,
    MacroNodeDefinition, MacroObjectCount, MacroRegistry, Pattern, PatternElement,
    PositionPredicate,
};

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
    assert!(matched.captures().get(&CaptureName::new("body")).is_some());
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
