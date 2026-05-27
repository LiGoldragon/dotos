//! Structural NOTA reader.
//!
//! This crate is the hand-authored recursion floor for the schema-derived
//! stack. It recognizes NOTA delimiters, spans, atoms, and block structure
//! before any schema can be loaded. It deliberately does not interpret those
//! structures as schema types, fields, imports, or macros; higher layers own
//! that semantic lowering.

mod parser;

pub use parser::{
    Atom, AtomClassification, Block, Delimiter, Document, NotaError, PipeText, SourcePosition,
    SourceSpan, StructureHeader, StructureShape, StructureSlot,
};
