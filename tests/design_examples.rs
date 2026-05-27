//! Design-illustrating tests for nota-next.
//!
//! Each test illustrates ONE load-bearing design point of the
//! structural NOTA reader with a short fixture and a focused
//! assertion. Test names start with `design_example_` so a reader
//! scanning the file knows which tests are for design representation
//! vs broader coverage.
//!
//! Companion to `tests/block_queries.rs` (the broader test surface).
//! When a design report cites a test, the test in this file should
//! be the canonical example.

use nota_next::Document;

/// Illustrates: every Block carries a SourceSpan that maps back to
/// the original text by byte offset AND line/column. The span
/// propagates through arbitrary nesting — a deeply nested block's
/// span points at its EXACT region of the source, not at an
/// approximation or a re-serialised form.
///
/// Intent records 770 + 771 (Maximum / High): the object-block pass
/// breaks NOTA text into delimiter-bounded blocks with source
/// locations; spans should track line and column directly rather
/// than normalize root objects onto one line.
///
/// This complements `parses_ordered_root_objects_and_reemits_from_spans`
/// in `block_queries.rs` — that test exercises `reemit`; this test
/// asserts the underlying `source_span` fields directly so the
/// design contract (positions are real line/column) shows in code.
#[test]
fn design_example_source_spans_propagate_through_nested_blocks() {
    let source = r#"(Record
  [Entry])"#;
    let document = Document::parse(source).expect("nota parses");
    let outer = document.root_object_at(0).expect("outer parenthesis");

    let outer_span = outer.source_span();
    assert_eq!(outer_span.start.line, 1);
    assert_eq!(outer_span.start.column, 1);
    assert_eq!(outer_span.start.byte_offset, 0);
    assert_eq!(outer_span.end.line, 2);
    assert_eq!(outer_span.end.column, 11);

    let inner = outer.root_object_at(1).expect("inner square-bracket");
    let inner_span = inner.source_span();
    assert_eq!(
        inner_span.start.line, 2,
        "nested block span tracks line through the newline",
    );
    assert_eq!(
        inner_span.start.column, 3,
        "nested block span tracks column at the opening delimiter",
    );
    assert_eq!(inner.reemit(document.source()), "[Entry]");
}

/// Illustrates: a bare PascalCase atom is `Block::Atom`, and the
/// reader exposes it ONLY through `Block::atom()` — there is no
/// `Block::is_pascal_case` factual method because PascalCase IS NOT
/// a structural fact at the NOTA layer. The reader returns a
/// candidate predicate `qualifies_as_pascal_case_symbol`; the schema
/// layer decides whether PascalCase is allowed at this position.
///
/// Intent records 786 + 800 (Maximum): raw NOTA structural reading
/// does not decide whether PascalCase is allowed for a final value
/// type; case legality belongs to the schema or macro context. The
/// distinction in classification methods is "qualifies as" not "is".
///
/// This is a SEPARATION test — confirms the boundary discipline by
/// asserting the absence of schema-level semantics in the reader API.
#[test]
fn design_example_reader_exposes_candidates_not_schema_semantics() {
    let document = Document::parse("Decision").expect("nota parses");
    let block = document.root_object_at(0).expect("first root");

    // Factual structural query — present.
    assert!(block.is_atom());
    assert!(!block.is_parenthesis());
    assert!(!block.is_square_bracket());
    assert!(!block.is_brace());
    assert!(!block.is_pipe_text());

    // Candidate query — present, names the classification candidate.
    assert!(block.qualifies_as_pascal_case_symbol());
    assert!(block.qualifies_as_symbol());
    assert!(!block.qualifies_as_camel_case_symbol());
    assert!(!block.qualifies_as_kebab_case_symbol());

    // Demote returns the underlying text. The reader does NOT promote
    // the text into any schema-level type — that's the higher layer.
    assert_eq!(block.demote_to_string(), Some("Decision"));
}
