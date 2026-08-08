//! Expectation-driven dotted-prefix reading.
//!
//! DOTOS is strictly typed and positional, so the expected type is known at
//! every position and the reader advances by mode. A dotted prefix — a leading
//! atom split at its first period into a prefix and a value — is read only in
//! the two modes that expect one; in every other mode a period is ordinary atom
//! text. This is the single implementation of that mechanism, exported so
//! downstream readers (schema-language above all) reuse it rather than
//! hand-rolling their own dotted-prefix reading.

use crate::codec::DotosDecodeError;
use crate::parser::{Atom, Block};

/// The three positions at which a reader may split a dotted prefix off a leading
/// atom. The split algorithm — divide the leading atom at its first top-level
/// period — is shared; the kinds differ in the head form each accepts, which is
/// how the reader catches a dotted prefix used in the wrong place while allowing
/// generic map keys to retain any parser-valid bare-atom spelling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DottedExpectation {
    /// A capitalized head naming a type or generic application, as in
    /// `Vector.X` or `Map.(Key Value)`.
    Capitalized,
    /// A leading lowercase name segment: map keys, import path segments, and
    /// field disambiguators.
    Uncapitalized,
    /// One parser-valid bare atom, irrespective of its first character. This
    /// is the generic map-key form: `/`, `/boot`, and punctuation-bearing keys
    /// remain dotted heads without being mistaken for identifier-like names.
    AnyBareAtom,
}

impl DottedExpectation {
    /// Human-readable name of this expectation for diagnostics.
    pub fn description(self) -> &'static str {
        match self {
            Self::Capitalized => "capitalized dotted prefix",
            Self::Uncapitalized => "uncapitalized dotted prefix",
            Self::AnyBareAtom => "bare-atom dotted prefix",
        }
    }

    fn accepts_head(self, prefix: &str) -> bool {
        match prefix.chars().next() {
            Some(first) => match self {
                Self::Capitalized => first.is_ascii_uppercase(),
                Self::Uncapitalized => first.is_ascii_lowercase(),
                Self::AnyBareAtom => true,
            },
            None => false,
        }
    }

    /// Read one dotted entry from the head of a block sequence under this
    /// expectation. The leading block must be a dot-application `key.value`
    /// whose head is an atom accepted by this expectation. The key is that head atom
    /// and the value is the application's payload — which may itself be a
    /// nested application when the value is dotted (`key.a.b.c`), since the raw
    /// grammar binds the period right-associatively. A dotted entry is always
    /// exactly one application block, so exactly one block is consumed; the
    /// former inline-versus-following-block split no longer exists now that the
    /// raw parser binds the period.
    pub fn read_entry(self, blocks: &[Block]) -> Result<DottedEntry, DotosDecodeError> {
        let block = blocks
            .first()
            .ok_or(DotosDecodeError::ExpectedDottedEntry {
                expectation: self.description(),
            })?;
        let (head, payload) =
            block
                .as_application()
                .ok_or(DotosDecodeError::ExpectedDottedEntry {
                    expectation: self.description(),
                })?;
        let key_atom = head.atom().ok_or(DotosDecodeError::ExpectedDottedEntry {
            expectation: self.description(),
        })?;
        if !self.accepts_head(key_atom.text()) {
            return Err(DotosDecodeError::DottedEntryCaseMismatch {
                expectation: self.description(),
                prefix: key_atom.text().to_owned(),
            });
        }
        Ok(DottedEntry {
            key: head.clone(),
            value: payload.clone(),
            consumed: 1,
        })
    }

    /// Read one dotted entry from a single already-extracted string under this
    /// expectation. This is the string-level entry form of the same mechanism
    /// as [`read_entry`](Self::read_entry): the same split-at-first-top-level-dot
    /// rule (shared through `Atom::split_text_at_first_dot`), the same
    /// per-kind head-case enforcement, and the same typed errors, returning the
    /// key and value as string slices. It serves a consumer that already holds
    /// an atom's text — macro-expanded schema-language payloads above all —
    /// rather than a block sequence, so it routes through the shared mechanism
    /// instead of re-deriving a local `split_once`. The whole value is carried
    /// inline after the period, since a lone string has no following block to
    /// supply it; a string that ends at the period is a missing value. Meaning
    /// is still expectation-driven: the caller declares the kind and nothing
    /// scans the text to decide it.
    pub fn read_string_entry(self, text: &str) -> Result<(&str, &str), DotosDecodeError> {
        let (prefix, remainder) =
            Atom::split_text_at_first_dot(text).ok_or(DotosDecodeError::ExpectedDottedEntry {
                expectation: self.description(),
            })?;
        if !self.accepts_head(prefix) {
            return Err(DotosDecodeError::DottedEntryCaseMismatch {
                expectation: self.description(),
                prefix: prefix.to_owned(),
            });
        }
        let value = remainder.ok_or(DotosDecodeError::DottedEntryMissingValue {
            expectation: self.description(),
        })?;
        Ok((prefix, value))
    }
}

/// One dotted entry read under a [`DottedExpectation`]: the key block split from
/// the leading atom's prefix, the value block, and how many source blocks the
/// entry consumed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DottedEntry {
    key: Block,
    value: Block,
    consumed: usize,
}

impl DottedEntry {
    /// The key block — the atom split from the leading prefix.
    pub fn key(&self) -> &Block {
        &self.key
    }

    /// The value block — the inline remainder atom or the following block.
    pub fn value(&self) -> &Block {
        &self.value
    }

    /// How many source blocks this entry consumed: one when the value stayed
    /// inline in the leading atom, two when the value is the following block.
    pub fn consumed(&self) -> usize {
        self.consumed
    }
}
