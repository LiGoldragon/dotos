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

use nota_next::{Block, Document, StructureHeader, StructureShape};

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

/// Illustrates: pipe-parenthesis and pipe-brace are recursive
/// delimiter forms. They are NOT string escapes. The reader exposes
/// them as normal nested blocks with distinct structural shapes, so a
/// schema layer can assign low-level declaration meaning to `(| ... |)`
/// and `{| ... |}` without losing access to inner objects.
#[test]
fn design_example_pipe_delimiters_are_recursive_blocks() {
    let source = "(| Kind (Decision Correction) |) {| Entry [Topic Kind] |}";
    let document = Document::parse(source).expect("nota parses");
    let enum_declaration = document.root_object_at(0).expect("pipe parenthesis");
    let struct_declaration = document.root_object_at(1).expect("pipe brace");

    assert!(enum_declaration.is_pipe_parenthesis());
    assert!(struct_declaration.is_pipe_brace());
    assert_eq!(
        enum_declaration.structure_shape(),
        StructureShape::PipeParenthesis
    );
    assert_eq!(
        struct_declaration.structure_shape(),
        StructureShape::PipeBrace
    );
    assert!(
        enum_declaration
            .root_object_at(1)
            .is_some_and(|block| block.is_parenthesis()),
        "the enum payload remains parsed as structure"
    );
    assert!(
        struct_declaration
            .root_object_at(1)
            .is_some_and(|block| block.is_square_bracket()),
        "the struct payload remains parsed as structure"
    );
}

/// Illustrates: the schema declaration target is name-first at-binding.
/// The raw NOTA parser keeps it structural by lowering `Name@{...}` and
/// `Name@[...]` to recursive declaration blocks, while `field@(...)`
/// becomes a normal two-object field pair. Parenthesis never opens an
/// enum declaration; it carries a type-reference or macro-call payload.
/// Schema decides what those objects mean.
#[test]
fn design_example_at_binding_exposes_schema_declarations_as_blocks() {
    let source =
        "Entry@{ topics@Topics topic@(Topic) records@(Vec Entry) } Kind@[Decision Correction]";
    let document = Document::parse(source).expect("nota parses");
    let struct_declaration = document.root_object_at(0).expect("struct declaration");
    let enum_declaration = document.root_object_at(1).expect("enum declaration");

    assert!(struct_declaration.is_pipe_brace());
    assert_eq!(
        struct_declaration
            .root_object_at(0)
            .and_then(Block::demote_to_string),
        Some("Entry")
    );
    assert_eq!(
        struct_declaration
            .root_object_at(1)
            .and_then(Block::demote_to_string),
        Some("topics@Topics"),
        "simple member bindings remain atom data for the schema layer"
    );
    let topic = struct_declaration.root_object_at(2).expect("topic field");
    assert!(topic.is_parenthesis());
    assert_eq!(
        topic.root_object_at(0).and_then(Block::demote_to_string),
        Some("topic")
    );
    assert_eq!(
        topic.root_object_at(1).and_then(Block::demote_to_string),
        Some("Topic"),
        "a single parenthesized reference is flattened into the field pair"
    );
    let records = struct_declaration
        .root_object_at(3)
        .expect("records composite field");
    assert!(records.is_parenthesis());
    assert_eq!(
        records.root_object_at(0).and_then(Block::demote_to_string),
        Some("records")
    );
    assert!(
        records
            .root_object_at(1)
            .is_some_and(|reference| reference.is_parenthesis()),
        "the composite reference remains a parenthesis object"
    );

    assert!(enum_declaration.is_pipe_parenthesis());
    assert_eq!(
        enum_declaration
            .root_object_at(0)
            .and_then(Block::demote_to_string),
        Some("Kind")
    );

    let alias = Document::parse("Topics@(Vec Topic)").expect("nota parses");
    let alias_binding = alias.root_object_at(0).expect("alias binding");
    assert!(
        alias_binding.is_parenthesis(),
        "Name@(…) is a binding to a parenthesized reference, not an enum declaration"
    );
    assert_eq!(
        alias_binding
            .root_object_at(0)
            .and_then(Block::demote_to_string),
        Some("Topics")
    );
}

/// Illustrates: the first parser pass already has enough information
/// to emit a compact first-two-level structure header. Slot 0 is the
/// document; the remaining slots are root blocks and their immediate
/// children in authored order. The header is structural only — it
/// records delimiter/atom shape and child counts, not schema meaning.
#[test]
fn design_example_structure_header_captures_first_two_levels() {
    let source = r#"(Input ((Record Entry) Drop))
(Output (Accepted))"#;
    let document = Document::parse(source).expect("nota parses");
    let header = document.structure_header();
    let observed: Vec<(StructureShape, u8)> = header
        .slots()
        .iter()
        .map(|slot| (slot.shape(), slot.child_count()))
        .collect();

    assert_eq!(
        observed,
        vec![
            (StructureShape::Document, 2),
            (StructureShape::Parenthesis, 2),
            (StructureShape::Atom, 0),
            (StructureShape::Parenthesis, 2),
            (StructureShape::Parenthesis, 2),
            (StructureShape::Atom, 0),
            (StructureShape::Parenthesis, 1),
        ],
    );

    assert_eq!(
        StructureHeader::from_packed_word(header.packed_word()),
        header,
        "the header is a stable 64-bit structural triage word",
    );
}

/// Illustrates: the packed 64-bit structure header is a triage word,
/// not a lossless replacement for the document tree. When one slot's
/// child count cannot fit in the 4-bit count field, the slot is marked
/// `Unknown` rather than pretending the exact shape survived.
#[test]
fn design_example_structure_header_marks_child_count_overflow() {
    let source = "(A B C D E F G H I J K L M N O P)";
    let document = Document::parse(source).expect("nota parses");
    let header = document.structure_header();

    let overflowed_slot = header.slots().get(1).expect("root object slot");
    assert_eq!(overflowed_slot.shape(), StructureShape::Unknown);
    assert_eq!(
        overflowed_slot.child_count(),
        15,
        "the count nibble is saturated only when the shape is marked unknown",
    );
    assert_eq!(
        StructureHeader::from_packed_word(header.packed_word()),
        header
    );
}

/// Illustrates: when the first-two-level structure contains more than
/// the eight slots available in the packed word, the final slot becomes
/// an overflow sentinel. Consumers can distinguish "complete eight-slot
/// header" from "prefix of a larger structure".
#[test]
fn design_example_structure_header_marks_slot_truncation() {
    let source = "A B C D E F G H I";
    let document = Document::parse(source).expect("nota parses");
    let header = document.structure_header();

    assert_eq!(header.slots().len(), StructureHeader::MAXIMUM_SLOTS);
    let last_slot = header.slots().last().expect("last packed slot");
    assert_eq!(last_slot.shape(), StructureShape::Unknown);
    assert_eq!(last_slot.child_count(), 15);
    assert_eq!(
        StructureHeader::from_packed_word(header.packed_word()),
        header
    );
}
