//! Structural NOTA reader and codec.
//!
//! This crate is the hand-authored recursion floor for the schema-derived
//! stack. It recognizes NOTA delimiters, spans, atoms, and block structure
//! before any schema can be loaded. Its codec owns NOTA value shapes for Rust
//! values, while higher layers own schema type vocabulary, fields, imports,
//! declarations, and macros.

extern crate self as nota_next;

mod codec;
mod macros;
mod parser;

pub use codec::{
    NotaBlock, NotaBody, NotaBodyDecode, NotaBodyEncode, NotaBodyEncoding, NotaCollection,
    NotaDecode, NotaDecodeError, NotaDocumentBody, NotaDocumentDecode, NotaDocumentEncode,
    NotaDocumentEncoding, NotaEncode, NotaNamedBodyFieldDecode, NotaNamedBodyFieldEncode,
    NotaNamedDocumentFieldDecode, NotaNamedDocumentFieldEncode, NotaSource, NotaString,
};
pub use macros::{
    AtomCase, AtomShape, CaptureName, CapturedValue, DelimitedShape, MacroCandidate, MacroConflict,
    MacroDelimiter, MacroError, MacroMatch, MacroNodeDefinition, MacroObjectCount, MacroRegistry,
    Pattern, PatternElement, PositionPredicate, SigilPosition, SigilSpec, StructuralMacroError,
    StructuralMacroNode,
};
pub use nota_next_derive::{NotaDecode, NotaEncode};
pub use parser::{
    Atom, AtomClassification, Block, Delimiter, Document, NotaError, PipeText, SourcePosition,
    SourceSpan, StructureHeader, StructureShape, StructureSlot,
};
