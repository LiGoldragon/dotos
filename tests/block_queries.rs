use dotos::{ApplicationForm, Delimiter, Document, DotosError};

#[test]
fn parses_ordered_root_objects_and_reemits_from_spans() {
    let source = "(State [Statement]) { Topic [Text] }";
    let document = Document::parse(source).expect("valid dotos");

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
    let document = Document::parse("(Record [Entry Query])").expect("valid dotos");
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
    let document = Document::parse("[alpha beta]").expect("valid dotos");
    let root = document.root_object_at(0).expect("root");

    assert_eq!(Delimiter::SquareBracket.opening_text(), "[");
    assert_eq!(Delimiter::SquareBracket.closing_text(), "]");
    assert_eq!(
        Delimiter::Parenthesis.wrap(["Kind".to_owned(), "(Decision)".to_owned()]),
        "(Kind (Decision))"
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
fn exposes_structural_candidates_without_content_classification() {
    let document = Document::parse(
        "TypeName field-name camelName schema:module:Type CustomMacro RecordPayload 42 7.5 name@host required* a&b score^2 100% path/to a;b",
    )
    .expect("valid dotos");
    let roots = document.root_objects();

    // Case-shaped structural candidate predicates are answered on demand from
    // the atom's characters; they do not stamp a meaning onto the atom.
    assert!(roots[0].qualifies_as_pascal_case_symbol());
    assert!(roots[1].qualifies_as_kebab_case_symbol());
    assert!(roots[2].qualifies_as_camel_case_symbol());
    assert!(roots[3].qualifies_as_symbol());
    assert!(roots[4].qualifies_as_symbol());
    assert_eq!(
        roots[4].demote_to_string(),
        Some("CustomMacro"),
        "macro names are plain symbols; schema context decides whether a symbol invokes a macro"
    );
    assert!(
        roots[5].qualifies_as_pascal_case_symbol(),
        "payload names are still plain PascalCase symbol candidates"
    );

    // `42` is not read as a number here: the parser records no content
    // classification, so it is an ordinary symbol-safe atom whose numeric
    // meaning is decided only at decode under an expected type. `7.5` carries
    // a period, which is a structural dot-application operator, so it parses as
    // an application whose flat text a numeric decoder reconstructs — never a
    // single atom.
    assert!(roots[6].qualifies_as_symbol());
    assert_eq!(roots[6].demote_to_string(), Some("42"));
    assert!(!roots[6].qualifies_as_pascal_case_symbol());
    assert!(!roots[6].qualifies_as_camel_case_symbol());
    assert!(!roots[6].qualifies_as_kebab_case_symbol());
    assert!(
        roots[7].is_application(),
        "the period in 7.5 binds an application"
    );
    assert!(!roots[7].qualifies_as_symbol());
    assert_eq!(roots[7].demote_to_string(), None);
    assert_eq!(roots[7].dotted_text(), Some("7.5".to_owned()));

    for root in &roots[8..] {
        assert!(root.qualifies_as_symbol());
    }
}

#[test]
fn double_semicolon_is_comment_and_single_semicolon_is_atom_text() {
    let source = r#"alpha;beta ;; comment text
 gamma;; trailing comment"#;
    let document = Document::parse(source).expect("valid dotos");
    let roots = document.root_objects();

    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0].demote_to_string(), Some("alpha;beta"));
    assert_eq!(roots[1].demote_to_string(), Some("gamma"));
}

#[test]
fn curly_text_is_delimiter_safe_and_not_recursively_parsed() {
    let source = "“macro body with ] and \" and apostrophe's text”";
    let document = Document::parse(source).expect("valid dotos");
    let root = document.root_object_at(0).expect("root");

    assert!(root.is_curly_text());
    assert_eq!(
        root.demote_to_string(),
        Some("macro body with ] and \" and apostrophe's text")
    );
    assert_eq!(root.reemit(document.source()), source);
}

#[test]
fn pipes_are_rejected_and_angles_are_structural() {
    for source in ["(| Kind |)", "[| raw |]", "Name|Transformer"] {
        assert!(Document::parse(source).is_err(), "{source:?} rejects");
    }

    let document = Document::parse("Vector<Ordered>").expect("angle application parses");
    let root = document.root_object_at(0).expect("root");
    assert_eq!(root.application_form(), Some(ApplicationForm::Angle));
    let (head, payload) = root.as_application().expect("application");
    assert_eq!(head.demote_to_string(), Some("Vector"));
    assert!(payload.is_angle());
}

#[test]
fn reports_unclosed_delimiters_with_source_position() {
    let error = Document::parse("(Record [Entry]").expect_err("invalid dotos");

    assert!(matches!(
        error,
        DotosError::UnclosedDelimiter {
            position,
            ..
        } if position.line == 1 && position.column == 1
    ));
}
