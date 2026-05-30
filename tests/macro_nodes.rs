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
