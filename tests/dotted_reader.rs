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
fn uncapitalized_reads_a_delimited_payload_value_from_one_application() {
    let document = Document::parse("alpha.(inner value)").expect("valid nota");
    let entry = DottedExpectation::Uncapitalized
        .read_entry(document.root_objects())
        .expect("dotted entry reads");

    assert_eq!(entry.key().demote_to_string(), Some("alpha"));
    assert!(entry.value().is_parenthesis());
    assert_eq!(entry.value().holds_root_objects(), 2);
    // The raw parser binds the period, so a dotted pair is always one
    // application block: exactly one block is consumed regardless of whether
    // the value is inline or delimited.
    assert_eq!(entry.consumed(), 1);
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
fn map_value_carrying_dots_rejoins_bare_and_round_trips() {
    // A period is a structural operator at the raw layer, but the expected
    // `String` value reclaims the dotted text, so a dotted string value stays
    // bare: the entry is `path.a.b.c` and the map is the plain
    // `Map.( key.Value … )` surface with no pipe escape.
    let map = NotaSource::new("Map.(path.a.b.c)")
        .parse::<BTreeMap<String, String>>()
        .expect("map decodes");

    assert_eq!(map.get("path"), Some(&String::from("a.b.c")));
    assert_eq!(map.to_nota(), "Map.(path.a.b.c)");
}

/// The block-level reader over an inline-value atom and the string-level reader
/// over that same atom text must agree on key and value for every expectation
/// kind. This is the split parity witness: one mechanism, two entry forms.
#[test]
fn string_level_split_matches_the_block_level_reader() {
    for (expectation, text) in [
        (DottedExpectation::Uncapitalized, "alpha.1"),
        (DottedExpectation::Capitalized, "Vector.X"),
        (DottedExpectation::Uncapitalized, "path.a.b.c"),
    ] {
        let document = Document::parse(text).expect("valid nota");
        let block_entry = expectation
            .read_entry(document.root_objects())
            .expect("block-level dotted entry reads");
        let (string_key, string_value) = expectation
            .read_string_entry(text)
            .expect("string-level dotted entry reads");

        assert_eq!(block_entry.key().demote_to_string(), Some(string_key));
        // The block-level value of a dotted chain is a nested application, so
        // its flat form is the reconstruction that matches the string-level
        // split's whole remainder.
        assert_eq!(
            block_entry.value().dotted_text(),
            Some(string_value.to_owned())
        );
        assert_eq!(block_entry.consumed(), 1);
    }
}

/// Case rejection parity: an expectation that rejects a head at the block level
/// rejects the same head text at the string level with the same typed error.
#[test]
fn string_level_case_rejection_matches_the_block_level_reader() {
    let lowercase_head = "vector.x";
    let block_error = DottedExpectation::Capitalized
        .read_entry(
            Document::parse(lowercase_head)
                .expect("valid nota")
                .root_objects(),
        )
        .expect_err("capitalized rejects a lowercase head");
    let string_error = DottedExpectation::Capitalized
        .read_string_entry(lowercase_head)
        .expect_err("capitalized rejects a lowercase head at string level");
    assert!(matches!(
        block_error,
        NotaDecodeError::DottedEntryCaseMismatch { .. }
    ));
    assert!(matches!(
        string_error,
        NotaDecodeError::DottedEntryCaseMismatch { .. }
    ));

    let uppercase_head = "Vector.x";
    let block_error = DottedExpectation::Uncapitalized
        .read_entry(
            Document::parse(uppercase_head)
                .expect("valid nota")
                .root_objects(),
        )
        .expect_err("uncapitalized rejects an uppercase head");
    let string_error = DottedExpectation::Uncapitalized
        .read_string_entry(uppercase_head)
        .expect_err("uncapitalized rejects an uppercase head at string level");
    assert!(matches!(
        block_error,
        NotaDecodeError::DottedEntryCaseMismatch { .. }
    ));
    assert!(matches!(
        string_error,
        NotaDecodeError::DottedEntryCaseMismatch { .. }
    ));
}

/// No-period parity: a period-free string is not a dotted entry, the same
/// rejection the block-level reader gives a period-free leading atom.
#[test]
fn string_level_period_free_text_is_not_a_dotted_entry() {
    let error = DottedExpectation::Uncapitalized
        .read_string_entry("alpha")
        .expect_err("a period-free string is not a dotted entry");
    assert!(matches!(error, NotaDecodeError::ExpectedDottedEntry { .. }));
}

/// A string ending at the period has no following block to supply the value,
/// so the string-level form reports a missing value.
#[test]
fn string_level_text_ending_at_the_period_is_a_missing_value() {
    let error = DottedExpectation::Uncapitalized
        .read_string_entry("alpha.")
        .expect_err("a string ending at the period has no value");
    assert!(matches!(
        error,
        NotaDecodeError::DottedEntryMissingValue { .. }
    ));
}

/// The schema-language motivating shapes: a `paramName.Type` parameter binding
/// splits at the first dot into key and type, and a `path.a.b.c` import path
/// keeps the dotted remainder whole as the value.
#[test]
fn string_level_reads_the_schema_language_motivating_shapes() {
    let (key, value) = DottedExpectation::Uncapitalized
        .read_string_entry("paramName.Type")
        .expect("parameter binding splits");
    assert_eq!(key, "paramName");
    assert_eq!(value, "Type");

    let (key, value) = DottedExpectation::Uncapitalized
        .read_string_entry("path.a.b.c")
        .expect("import path splits at the first dot");
    assert_eq!(key, "path");
    assert_eq!(value, "a.b.c");
}
