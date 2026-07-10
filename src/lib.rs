//! Structural NOTA reader and codec.
//!
//! This crate is the hand-authored recursion floor for the schema-derived
//! stack. It recognizes NOTA delimiters, spans, atoms, and block structure
//! before any schema can be loaded. Its codec owns NOTA value shapes for Rust
//! values, while higher layers own schema type vocabulary, fields, imports,
//! declarations, and macros.

extern crate self as nota;

mod codec;
mod expectation;
mod instance_schema;
mod macros;
mod parser;

pub use codec::{
    ByteSequence, FixedByteSequence, NotaBlock, NotaBody, NotaBodyDecode, NotaBodyEncode,
    NotaBodyEncoding, NotaCollection, NotaDecode, NotaDecodeError, NotaDocumentBody,
    NotaDocumentDecode, NotaDocumentEncode, NotaDocumentEncoding, NotaEncode,
    NotaNamedBodyFieldDecode, NotaNamedBodyFieldEncode, NotaNamedDocumentFieldDecode,
    NotaNamedDocumentFieldEncode, NotaSource, NotaString,
};
pub use expectation::{DottedEntry, DottedExpectation};
pub use instance_schema::{
    DecodedWithSchema, InstanceSchema, InstanceSchemaBody, NotaDecodeTraced, TypeReference,
};
pub use macros::{
    AtomCase, AtomShape, BlockShape, CaptureName, CapturedValue, DelimitedShape, MacroCandidate,
    MacroConflict, MacroDelimiter, MacroError, MacroMatch, MacroNodeDefinition, MacroObjectCount,
    MacroRegistry, Pattern, PatternElement, PositionPredicate, SigilPosition, SigilSpec,
    StructuralMacroError, StructuralMacroNode, StructuralMacroNodeError, StructuralVariant,
    StructuralVariantConflict, StructuralVariantError, StructuralVariantSet,
};
pub use nota_derive::{NotaDecode, NotaDecodeTraced, NotaEncode, StructuralMacroNode};
pub use parser::{
    Atom, Block, Delimiter, Document, NotaError, PipeText, SourcePosition, SourceSpan,
    StructureHeader, StructureShape, StructureSlot,
};
