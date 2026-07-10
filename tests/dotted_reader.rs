use std::collections::BTreeMap;

use nota::{Document, DottedExpectation, NotaDecodeError, NotaEncode, NotaSource};

#[test]
fn uncapitalized_reads_inline_value_and_consumes_one_block() {
    let document = Document::parse("alpha.1").expect("valid nota");
    let entry = DottedExpectation::Uncapitalized
        .read_entry(document.root_objects())
        .expect("dotted entry reads");

    assert_eq!(entry.key().demote_to_string(), Some("alpha"));
    assert_eq!(entry.value().demote_to_string(), Some("1"));
    assert_eq!(entry.consumed(), 1);
}

#[test]
fn uncapitalized_reads_following_block_value_and_consumes_two_blocks() {
    let document = Document::parse("alpha.(inner value)").expect("valid nota");
    let entry = DottedExpectation::Uncapitalized
        .read_entry(document.root_objects())
        .expect("dotted entry reads");

    assert_eq!(entry.key().demote_to_string(), Some("alpha"));
    assert!(entry.value().is_parenthesis());
    assert_eq!(entry.value().holds_root_objects(), 2);
    assert_eq!(entry.consumed(), 2);
}

#[test]
fn capitalized_reads_type_application_head() {
    let document = Document::parse("Vector.X").expect("valid nota");
    let entry = DottedExpectation::Capitalized
        .read_entry(document.root_objects())
        .expect("dotted entry reads");

    assert_eq!(entry.key().demote_to_string(), Some("Vector"));
    assert_eq!(entry.value().demote_to_string(), Some("X"));
    assert_eq!(entry.consumed(), 1);
}

#[test]
fn head_case_is_checked_against_the_expectation() {
    let lowercase = Document::parse("vector.x").expect("valid nota");
    let capitalized_error = DottedExpectation::Capitalized
        .read_entry(lowercase.root_objects())
        .expect_err("capitalized rejects a lowercase head");
    assert!(matches!(
        capitalized_error,
        NotaDecodeError::DottedEntryCaseMismatch { .. }
    ));

    let uppercase = Document::parse("Vector.x").expect("valid nota");
    let uncapitalized_error = DottedExpectation::Uncapitalized
        .read_entry(uppercase.root_objects())
        .expect_err("uncapitalized rejects an uppercase head");
    assert!(matches!(
        uncapitalized_error,
        NotaDecodeError::DottedEntryCaseMismatch { .. }
    ));
}

#[test]
fn a_leading_atom_without_a_period_is_not_a_dotted_entry() {
    let document = Document::parse("alpha").expect("valid nota");
    let error = DottedExpectation::Uncapitalized
        .read_entry(document.root_objects())
        .expect_err("a period-free atom is not a dotted entry");
    assert!(matches!(error, NotaDecodeError::ExpectedDottedEntry { .. }));
}

#[test]
fn map_value_may_itself_be_dotted_and_round_trips_exactly() {
    let map = NotaSource::new("{path.a.b.c}")
        .parse::<BTreeMap<String, String>>()
        .expect("map decodes");

    assert_eq!(map.get("path"), Some(&String::from("a.b.c")));
    assert_eq!(map.to_nota(), "{path.a.b.c}");
}
