//! Structural NOTA reader and codec.
//!
//! This crate is the hand-authored recursion floor for the schema-derived
//! stack. It recognizes NOTA delimiters, spans, atoms, and block structure
//! before any schema can be loaded. Its codec owns NOTA value shapes for Rust
//! values, while higher layers own schema type vocabulary, fields, imports,
//! declarations, and macros.

mod codec;
mod parser;

pub use codec::{
    NotaBlock, NotaCollection, NotaDecode, NotaDecodeError, NotaEncode, NotaSource, NotaString,
};
pub use nota_next_derive::{NotaDecode, NotaEncode};
pub use parser::{
    Atom, AtomClassification, Block, Delimiter, Document, NotaError, PipeText, SourcePosition,
    SourceSpan, StructureHeader, StructureShape, StructureSlot,
};
