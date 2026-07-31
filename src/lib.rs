//! Structural DOTOS reader and codec.
//!
//! This crate is the hand-authored recursion floor for the schema-derived
//! stack. It recognizes DOTOS delimiters, spans, atoms, and block structure
//! before any schema can be loaded. Its codec owns DOTOS value shapes for Rust
//! values, while higher layers own schema type vocabulary, fields, imports,
//! declarations, and macros.

extern crate self as dotos;

mod codec;
mod expectation;
mod instance_schema;
mod macros;
mod parser;
mod pretty;

pub use codec::{
    ByteSequence, DotosBlock, DotosBody, DotosBodyDecode, DotosBodyEncode, DotosBodyEncoding,
    DotosCollection, DotosDecode, DotosDecodeError, DotosDocumentBody, DotosDocumentDecode,
    DotosDocumentEncode, DotosDocumentEncoding, DotosEncode, DotosNamedBodyFieldDecode,
    DotosNamedBodyFieldEncode, DotosNamedDocumentFieldDecode, DotosNamedDocumentFieldEncode,
    DotosSource, DotosString, FixedByteSequence,
};
pub use dotos_derive::{DotosDecode, DotosDecodeTraced, DotosEncode, StructuralMacroNode};
pub use expectation::{DottedEntry, DottedExpectation};
pub use instance_schema::{
    DecodedWithSchema, DotosDecodeTraced, InstanceSchema, InstanceSchemaBody, TypeReference,
};
pub use macros::{
    AtomCase, AtomShape, BlockShape, CaptureName, CapturedValue, DelimitedShape, MacroCandidate,
    MacroConflict, MacroDelimiter, MacroError, MacroMatch, MacroNodeDefinition, MacroObjectCount,
    MacroRegistry, Pattern, PatternElement, PositionPredicate, SigilPosition, SigilSpec,
    StructuralMacroError, StructuralMacroNode, StructuralMacroNodeError, StructuralVariant,
    StructuralVariantConflict, StructuralVariantError, StructuralVariantSet,
};
pub use parser::{
    Atom, AtomClassification, Block, Delimiter, Document, DotosError, ParseMode, PipeText,
    SourcePosition, SourceSpan, StructureHeader, StructureShape, StructureSlot,
};
pub use pretty::{DotosOutputForm, PrettyLayout};
