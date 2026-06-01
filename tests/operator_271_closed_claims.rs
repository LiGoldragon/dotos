//! Architectural-truth witnesses for the closed claims in operator 271
//! `reports/operator/271-context-maintenance-current-state-2026-06-01.md`.
//!
//! Coverage in this file:
//! - Claim 2 — `FieldEncode` zero-sized method holder CLOSED
//!   (`f5906bae` made FieldEncode data-bearing).
//! - Claim 3 — `CodecDerive` single-field wrapper is the correct
//!   workspace-discipline answer (resolved-not-a-bug per designer 448).
//!
//! The witnesses are source-AST scans against the derive crate source.
//! The derive crate is a `proc-macro = true` library whose internals are
//! not callable from integration tests, so the witness shape is "read
//! the source, assert the expected shape is present, assert the retired
//! shape is absent."
//!
//! The companion behavioural witnesses for the derive surface live in
//! `tests/derive.rs` (round-trip behaviour of `NotaDecode` / `NotaEncode`).
//! Those tests prove the derive *works*; this file proves the derive's
//! internal nouns are shaped per workspace discipline.

const DERIVE_SOURCE: &str = include_str!("../derive/src/lib.rs");

/// Helper noun for source-AST sweep results. Owns the source string and
/// the assertion verbs the witnesses share.
struct DeriveSourceWitness<'source> {
    source: &'source str,
}

impl<'source> DeriveSourceWitness<'source> {
    fn new(source: &'source str) -> Self {
        Self { source }
    }

    /// Assert the source contains the exact line; fail with the line if absent.
    fn must_contain(&self, needle: &str, claim: &str) {
        assert!(
            self.source.contains(needle),
            "claim {claim}: derive source must contain {needle:?}"
        );
    }

    /// Assert the source does NOT contain the needle.
    fn must_not_contain(&self, needle: &str, claim: &str) {
        assert!(
            !self.source.contains(needle),
            "claim {claim}: derive source must not contain {needle:?}"
        );
    }
}

/// Claim 2 — `FieldEncode` is a data-bearing wrapper, not a ZST namespace.
/// The shape after `f5906bae`:
///
/// ```rust
/// struct FieldEncode<'field> {
///     field: &'field Field,
/// }
/// ```
///
/// The retired ZST shape `struct FieldEncode;` must be absent.
#[test]
fn field_encode_carries_field_data() {
    let witness = DeriveSourceWitness::new(DERIVE_SOURCE);

    // Positive shape — the data-bearing wrapper exists.
    witness.must_contain("struct FieldEncode<'field> {", "2");
    witness.must_contain("field: &'field Field,", "2");
    witness.must_contain("impl<'field> FieldEncode<'field>", "2");

    // The constructor takes a Field reference and binds it as the wrapper.
    witness.must_contain("fn new(field: &'field Field) -> Self", "2");
    witness.must_contain("Self { field }", "2");

    // body_named is a method on &self, NOT an associated function on a ZST.
    witness.must_contain("fn body_named(&self) -> Result<TokenStreamTwo, Error>", "2");
    witness.must_contain("self.field.ident.as_ref().expect(", "2");
    witness.must_contain(
        "FieldNotaAttributes::from_attributes(&self.field.attrs)?",
        "2",
    );

    // Negative witness — the ZST shape is gone.
    witness.must_not_contain("struct FieldEncode;", "2");
    witness.must_not_contain("fn body_named(field: &Field)", "2");
}

/// Claim 2 — Call sites use `FieldEncode::new(field).body_named()`, not
/// `FieldEncode::body_named(field)`. The call-site shape proves the
/// type is used as a value, not as a namespace.
#[test]
fn field_encode_call_sites_construct_and_dispatch() {
    let witness = DeriveSourceWitness::new(DERIVE_SOURCE);

    // The call site at struct-derive's named-fields branch maps each field
    // through `FieldEncode::new(field).body_named()` — the constructor +
    // method shape proves the wrapper is consumed as a value.
    witness.must_contain("FieldEncode::new(field).body_named()", "2");
    // The pre-fix shape would have been `FieldEncode::body_named(field)`.
    witness.must_not_contain("FieldEncode::body_named(", "2");
    // And the closure-free `.map(FieldEncode::body_named)` form would
    // resolve only if `body_named` were an associated function.
    witness.must_not_contain(".map(FieldEncode::body_named)", "2");
}

/// Claim 3 — `CodecDerive { input: DeriveInput }` is the correct
/// workspace-discipline answer to "the verb needs a noun and the natural
/// noun is foreign". Designer 448 + operator 269/270 converge on the
/// single-field wrapper around a `syn` type as legitimate.
///
/// The witness asserts the current shape and the four method placements
/// that pay the wrapper's keep: `new`, `expand_decode`, `expand_encode`,
/// and the inner `expand` that the entry points share.
#[test]
fn codec_derive_wraps_syn_derive_input_with_methods() {
    let witness = DeriveSourceWitness::new(DERIVE_SOURCE);

    // The current canonical shape.
    witness.must_contain("struct CodecDerive {", "3");
    witness.must_contain("input: DeriveInput,", "3");
    witness.must_contain("impl CodecDerive {", "3");

    // Constructor + the four methods that make the wrapper data-bearing.
    witness.must_contain("fn new(input: DeriveInput) -> Self", "3");
    witness.must_contain("Self { input }", "3");
    witness.must_contain("fn expand_decode(self) -> TokenStreamTwo", "3");
    witness.must_contain("fn expand_encode(self) -> TokenStreamTwo", "3");
    witness.must_contain(
        "fn expand(self, direction: CodecDirection) -> TokenStreamTwo",
        "3",
    );
}

/// Claim 3 — Both proc-macro entry points (`derive_nota_decode` and
/// `derive_nota_encode`) construct a `CodecDerive` from the parsed input
/// and dispatch the operation through the wrapper, not through a free
/// function. The wrapper IS the operation noun.
#[test]
fn codec_derive_is_constructed_by_both_proc_macro_entry_points() {
    let witness = DeriveSourceWitness::new(DERIVE_SOURCE);

    witness.must_contain("CodecDerive::new(input).expand_decode()", "3");
    witness.must_contain("CodecDerive::new(input).expand_encode()", "3");

    // No bare free-function form for the operation.
    witness.must_not_contain("fn expand_codec(", "3");
    witness.must_not_contain("pub fn expand_codec(", "3");
}

/// Claim 2 + 3 — Sibling wrapper `FieldDecode<'field>` MUST keep mirroring
/// the `FieldEncode<'field>` shape so the data-bearing rule is consistent
/// across the encode/decode pair. Operator 271 explicitly calls out the
/// mirror as the closure shape.
#[test]
fn field_decode_and_field_encode_mirror_each_other_as_data_bearing_wrappers() {
    let witness = DeriveSourceWitness::new(DERIVE_SOURCE);

    // Both wrappers carry a borrowed field reference.
    witness.must_contain("struct FieldDecode<'field>", "2+3");
    witness.must_contain("struct FieldEncode<'field>", "2+3");

    // Neither is a ZST namespace.
    witness.must_not_contain("struct FieldDecode;", "2+3");
    witness.must_not_contain("struct FieldEncode;", "2+3");
}

/// Claim 2 + 3 — the derive crate as a whole carries NO unit-struct method
/// holder pattern. Every wrapper carries data (the positive single-field
/// pattern per designer 448's six categories) or a marker carrying its own
/// state. The `pair-rule sweep` per `skills/architectural-truth-tests.md`
/// §"Pair-rule sweeps" requires both Sweep A (single-field wrappers — the
/// valid shape) and Sweep B (ZST namespace holders — the invalid shape) to
/// run in the same audit. This witness is Sweep B; the broader Sweep A
/// for the derive crate is the body of designer 448.
#[test]
fn derive_crate_carries_no_zst_method_holders() {
    let lines: Vec<&str> = DERIVE_SOURCE.lines().collect();
    let mut offenders: Vec<(usize, &str)> = Vec::new();

    for (offset, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        // Match `struct Name;` or `pub struct Name;` patterns at file scope.
        if trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") {
            let after_struct = trimmed
                .trim_start_matches("pub ")
                .trim_start_matches("struct ");
            // A unit struct declaration ends with `;` directly after the name.
            if let Some(name_end) = after_struct.find(';')
                && let Some(name_run_end) = after_struct[..name_end]
                    .find(|character: char| !character.is_ascii_alphanumeric() && character != '_')
                    .or(Some(name_end))
            {
                // If the character between the name and `;` is whitespace or
                // nothing, this IS a unit struct.
                let between = &after_struct[name_run_end..name_end];
                if between.chars().all(char::is_whitespace) {
                    offenders.push((offset + 1, line));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "claim 2+3: derive crate must not declare ZST namespace structs; found {offenders:?}"
    );
}
