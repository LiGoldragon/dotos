use nota_next::{AtomClassification, Delimiter, Document, NotaError};

#[test]
fn parses_ordered_root_objects_and_reemits_from_spans() {
    let source = "(State [Statement]) { Topic [Text] }";
    let document = Document::parse(source).expect("valid nota");

    assert_eq!(document.holds_root_objects(), 2);
    let first = document.root_object_at(0).expect("first root");
    let second = document.root_object_at(1).expect("second root");

    assert!(first.is_parenthesis());
    assert!(second.is_brace());
    assert_eq!(first.reemit(document.source()), "(State [Statement])");
    assert_eq!(second.reemit(document.source()), "{ Topic [Text] }");
}

#[test]
fn exposes_recursive_shape_predicates() {
    let document = Document::parse("(Record [Entry Query])").expect("valid nota");
    let root = document.root_object_at(0).expect("root");

    assert!(root.is_parenthesis());
    assert!(root.holds_two_root_objects());
    assert!(root.root_object_at(0).is_some_and(|block| {
        block.qualifies_as_pascal_case_symbol() && block.demote_to_string() == Some("Record")
    }));
    assert!(
        root.root_object_at(1)
            .is_some_and(|block| block.is_square_bracket())
    );
}

#[test]
fn exposes_delimiter_text_and_child_helpers() {
    let document = Document::parse("[alpha beta]").expect("valid nota");
    let root = document.root_object_at(0).expect("root");

    assert_eq!(Delimiter::SquareBracket.opening_text(), "[");
    assert_eq!(Delimiter::SquareBracket.closing_text(), "]");
    assert_eq!(
        Delimiter::PipeParenthesis.wrap(["Kind".to_owned(), "(Decision)".to_owned()]),
        "(|Kind (Decision)|)"
    );
    assert!(root.is_delimited_with(Delimiter::SquareBracket));
    assert_eq!(
        root.as_delimited(Delimiter::SquareBracket)
            .expect("square children")
            .len(),
        2
    );
    assert!(root.as_delimited(Delimiter::Brace).is_none());
}

#[test]
fn classifies_atoms_as_candidates_without_schema_semantics() {
    let document = Document::parse(
        "TypeName field-name camelName schema:module:Type CustomMacro RecordPayload 42 7.5 name@host required* a&b score^2 100% x>y x<y path/to a;b",
    )
    .expect("valid nota");
    let roots = document.root_objects();

    assert!(roots[0].qualifies_as_pascal_case_symbol());
    assert!(roots[1].qualifies_as_kebab_case_symbol());
    assert!(roots[2].qualifies_as_camel_case_symbol());
    assert!(roots[3].qualifies_as_symbol());
    assert!(roots[4].qualifies_as_symbol());
    assert_eq!(
        roots[4].atom().expect("atom").classification(),
        AtomClassification::SymbolCandidate,
        "macro names are plain symbols; schema context decides whether a symbol invokes a macro"
    );
    assert_eq!(roots[4].demote_to_string(), Some("CustomMacro"));
    assert!(
        roots[5].qualifies_as_pascal_case_symbol(),
        "payload names are still plain PascalCase symbol candidates"
    );
    assert_eq!(
        roots[6].atom().expect("atom").classification(),
        AtomClassification::IntegerCandidate
    );
    assert_eq!(
        roots[7].atom().expect("atom").classification(),
        AtomClassification::DecimalCandidate
    );
    for root in &roots[8..] {
        assert_eq!(
            root.atom().expect("atom").classification(),
            AtomClassification::SymbolCandidate
        );
    }
}

#[test]
fn double_semicolon_is_comment_and_single_semicolon_is_atom_text() {
    let source = "alpha;beta ;; comment text\n gamma;; trailing comment";
    let document = Document::parse(source).expect("valid nota");
    let roots = document.root_objects();

    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0].demote_to_string(), Some("alpha;beta"));
    assert_eq!(roots[1].demote_to_string(), Some("gamma"));
}

#[test]
fn pipe_parenthesis_and_pipe_brace_are_recursive_delimiters() {
    let source = "(| Kind (Decision [Reason]) |) {| Entry [Topic [Tag]] |}";
    let document = Document::parse(source).expect("valid nota");
    let roots = document.root_objects();

    assert_eq!(roots.len(), 2);
    assert!(roots[0].is_pipe_parenthesis());
    assert!(roots[1].is_pipe_brace());
    assert_eq!(roots[0].holds_root_objects(), 2);
    assert_eq!(roots[1].holds_root_objects(), 2);
    assert!(
        roots[0]
            .root_object_at(1)
            .is_some_and(|block| block.is_parenthesis()),
        "pipe parenthesis is recursive, not raw pipe text"
    );
    assert!(
        roots[1]
            .root_object_at(1)
            .and_then(|block| block.root_object_at(1))
            .is_some_and(|block| block.is_square_bracket()),
        "pipe brace keeps nested native collection objects visible to schema"
    );
    assert_eq!(
        roots[0].reemit(document.source()),
        "(| Kind (Decision [Reason]) |)"
    );
    assert_eq!(
        roots[1].reemit(document.source()),
        "{| Entry [Topic [Tag]] |}"
    );
}

#[test]
fn pipe_brace_can_nest_pipe_brace_declarations() {
    let source =
        "{| Entry receipt {| Receipt recordIdentifier RecordIdentifier |} later Receipt |}";
    let document = Document::parse(source).expect("valid nota");
    let root = document.root_object_at(0).expect("root");

    assert!(root.is_pipe_brace());
    assert!(
        root.root_object_at(2)
            .is_some_and(|block| block.is_pipe_brace())
    );
    assert_eq!(root.reemit(document.source()), source);

    let compact_source =
        "{|Entry receipt {|Receipt recordIdentifier RecordIdentifier|} later Receipt|}";
    let compact_document = Document::parse(compact_source).expect("compact valid nota");
    let compact_root = compact_document.root_object_at(0).expect("compact root");
    assert!(compact_root.is_pipe_brace());
    assert!(
        compact_root
            .root_object_at(2)
            .is_some_and(|block| block.is_pipe_brace())
    );
}

#[test]
fn pipe_text_is_square_bracket_safe_and_not_recursively_parsed() {
    let source = "[|macro body with ] and \" and apostrophe's text|]";
    let document = Document::parse(source).expect("valid nota");
    let root = document.root_object_at(0).expect("root");

    assert!(root.is_pipe_text());
    assert_eq!(
        root.demote_to_string(),
        Some("macro body with ] and \" and apostrophe's text")
    );
    assert_eq!(root.reemit(document.source()), source);
}

#[test]
fn pipe_text_escapes_single_pipe_close_marker() {
    let source = "[|macro body can contain \\|] without ending|]";
    let document = Document::parse(source).expect("valid nota");
    let root = document.root_object_at(0).expect("root");

    assert!(root.is_pipe_text());
    assert_eq!(
        root.demote_to_string(),
        Some("macro body can contain |] without ending")
    );
    assert_eq!(root.reemit(document.source()), source);
}

#[test]
fn reports_unclosed_delimiters_with_source_position() {
    let error = Document::parse("(Record [Entry]").expect_err("invalid nota");

    assert!(matches!(
        error,
        NotaError::UnclosedDelimiter {
            position,
            ..
        } if position.line == 1 && position.column == 1
    ));
}
