use dotos::{Document, DotosOutputForm, PrettyLayout};

/// Collapse any DOTOS source back to its canonical single-line form by rendering
/// with an unbounded line width, so every block fits inline. This is the
/// round-trip oracle: a readability projection is correct exactly when
/// collapsing it reproduces the collapsed source.
fn collapse(source: &str) -> String {
    PrettyLayout::new(usize::MAX, 2)
        .render_dotos(source)
        .expect("valid dotos")
}

/// The block trees of two DOTOS sources are structurally identical when they
/// collapse to the same canonical single line.
fn assert_reparses_equal(source: &str, pretty: &str) {
    assert_eq!(
        collapse(source),
        collapse(pretty),
        "pretty output must re-parse to the same document as its source"
    );
}

#[test]
fn a_document_that_fits_the_width_is_returned_on_one_line() {
    let source = "(Small alpha beta)";
    let pretty = PrettyLayout::standard()
        .render_dotos(source)
        .expect("valid dotos");
    assert_eq!(pretty, source);
}

#[test]
fn a_block_past_the_width_breaks_at_each_child_with_indentation() {
    let source = "(Alpha (Beta gamma) delta)";
    let pretty = PrettyLayout::new(20, 2)
        .render_dotos(source)
        .expect("valid dotos");
    assert_eq!(pretty, "(\n  Alpha\n  (Beta gamma)\n  delta\n)");
}

#[test]
fn a_child_that_fits_stays_inline_while_its_parent_breaks() {
    // `(Beta gamma)` fits within the width at its indentation, so only the outer
    // block breaks: the inner block stays on one line.
    let source = "(Alpha (Beta gamma) delta)";
    let pretty = PrettyLayout::new(20, 2)
        .render_dotos(source)
        .expect("valid dotos");
    assert!(pretty.contains("  (Beta gamma)\n"));
    assert_reparses_equal(source, &pretty);
}

#[test]
fn nested_blocks_indent_one_step_per_depth() {
    let source = "(One (Two (Three deep)))";
    let pretty = PrettyLayout::new(16, 2)
        .render_dotos(source)
        .expect("valid dotos");
    assert_eq!(pretty, "(\n  One\n  (\n    Two\n    (Three deep)\n  )\n)");
    assert_reparses_equal(source, &pretty);
}

#[test]
fn indentation_step_is_configurable() {
    let source = "(Alpha (Beta gamma) delta)";
    let pretty = PrettyLayout::new(20, 4)
        .render_dotos(source)
        .expect("valid dotos");
    assert_eq!(pretty, "(\n    Alpha\n    (Beta gamma)\n    delta\n)");
}

#[test]
fn pipe_text_bytes_survive_a_break_verbatim() {
    // A pipe-text leaf carries a raw newline. The layout must never indent
    // inside it, and the exact bytes must round-trip.
    let source = "(Note (|first line
second line|) tail)";
    let pretty = PrettyLayout::new(12, 2)
        .render_dotos(source)
        .expect("valid dotos");
    assert!(pretty.contains(
        "(|first line
second line|)"
    ));
    assert_reparses_equal(source, &pretty);
}

#[test]
fn an_empty_delimited_block_stays_inline() {
    let source = "(Wrapper () [])";
    let pretty = PrettyLayout::new(5, 2)
        .render_dotos(source)
        .expect("valid dotos");
    assert!(pretty.contains("()"));
    assert!(pretty.contains("[]"));
    assert_reparses_equal(source, &pretty);
}

#[test]
fn brace_map_entries_break_onto_their_own_lines() {
    let source = "{alpha.[one two] beta.[three four] gamma.[five six]}";
    let pretty = PrettyLayout::new(24, 2)
        .render_dotos(source)
        .expect("valid dotos");
    assert_eq!(
        pretty,
        "{\n  alpha.[one two]\n  beta.[three four]\n  gamma.[five six]\n}"
    );
    assert_reparses_equal(source, &pretty);
}

#[test]
fn multiple_root_objects_each_start_a_line() {
    let source = "(First a) (Second b)";
    let pretty = PrettyLayout::new(10, 2)
        .render_dotos(source)
        .expect("valid dotos");
    assert_eq!(pretty, "(First a)\n(Second b)");
    assert_reparses_equal(source, &pretty);
}

#[test]
fn reformatting_is_idempotent() {
    let source = "(Alpha (Beta (Gamma one two three)) delta epsilon zeta)";
    let layout = PrettyLayout::new(20, 2);
    let once = layout.render_dotos(source).expect("valid dotos");
    let twice = layout.render_dotos(&once).expect("valid dotos");
    assert_eq!(once, twice);
    assert_reparses_equal(source, &once);
}

#[test]
fn every_sample_round_trips_across_widths() {
    let samples = [
        "(Small alpha beta)",
        "(Alpha (Beta gamma) delta)",
        "(One (Two (Three (Four deep))))",
        "{alpha.[one] beta.[two] gamma.[three]}",
        "(Note [|a raw
block|] tail)",
        "(Mixed [a b c] {key.value} (Inner x y) plain)",
        "(First a) (Second (Nested b c)) (Third d)",
    ];
    for source in samples {
        for width in [0_usize, 1, 8, 20, 40, 80, usize::MAX] {
            let pretty = PrettyLayout::new(width, 2)
                .render_dotos(source)
                .unwrap_or_else(|error| panic!("sample {source:?} at width {width}: {error}"));
            assert_eq!(
                collapse(source),
                collapse(&pretty),
                "sample {source:?} at width {width} must re-parse to its source"
            );
        }
    }
}

#[test]
fn output_form_canonical_is_byte_identical_and_pretty_reflows() {
    let canonical = "(Alpha (Beta gamma) delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma)";

    let unchanged = DotosOutputForm::from_pretty_requested(false).render(canonical);
    assert_eq!(unchanged, canonical);

    let pretty = DotosOutputForm::from_pretty_requested(true).render(canonical);
    assert!(pretty.contains('\n'));
    assert_eq!(collapse(canonical), collapse(&pretty));
}

#[test]
fn a_document_can_be_rendered_from_a_parsed_value() {
    let document = Document::parse("(Alpha (Beta gamma) delta)").expect("valid dotos");
    let pretty = PrettyLayout::new(20, 2).render_document(&document);
    assert_eq!(pretty, "(\n  Alpha\n  (Beta gamma)\n  delta\n)");
}
